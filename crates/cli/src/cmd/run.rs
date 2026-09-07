//! `nrmeasure run`: execute a measurement, stamp it with the full D-14 environment
//! snapshot, and write a run directory the provenance gate accepts.
//!
//! Order of operations (the whole point of this command): resolve tool paths and
//! fail early if one is missing; assert every D-06 precondition and refuse before
//! touching anything if one is violated; snapshot interference before the tools
//! run; execute `cyclictest` (and `hwlatdetect` if requested, never concurrently);
//! parse and reconcile the raw captures; snapshot interference again and render the
//! D-15/D-24 verdict (D-24's tail metrics are computed from the just-parsed
//! histogram, which is why parsing now happens before this step rather than after
//! it); snapshot the D-14 environment; create the run directory and place the
//! captures in unmodified; assemble, validate and write `manifest.json`; render and
//! write `REPORT.md`.
//!
//! `--allow-precondition-violation` (D-17) is the one narrow exception to "refuse
//! before touching anything if one is violated" above: accepted only for
//! `--class calibration-contaminated`, it waives the refusal alone. Every one of
//! the 15 precondition results is still recorded with its real observed value, and
//! the resulting manifest always has `excluded_from_series` forced to `true`. See
//! [`execute`] and [`precondition_waiver_reason`].

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, ValueEnum};
use nr_capture::sources::{FixtureFacts, SystemFacts, parse_cpu_list};
use nr_capture::{environment, hwnoise, interference, preconditions, smi};
use nr_histogram::hist::{CyclictestRun, parse_hist_file};
use nr_histogram::json::{parse_json_file, reconcile};
use nr_manifest::{
    AdmissionDisposition, AdmissionEvidence, AdmissionEvidenceSource, ArtifactKind,
    ArtifactPathMapping, ArtifactRecord, AttemptFailure, AttemptRecord, AttemptStatus,
    GitShaSource, HarnessInfo, InstrumentClass, PreconditionResult, PreconditionStatus,
    ProvenanceTier, RequestedRun, RunClass, RunManifest, SeriesAdmission, StorageLocation,
    ThermalProfile, ToolInvocation,
};
use nr_metrics::report::render_run_report;
use time::OffsetDateTime;

use crate::rundir::{self, RunDir};
use crate::tools::{self, CYCLICTEST_PATH_ENV, HWLATDETECT_PATH_ENV, RTLA_PATH_ENV};

/// A fixture-backed facts source is for tests only. A run that would feed the
/// headline series must read the real machine or it is not a measurement.
const FACTS_FIXTURE_ENV: &str = "NRMEASURE_FACTS_FIXTURE";

/// Test-only companion to [`FACTS_FIXTURE_ENV`]: `interference::snapshot` (unlike
/// the `SystemFacts` trait) reads `/proc/interrupts` directly and is Linux-gated
/// with no facts-source seam of its own (a difference from this plan's own
/// `<interfaces>` block; see the plan summary). Points at a `/proc/interrupts`
/// shaped text file, reused for both the before and after snapshot in a test.
const INTERRUPTS_FIXTURE_ENV: &str = "NRMEASURE_INTERRUPTS_FIXTURE";

/// Third and last of the fixture-text seams: stands in for a live `MSR_SMI_COUNT` read,
/// read once and reused for both the before and after snapshot of a run, exactly like
/// [`INTERRUPTS_FIXTURE_ENV`]. Not a fake-binary path like `RTLA_PATH_ENV`: reading a
/// register needs no realistic subprocess output to fake, only the value itself.
const SMI_FIXTURE_ENV: &str = "NRMEASURE_SMI_FIXTURE";

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum RunClassArg {
    Recon,
    Screen,
    CalibrationClean,
    CalibrationContaminated,
    Investigation,
    Headline,
    Weekly,
    Soak,
}

impl From<RunClassArg> for RunClass {
    fn from(value: RunClassArg) -> Self {
        match value {
            RunClassArg::Recon => RunClass::Recon,
            RunClassArg::Screen => RunClass::Screen,
            RunClassArg::CalibrationClean => RunClass::CalibrationClean,
            RunClassArg::CalibrationContaminated => RunClass::CalibrationContaminated,
            RunClassArg::Investigation => RunClass::Investigation,
            RunClassArg::Headline => RunClass::Headline,
            RunClassArg::Weekly => RunClass::Weekly,
            RunClassArg::Soak => RunClass::Soak,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum InstrumentClassArg {
    HeadlineSeries,
    Investigation,
}

impl From<InstrumentClassArg> for InstrumentClass {
    fn from(value: InstrumentClassArg) -> Self {
        match value {
            InstrumentClassArg::HeadlineSeries => InstrumentClass::HeadlineSeries,
            InstrumentClassArg::Investigation => InstrumentClass::Investigation,
        }
    }
}

/// `hot-screen` exempts the run from `ThermalHeadroomAtStart` and is accepted only for
/// `--class screen` (see the early guard in `execute`); every other class must declare
/// `normal`, the default. Finding 5 of `01-EXTERNAL-AUDIT.md`.
#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ThermalProfileArg {
    Normal,
    HotScreen,
}

impl From<ThermalProfileArg> for ThermalProfile {
    fn from(value: ThermalProfileArg) -> Self {
        match value {
            ThermalProfileArg::Normal => ThermalProfile::Normal,
            ThermalProfileArg::HotScreen => ThermalProfile::HotScreen,
        }
    }
}

#[derive(ClapArgs, Debug, Clone)]
pub struct Args {
    /// Rig identifier, e.g. precision3591.
    #[arg(long)]
    pub rig_slug: String,

    /// Run cadence class.
    #[arg(long, value_enum)]
    pub class: RunClassArg,

    /// Instrument class: the published headline series or an investigation run.
    #[arg(long, value_enum, default_value = "headline-series")]
    pub instrument: InstrumentClassArg,

    /// The measurement CPU list, e.g. 6-11.
    #[arg(long)]
    pub cpus: String,

    /// The cyclictest main thread affinity, e.g. 0,1.
    #[arg(long)]
    pub main_cpus: String,

    /// Measurement duration in seconds.
    #[arg(long)]
    pub duration: u64,

    /// cyclictest sampling interval in microseconds.
    #[arg(long, default_value_t = 200)]
    pub interval: u64,

    /// cyclictest histogram bound in microseconds.
    #[arg(long, default_value_t = 400)]
    pub histogram_max: u64,

    /// cyclictest SCHED_FIFO priority.
    #[arg(long, default_value_t = 99)]
    pub priority: u32,

    /// Also run hwlatdetect after cyclictest (D-09). Default off.
    #[arg(long)]
    pub with_hwlatdetect: bool,

    /// hwlatdetect duration in seconds.
    #[arg(long, default_value_t = 900)]
    pub hwlatdetect_duration: u64,

    /// Restricts hwlatdetect's sampling to this CPU list (hwlatdetect's own
    /// `--cpu-list`), e.g. `0-11` for a P-core-only D-18 arm. Ignored unless
    /// `--with-hwlatdetect` is also given. Default (unset) samples every CPU, hwlatdetect's
    /// own default.
    ///
    /// `hwlatdetect`'s tracer runs a single non-migrating thread that `isolcpus` keeps off
    /// the isolated cores entirely on this kernel, so no `--cpu-list` value makes it able to
    /// characterise cpus 6 to 11; see `docs/rig/firmware-floor-rt-vs-stock.md`. Use
    /// `--with-hwnoise` for a figure about those cores.
    #[arg(long)]
    pub hwlatdetect_cpu_list: Option<String>,

    /// Take a firmware screen with `rtla hwnoise` after cyclictest. One osnoise sampling
    /// thread per CPU in --hwnoise-cpus, which is why this differs from --with-hwlatdetect:
    /// hwlatdetect's tracer runs a single non-migrating thread that isolcpus keeps off the
    /// isolated cores entirely. See docs/rig/firmware-floor-rt-vs-stock.md. Mutually
    /// exclusive with `--with-hwlatdetect`.
    #[arg(long)]
    pub with_hwnoise: bool,

    /// The CPUs rtla hwnoise samples, one thread each, e.g. 6-11. Defaults to --cpus.
    #[arg(long)]
    pub hwnoise_cpus: Option<String>,

    /// Where rtla's own control threads run, kept off the measured cores, e.g. 0-5.
    #[arg(long, default_value = "0-5")]
    pub hwnoise_housekeeping: String,

    /// rtla hwnoise session duration in seconds.
    #[arg(long, default_value_t = 900)]
    pub hwnoise_duration: u64,

    /// rtla's -P priority spec. f:99 is SCHED_FIFO 99, matching the cyclictest priority.
    #[arg(long, default_value = "f:99")]
    pub hwnoise_priority: String,

    /// Optional cyclictest breaktrace threshold in microseconds; also adds
    /// --tracemark.
    #[arg(long)]
    pub breaktrace: Option<u64>,

    /// Root directory for run directories.
    #[arg(long, default_value = "./measurements")]
    pub measurements_root: PathBuf,

    /// Contamination threshold configuration path.
    #[arg(long, default_value = "./config/contamination-thresholds.json")]
    pub thresholds: PathBuf,

    /// Free-text note recorded in the manifest.
    #[arg(long)]
    pub note: Option<String>,

    /// Assert preconditions and print what would run; write nothing.
    #[arg(long)]
    pub dry_run: bool,

    /// Waives the D-06 refusal for a precondition violation. Accepted ONLY when
    /// `--class` is `calibration-contaminated`; every other run class is
    /// rejected outright, before anything else is checked (see `execute`). The
    /// preconditions are still evaluated in full and every one of the 15
    /// results is still recorded in the manifest with its real observed value:
    /// this flag waives the refusal, never the assertion or the record. A run
    /// taken with this flag always has `excluded_from_series` forced to `true`
    /// with a stated reason; the operator cannot override that.
    #[arg(long)]
    pub allow_precondition_violation: bool,

    /// What this run declares about its own thermal intent, before it starts.
    /// `hot-screen` exempts the run from `ThermalHeadroomAtStart`, because a firmware
    /// screen saturates the machine on purpose; it is accepted ONLY when `--class` is
    /// `screen`, rejected outright otherwise (see `execute`). The exemption follows
    /// this declaration, never the observed temperature: a `normal`-profile run (the
    /// default) is refused for starting hot regardless of its run class. Finding 5 of
    /// `01-EXTERNAL-AUDIT.md`.
    #[arg(long, value_enum, default_value = "normal")]
    pub thermal_profile: ThermalProfileArg,
}

/// Everything `run` would otherwise read from the environment: tool paths and the
/// two test-only fixture seams. Bundled into one struct and resolved exactly once
/// (in [`Overrides::from_env`]) so the orchestration itself (`execute`) never
/// touches `std::env` and unit tests never need to mutate process-global state to
/// exercise it. Reading an environment variable is safe; the 2024 edition made
/// `std::env::set_var`/`remove_var` `unsafe fn`, and this workspace forbids
/// `unsafe_code` outright, so tests construct an `Overrides` value directly
/// instead.
struct Overrides {
    facts_fixture_path: Option<PathBuf>,
    interrupts_fixture_path: Option<PathBuf>,
    smi_fixture_path: Option<PathBuf>,
    cyclictest_path: PathBuf,
    hwlatdetect_path: PathBuf,
    rtla_path: PathBuf,
}

impl Overrides {
    fn from_env() -> Self {
        Overrides {
            facts_fixture_path: std::env::var(FACTS_FIXTURE_ENV).ok().map(PathBuf::from),
            interrupts_fixture_path: std::env::var(INTERRUPTS_FIXTURE_ENV)
                .ok()
                .map(PathBuf::from),
            smi_fixture_path: std::env::var(SMI_FIXTURE_ENV).ok().map(PathBuf::from),
            cyclictest_path: tools::resolve_tool_path(CYCLICTEST_PATH_ENV, "cyclictest"),
            hwlatdetect_path: tools::resolve_tool_path(HWLATDETECT_PATH_ENV, "hwlatdetect"),
            rtla_path: tools::resolve_tool_path(RTLA_PATH_ENV, "rtla"),
        }
    }
}

pub fn run(args: Args) -> Result<i32> {
    execute(args, &Overrides::from_env())
}

/// Where in the pipeline an attempt failed, recorded in `ATTEMPT.json` verbatim.
/// Exhaustive: every fallible step from the first tool spawn to the final manifest
/// write is attributed to exactly one of these seven. Finding 7 of
/// `01-EXTERNAL-AUDIT.md`.
#[derive(Clone, Copy, Debug)]
enum Stage {
    /// A tool's process could not be spawned at all (`tools::run_tool` itself
    /// returned `Err`).
    ToolSpawn,
    /// Turning a tool's own output into something on disk once it has exited:
    /// writing its stderr sidecar, writing hwlatdetect's raw stdout, or
    /// checksumming a capture that is already sitting in the run directory. Also
    /// covers the interference snapshot bracketing each tool, since that snapshot
    /// exists solely to feed the verdict below.
    ToolExit,
    Parse,
    Reconcile,
    Verdict,
    EnvironmentSnapshot,
    ManifestWrite,
}

impl Stage {
    fn as_str(self) -> &'static str {
        match self {
            Stage::ToolSpawn => "tool-spawn",
            Stage::ToolExit => "tool-exit",
            Stage::Parse => "parse",
            Stage::Reconcile => "reconcile",
            Stage::Verdict => "verdict",
            Stage::EnvironmentSnapshot => "environment-snapshot",
            Stage::ManifestWrite => "manifest-write",
        }
    }
}

/// Carries which [`Stage`] failed, the underlying error, and every tool invocation
/// recorded so far, so the caller can rewrite `ATTEMPT.json` with a named stage
/// and an accurate `tools` list rather than a bare message.
struct StagedError {
    stage: Stage,
    error: anyhow::Error,
    tools: Vec<ToolInvocation>,
}

/// Attaches a [`Stage`] (and a snapshot of the tools invoked so far) to a fallible
/// step's `Result`, for `?`-based propagation out of the staged pipeline closure in
/// [`execute`].
trait StageExt<T> {
    fn stage(self, stage: Stage, tools: &[ToolInvocation]) -> Result<T, StagedError>;
}

impl<T, E> StageExt<T> for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn stage(self, stage: Stage, tools: &[ToolInvocation]) -> Result<T, StagedError> {
        self.map_err(|error| StagedError {
            stage,
            error: error.into(),
            tools: tools.to_vec(),
        })
    }
}

fn execute(args: Args, overrides: &Overrides) -> Result<i32> {
    let run_class: RunClass = args.class.into();
    let instrument_class: InstrumentClass = args.instrument.into();
    let thermal_profile: ThermalProfile = args.thermal_profile.into();

    // D-06/D-17: checked first and unconditionally, ahead of every other check
    // below (including the fixture-facts guard immediately following), so the
    // rejection is uniform regardless of what else the invocation set. A hole
    // here is a hole in PLAT-02: without this restriction the flag would let a
    // headline run be published with its preconditions silently waived.
    if args.allow_precondition_violation && !matches!(run_class, RunClass::CalibrationContaminated)
    {
        anyhow::bail!(
            "--allow-precondition-violation is accepted only for --class \
             calibration-contaminated (got {run_class:?}): this flag exists to take the D-17 \
             deliberately contaminated calibration arm without lying to the harness, and must \
             never be available to waive preconditions on a publishable run"
        );
    }

    // Finding 5 of `01-EXTERNAL-AUDIT.md`: the same shape of guard, for the same
    // reason. A headline run must never be able to declare itself thermally exempt.
    if matches!(thermal_profile, ThermalProfile::HotScreen)
        && !matches!(run_class, RunClass::Screen)
    {
        anyhow::bail!(
            "--thermal-profile hot-screen is accepted only for --class screen (got \
             {run_class:?}): this profile exempts a run from ThermalHeadroomAtStart because a \
             firmware screen saturates the machine on purpose, and must never be available to \
             declare a publishable run thermally exempt"
        );
    }

    // Two firmware instruments in one run would either interleave or serialise, and either
    // way the second one's window is measured under conditions the first one created (the
    // same reason cyclictest and hwlatdetect never run concurrently). The message names both
    // flags so an operator taking two runs instead knows exactly what to split.
    if args.with_hwnoise && args.with_hwlatdetect {
        anyhow::bail!(
            "--with-hwnoise and --with-hwlatdetect are mutually exclusive: a run takes at \
             most one firmware screen. Take two runs instead."
        );
    }

    if overrides.facts_fixture_path.is_some()
        && matches!(
            run_class,
            RunClass::Headline | RunClass::Weekly | RunClass::Soak
        )
    {
        anyhow::bail!(
            "NRMEASURE_FACTS_FIXTURE cannot be used with run class {run_class:?}: a run that \
             would feed the headline series must read the real machine or it is not a \
             measurement"
        );
    }

    // Matches the facts-fixture refusal above: one interrupts fixture text is read
    // for both the before and the after interference snapshot, so every delta
    // computed from it is exactly zero, which reads as a perfectly quiet machine
    // rather than as a value that was never measured. Finding 8 of
    // `01-EXTERNAL-AUDIT.md`.
    if overrides.interrupts_fixture_path.is_some()
        && matches!(
            run_class,
            RunClass::Headline | RunClass::Weekly | RunClass::Soak
        )
    {
        anyhow::bail!(
            "{INTERRUPTS_FIXTURE_ENV} cannot be used with run class {run_class:?}: one fixture \
             text is read for every snapshot, so every interference delta it produces is zero. \
             A run that would feed the headline series must read the real machine or it is not \
             a measurement"
        );
    }

    let target_cpus = parse_cpu_list(&args.cpus);

    // Which fixture seams (if any) are driving this run, computed once so the
    // manifest's `fixtures_used` field and the forced-exclusion reason below always
    // agree with each other.
    let fixtures_used = fixtures_used_names(overrides);

    // Step 1: resolve tool paths and versions, and load every input the run will need at
    // the end. Fail early if any of it is missing.
    //
    // The thresholds file is loaded HERE rather than at step 6 where it is used. It was
    // originally loaded at the point of use, which meant a missing or unreadable file was
    // discovered only after the measurement had already run: on 2026-09-01 a full one-hour
    // calibration run completed and was then discarded because this path did not resolve
    // under systemd (whose working directory is `/`, not the repository root). Anything the
    // run cannot finish without belongs in this step, next to the tool checks.
    let thresholds = interference::Thresholds::load(&args.thresholds).with_context(|| {
        format!(
            "failed to load thresholds from {}; it is read before the measurement starts so a \
             bad path costs a second rather than the whole run. Pass --thresholds with an \
             absolute path when running outside the repository root, for example under systemd",
            args.thresholds.display()
        )
    })?;

    let cyclictest_path = &overrides.cyclictest_path;
    let cyclictest_version =
        tools::resolve_and_verify("cyclictest", cyclictest_path, &["--version"])
            .context("cyclictest is required to take a measurement")?;

    let hwlatdetect_path = &overrides.hwlatdetect_path;
    let hwlatdetect_version = if args.with_hwlatdetect {
        Some(
            tools::resolve_and_verify("hwlatdetect", hwlatdetect_path, &["--version"])
                .context("hwlatdetect was requested with --with-hwlatdetect")?,
        )
    } else {
        None
    };

    // rtla has no --version flag; its build number appears in `rtla hwnoise --help`'s first
    // line instead (docs/rig/recon-2026-09-05/FINDINGS.md), so that is what this project
    // probes for a version string.
    let rtla_path = &overrides.rtla_path;
    let rtla_version = if args.with_hwnoise {
        Some(
            tools::resolve_and_verify("rtla", rtla_path, &["hwnoise", "--help"])
                .context("rtla is required to take a firmware screen with --with-hwnoise")?,
        )
    } else {
        None
    };

    // Step 2: build facts, run every precondition, refuse before touching
    // anything on a violation.
    let facts = build_facts(overrides.facts_fixture_path.as_deref())?;
    let spec = preconditions::PreconditionSpec {
        instrument_class: instrument_class.clone(),
        run_class: run_class.clone(),
        target_cpus: target_cpus.clone(),
        thermal_profile: thermal_profile.clone(),
    };
    let results: Vec<PreconditionResult> = preconditions::run_all(facts.as_ref(), &spec);
    if let Err(refusal) = preconditions::refuse_on_violation(&results, &instrument_class) {
        if args.allow_precondition_violation {
            // The class restriction above guarantees run_class is
            // CalibrationContaminated here. Only the refusal is waived: every
            // result in `results` still flows into the manifest unchanged below,
            // and excluded_from_series is forced true regardless (see
            // precondition_waiver_reason).
            eprintln!(
                "warning: proceeding despite {} precondition violation(s) because \
                 --allow-precondition-violation was given for this calibration-contaminated \
                 run; every result is still recorded and this run is forced \
                 excluded_from_series:\n{refusal}",
                refusal.offenses.len()
            );
        } else {
            eprintln!("{refusal}");
            return Ok(2);
        }
    }

    if args.dry_run {
        print_dry_run(&args, target_cpus.len());
        return Ok(0);
    }

    let utc_start = OffsetDateTime::now_utc();
    let interrupts_fixture = match &overrides.interrupts_fixture_path {
        Some(path) => Some(std::fs::read_to_string(path).with_context(|| {
            format!(
                "failed to read {INTERRUPTS_FIXTURE_ENV} at {}",
                path.display()
            )
        })?),
        None => None,
    };
    let smi_fixture =
        match &overrides.smi_fixture_path {
            Some(path) => Some(std::fs::read_to_string(path).with_context(|| {
                format!("failed to read {SMI_FIXTURE_ENV} at {}", path.display())
            })?),
            None => None,
        };

    // Step 3: create the run directory and its ATTEMPT.json immediately, before
    // either instrument runs. Nothing in the directory name depends on the
    // measurement. This used to happen only after parsing, reconciliation and the
    // contamination verdict had all succeeded (step 9 below, historically), with
    // raw output sitting in a scratch `TempDir` (tempfile's `tempdir()` helper)
    // until then: any failure among those steps dropped it and deleted the
    // capture, leaving an hour of rig time surviving only as a line on stderr.
    // Finding 7 of `01-EXTERNAL-AUDIT.md`. There is no code path left, from here
    // on, that can discard a capture: every failure below rewrites ATTEMPT.json
    // instead.
    let run_dir = RunDir::create(
        &args.measurements_root,
        &args.rig_slug,
        &run_class,
        utc_start,
    )
    .context("failed to create the run directory")?;

    // Computed once and reused everywhere an argv token or executed_path is redacted
    // below. Deliberately not just `$HOME`: `scripts/nr-run-measurement` launches this
    // binary as a root `systemd-run` transient unit with no `User=`/`PAMName=`, which
    // does not reliably set `HOME` to the operator's real home directory, so relying on
    // it alone let `/home/<user>/...` leak into three committed manifests before this
    // was caught (plan 01-23, T-1-95). See `redaction_prefixes`.
    let redact_prefixes = redaction_prefixes(&run_dir.path);

    // Computed once and reused for every ATTEMPT.json write and the eventual
    // manifest: neither changes mid-run, and recomputing harness_info() again
    // later would re-hash the executable for no reason.
    let harness = harness_info();
    let requested = RequestedRun {
        run_class: run_class.clone(),
        instrument_class: instrument_class.clone(),
        cpus: args.cpus.clone(),
        main_cpus: args.main_cpus.clone(),
        duration_seconds: args.duration,
        with_hwlatdetect: args.with_hwlatdetect,
        hwlatdetect_duration_seconds: args.with_hwlatdetect.then_some(args.hwlatdetect_duration),
    };

    write_attempt(
        &run_dir.path,
        &AttemptRecord {
            schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
            run_id: run_dir.run_id.clone(),
            utc_start,
            utc_end: None,
            status: AttemptStatus::InProgress,
            harness: harness.clone(),
            requested: requested.clone(),
            tools: Vec::new(),
            preserved: Vec::new(),
            failure: None,
            usable_for_numerical_analysis: false,
        },
    )
    .context("failed to write the initial ATTEMPT.json")?;

    // Step 4: the pre-run thermal reading, before either instrument runs, so the
    // manifest's temp_c_start is genuinely the start. The D-14 snapshot at step 9
    // runs after both instruments finish and supplies temp_c_end; reading the
    // zones only there put the end temperature in the start field.
    let thermal_start = nr_capture::sources::discover_thermal_zones_c(facts.as_ref());

    // Every fallible step from here (the first interference snapshot) through the
    // manifest write is staged: on an `Err`, the match below rewrites ATTEMPT.json
    // with the failing stage, the verbatim error, every tool invoked so far, and
    // usable_for_numerical_analysis: false, then returns a non-zero exit code. The
    // directory is never deleted and manifest.json is never written on this path.
    let attempt_result: Result<(RunManifest, CyclictestRun), StagedError> = (|| {
        let mut tool_invocations: Vec<ToolInvocation> = Vec::new();
        let mut artifacts: Vec<ArtifactRecord> = Vec::new();
        let mut run_bytes = 0u64;
        // One entry per instrument, in execution order: each brackets exactly one
        // `tools::run_tool` call with its own before/after snapshot and a
        // monotonic elapsed time, with nothing else between the two snapshots.
        // Finding 7 of `01-EXTERNAL-AUDIT.md`.
        let mut windows: Vec<nr_manifest::InstrumentWindow> = Vec::new();
        // Paired with each placed artifact's executed path, so the argv
        // finalization below can name which argv element became which artifact
        // without re-deriving either.
        let mut placed_paths: Vec<(String, &'static str)> = Vec::new();
        // Every firmware screen this run takes, in execution order. At most one entry today
        // (--with-hwnoise and --with-hwlatdetect's own hwlatdetect.txt artifact are mutually
        // exclusive), but a Vec because a manifest may carry more than one over time.
        let mut firmware_screens: Vec<nr_manifest::FirmwareScreen> = Vec::new();

        // MSR_SMI_COUNT (D-27) brackets the whole run, immediately before the first
        // interference snapshot and (below) immediately after the last instrument's window
        // closes: a provenance field independent of which instrument, if any, also ran.
        // Never fallible at this call site: an unreadable register is a stated reason inside
        // the returned tuple, not an Err that would abort a run over a provenance field.
        let (smi_before, smi_before_reason) =
            take_smi_snapshot(&target_cpus, smi_fixture.as_deref());

        // Step 5: execute cyclictest, writing its output directly into the run
        // directory. There is no scratch directory left to write into: the run
        // directory already exists, and nothing needs protecting from it.
        let hist_path = run_dir.path.join("cyclictest.hist");
        let json_path = run_dir.path.join("cyclictest.json");
        let cyclictest_argv = build_cyclictest_argv(
            &args,
            target_cpus.len(),
            &hist_path.display().to_string(),
            &json_path.display().to_string(),
        );
        let cyclictest_before =
            take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())
                .context("failed to snapshot interference before cyclictest")
                .stage(Stage::Verdict, &tool_invocations)?;
        let cyclictest_start = Instant::now();
        let cyclictest_output = tools::run_tool(
            "cyclictest",
            cyclictest_path,
            &cyclictest_version,
            cyclictest_argv,
        )
        .context("failed to execute cyclictest")
        .stage(Stage::ToolSpawn, &tool_invocations)?;
        let cyclictest_elapsed = cyclictest_start.elapsed();
        let cyclictest_after =
            take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())
                .context("failed to snapshot interference after cyclictest")
                .stage(Stage::Verdict, &tool_invocations)?;

        warn_on_tool_failure(&cyclictest_output);
        tool_invocations.push(cyclictest_output.invocation);
        if let Some(record) = capture_stderr_sidecar(
            &run_dir.path,
            "cyclictest",
            &cyclictest_output.stderr,
            &mut run_bytes,
        )
        .stage(Stage::ToolExit, &tool_invocations)?
        {
            artifacts.push(record);
        }
        windows.push(nr_manifest::InstrumentWindow {
            instrument: "cyclictest".to_string(),
            elapsed_seconds: cyclictest_elapsed.as_secs_f64(),
            requested_seconds: Some(args.duration),
            delta: window_delta(&cyclictest_before, &cyclictest_after),
            before: cyclictest_before,
            after: cyclictest_after,
        });

        // Step 6: execute hwlatdetect next, never concurrently, if requested (D-09).
        if args.with_hwlatdetect {
            let version = hwlatdetect_version.expect("captured above when with_hwlatdetect is set");
            let hwlatdetect_argv = build_hwlatdetect_argv(&args);
            let hwlatdetect_before =
                take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())
                    .context("failed to snapshot interference before hwlatdetect")
                    .stage(Stage::Verdict, &tool_invocations)?;
            let hwlatdetect_start = Instant::now();
            let hwlatdetect_output =
                tools::run_tool("hwlatdetect", hwlatdetect_path, &version, hwlatdetect_argv)
                    .context("failed to execute hwlatdetect")
                    .stage(Stage::ToolSpawn, &tool_invocations)?;
            let hwlatdetect_elapsed = hwlatdetect_start.elapsed();
            let hwlatdetect_after =
                take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())
                    .context("failed to snapshot interference after hwlatdetect")
                    .stage(Stage::Verdict, &tool_invocations)?;

            warn_on_tool_failure(&hwlatdetect_output);
            tool_invocations.push(hwlatdetect_output.invocation);
            if let Some(record) = capture_stderr_sidecar(
                &run_dir.path,
                "hwlatdetect",
                &hwlatdetect_output.stderr,
                &mut run_bytes,
            )
            .stage(Stage::ToolExit, &tool_invocations)?
            {
                artifacts.push(record);
            }

            let hwlat_path = run_dir.path.join("hwlatdetect.txt");
            std::fs::write(&hwlat_path, &hwlatdetect_output.stdout)
                .context("failed to write hwlatdetect.txt")
                .stage(Stage::ToolExit, &tool_invocations)?;
            artifacts.push(
                place_artifact(
                    &hwlat_path,
                    &run_dir.path,
                    "hwlatdetect.txt",
                    ArtifactKind::HwlatdetectText,
                    &mut run_bytes,
                )
                .stage(Stage::ToolExit, &tool_invocations)?,
            );
            placed_paths.push((hwlat_path.display().to_string(), "hwlatdetect.txt"));
            windows.push(nr_manifest::InstrumentWindow {
                instrument: "hwlatdetect".to_string(),
                elapsed_seconds: hwlatdetect_elapsed.as_secs_f64(),
                requested_seconds: Some(args.hwlatdetect_duration),
                delta: window_delta(&hwlatdetect_before, &hwlatdetect_after),
                before: hwlatdetect_before,
                after: hwlatdetect_after,
            });
        } else if args.with_hwnoise {
            let version = rtla_version
                .clone()
                .expect("captured above when with_hwnoise is set");
            let hwnoise_cpus_str = args
                .hwnoise_cpus
                .clone()
                .unwrap_or_else(|| args.cpus.clone());
            let hwnoise_cpus = parse_cpu_list(&hwnoise_cpus_str);
            let hwnoise_argv = build_hwnoise_argv(&args);
            let hwnoise_before =
                take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())
                    .context("failed to snapshot interference before rtla hwnoise")
                    .stage(Stage::Verdict, &tool_invocations)?;
            let hwnoise_start = Instant::now();
            let hwnoise_output = tools::run_tool("rtla", rtla_path, &version, hwnoise_argv)
                .context("failed to execute rtla hwnoise")
                .stage(Stage::ToolSpawn, &tool_invocations)?;
            let hwnoise_elapsed = hwnoise_start.elapsed();
            let hwnoise_after =
                take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())
                    .context("failed to snapshot interference after rtla hwnoise")
                    .stage(Stage::Verdict, &tool_invocations)?;

            warn_on_tool_failure(&hwnoise_output);
            let rtla_full_argv = hwnoise_output.invocation.argv.clone();
            tool_invocations.push(hwnoise_output.invocation);
            if let Some(record) = capture_stderr_sidecar(
                &run_dir.path,
                "rtla-hwnoise",
                &hwnoise_output.stderr,
                &mut run_bytes,
            )
            .stage(Stage::ToolExit, &tool_invocations)?
            {
                artifacts.push(record);
            }

            let hwnoise_path = run_dir.path.join("rtla-hwnoise.txt");
            std::fs::write(&hwnoise_path, &hwnoise_output.stdout)
                .context("failed to write rtla-hwnoise.txt")
                .stage(Stage::ToolExit, &tool_invocations)?;
            artifacts.push(
                place_artifact(
                    &hwnoise_path,
                    &run_dir.path,
                    "rtla-hwnoise.txt",
                    ArtifactKind::RtlaHwnoise,
                    &mut run_bytes,
                )
                .stage(Stage::ToolExit, &tool_invocations)?,
            );
            placed_paths.push((hwnoise_path.display().to_string(), "rtla-hwnoise.txt"));
            windows.push(nr_manifest::InstrumentWindow {
                instrument: "rtla-hwnoise".to_string(),
                elapsed_seconds: hwnoise_elapsed.as_secs_f64(),
                requested_seconds: Some(args.hwnoise_duration),
                delta: window_delta(&hwnoise_before, &hwnoise_after),
                before: hwnoise_before,
                after: hwnoise_after,
            });

            // The capture is never discarded on a parse failure (Stage::Parse rewrites
            // ATTEMPT.json with the raw output preserved, usable_for_numerical_analysis:
            // false; see the Stage::Parse handling below for cyclictest's own captures).
            // This is that same path, not a second one.
            let hwnoise_text = String::from_utf8_lossy(&hwnoise_output.stdout).into_owned();
            let parsed = hwnoise::parse_hwnoise(&hwnoise_text, &hwnoise_cpus)
                .context("failed to parse rtla hwnoise's output")
                .stage(Stage::Parse, &tool_invocations)?;

            if !parsed.missing_cpus.is_empty() {
                eprintln!(
                    "warning: rtla hwnoise produced no rows for cpu(s) {}: sampled but \
                     observed nothing, or was never actually sampled; see \
                     docs/rig/firmware-floor-rt-vs-stock.md",
                    parsed
                        .missing_cpus
                        .iter()
                        .map(u32::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }

            let observed_rows: Vec<&hwnoise::HwnoiseRow> = parsed
                .rows
                .iter()
                .filter(|row| parsed.observed_cpus.contains(&row.cpu))
                .collect();
            let max_us = observed_rows.iter().map(|row| row.max_single_us).max();
            let events_recorded: u64 = observed_rows.iter().map(|row| row.hw_count).sum();
            let per_cpu_exposure_seconds = observed_rows
                .iter()
                .map(|row| nr_manifest::CpuExposure {
                    cpu: row.cpu,
                    seconds: row.runtime_us as f64 / 1_000_000.0,
                })
                .collect();

            firmware_screens.push(nr_manifest::FirmwareScreen {
                instrument: "rtla-hwnoise".to_string(),
                tool_version: version,
                argv: rtla_full_argv
                    .iter()
                    .map(|arg| redact_home_prefix(arg, &redact_prefixes))
                    .collect(),
                requested_cpus: hwnoise_cpus,
                observed_cpus: parsed.observed_cpus,
                per_cpu_exposure_seconds,
                max_us,
                max_population: "the largest Max Single value (one-shot hardware-noise event) \
                    across the observed CPUs' final rtla hwnoise rows"
                    .to_string(),
                events_recorded,
            });
        }

        // The MSR_SMI_COUNT bracket's other half: the last instrument's window has now
        // closed, whichever one (if any) that was.
        let (smi_after, smi_after_reason) = take_smi_snapshot(&target_cpus, smi_fixture.as_deref());
        let smi_counts = nr_manifest::SmiCounts {
            register: smi::SMI_COUNT_REGISTER.to_string(),
            delta: smi::delta(&smi_before, &smi_after),
            before: smi_before,
            after: smi_after,
            // Only one reason fits this field; a failure on the before read is named ahead
            // of one on the after read; either way it says why, never leaving a silent zero
            // for the CPUs it names.
            unavailable_reason: smi_before_reason.or(smi_after_reason),
        };

        // Checksum cyclictest's own captures now that both instruments (if two
        // were requested) have run to completion.
        artifacts.push(
            place_artifact(
                &hist_path,
                &run_dir.path,
                "cyclictest.hist",
                ArtifactKind::CyclictestHist,
                &mut run_bytes,
            )
            .stage(Stage::ToolExit, &tool_invocations)?,
        );
        placed_paths.push((hist_path.display().to_string(), "cyclictest.hist"));
        if json_path.is_file() {
            artifacts.push(
                place_artifact(
                    &json_path,
                    &run_dir.path,
                    "cyclictest.json",
                    ArtifactKind::CyclictestJson,
                    &mut run_bytes,
                )
                .stage(Stage::ToolExit, &tool_invocations)?,
            );
            placed_paths.push((json_path.display().to_string(), "cyclictest.json"));
        }

        // Step 7: parse the raw captures and reconcile them. Ahead of the D-15/D-24
        // verdict below (it used to follow it): D-24's tail metrics need the parsed
        // histogram, and nothing in between depends on the other's output, so
        // parsing here costs nothing. Parsing, reconciliation and the artifact
        // checksums above all sit outside every window pushed so far, which is
        // what keeps each window a measure of its own instrument alone.
        let cyclictest_run: CyclictestRun = parse_hist_file(&hist_path, Some(args.histogram_max))
            .context("failed to parse cyclictest's .hist output")
            .stage(Stage::Parse, &tool_invocations)?;
        if json_path.is_file() {
            let summary = parse_json_file(&json_path)
                .context("failed to parse cyclictest's --json output")
                .stage(Stage::Parse, &tool_invocations)?;
            reconcile(&summary, &cyclictest_run)
                .context("cyclictest --json and .hist disagree; refusing a mismatched pairing")
                .stage(Stage::Reconcile, &tool_invocations)?;
        }
        write_hist_tsv(&run_dir.path, &cyclictest_run)
            .stage(Stage::ManifestWrite, &tool_invocations)?;

        // Step 8: the D-15/D-24 verdict. The outer before/after bracket reuses the
        // first window's before and the last window's after, so the top-level
        // fields keep their whole-run meaning for readers and for committed
        // manifests; the denominator is the cyclictest window's own measured
        // elapsed time, never the requested --duration (finding 7,
        // 01-EXTERNAL-AUDIT.md: a firmware screen run after cyclictest used to
        // inflate this denominator's numerator without inflating the denominator
        // itself).
        let outer_before = windows[0].before.clone();
        let outer_after = windows
            .last()
            .expect("the cyclictest window is always pushed before this point")
            .after
            .clone();
        let cyclictest_elapsed_seconds = windows[0].elapsed_seconds;
        // `thresholds` was loaded in step 1, before the measurement ran.
        let mut outcome = interference::verdict(
            outer_before,
            outer_after,
            &cyclictest_run,
            &thresholds,
            Duration::from_secs_f64(cyclictest_elapsed_seconds),
        )
        .context("failed to compute the D-15/D-24 contamination verdict")
        .stage(Stage::Verdict, &tool_invocations)?;
        outcome.pair.windows = windows;
        // A fixture-driven interference snapshot pair reads the same text for both
        // "before" and "after", so its own delta is exactly zero regardless of what
        // `evaluate`/`evaluate_tail` conclude from it. Overwrite the verdict's own
        // recorded reason so it says why, rather than letting a fabricated zero read
        // as a quiet machine. Finding 8 of `01-EXTERNAL-AUDIT.md`.
        if !fixtures_used.is_empty() {
            outcome.reason = Some(fixture_usage_reason(&fixtures_used));
        }

        // Step 9: the D-14 environment snapshot.
        let env_snapshot =
            environment::snapshot(facts.as_ref(), &args.rig_slug, Some(&thermal_start))
                .context("failed to capture the environment snapshot")
                .stage(Stage::EnvironmentSnapshot, &tool_invocations)?;

        let utc_end = OffsetDateTime::now_utc();

        // The argv is recorded exactly as it was executed (finding 6 of
        // 01-EXTERNAL-AUDIT.md: relativizing it against the run directory matched
        // nothing, because the tools used to write into a scratch tempdir under
        // /tmp, not the run directory). artifact_paths maps each output-file path
        // in argv to the committed artifact it became, matched by substring rather
        // than exact element equality because cyclictest's paths arrive as
        // `--histfile=<path>`, one argument, not two. A home directory prefix in
        // either is redacted (T-1-06).
        let tool_invocations: Vec<ToolInvocation> = tool_invocations
            .into_iter()
            .map(|invocation| {
                let artifact_paths = placed_paths
                    .iter()
                    .filter(|(executed_path, _)| {
                        invocation
                            .argv
                            .iter()
                            .any(|arg| arg.contains(executed_path.as_str()))
                    })
                    .map(|(executed_path, artifact_name)| ArtifactPathMapping {
                        executed_path: redact_home_prefix(executed_path, &redact_prefixes),
                        artifact_path: artifact_name.to_string(),
                    })
                    .collect();
                let argv = invocation
                    .argv
                    .iter()
                    .map(|arg| redact_home_prefix(arg, &redact_prefixes))
                    .collect();
                ToolInvocation {
                    argv,
                    artifact_paths,
                    ..invocation
                }
            })
            .collect();

        // D-28: run admission is decided only from evidence causally upstream of the
        // measured latency, never from its shape (01-REVIEW-2026-09-06.md finding A1).
        // --allow-precondition-violation (D-17) and fixture use (finding 8 of
        // 01-EXTERNAL-AUDIT.md) are two of the seven evidence sources
        // determine_admission consults, not two separate forcing branches; see its own
        // doc comment for the full list and their fixed order.
        let precondition_waiver_text = args
            .allow_precondition_violation
            .then(|| precondition_waiver_reason(&results));
        let counter_thresholds = match &thresholds {
            interference::Thresholds::Calibrated(limits) => Some(limits),
            // Calibrating a threshold set needs a body of admitted runs, and a body of
            // admitted runs needs runs to be admitted, so admission cannot depend on a
            // calibrated threshold set without a circular dependency (D-28). Neither an
            // uncalibrated file nor a pair derived from exactly two runs is a
            // calibrated ceiling; both are a deliberate None here, not an unhandled
            // case.
            interference::Thresholds::Uncalibrated | interference::Thresholds::Provisional(_) => {
                None
            }
        };
        let admission = determine_admission(&AdmissionInputs {
            preconditions: &results,
            instrument_class: &instrument_class,
            tools: &tool_invocations,
            fixtures_used: &fixtures_used,
            precondition_waiver: precondition_waiver_text.as_deref(),
            interference_delta: &outcome.pair.delta,
            counter_thresholds,
            run_duration: Duration::from_secs_f64(cyclictest_elapsed_seconds),
            smi: &smi_counts,
            package_temp_c_max: env_snapshot.power.package_temp_c_max,
        });
        let excluded_from_series = !admission.admitted;
        let exclusion_reason =
            (!admission.exclusions.is_empty()).then(|| admission.exclusions.join("; "));

        // Step 10: assemble, validate and write manifest.json (written last).
        rundir::refuse_if_manifest_exists(&run_dir.path)
            .context("run directory was populated between creation and the final write")
            .stage(Stage::ManifestWrite, &tool_invocations)?;

        let manifest = RunManifest {
            schema_version: nr_manifest::SCHEMA_VERSION,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            run_id: run_dir.run_id.clone(),
            run_class,
            instrument_class,
            thermal_profile: Some(thermal_profile),
            utc_start,
            utc_end,
            harness: harness.clone(),
            host: env_snapshot.host,
            kernel: env_snapshot.kernel,
            os: env_snapshot.os,
            tuning: env_snapshot.tuning,
            power: env_snapshot.power,
            network: env_snapshot.network,
            preconditions: results,
            interference: outcome.pair,
            tools: tool_invocations,
            artifacts,
            firmware_screens,
            smi_counts: Some(smi_counts),
            series_admission: Some(admission),
            absent_fields: env_snapshot.absent_fields,
            excluded_from_series,
            exclusion_reason,
            fixtures_used: fixtures_used.clone(),
            notes: args.note.clone(),
        };

        nr_manifest::validate(&run_dir.path, &manifest)
            .map_err(|errors| {
                anyhow::anyhow!("generated manifest failed its own validation: {errors:?}")
            })
            .stage(Stage::ManifestWrite, &manifest.tools)?;
        let manifest_json = serde_json::to_string_pretty(&manifest)
            .context("failed to serialise the manifest")
            .stage(Stage::ManifestWrite, &manifest.tools)?;
        std::fs::write(run_dir.path.join("manifest.json"), manifest_json)
            .context("failed to write manifest.json")
            .stage(Stage::ManifestWrite, &manifest.tools)?;

        Ok((manifest, cyclictest_run))
    })();

    let (manifest, cyclictest_run) = match attempt_result {
        Ok(pair) => pair,
        Err(staged) => {
            eprintln!("error: {:#}", staged.error);
            write_attempt(
                &run_dir.path,
                &AttemptRecord {
                    schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
                    run_id: run_dir.run_id.clone(),
                    utc_start,
                    utc_end: Some(OffsetDateTime::now_utc()),
                    status: AttemptStatus::Failed,
                    harness,
                    requested,
                    preserved: scan_preserved_files(&run_dir.path),
                    tools: staged.tools,
                    failure: Some(AttemptFailure {
                        stage: staged.stage.as_str().to_string(),
                        message: format!("{:#}", staged.error),
                    }),
                    usable_for_numerical_analysis: false,
                },
            )
            .context("failed to write the failed ATTEMPT.json")?;
            return Ok(1);
        }
    };

    // The attempt succeeded and manifest.json is on disk: rewrite ATTEMPT.json as
    // completed before doing anything else, so a failure rendering REPORT.md below
    // (which is regenerable and not required for the directory to verify) cannot
    // leave the attempt record looking unfinished.
    write_attempt(
        &run_dir.path,
        &AttemptRecord {
            schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
            run_id: run_dir.run_id.clone(),
            utc_start,
            utc_end: Some(manifest.utc_end),
            status: AttemptStatus::Completed,
            harness: manifest.harness.clone(),
            requested,
            tools: manifest.tools.clone(),
            preserved: manifest.artifacts.clone(),
            failure: None,
            usable_for_numerical_analysis: true,
        },
    )
    .context("failed to write the completed ATTEMPT.json")?;

    // Step 11: render and write REPORT.md, generated from the manifest that was
    // just written, never hand-maintained.
    let report =
        render_run_report(&manifest, &cyclictest_run).context("failed to render REPORT.md")?;
    std::fs::write(run_dir.path.join("REPORT.md"), report).context("failed to write REPORT.md")?;

    // Step 12: print the run directory and the headline numbers.
    print_summary(&run_dir, &cyclictest_run, &manifest)?;

    Ok(0)
}

/// Serialises and writes `run_dir/ATTEMPT.json`, overwriting whatever was there
/// before. Called once when the attempt starts (`status: in-progress`), and again
/// on every exit path (`completed` or `failed`), so the file on disk is always the
/// record of the attempt's current state, never a stale snapshot from an earlier
/// call.
fn write_attempt(run_dir: &Path, record: &AttemptRecord) -> Result<()> {
    let json = serde_json::to_string_pretty(record).context("failed to serialise ATTEMPT.json")?;
    std::fs::write(run_dir.join("ATTEMPT.json"), json).context("failed to write ATTEMPT.json")
}

/// Every regular file directly inside `run_dir`, excluding `ATTEMPT.json` itself and
/// any `manifest.json` (a failed attempt must never carry one), checksummed and
/// classified by name. Populates a failed attempt's `preserved` array: whatever
/// exists on disk at the moment of failure is preserved evidence, regardless of
/// which step produced it, so this is a directory scan rather than an incrementally
/// tracked list. A file that cannot be stat'd or hashed (a symlink race, in
/// practice unreachable here) is silently skipped rather than failing the whole
/// failure-reporting path a second time.
// `pub(crate)`: `cmd::attempt` reuses this to close out an attempt the harness itself
// never got to finish (a killed process, not a normal failure exit).
pub(crate) fn scan_preserved_files(run_dir: &Path) -> Vec<ArtifactRecord> {
    let Ok(entries) = std::fs::read_dir(run_dir) else {
        return Vec::new();
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    paths.sort();

    paths
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            if name == "ATTEMPT.json" || name == "manifest.json" {
                return None;
            }
            let bytes = std::fs::metadata(&path).ok()?.len();
            let blake3 = nr_manifest::blake3_file(&path).ok()?;
            Some(ArtifactRecord {
                kind: artifact_kind_for_filename(&name),
                path: name,
                bytes,
                blake3,
                stored: StorageLocation::InRepo,
            })
        })
        .collect()
}

/// Maps a run directory's fixed, harness-chosen file names to the artifact kind
/// they represent. Anything else (a `<tool>.stderr.txt` sidecar, or a file this
/// function does not recognise) is `ArtifactKind::Other`.
fn artifact_kind_for_filename(name: &str) -> ArtifactKind {
    match name {
        "cyclictest.hist" => ArtifactKind::CyclictestHist,
        "cyclictest.json" => ArtifactKind::CyclictestJson,
        "hwlatdetect.txt" => ArtifactKind::HwlatdetectText,
        "rtla-hwnoise.txt" => ArtifactKind::RtlaHwnoise,
        _ => ArtifactKind::Other,
    }
}

/// Writes `stderr` to `<tool_name>.stderr.txt` inside the run directory when it is
/// non-empty, and returns the resulting [`ArtifactRecord`] so it can be folded into
/// `manifest.artifacts` on success (and, on a failure, it is discovered again by
/// [`scan_preserved_files`], independent of this return value). `None` when
/// `stderr` is empty: an empty sidecar file would be one more file to account for
/// and carries no evidence.
///
/// T-1-62: stderr is captured verbatim as evidence, but it is free-form output a
/// tool wrote on its own initiative and can contain the operator's username, a
/// home directory path, or this machine's hostname. [`redact_stderr`] scrubs all
/// three before the bytes ever touch disk; the visible `[redacted]` token left
/// behind is the record that something was found and scrubbed, the same
/// convention `KernelInfo::redact_cmdline` and `redact_home_prefix` already use.
fn capture_stderr_sidecar(
    run_dir: &Path,
    tool_name: &str,
    stderr: &[u8],
    run_bytes_so_far: &mut u64,
) -> Result<Option<ArtifactRecord>> {
    if stderr.is_empty() {
        return Ok(None);
    }
    let redacted = redact_stderr(&String::from_utf8_lossy(stderr));
    let file_name = format!("{tool_name}.stderr.txt");
    let dest = run_dir.join(&file_name);
    std::fs::write(&dest, &redacted).with_context(|| format!("failed to write {file_name}"))?;
    let bytes = redacted.len() as u64;
    let blake3 = nr_manifest::blake3_file(&dest)
        .map_err(|source| anyhow::anyhow!("failed to checksum {file_name}: {source}"))?;
    *run_bytes_so_far += bytes;
    Ok(Some(ArtifactRecord {
        path: file_name,
        bytes,
        blake3,
        kind: ArtifactKind::Other,
        stored: StorageLocation::InRepo,
    }))
}

/// Replaces every occurrence of the operator's username, a `/home/`-shaped path
/// prefix, and this machine's own hostname in free-form tool stderr text with the
/// literal token `[redacted]` (T-1-62). Unlike `redact_home_prefix` (anchored at
/// the start of a single argv token), stderr is unstructured, possibly
/// multi-line text a tool wrote unprompted, so every occurrence anywhere in the
/// text is scanned and replaced, not just a leading match.
fn redact_stderr(text: &str) -> String {
    let mut redacted = text.to_string();
    if let Some(user) = std::env::var("USER").ok().filter(|value| !value.is_empty()) {
        redacted = redacted.replace(&user, "[redacted]");
    }
    redacted = redacted.replace("/home/", "[redacted]/");
    if let Some(hostname) = current_hostname() {
        redacted = redacted.replace(&hostname, "[redacted]");
    }
    redacted
}

/// This machine's hostname, read once per invocation purely to check whether a
/// tool's stderr happens to contain it. Never itself recorded in a manifest or
/// attempt record: `HostInfo`'s own documentation already forbids storing any
/// network-derived machine name, and this function exists only to grep stderr
/// against it, matching T-1-06's existing hostname-redaction convention (the
/// D-14 environment probes redact it out of `uname -a` and journalctl lines).
fn current_hostname() -> Option<String> {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

fn print_summary(run_dir: &RunDir, run: &CyclictestRun, manifest: &RunManifest) -> Result<()> {
    let percentiles = run
        .percentiles(&[0.5, 0.95, 0.99])
        .context("failed to compute summary percentiles")?;
    println!("wrote {}", run_dir.path.display());
    let (p50, p95, p99) = (
        percentiles.values.first().map(|(_, v)| *v).unwrap_or(0),
        percentiles.values.get(1).map(|(_, v)| *v).unwrap_or(0),
        percentiles.values.get(2).map(|(_, v)| *v).unwrap_or(0),
    );
    println!(
        "p50={p50} us p95={p95} us p99={p99} us max={} us samples={} overflow={}",
        percentiles.max_us, percentiles.total_samples, percentiles.overflow_samples
    );
    println!(
        "contamination verdict: {:?}, excluded_from_series: {}",
        manifest.interference.verdict, manifest.excluded_from_series
    );
    if let Some(tail) = &manifest.interference.tail_metrics {
        println!(
            "tail excursion ratio={:.1} thread-max spread={:.1}% overflow rate={:.4}/s",
            tail.tail_excursion_ratio,
            tail.thread_max_spread * 100.0,
            tail.overflow_rate_per_s
        );
    }
    if let Some(smi) = &manifest.smi_counts {
        match &smi.unavailable_reason {
            Some(reason) => println!("SMI count over the run: unavailable ({reason})"),
            None => {
                let deltas: Vec<String> = smi
                    .delta
                    .iter()
                    .map(|c| format!("cpu{}={}", c.cpu, c.count))
                    .collect();
                println!("SMI count over the run: {}", deltas.join(" "));
            }
        }
    }
    Ok(())
}

/// BENCH-06: a tool failure is recorded, not hidden. A non-zero exit does not
/// abort the write (the manifest records it, see [`determine_admission`]), but the
/// tool's own stderr is surfaced immediately so the operator does not have to go
/// digging for it.
/// Whether a nonzero exit actually means the tool failed.
///
/// `hwlatdetect` exits with `(maxlatency > hardlimit)` and defaults `hardlimit` to the
/// latency threshold when `--hardlimit` is not passed (`/usr/sbin/hwlatdetect` lines 458 and
/// 549). A firmware screen that observes anything above the threshold therefore exits 1 by
/// design, having run to completion and written a full capture. That is the finding the screen
/// exists to produce, not an error.
///
/// Treating it as a failure excluded both D-18 arms on 2026-09-05 with the reason "hwlatdetect
/// exited with code 1", which reads as a broken capture. Those manifests are published and
/// D-12 forbids editing them, so the wrong reason stands in
/// `measurements/2026-09-05-precision3591-screen{,-02}` and is explained in
/// `docs/rig/firmware-floor-rt-vs-stock.md`.
///
/// Any other nonzero exit from `hwlatdetect`, and any nonzero exit from any other tool, is
/// still a failure: `cyclictest` has no such convention.
fn is_tool_failure(tool: &ToolInvocation) -> bool {
    match (tool.name.as_str(), tool.exit_code) {
        ("hwlatdetect", 1) => false,
        (_, code) => code != 0,
    }
}

fn warn_on_tool_failure(output: &tools::ToolOutput) {
    if is_tool_failure(&output.invocation) {
        eprintln!(
            "warning: {} exited with code {}",
            output.invocation.name, output.invocation.exit_code
        );
        if !output.stderr.is_empty() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }
}

fn print_dry_run(args: &Args, thread_count: usize) {
    println!("dry run: every precondition passed. Nothing was executed or written.");
    let cyclictest_argv = build_cyclictest_argv(
        args,
        thread_count,
        "<run-dir>/cyclictest.hist",
        "<run-dir>/cyclictest.json",
    );
    println!("would run: cyclictest {}", cyclictest_argv.join(" "));
    if args.with_hwlatdetect {
        let hwlatdetect_argv = build_hwlatdetect_argv(args);
        println!("would run: hwlatdetect {}", hwlatdetect_argv.join(" "));
    }
    if args.with_hwnoise {
        let hwnoise_argv = build_hwnoise_argv(args);
        println!("would run: rtla {}", hwnoise_argv.join(" "));
    }
}

/// The exact hwlatdetect invocation this command builds, shared between the real
/// execution path and `print_dry_run` so the two can never drift apart. `--cpu-list`
/// is appended only when `--hwlatdetect-cpu-list` was given; hwlatdetect's own default
/// (sample every CPU) applies otherwise.
fn build_hwlatdetect_argv(args: &Args) -> Vec<String> {
    let mut argv = vec![format!("--duration={}", args.hwlatdetect_duration)];
    if let Some(cpu_list) = &args.hwlatdetect_cpu_list {
        argv.push(format!("--cpu-list={cpu_list}"));
    }
    argv
}

/// The exact `rtla hwnoise` invocation this command builds, shared between the real
/// execution path and `print_dry_run` so the two can never drift apart (the same pattern
/// `build_hwlatdetect_argv` follows). `"hwnoise"` is `rtla`'s own subcommand, not the
/// program name recorded separately in `ToolInvocation`; `tools::run_tool("rtla", ...)`
/// prepends that, so the fully recorded argv reads `["rtla", "hwnoise", "-c", ...]`.
fn build_hwnoise_argv(args: &Args) -> Vec<String> {
    let cpus = args
        .hwnoise_cpus
        .clone()
        .unwrap_or_else(|| args.cpus.clone());
    vec![
        "hwnoise".to_string(),
        "-c".to_string(),
        cpus,
        "-H".to_string(),
        args.hwnoise_housekeeping.clone(),
        "-P".to_string(),
        args.hwnoise_priority.clone(),
        "-d".to_string(),
        format!("{}s", args.hwnoise_duration),
    ]
}

/// The exact cyclictest invocation this command builds. `--distance=0` keeps every
/// thread on the same interval, which is what makes the per-thread histograms
/// comparable.
fn build_cyclictest_argv(
    args: &Args,
    thread_count: usize,
    histfile: &str,
    jsonfile: &str,
) -> Vec<String> {
    let mut argv = vec![
        format!("--mainaffinity={}", args.main_cpus),
        format!("--affinity={}", args.cpus),
        format!("--threads={thread_count}"),
        "--mlockall".to_string(),
        "--policy=fifo".to_string(),
        format!("--priority={}", args.priority),
        format!("--interval={}", args.interval),
        "--distance=0".to_string(),
        format!("--histogram={}", args.histogram_max),
        format!("--histfile={histfile}"),
        format!("--json={jsonfile}"),
        format!("--duration={}", args.duration),
    ];
    if let Some(us) = args.breaktrace {
        argv.push(format!("--breaktrace={us}"));
        argv.push("--tracemark".to_string());
    }
    argv
}

fn build_facts(fixture_path: Option<&Path>) -> Result<Box<dyn SystemFacts>> {
    if let Some(path) = fixture_path {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {FACTS_FIXTURE_ENV} at {}", path.display()))?;
        let (simple_text, blocks) = extract_multiline_blocks(&text);
        let mut facts = FixtureFacts::parse(&simple_text);
        for (key, value) in blocks {
            facts = facts.with(&key, &value);
        }
        return Ok(Box::new(facts));
    }
    live_facts()
}

/// `FixtureFacts::parse`'s format is one assignment per line, which has no way to
/// represent a value with embedded newlines, e.g. the real, multi-line content of
/// `/proc/cpuinfo` or `/etc/os-release` the D-14 environment snapshot reads.
/// Extends the format, in this crate only, with an `@begin <key>` / `@end` block
/// whose body (joined with real newlines) is applied afterward via
/// `FixtureFacts::with`, which takes a plain string and has no line-splitting
/// concern of its own. `nr_capture`'s fixture format itself is untouched; this is
/// a preprocessing step entirely on this crate's side of the seam.
fn extract_multiline_blocks(text: &str) -> (String, Vec<(String, String)>) {
    let mut simple_lines = Vec::new();
    let mut blocks: Vec<(String, String)> = Vec::new();
    let mut current: Option<(String, Vec<&str>)> = None;

    for line in text.lines() {
        if let Some(key) = line.strip_prefix("@begin ") {
            current = Some((key.trim().to_string(), Vec::new()));
        } else if line.trim() == "@end" {
            if let Some((key, body)) = current.take() {
                blocks.push((key, body.join("\n")));
            }
        } else if let Some((_, body)) = current.as_mut() {
            body.push(line);
        } else {
            simple_lines.push(line);
        }
    }

    (simple_lines.join("\n"), blocks)
}

#[cfg(target_os = "linux")]
fn live_facts() -> Result<Box<dyn SystemFacts>> {
    let live = nr_capture::sources::live().context("failed to initialise live system facts")?;
    Ok(Box::new(live))
}

#[cfg(not(target_os = "linux"))]
fn live_facts() -> Result<Box<dyn SystemFacts>> {
    anyhow::bail!(
        "live system facts are only available on Linux; set {FACTS_FIXTURE_ENV} to run against a \
         fixture on this host"
    )
}

fn take_interference_snapshot(
    cpus: &[u32],
    fixture_text: Option<&str>,
) -> Result<nr_manifest::InterferenceSnapshot> {
    match fixture_text {
        Some(text) => Ok(interference::snapshot_from_text(text, cpus)?),
        None => live_interference_snapshot(cpus),
    }
}

/// `MSR_SMI_COUNT` on every CPU in `cpus`: the fixture text (read once, reused for both the
/// before and after snapshot of a fixture-driven run) when one is given, the live reader
/// otherwise. Unlike [`take_interference_snapshot`], never fallible at this call site:
/// `smi::read_smi_counts`/`smi::parse_smi_snapshot` are already infallible by design (an
/// unreadable register is a stated reason inside the returned tuple), and `smi::read_smi_counts`
/// is itself gated for both Linux and non-Linux hosts, so there is no separate live wrapper to
/// write here.
fn take_smi_snapshot(
    cpus: &[u32],
    fixture_text: Option<&str>,
) -> (Vec<nr_manifest::CpuCounter>, Option<String>) {
    match fixture_text {
        Some(text) => smi::parse_smi_snapshot(text, cpus),
        None => smi::read_smi_counts(cpus),
    }
}

/// The after-minus-before difference for each counter family, saturating at zero. Mirrors
/// `nr_capture::interference::compute_delta` (private to that crate) rather than exposing it:
/// building one [`nr_manifest::InstrumentWindow`] per instrument here is the only place outside
/// `interference::verdict` itself that needs a delta, so a small, local duplicate costs less
/// than widening that crate's public API for one caller.
fn window_delta(
    before: &nr_manifest::InterferenceSnapshot,
    after: &nr_manifest::InterferenceSnapshot,
) -> nr_manifest::InterferenceDelta {
    fn diff(
        before: &[nr_manifest::CpuCounter],
        after: &[nr_manifest::CpuCounter],
    ) -> Vec<nr_manifest::CpuCounter> {
        let before_map: std::collections::BTreeMap<u32, u64> =
            before.iter().map(|c| (c.cpu, c.count)).collect();
        after
            .iter()
            .map(|c| nr_manifest::CpuCounter {
                cpu: c.cpu,
                count: c
                    .count
                    .saturating_sub(before_map.get(&c.cpu).copied().unwrap_or(0)),
            })
            .collect()
    }
    nr_manifest::InterferenceDelta {
        cal_ipis: diff(&before.cal_ipis, &after.cal_ipis),
        tlb_ipis: diff(&before.tlb_ipis, &after.tlb_ipis),
        rescheduling_ipis: diff(&before.rescheduling_ipis, &after.rescheduling_ipis),
        irqs: diff(&before.irqs, &after.irqs),
    }
}

#[cfg(target_os = "linux")]
fn live_interference_snapshot(cpus: &[u32]) -> Result<nr_manifest::InterferenceSnapshot> {
    Ok(nr_capture::interference::snapshot(cpus)?)
}

#[cfg(not(target_os = "linux"))]
fn live_interference_snapshot(_cpus: &[u32]) -> Result<nr_manifest::InterferenceSnapshot> {
    anyhow::bail!(
        "reading /proc/interrupts requires Linux; set {INTERRUPTS_FIXTURE_ENV} to run against a \
         fixture on this host"
    )
}

/// Ties the manifest to the exact harness build that produced it (T-1-10).
/// `git_sha`/`git_dirty`/`git_sha_source` are embedded at compile time by
/// `crates/cli/build.rs`, so a stale executable running inside a newer checkout, or
/// a run launched with no git checkout as its working directory (the systemd-run
/// case that produced seven "unknown" manifests; finding 6 of
/// `01-EXTERNAL-AUDIT.md`), still records a real identity. `invoked_from_git_sha` is
/// the one field that still reads the working directory, kept separate specifically
/// so the two can be compared.
fn harness_info() -> HarnessInfo {
    let (executable_blake3, executable_bytes) = match std::env::current_exe() {
        Ok(path) => (
            nr_manifest::blake3_file(&path).ok(),
            std::fs::metadata(&path).ok().map(|m| m.len()),
        ),
        Err(_) => (None, None),
    };
    HarnessInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: env!("NR_BUILD_GIT_SHA").to_string(),
        git_dirty: env!("NR_BUILD_GIT_DIRTY") == "true",
        git_sha_source: Some(match env!("NR_BUILD_GIT_SHA_SOURCE") {
            "build-time" => GitShaSource::BuildTime,
            _ => GitShaSource::Unavailable,
        }),
        executable_blake3,
        executable_bytes,
        invoked_from_git_sha: git_output(&["rev-parse", "HEAD"]),
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The absolute prefixes considered identity-bearing for [`redact_home_prefix`], longest
/// first so the most specific one wins.
///
/// Not derived from `$HOME` alone. `scripts/nr-run-measurement` launches this binary as a
/// root `systemd-run` transient unit with no `User=`/`PAMName=`, and a system-scope unit's
/// environment is not a login shell's: `HOME` is not reliably the operator's real home
/// directory there, or may be unset entirely. Trusting it exclusively let
/// `/home/<user>/...` reach three committed manifests unredacted before this was caught
/// (plan 01-23, T-1-95) despite `argv_redacts_a_home_directory_prefix` passing throughout,
/// because that test forces `HOME` via `.env(...)` and so never exercised the real gap.
///
/// The reliable signal instead: a structural match against the conventional Unix
/// home-directory shape, `/home/<user>` or (dev-host-only) `/Users/<user>`, found by
/// walking `run_directory`'s own ancestors (see [`conventional_home_prefix`]) for a
/// component whose immediate parent is literally named `home` or `Users`. This needs no
/// environment variable at all: it is a pure string match against the same path the
/// harness already computed (`measurements_root.join(run_id)`, `rundir.rs::create`), so it
/// works whether or not the process's environment carries a correct `HOME`. It also does
/// not fire on an ordinary scratch directory such as a test's own `tempfile::tempdir()`
/// (`/tmp/.tmpXXXXXX`, or macOS's `/var/folders/.../T/.tmpXXXXXX`), which is exactly the
/// case `argv_is_recorded_as_executed` requires stay byte-for-byte unredacted: an earlier
/// version of this function instead redacted `run_directory`'s ancestor two levels up
/// unconditionally, which happened to be that test's own tempdir root and broke its
/// fidelity assertion for a directory that identifies nobody. `$HOME` is tried second, as
/// a fallback, for a real deployment that does not happen to sit under `home`/`Users` but
/// whose environment is reliable anyway.
fn redaction_prefixes(run_directory: &Path) -> Vec<String> {
    let mut prefixes = Vec::new();
    if let Some(conventional) = conventional_home_prefix(run_directory) {
        prefixes.push(conventional);
    }
    if let Some(home) = std::env::home_dir() {
        if !home.as_os_str().is_empty() {
            let home = home.to_string_lossy().into_owned();
            if !prefixes.contains(&home) {
                prefixes.push(home);
            }
        }
    }
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));
    prefixes
}

/// Walks `path` and its ancestors looking for one whose own immediate parent is named
/// `home` or `Users`, i.e. a path shaped like `/home/<user>` or `/Users/<user>`. Returns
/// the first (deepest, but there is realistically only ever one) such ancestor found,
/// stringified exactly as it appears in `path` -- no canonicalization, so it matches
/// whatever relative form or symlink component the caller's own path already has.
fn conventional_home_prefix(path: &Path) -> Option<String> {
    path.ancestors()
        .find(|ancestor| {
            matches!(
                ancestor
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str()),
                Some("home") | Some("Users")
            )
        })
        .map(|ancestor| ancestor.to_string_lossy().into_owned())
}

/// Replaces a leading match of any entry in `prefixes` (see [`redaction_prefixes`]) in
/// `value` with the literal token `[redacted]`, following the same visible-redaction
/// convention as `KernelInfo::redact_cmdline` and T-1-06's hostname redaction. An executed
/// argv can carry a home directory (`--measurements-root /home/<user>/neurorust/
/// measurements` is passed on every rig invocation by `scripts/nr-run-measurement`, and the
/// scratch directory the tools write into would carry the same exposure if `$TMPDIR` or an
/// equivalent ever pointed under home); a published manifest must never carry it verbatim.
/// Handles both a bare path (e.g. `executed_path`, or a standalone argv token) and a
/// `--flag=/abs/path` token, splitting on the first `=` the same way `KernelInfo::
/// redact_cmdline` does: a naive whole-string match would never fire for a `--histfile=...`
/// token, since the string starts with `--histfile=`, not with the path. A value matching
/// no prefix, for example the scratch `/tmp/.tmpXXXXXX` paths the tools actually write into
/// today, passes through unchanged: those are process-lifetime temporary names and identify
/// nobody.
fn redact_home_prefix(value: &str, prefixes: &[String]) -> String {
    if let Some((flag, rest)) = value.split_once('=') {
        if flag.starts_with('-') {
            return format!("{flag}={}", redact_home_prefix_in_path(rest, prefixes));
        }
    }
    redact_home_prefix_in_path(value, prefixes)
}

fn redact_home_prefix_in_path(value: &str, prefixes: &[String]) -> String {
    for prefix in prefixes {
        if let Some(rest) = value.strip_prefix(prefix.as_str()) {
            return format!("[redacted]{rest}");
        }
    }
    value.to_string()
}

/// Checksums `src` and records whether it stayed in-repo or exceeded the size
/// policy (`rundir::exceeds_in_repo_limit`). Copies `src` into `run_dir` under
/// `target_name` first, unless the two are already the same file: every caller in
/// this module now writes a tool's output directly into the run directory (there is
/// no scratch directory left to copy out of), so `src` and `run_dir.join(target_name)`
/// are the same path on every real call, and the case is handled explicitly rather
/// than relying on `std::fs::copy`'s platform-dependent behaviour when source and
/// destination coincide. D-12: the file's bytes are never rewritten, only
/// checksummed on top of it.
fn place_artifact(
    src: &Path,
    run_dir: &Path,
    target_name: &str,
    kind: ArtifactKind,
    run_bytes_so_far: &mut u64,
) -> Result<ArtifactRecord> {
    let bytes = std::fs::metadata(src)
        .with_context(|| format!("failed to stat {}", src.display()))?
        .len();
    let blake3 = nr_manifest::blake3_file(src)
        .map_err(|source| anyhow::anyhow!("failed to checksum {}: {source}", src.display()))?;

    if rundir::exceeds_in_repo_limit(bytes, *run_bytes_so_far) {
        // Over the size policy: recorded as an external pointer rather than copied
        // in. The operator compresses and uploads the artifact and supplies the
        // URL by hand before this run can pass `nr_manifest::validate`; neither
        // cyclictest nor hwlatdetect captures ever approach this threshold, so
        // this plan's own tests never exercise it. See rundir.rs's documented
        // policy.
        eprintln!(
            "warning: {target_name} ({bytes} bytes) exceeds the in-repo size policy; recorded as \
             an external pointer with no URL yet. Upload it and add the URL to manifest.json \
             before this run can be verified."
        );
        return Ok(ArtifactRecord {
            path: target_name.to_string(),
            bytes,
            blake3,
            kind,
            stored: StorageLocation::External { url: String::new() },
        });
    }

    let dest = run_dir.join(target_name);
    if src != dest {
        std::fs::copy(src, &dest)
            .with_context(|| format!("failed to place {target_name} into the run directory"))?;
    }
    *run_bytes_so_far += bytes;

    Ok(ArtifactRecord {
        path: target_name.to_string(),
        bytes,
        blake3,
        kind,
        stored: StorageLocation::InRepo,
    })
}

/// `hist.tsv`: bin_us then count, tab separated, one row per non-empty bin, so a
/// reader can plot without a Rust toolchain. The 888 overflow samples in the real
/// reference capture have no bin index at all (cyclictest reports them separately,
/// in the `.hist` footer, not as a histogram row), so they cannot appear here; the
/// row-sum is the binned sample total, not `Percentiles::total_samples`.
/// `hist.tsv`'s exact on-disk format: `bin_us` TAB `count`, one line per non-empty bin, no
/// header. Shared by the writer ([`write_hist_tsv`]) and strict verification's re-derivation
/// check (`cmd::verify::check_derived_figures`), so the two can never quietly drift apart from
/// each other. Finding 6 of `01-EXTERNAL-AUDIT.md`.
pub(crate) fn format_hist_tsv(run: &CyclictestRun) -> String {
    let mut out = String::new();
    for (bin_us, count) in run.to_bin_table() {
        if count > 0 {
            out.push_str(&format!("{bin_us}\t{count}\n"));
        }
    }
    out
}

fn write_hist_tsv(run_dir: &Path, run: &CyclictestRun) -> Result<()> {
    std::fs::write(run_dir.join("hist.tsv"), format_hist_tsv(run))
        .context("failed to write hist.tsv")
}

/// Everything the admission gate is allowed to see. The absence of `ContaminationVerdict`,
/// `TailMetrics` and `InterferenceSnapshotPair` from this struct is the mechanism, not an
/// oversight: the gate cannot exclude a run for the shape of its own latency because it is never
/// handed the shape. Adding either to this struct is the change a reviewer should refuse.
struct AdmissionInputs<'a> {
    preconditions: &'a [PreconditionResult],
    instrument_class: &'a InstrumentClass,
    tools: &'a [ToolInvocation],
    fixtures_used: &'a [String],
    /// `Some(reason)` when `--allow-precondition-violation` was given.
    precondition_waiver: Option<&'a str>,
    interference_delta: &'a nr_manifest::InterferenceDelta,
    /// `Some` only when `config/contamination-thresholds.json` is `calibrated`. `None` under the
    /// shipped provisional file, in which case the counters are recorded and not thresholded.
    counter_thresholds: Option<&'a interference::CalibratedThresholds>,
    run_duration: Duration,
    smi: &'a nr_manifest::SmiCounts,
    package_temp_c_max: Option<f32>,
}

/// The observed text for `AdmissionEvidenceSource::InterferenceCounters`: the maximum per-CPU
/// delta of each of the four D-15 counters, and the run duration in whole seconds. Recorded
/// whether or not a calibrated threshold exists to judge it against.
fn format_counter_deltas(delta: &nr_manifest::InterferenceDelta, run_duration: Duration) -> String {
    fn max_of(counters: &[nr_manifest::CpuCounter]) -> (u64, u32) {
        counters
            .iter()
            .max_by_key(|counter| counter.count)
            .map(|counter| (counter.count, counter.cpu))
            .unwrap_or((0, 0))
    }
    let (cal_max, cal_cpu) = max_of(&delta.cal_ipis);
    let (tlb_max, tlb_cpu) = max_of(&delta.tlb_ipis);
    let (res_max, res_cpu) = max_of(&delta.rescheduling_ipis);
    let (irq_max, irq_cpu) = max_of(&delta.irqs);
    format!(
        "cal max {cal_max} on cpu{cal_cpu}, tlb max {tlb_max} on cpu{tlb_cpu}, res max {res_max} \
         on cpu{res_cpu}, irq max {irq_max} on cpu{irq_cpu}, over {}s",
        run_duration.as_secs()
    )
}

/// D-28: decides admission to the headline regression series from upstream evidence only.
fn determine_admission(inputs: &AdmissionInputs<'_>) -> SeriesAdmission {
    let mut evidence = Vec::with_capacity(7);
    let mut exclusions = Vec::new();

    // 1. PreconditionWaiver. D-17 makes this unconditional and it stays unconditional: a
    // waived calibration-contaminated run is always excluded, whatever every other source
    // below finds.
    match inputs.precondition_waiver {
        Some(reason) => {
            evidence.push(AdmissionEvidence {
                source: AdmissionEvidenceSource::PreconditionWaiver,
                observed: reason.to_string(),
                disposition: AdmissionDisposition::Excluding,
            });
            exclusions.push(reason.to_string());
        }
        None => evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::PreconditionWaiver,
            observed: "not given".to_string(),
            disposition: AdmissionDisposition::Clean,
        }),
    }

    // 2. FixtureUse. No fixture can ever reach the series regardless of what its
    // fabricated deltas say (finding 8 of 01-EXTERNAL-AUDIT.md).
    if inputs.fixtures_used.is_empty() {
        evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::FixtureUse,
            observed: "none".to_string(),
            disposition: AdmissionDisposition::Clean,
        });
    } else {
        let reason = fixture_usage_reason(inputs.fixtures_used);
        evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::FixtureUse,
            observed: reason.clone(),
            disposition: AdmissionDisposition::Excluding,
        });
        exclusions.push(reason);
    }

    // 3. ToolExitCodes. Reuses is_tool_failure unchanged: hwlatdetect exit 1 stays a
    // finding, not a failure.
    let observed_tools = inputs
        .tools
        .iter()
        .map(|tool| format!("{}={}", tool.name, tool.exit_code))
        .collect::<Vec<_>>()
        .join(" ");
    match inputs.tools.iter().find(|tool| is_tool_failure(tool)) {
        Some(failed) => {
            evidence.push(AdmissionEvidence {
                source: AdmissionEvidenceSource::ToolExitCodes,
                observed: observed_tools,
                disposition: AdmissionDisposition::Excluding,
            });
            exclusions.push(format!(
                "{} exited with code {}",
                failed.name, failed.exit_code
            ));
        }
        None => evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::ToolExitCodes,
            observed: observed_tools,
            disposition: AdmissionDisposition::Clean,
        }),
    }

    // 4. Preconditions. Mirrors preconditions::refuse_on_violation exactly: any Fail, or
    // any Unavailable on a HeadlineSeries run. Recorded even though a real run normally
    // never reaches this gate with a violation (refuse_on_violation already refused it
    // before touching anything), because a waived run must show what was waived.
    let pass = inputs
        .preconditions
        .iter()
        .filter(|result| result.status == PreconditionStatus::Pass)
        .count();
    let fail = inputs
        .preconditions
        .iter()
        .filter(|result| result.status == PreconditionStatus::Fail)
        .count();
    let not_applicable = inputs
        .preconditions
        .iter()
        .filter(|result| result.status == PreconditionStatus::NotApplicable)
        .count();
    let unavailable = inputs
        .preconditions
        .iter()
        .filter(|result| result.status == PreconditionStatus::Unavailable)
        .count();
    let offending: Vec<&PreconditionResult> = inputs
        .preconditions
        .iter()
        .filter(|result| {
            result.status == PreconditionStatus::Fail
                || (result.status == PreconditionStatus::Unavailable
                    && *inputs.instrument_class == InstrumentClass::HeadlineSeries)
        })
        .collect();
    evidence.push(AdmissionEvidence {
        source: AdmissionEvidenceSource::Preconditions,
        observed: format!(
            "{pass} pass, {fail} fail, {not_applicable} not-applicable, {unavailable} unavailable"
        ),
        disposition: if offending.is_empty() {
            AdmissionDisposition::Clean
        } else {
            AdmissionDisposition::Excluding
        },
    });
    if !offending.is_empty() {
        exclusions.push(format!(
            "preconditions: {} check(s) failed or unavailable: {}",
            offending.len(),
            offending
                .iter()
                .map(|result| format!(
                    "{:?} (observed {:?}, expected {:?})",
                    result.check, result.observed, result.expected
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    // 5. InterferenceCounters. Reads only the counters and the calibrated limit, never
    // the shape of the measured latency: that is the whole point of this gate (D-28).
    let observed_counters = format_counter_deltas(inputs.interference_delta, inputs.run_duration);
    match inputs.counter_thresholds {
        Some(limits) => match interference::counter_breach(
            inputs.interference_delta,
            limits,
            inputs.run_duration,
        ) {
            Some(reason) => {
                evidence.push(AdmissionEvidence {
                    source: AdmissionEvidenceSource::InterferenceCounters,
                    observed: observed_counters,
                    disposition: AdmissionDisposition::Excluding,
                });
                exclusions.push(reason);
            }
            None => evidence.push(AdmissionEvidence {
                source: AdmissionEvidenceSource::InterferenceCounters,
                observed: observed_counters,
                disposition: AdmissionDisposition::Clean,
            }),
        },
        None => evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::InterferenceCounters,
            observed: observed_counters,
            disposition: AdmissionDisposition::NotThresholded,
        }),
    }

    // 6. SmiDelta. No calibrated SMI ceiling exists yet, so this is recorded and never
    // excludes: a deliberate NotThresholded, not a forgotten comparison.
    match &inputs.smi.unavailable_reason {
        Some(reason) => evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::SmiDelta,
            observed: reason.clone(),
            disposition: AdmissionDisposition::Unavailable,
        }),
        None => {
            let observed = inputs
                .smi
                .delta
                .iter()
                .map(|counter| format!("cpu{}={}", counter.cpu, counter.count))
                .collect::<Vec<_>>()
                .join(" ");
            evidence.push(AdmissionEvidence {
                source: AdmissionEvidenceSource::SmiDelta,
                observed,
                disposition: AdmissionDisposition::NotThresholded,
            });
        }
    }

    // 7. ThermalMaximum. THERMAL_HEADROOM_CEILING_C is a start-of-run gate by its own
    // documentation and there is no calibrated mid-run ceiling, so this records and
    // never excludes.
    match inputs.package_temp_c_max {
        Some(temp) => evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::ThermalMaximum,
            observed: format!("{temp:.1} C"),
            disposition: AdmissionDisposition::NotThresholded,
        }),
        None => evidence.push(AdmissionEvidence {
            source: AdmissionEvidenceSource::ThermalMaximum,
            observed: "not recorded".to_string(),
            disposition: AdmissionDisposition::Unavailable,
        }),
    }

    SeriesAdmission {
        admitted: exclusions.is_empty(),
        exclusions,
        evidence,
    }
}

/// The `exclusion_reason` for a `calibration-contaminated` run taken with
/// `--allow-precondition-violation`: names every one of the 15 checks that did
/// not pass, with its real observed and expected value, so the published
/// manifest says why the run is excluded rather than only that it is. `results`
/// is the full, unfiltered precondition list (see `execute`, step 2); nothing
/// here changes what was recorded, only how the reason is worded.
fn precondition_waiver_reason(results: &[PreconditionResult]) -> String {
    let offending: Vec<String> = results
        .iter()
        .filter(|r| {
            r.status == PreconditionStatus::Fail || r.status == PreconditionStatus::Unavailable
        })
        .map(|r| {
            format!(
                "{:?} (observed {:?}, expected {:?})",
                r.check, r.observed, r.expected
            )
        })
        .collect();

    if offending.is_empty() {
        "excluded_from_series forced true: --allow-precondition-violation was given for this \
         calibration-contaminated run, though no precondition actually failed"
            .to_string()
    } else {
        format!(
            "excluded_from_series forced true: --allow-precondition-violation waived {} \
             precondition violation(s) for this calibration-contaminated run: {}",
            offending.len(),
            offending.join("; ")
        )
    }
}

/// The environment variable names of every fixture seam active for this invocation,
/// in a fixed order (facts before interrupts) so `RunManifest::fixtures_used` and any
/// message built from it never varies between two runs of the same invocation. Empty
/// for a real measurement.
fn fixtures_used_names(overrides: &Overrides) -> Vec<String> {
    let mut names = Vec::new();
    if overrides.facts_fixture_path.is_some() {
        names.push(FACTS_FIXTURE_ENV.to_string());
    }
    if overrides.interrupts_fixture_path.is_some() {
        names.push(INTERRUPTS_FIXTURE_ENV.to_string());
    }
    if overrides.smi_fixture_path.is_some() {
        names.push(SMI_FIXTURE_ENV.to_string());
    }
    names
}

/// Explains, for a human reading the manifest, why a fixture-driven run's
/// interference deltas cannot be read as evidence of a quiet machine: one fixture
/// text is read repeatedly for every snapshot, so every delta computed from it is
/// exactly zero by construction, never because the machine was actually observed to
/// be quiet. Used both as `RunManifest::exclusion_reason` and as the contamination
/// verdict's own recorded reason, so the two never disagree. Finding 8 of
/// `01-EXTERNAL-AUDIT.md`.
fn fixture_usage_reason(fixtures_used: &[String]) -> String {
    format!(
        "excluded_from_series forced true: this run was driven by fixture data ({}) rather \
         than the real machine; a fixture text is read repeatedly for every interference \
         snapshot, so every delta it produces is exactly zero, which must never be read as \
         evidence of a quiet machine",
        fixtures_used.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIOLATED_FACTS: &str =
        include_str!("../../../capture/tests/fixtures/violated-sysfs-tuning.txt");
    const CLEAN_FACTS: &str =
        include_str!("../../../capture/tests/fixtures/probe-sysfs-tuning.txt");
    const INTERRUPTS: &str =
        include_str!("../../../capture/tests/fixtures/probe-proc-interrupts.txt");

    /// `probe-sysfs-tuning.txt` carries only the sysfs tuning probe; it has no
    /// `/proc/cpuinfo`, `/proc/meminfo`, `/proc/cmdline`, or `/etc/os-release`
    /// data at all. Appended to a fixture (via `tuned_facts_text`) so the D-14
    /// host/kernel/os fields are genuinely populated rather than empty, using the
    /// `@begin`/`@end` extension `extract_multiline_blocks` understands for the
    /// two values that do not fit on one line.
    const D14_ENVIRONMENT_FIXTURE: &str = "\
/proc/meminfo=MemTotal:       31457280 kB
/proc/cmdline=BOOT_IMAGE=/vmlinuz-7.0.0-30-realtime root=UUID=1111-2222 ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5,12-21
/proc/sys/kernel/osrelease=7.0.0-30-realtime
/proc/sys/kernel/version=#30-Ubuntu SMP PREEMPT_RT Fri Jul 31 18:22:54 UTC 2026
@begin /proc/cpuinfo
processor\t: 0
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) Ultra 9 185H
physical id\t: 0
core id\t\t: 0
microcode\t: 0x28

processor\t: 1
vendor_id\t: GenuineIntel
model name\t: Intel(R) Core(TM) Ultra 9 185H
physical id\t: 0
core id\t\t: 1
microcode\t: 0x28
@end
@begin /etc/os-release
NAME=\"Ubuntu\"
VERSION=\"26.04.1 LTS\"
@end
";

    const FAKE_HIST: &str = "# Histogram\n\
000001 000005\n\
000002 000003\n\
# Min Latencies: 00001\n\
# Avg Latencies: 00001\n\
# Max Latencies: 00002\n\
# Histogram Overflows: 00000\n\
# Histogram Overflow at cycle number:\n\
# Thread 0: \n";

    fn base_args(measurements_root: PathBuf) -> Args {
        Args {
            rig_slug: "precision3591".to_string(),
            // Not Headline/Weekly/Soak: those refuse a fixture-backed facts
            // source outright (fixture_facts_refused_for_publishable_classes,
            // task 3), which is not what these two tests are exercising.
            class: RunClassArg::Recon,
            instrument: InstrumentClassArg::HeadlineSeries,
            cpus: "6-11".to_string(),
            main_cpus: "0,1".to_string(),
            duration: 1,
            interval: 200,
            histogram_max: 400,
            priority: 99,
            with_hwlatdetect: false,
            hwlatdetect_duration: 900,
            hwlatdetect_cpu_list: None,
            with_hwnoise: false,
            hwnoise_cpus: None,
            hwnoise_housekeeping: "0-5".to_string(),
            hwnoise_duration: 900,
            hwnoise_priority: "f:99".to_string(),
            breaktrace: None,
            measurements_root,
            thresholds: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../config/contamination-thresholds.json"),
            note: None,
            dry_run: false,
            allow_precondition_violation: false,
            thermal_profile: ThermalProfileArg::Normal,
        }
    }

    fn write_fixture(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).expect("write fixture");
        path
    }

    /// `CLEAN_FACTS` is the real rig's own recon capture, and the rig has never
    /// actually been observed in a tuned state (docs/rig/recon-2026-08-31/
    /// FINDINGS.md; `crates/capture/tests/preconditions.rs` names this same
    /// fixture `CLEAN` while documenting the same fact). Deriving a genuinely
    /// passing scenario from it, rather than fabricating one from nothing, keeps
    /// this test grounded in the real fixture's structure. Governor, the systemd
    /// default target, `no_turbo` and thermal readings are the only fields this
    /// crate's 15 checks treat as a hard `Fail`; every other field in `CLEAN_FACTS`
    /// already passes or is merely `Unavailable` (tolerated here via
    /// `InstrumentClass::Investigation`, the class this test uses). This includes
    /// `NoActiveLoginSessions`: `CLEAN_FACTS` predates that check and carries no
    /// `login.local_sessions` data, so it too reads `Unavailable` here.
    fn tuned_facts_text() -> String {
        let text = CLEAN_FACTS
            .replace(".governor=powersave", ".governor=performance")
            .replace(
                "systemd.default_target=graphical.target",
                "systemd.default_target=multi-user.target",
            )
            .replace(
                "intel_pstate.no_turbo=unavailable",
                "intel_pstate.no_turbo=1",
            );
        let simple: String = text
            .lines()
            .map(|line| match line.split_once('=') {
                Some((key, _)) if key.starts_with("thermal.") => format!("{key}=40000"),
                _ => line.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!("{simple}\n{D14_ENVIRONMENT_FIXTURE}")
    }

    fn write_fake_cyclictest(dir: &Path) -> PathBuf {
        let path = dir.join("fake-cyclictest.sh");
        let script = format!(
            "#!/bin/sh\nhf=\"\"\njf=\"\"\nfor arg in \"$@\"; do\n  case \"$arg\" in\n    \
             --version) echo 'cyclictest V 2.80'; exit 0 ;;\n    --histfile=*) hf=\"${{arg#--histfile=}}\" ;;\n    \
             --json=*) jf=\"${{arg#--json=}}\" ;;\n  esac\ndone\ncat > \"$hf\" <<'HIST'\n{FAKE_HIST}HIST\nexit 0\n"
        );
        std::fs::write(&path, script).expect("write fake cyclictest");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path).expect("stat").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).expect("chmod");
        }
        path
    }

    /// Every test builds its own [`Overrides`] value and calls [`execute`]
    /// directly, rather than going through the process environment `run()` reads:
    /// no `unsafe` (forbidden workspace-wide) and no shared mutable state to race
    /// against another test.
    fn overrides(
        facts_fixture_path: Option<PathBuf>,
        interrupts_fixture_path: Option<PathBuf>,
        cyclictest_path: PathBuf,
    ) -> Overrides {
        Overrides {
            facts_fixture_path,
            interrupts_fixture_path,
            smi_fixture_path: None,
            cyclictest_path,
            hwlatdetect_path: PathBuf::from("hwlatdetect"),
            rtla_path: PathBuf::from("rtla"),
        }
    }

    #[test]
    fn refusal_writes_nothing() {
        let scratch = tempfile::TempDir::new().expect("tempdir");
        let facts_path = write_fixture(scratch.path(), "facts.txt", VIOLATED_FACTS);
        let fake_cyclictest = write_fake_cyclictest(scratch.path());
        let measurements_root = scratch.path().join("measurements");
        std::fs::create_dir_all(&measurements_root).expect("mkdir");

        // Tool resolution (step 1) happens before preconditions (step 2), so even
        // a refusal-focused test needs a resolvable cyclictest; a real refusal
        // never reaches the point of calling it.
        let result = execute(
            base_args(measurements_root.clone()),
            &overrides(Some(facts_path), None, fake_cyclictest),
        );

        assert_eq!(result.expect("run() returns an exit code, not an error"), 2);
        assert_eq!(
            std::fs::read_dir(&measurements_root)
                .expect("read_dir")
                .count(),
            0,
            "a refused run must not create any run directory"
        );
    }

    #[test]
    fn raw_capture_is_byte_identical() {
        let scratch = tempfile::TempDir::new().expect("tempdir");
        let facts_path = write_fixture(scratch.path(), "facts.txt", &tuned_facts_text());
        let interrupts_path = write_fixture(scratch.path(), "interrupts.txt", INTERRUPTS);
        let fake_cyclictest = write_fake_cyclictest(scratch.path());
        let measurements_root = scratch.path().join("measurements");
        std::fs::create_dir_all(&measurements_root).expect("mkdir");

        let mut args = base_args(measurements_root.clone());
        // Every remaining field CLEAN_FACTS cannot supply (ssh session count,
        // display manager state, and similar) is Unavailable, not Fail; tolerated
        // under Investigation, refused under HeadlineSeries.
        args.instrument = InstrumentClassArg::Investigation;

        let result = execute(
            args,
            &overrides(Some(facts_path), Some(interrupts_path), fake_cyclictest),
        );

        assert_eq!(result.expect("run() should succeed"), 0);

        let run_dirs: Vec<_> = std::fs::read_dir(&measurements_root)
            .expect("read_dir")
            .filter_map(|entry| entry.ok())
            .collect();
        assert_eq!(run_dirs.len(), 1, "exactly one run directory is written");

        let written = std::fs::read(run_dirs[0].path().join("cyclictest.hist"))
            .expect("read the placed capture");
        let expected = FAKE_HIST.as_bytes();
        assert_eq!(
            written, expected,
            "the placed capture must be byte-identical to what the tool emitted"
        );

        let manifest_text = std::fs::read_to_string(run_dirs[0].path().join("manifest.json"))
            .expect("read manifest.json");
        let manifest: RunManifest =
            serde_json::from_str(&manifest_text).expect("manifest.json parses");
        let recorded = manifest
            .artifacts
            .iter()
            .find(|artifact| artifact.path == "cyclictest.hist")
            .expect("cyclictest.hist is recorded");
        assert_eq!(recorded.blake3, blake3::hash(expected).to_hex().to_string());
    }

    #[test]
    fn hwlatdetect_argv_omits_cpu_list_by_default() {
        let mut args = base_args(PathBuf::from("/tmp"));
        args.hwlatdetect_duration = 600;
        assert_eq!(build_hwlatdetect_argv(&args), vec!["--duration=600"]);
    }

    #[test]
    fn hwlatdetect_argv_appends_cpu_list_when_given() {
        let mut args = base_args(PathBuf::from("/tmp"));
        args.hwlatdetect_duration = 600;
        args.hwlatdetect_cpu_list = Some("0-11".to_string());
        assert_eq!(
            build_hwlatdetect_argv(&args),
            vec!["--duration=600", "--cpu-list=0-11"]
        );
    }

    fn invocation(name: &str, exit_code: i32) -> ToolInvocation {
        ToolInvocation {
            name: name.to_string(),
            version: "test".to_string(),
            argv: vec![],
            exit_code,
            artifact_paths: vec![],
        }
    }

    /// One `Pass` result per entry of `PreconditionCheck::ALL` (15 today), the baseline
    /// every `determine_admission` test below starts from and mutates one entry of.
    fn all_pass_preconditions() -> Vec<PreconditionResult> {
        nr_manifest::PreconditionCheck::ALL
            .iter()
            .map(|check| PreconditionResult {
                check: check.clone(),
                status: PreconditionStatus::Pass,
                observed: "ok".to_string(),
                expected: "ok".to_string(),
            })
            .collect()
    }

    fn empty_delta() -> nr_manifest::InterferenceDelta {
        nr_manifest::InterferenceDelta {
            cal_ipis: vec![],
            tlb_ipis: vec![],
            rescheduling_ipis: vec![],
            irqs: vec![],
        }
    }

    fn benign_smi() -> nr_manifest::SmiCounts {
        nr_manifest::SmiCounts {
            register: "0x34".to_string(),
            before: vec![],
            after: vec![],
            delta: vec![],
            unavailable_reason: None,
        }
    }

    /// hwlatdetect exits with `(maxlatency > hardlimit)` and defaults `hardlimit` to the
    /// threshold, so a screen that observes anything above the threshold exits 1 by design.
    /// Treating that as a tool failure excluded both D-18 arms on 2026-09-05 with the reason
    /// "hwlatdetect exited with code 1", which reads as a broken capture rather than the
    /// finding it is. Those two manifests are published and D-12 forbids editing them; this
    /// stops it recurring. Adapted to `determine_admission`'s signature in plan 01-24 (D-28);
    /// the assertions are unchanged.
    #[test]
    fn hwlatdetect_exit_one_is_a_finding_not_a_failure() {
        let admission = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0), invocation("hwlatdetect", 1)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(
            admission.admitted,
            "exit 1 from hwlatdetect must not exclude the run: {:?}",
            admission.exclusions
        );
        assert_eq!(admission.exclusions, Vec::<String>::new());
    }

    /// Any other nonzero exit from hwlatdetect is still a real failure, and every nonzero
    /// exit from any other tool remains one. Adapted to `determine_admission`'s signature in
    /// plan 01-24 (D-28); the assertions are unchanged.
    #[test]
    fn other_nonzero_exits_still_exclude() {
        let admission = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("hwlatdetect", 2)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(!admission.admitted);
        assert!(
            admission
                .exclusions
                .iter()
                .any(|reason| reason.contains("hwlatdetect exited with code 2")),
            "{:?}",
            admission.exclusions
        );

        let admission = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 1)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(!admission.admitted, "cyclictest has no such convention");
        assert!(
            admission
                .exclusions
                .iter()
                .any(|reason| reason.contains("cyclictest exited with code 1")),
            "{:?}",
            admission.exclusions
        );
    }

    /// D-28/01-REVIEW-2026-09-06.md finding A1: a run whose only problem is the shape of
    /// its own latency distribution is admitted. Uses the real, committed 2026-08-28
    /// global-stall capture (the same file `fake-cyclictest.sh` serves by default in
    /// `crates/cli/tests/run_pipeline.rs`) and the real shipped
    /// `config/contamination-thresholds.json`, so both halves of the claim are checked
    /// against production data rather than a hand-picked pair of numbers: the capture
    /// really does score Contaminated under the shipped thresholds, and admission is
    /// unaffected by that regardless. This is the 01-13 case: a clean headline capture
    /// whose `tail_excursion_ratio` happens to land above the provisional limit must
    /// still reach `excluded_from_series: false`.
    #[test]
    fn a_bad_looking_shape_alone_does_not_exclude() {
        use nr_manifest::ContaminationVerdict;

        let hist_path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist"
        ));
        let cyclictest_run =
            parse_hist_file(hist_path, Some(400)).expect("the committed 2026-08-28 capture parses");

        let thresholds_path = Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/contamination-thresholds.json"
        ));
        let thresholds = interference::Thresholds::load(thresholds_path)
            .expect("the shipped thresholds file loads");

        let snapshot = nr_manifest::InterferenceSnapshot {
            isolated_cpus: vec![6, 7, 8, 9, 10, 11],
            cal_ipis: vec![],
            tlb_ipis: vec![],
            rescheduling_ipis: vec![],
            irqs: vec![],
        };
        let outcome = interference::verdict(
            snapshot.clone(),
            snapshot,
            &cyclictest_run,
            &thresholds,
            Duration::from_secs(600),
        )
        .expect("the D-24 tail signal computes over a real capture");
        assert_eq!(
            outcome.pair.verdict,
            ContaminationVerdict::Contaminated,
            "the 2026-08-28 capture is this project's own on-record global-stall shape; if \
             this assertion ever fails, the fixture or the shipped thresholds changed and \
             this test's premise needs re-checking, not the admission gate"
        );

        let admission = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &outcome.pair.delta,
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(
            admission.admitted,
            "a bad-looking shape alone must not exclude: {:?}",
            admission.exclusions
        );
        assert_eq!(admission.exclusions, Vec::<String>::new());
        let excluded_from_series = !admission.admitted;
        assert!(
            !excluded_from_series,
            "excluded_from_series must be false while interference.verdict is Contaminated"
        );
    }

    /// A run with one `Fail` precondition is excluded, and the exclusion text names that
    /// check and its observed value.
    #[test]
    fn a_failed_precondition_excludes_and_names_the_check() {
        let mut preconditions = all_pass_preconditions();
        preconditions[0] = PreconditionResult {
            check: nr_manifest::PreconditionCheck::NoActiveSshSessions,
            status: PreconditionStatus::Fail,
            observed: "2 sessions".to_string(),
            expected: "0 sessions".to_string(),
        };

        let admission = determine_admission(&AdmissionInputs {
            preconditions: &preconditions,
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(!admission.admitted);
        assert!(
            admission
                .exclusions
                .iter()
                .any(|reason| reason.contains("NoActiveSshSessions")
                    && reason.contains("2 sessions")),
            "exclusion text should name the failed check and its observed value: {:?}",
            admission.exclusions
        );
    }

    /// The same result with status `Unavailable` excludes a `headline-series` run and
    /// does not exclude an `investigation` run, mirroring
    /// `preconditions::refuse_on_violation` exactly.
    #[test]
    fn an_unavailable_precondition_excludes_a_headline_run_only() {
        let mut preconditions = all_pass_preconditions();
        preconditions[0] = PreconditionResult {
            check: nr_manifest::PreconditionCheck::NoActiveSshSessions,
            status: PreconditionStatus::Unavailable,
            observed: "could not read".to_string(),
            expected: "0 sessions".to_string(),
        };

        let headline = determine_admission(&AdmissionInputs {
            preconditions: &preconditions,
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(
            !headline.admitted,
            "an unavailable check must exclude a headline-series run: {:?}",
            headline.exclusions
        );

        let investigation = determine_admission(&AdmissionInputs {
            preconditions: &preconditions,
            instrument_class: &InstrumentClass::Investigation,
            tools: &[invocation("cyclictest", 0)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(
            investigation.admitted,
            "an investigation run may proceed with an unavailable check: {:?}",
            investigation.exclusions
        );
    }

    /// `cyclictest` exit 1 excludes; `hwlatdetect` exit 1 does not (the existing
    /// convention, unchanged).
    #[test]
    fn a_tool_failure_still_excludes() {
        let cyclictest_failed = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 1)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(
            !cyclictest_failed.admitted,
            "cyclictest exit 1 must exclude"
        );

        let hwlatdetect_finding = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0), invocation("hwlatdetect", 1)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(
            hwlatdetect_finding.admitted,
            "hwlatdetect exit 1 is a finding, not a failure: {:?}",
            hwlatdetect_finding.exclusions
        );
    }

    /// With a `Calibrated` threshold set, a CAL delta above the per-run-hour limit
    /// excludes, and the text names the counter, the CPU and the limit.
    #[test]
    fn calibrated_counter_breach_excludes() {
        let limits = interference::CalibratedThresholds {
            cal_delta_max: 5.0,
            tlb_delta_max: 5.0,
            res_delta_max: 5.0,
            device_irq_delta_max: 5.0,
            context_switch_delta_max: 5.0,
            derived_from: vec!["a".to_string(), "b".to_string()],
        };
        let delta = nr_manifest::InterferenceDelta {
            cal_ipis: vec![nr_manifest::CpuCounter { cpu: 9, count: 100 }],
            tlb_ipis: vec![],
            rescheduling_ipis: vec![],
            irqs: vec![],
        };

        let admission = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &delta,
            counter_thresholds: Some(&limits),
            run_duration: Duration::from_secs(3600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(!admission.admitted);
        assert!(
            admission
                .exclusions
                .iter()
                .any(|reason| reason.contains("CAL") && reason.contains("cpu9")),
            "exclusion text should name the counter and the CPU: {:?}",
            admission.exclusions
        );
    }

    /// With the shipped provisional file (`counter_thresholds: None`), the
    /// interference-counter evidence entry is `NotThresholded` and does not exclude,
    /// however large the observed delta is: there is no calibrated ceiling to judge it
    /// against yet, and the magnitude alone must not stand in for one.
    #[test]
    fn uncalibrated_counters_are_recorded_not_thresholded() {
        let delta = nr_manifest::InterferenceDelta {
            cal_ipis: vec![nr_manifest::CpuCounter {
                cpu: 9,
                count: 1_000_000,
            }],
            tlb_ipis: vec![],
            rescheduling_ipis: vec![],
            irqs: vec![],
        };

        let admission = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &delta,
            counter_thresholds: None,
            run_duration: Duration::from_secs(3600),
            smi: &benign_smi(),
            package_temp_c_max: Some(45.0),
        });
        assert!(
            admission.admitted,
            "no calibrated ceiling exists yet, so a large delta must not exclude: {:?}",
            admission.exclusions
        );
        let counters_evidence = admission
            .evidence
            .iter()
            .find(|entry| entry.source == AdmissionEvidenceSource::InterferenceCounters)
            .expect("InterferenceCounters evidence is always recorded");
        assert_eq!(
            counters_evidence.disposition,
            AdmissionDisposition::NotThresholded
        );
    }

    /// D-28: a compile-level guarantee. This test constructs `AdmissionInputs` and calls
    /// `determine_admission` with no `ContaminationVerdict` or `TailMetrics` value
    /// anywhere in scope; if the gate ever needed either, this test would not compile.
    #[test]
    fn admission_never_reads_the_verdict() {
        let admission = determine_admission(&AdmissionInputs {
            preconditions: &all_pass_preconditions(),
            instrument_class: &InstrumentClass::HeadlineSeries,
            tools: &[invocation("cyclictest", 0)],
            fixtures_used: &[],
            precondition_waiver: None,
            interference_delta: &empty_delta(),
            counter_thresholds: None,
            run_duration: Duration::from_secs(3600),
            smi: &benign_smi(),
            package_temp_c_max: None,
        });
        assert!(
            admission.admitted,
            "no evidence source here excludes: {:?}",
            admission.exclusions
        );
    }

    /// T-1-62: a home-directory path and the operator's username, however they
    /// appear inside free-form stderr text (not just at the start of a token,
    /// unlike `redact_home_prefix`), are both replaced with the visible
    /// `[redacted]` token.
    #[test]
    fn redact_stderr_scrubs_a_home_path_and_the_operator_username() {
        let user = std::env::var("USER").unwrap_or_else(|_| "test-operator".to_string());
        let text = format!(
            "cyclictest: warning: could not open /home/{user}/.cache/foo, running as {user} anyway\n"
        );
        let redacted = redact_stderr(&text);
        assert!(
            !redacted.contains("/home/"),
            "the home-directory prefix must not survive redaction: {redacted:?}"
        );
        if std::env::var("USER").is_ok() {
            assert!(
                !redacted.contains(&user),
                "the operator's username must not survive redaction: {redacted:?}"
            );
        }
        assert!(redacted.contains("[redacted]"), "redacted: {redacted:?}");
    }

    #[test]
    fn redact_stderr_is_a_no_op_on_text_with_nothing_to_redact() {
        let text = "cyclictest: no errors\n";
        assert_eq!(redact_stderr(text), text);
    }
}
