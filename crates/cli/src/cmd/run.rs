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
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, ValueEnum};
use nr_capture::sources::{FixtureFacts, SystemFacts, parse_cpu_list};
use nr_capture::{environment, interference, preconditions};
use nr_histogram::hist::{CyclictestRun, parse_hist_file};
use nr_histogram::json::{parse_json_file, reconcile};
use nr_manifest::{
    ArtifactKind, ArtifactPathMapping, ArtifactRecord, ContaminationVerdict, GitShaSource,
    HarnessInfo, InstrumentClass, PreconditionResult, PreconditionStatus, ProvenanceTier, RunClass,
    RunManifest, StorageLocation, ToolInvocation,
};
use nr_metrics::report::render_run_report;
use time::OffsetDateTime;

use crate::rundir::{self, RunDir};
use crate::tools::{self, CYCLICTEST_PATH_ENV, HWLATDETECT_PATH_ENV};

/// A fixture-backed facts source is for tests only. A run that would feed the
/// headline series must read the real machine or it is not a measurement.
const FACTS_FIXTURE_ENV: &str = "NRMEASURE_FACTS_FIXTURE";

/// Test-only companion to [`FACTS_FIXTURE_ENV`]: `interference::snapshot` (unlike
/// the `SystemFacts` trait) reads `/proc/interrupts` directly and is Linux-gated
/// with no facts-source seam of its own (a difference from this plan's own
/// `<interfaces>` block; see the plan summary). Points at a `/proc/interrupts`
/// shaped text file, reused for both the before and after snapshot in a test.
const INTERRUPTS_FIXTURE_ENV: &str = "NRMEASURE_INTERRUPTS_FIXTURE";

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
    #[arg(long)]
    pub hwlatdetect_cpu_list: Option<String>,

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
    cyclictest_path: PathBuf,
    hwlatdetect_path: PathBuf,
}

impl Overrides {
    fn from_env() -> Self {
        Overrides {
            facts_fixture_path: std::env::var(FACTS_FIXTURE_ENV).ok().map(PathBuf::from),
            interrupts_fixture_path: std::env::var(INTERRUPTS_FIXTURE_ENV)
                .ok()
                .map(PathBuf::from),
            cyclictest_path: tools::resolve_tool_path(CYCLICTEST_PATH_ENV, "cyclictest"),
            hwlatdetect_path: tools::resolve_tool_path(HWLATDETECT_PATH_ENV, "hwlatdetect"),
        }
    }
}

pub fn run(args: Args) -> Result<i32> {
    execute(args, &Overrides::from_env())
}

fn execute(args: Args, overrides: &Overrides) -> Result<i32> {
    let run_class: RunClass = args.class.into();
    let instrument_class: InstrumentClass = args.instrument.into();

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

    let target_cpus = parse_cpu_list(&args.cpus);

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

    // Step 2: build facts, run every precondition, refuse before touching
    // anything on a violation.
    let facts = build_facts(overrides.facts_fixture_path.as_deref())?;
    let spec = preconditions::PreconditionSpec {
        instrument_class: instrument_class.clone(),
        run_class: run_class.clone(),
        target_cpus: target_cpus.clone(),
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

    // Step 3: the before interference snapshot, and the pre-run thermal reading.
    let before = take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())?;
    // Taken here, before either instrument runs, so the manifest's temp_c_start is genuinely
    // the start. The D-14 snapshot at step 8 runs after both instruments finish and supplies
    // temp_c_end; reading the zones only there put the end temperature in the start field.
    let thermal_start = nr_capture::sources::discover_thermal_zones_c(facts.as_ref());

    // Step 4: execute cyclictest.
    let scratch = tempfile::tempdir().context("failed to create a scratch directory")?;
    let hist_path = scratch.path().join("cyclictest.hist");
    let json_path = scratch.path().join("cyclictest.json");

    let cyclictest_argv = build_cyclictest_argv(
        &args,
        target_cpus.len(),
        &hist_path.display().to_string(),
        &json_path.display().to_string(),
    );
    let cyclictest_output = tools::run_tool(
        "cyclictest",
        cyclictest_path,
        &cyclictest_version,
        cyclictest_argv,
    )
    .context("failed to execute cyclictest")?;
    warn_on_tool_failure(&cyclictest_output);
    let mut tool_invocations = vec![cyclictest_output.invocation];

    // Step 5: execute hwlatdetect next, never concurrently, if requested (D-09).
    let mut hwlatdetect_raw: Option<Vec<u8>> = None;
    if args.with_hwlatdetect {
        let version = hwlatdetect_version.expect("captured above when with_hwlatdetect is set");
        let hwlatdetect_argv = build_hwlatdetect_argv(&args);
        let hwlatdetect_output =
            tools::run_tool("hwlatdetect", hwlatdetect_path, &version, hwlatdetect_argv)
                .context("failed to execute hwlatdetect")?;
        warn_on_tool_failure(&hwlatdetect_output);
        hwlatdetect_raw = Some(hwlatdetect_output.stdout);
        tool_invocations.push(hwlatdetect_output.invocation);
    }

    // Step 6: parse the raw captures and reconcile them. Moved ahead of the D-15/D-24
    // verdict below (it used to follow it): D-24's tail metrics need the parsed
    // histogram, and nothing in between depends on the other's output, so parsing
    // here costs nothing.
    let cyclictest_run: CyclictestRun = parse_hist_file(&hist_path, Some(args.histogram_max))
        .context("failed to parse cyclictest's .hist output")?;
    if json_path.is_file() {
        let summary =
            parse_json_file(&json_path).context("failed to parse cyclictest's --json output")?;
        reconcile(&summary, &cyclictest_run)
            .context("cyclictest --json and .hist disagree; refusing a mismatched pairing")?;
    }

    // Step 7: the after interference snapshot and the D-15/D-24 verdict.
    let after = take_interference_snapshot(&target_cpus, interrupts_fixture.as_deref())?;
    // `thresholds` was loaded in step 1, before the measurement ran.
    let outcome = interference::verdict(
        before,
        after,
        &cyclictest_run,
        &thresholds,
        Duration::from_secs(args.duration),
    )
    .context("failed to compute the D-15/D-24 contamination verdict")?;

    // Step 8: the D-14 environment snapshot.
    let env_snapshot = environment::snapshot(facts.as_ref(), &args.rig_slug, Some(&thermal_start))
        .context("failed to capture the environment snapshot")?;

    let utc_end = OffsetDateTime::now_utc();

    // Step 9: create the run directory and place the raw captures in unmodified.
    let run_dir = RunDir::create(
        &args.measurements_root,
        &args.rig_slug,
        &run_class,
        utc_start,
    )
    .context("failed to create the run directory")?;

    let mut run_bytes = 0u64;
    let mut artifacts = Vec::new();
    // Paired with each placed artifact's scratch path, so the argv finalization
    // below can name which argv element became which artifact without
    // re-deriving either.
    let mut placed_scratch_paths: Vec<(String, &'static str)> = Vec::new();

    artifacts.push(place_artifact(
        &hist_path,
        &run_dir.path,
        "cyclictest.hist",
        ArtifactKind::CyclictestHist,
        &mut run_bytes,
    )?);
    placed_scratch_paths.push((hist_path.display().to_string(), "cyclictest.hist"));

    if json_path.is_file() {
        artifacts.push(place_artifact(
            &json_path,
            &run_dir.path,
            "cyclictest.json",
            ArtifactKind::CyclictestJson,
            &mut run_bytes,
        )?);
        placed_scratch_paths.push((json_path.display().to_string(), "cyclictest.json"));
    }
    if let Some(raw) = &hwlatdetect_raw {
        let hwlat_path = scratch.path().join("hwlatdetect.txt");
        std::fs::write(&hwlat_path, raw).context("failed to stage the hwlatdetect capture")?;
        artifacts.push(place_artifact(
            &hwlat_path,
            &run_dir.path,
            "hwlatdetect.txt",
            ArtifactKind::HwlatdetectText,
            &mut run_bytes,
        )?);
        placed_scratch_paths.push((hwlat_path.display().to_string(), "hwlatdetect.txt"));
    }
    write_hist_tsv(&run_dir.path, &cyclictest_run)?;

    // The argv is recorded exactly as it was executed (finding 6 of
    // 01-EXTERNAL-AUDIT.md: relativizing it against the run directory matched
    // nothing, because the tools actually wrote into a scratch tempdir under
    // /tmp, not the run directory). artifact_paths maps each output-file path in
    // argv to the committed artifact it became, matched by substring rather than
    // exact element equality because cyclictest's paths arrive as
    // `--histfile=<path>`, one argument, not two. A home directory prefix in
    // either is redacted (T-1-06); the scratch /tmp paths themselves are left
    // verbatim, since they are process-lifetime temporary names that identify
    // nobody.
    let tool_invocations: Vec<ToolInvocation> = tool_invocations
        .into_iter()
        .map(|invocation| {
            let artifact_paths = placed_scratch_paths
                .iter()
                .filter(|(scratch_path, _)| {
                    invocation
                        .argv
                        .iter()
                        .any(|arg| arg.contains(scratch_path.as_str()))
                })
                .map(|(scratch_path, artifact_name)| ArtifactPathMapping {
                    executed_path: redact_home_prefix(scratch_path),
                    artifact_path: artifact_name.to_string(),
                })
                .collect();
            let argv = invocation
                .argv
                .iter()
                .map(|arg| redact_home_prefix(arg))
                .collect();
            ToolInvocation {
                argv,
                artifact_paths,
                ..invocation
            }
        })
        .collect();

    // D-17: a run taken with --allow-precondition-violation is always excluded
    // from the series, unconditionally overriding whatever determine_exclusion
    // would otherwise compute from tool exit codes or the contamination
    // verdict. This is not a default the operator can turn off.
    let (excluded_from_series, exclusion_reason) = if args.allow_precondition_violation {
        (true, Some(precondition_waiver_reason(&results)))
    } else {
        determine_exclusion(
            &tool_invocations,
            &outcome.pair.verdict,
            outcome.reason.as_deref(),
            outcome.pair.thresholds_provisional.unwrap_or(false),
        )
    };

    // Step 10: assemble, validate and write manifest.json (written last).
    rundir::refuse_if_manifest_exists(&run_dir.path)
        .context("run directory was populated between creation and the final write")?;

    let manifest = RunManifest {
        schema_version: nr_manifest::SCHEMA_VERSION,
        provenance_tier: ProvenanceTier::HarnessGenerated,
        run_id: run_dir.run_id.clone(),
        run_class,
        instrument_class,
        utc_start,
        utc_end,
        harness: harness_info(),
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
        absent_fields: env_snapshot.absent_fields,
        excluded_from_series,
        exclusion_reason,
        notes: args.note,
    };

    nr_manifest::validate(&run_dir.path, &manifest).map_err(|errors| {
        anyhow::anyhow!("generated manifest failed its own validation: {errors:?}")
    })?;
    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("failed to serialise the manifest")?;
    std::fs::write(run_dir.path.join("manifest.json"), manifest_json)
        .context("failed to write manifest.json")?;

    // Step 11: render and write REPORT.md, generated from the manifest that was
    // just written, never hand-maintained.
    let report =
        render_run_report(&manifest, &cyclictest_run).context("failed to render REPORT.md")?;
    std::fs::write(run_dir.path.join("REPORT.md"), report).context("failed to write REPORT.md")?;

    // Step 12: print the run directory and the headline numbers.
    print_summary(&run_dir, &cyclictest_run, &manifest)?;

    Ok(0)
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
    Ok(())
}

/// BENCH-06: a tool failure is recorded, not hidden. A non-zero exit does not
/// abort the write (the manifest records it, see [`determine_exclusion`]), but the
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

/// Replaces a leading match of `std::env::home_dir()` in `value` with the literal
/// token `[redacted]`, following the same visible-redaction convention as
/// `KernelInfo::redact_cmdline` and T-1-06's hostname redaction. An executed argv
/// can carry a home directory (`--measurements-root /home/<user>/neurorust/
/// measurements` is passed on every rig invocation by `scripts/nr-run-measurement`,
/// and the scratch directory the tools write into would carry the same exposure if
/// `$TMPDIR` or an equivalent ever pointed under home); a published manifest must
/// never carry it verbatim. Handles both a bare path (e.g. `executed_path`, or a
/// standalone argv token) and a `--flag=/abs/path` token, splitting on the first
/// `=` the same way `KernelInfo::redact_cmdline` does: a naive whole-string match
/// would never fire for a `--histfile=...` token, since the string starts with
/// `--histfile=`, not with the path. A value with no home-directory prefix, for
/// example the scratch `/tmp/.tmpXXXXXX` paths the tools actually write into
/// today, passes through unchanged: those are process-lifetime temporary names
/// and identify nobody.
fn redact_home_prefix(value: &str) -> String {
    if let Some((flag, rest)) = value.split_once('=') {
        if flag.starts_with('-') {
            return format!("{flag}={}", redact_home_prefix_in_path(rest));
        }
    }
    redact_home_prefix_in_path(value)
}

fn redact_home_prefix_in_path(value: &str) -> String {
    let Some(home) = std::env::home_dir() else {
        return value.to_string();
    };
    if home.as_os_str().is_empty() {
        return value.to_string();
    }
    let home = home.to_string_lossy();
    match value.strip_prefix(home.as_ref()) {
        Some(rest) => format!("[redacted]{rest}"),
        None => value.to_string(),
    }
}

/// Copies `src` into `run_dir` under `target_name`, checksums it, and records
/// whether it stayed in-repo or exceeded the size policy (`rundir::
/// exceeds_in_repo_limit`). D-12: the placed copy is byte-identical to what the
/// tool emitted; only the checksum is computed on top of it, nothing is rewritten.
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
    std::fs::copy(src, &dest)
        .with_context(|| format!("failed to place {target_name} into the run directory"))?;
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

/// BENCH-06: a tool failure, a non-clean contamination verdict, or a verdict reached
/// against provisional (not yet calibrated) thresholds does not drop the run. It is
/// retained and published, only marked excluded from the regression series with a
/// reason. `Uncalibrated` and a provisional `Clean`/`Contaminated` (D-24; see
/// `crates/capture/src/interference.rs`'s `Thresholds::Provisional`) are both treated
/// the same way here: a run whose contamination status cannot be judged, or can only
/// be judged against a threshold set derived from two runs, is not a defensible
/// headline figure either, matching this project's rig-discipline stance that an
/// unknown, or an unproven, is never silently treated as clean.
fn determine_exclusion(
    tool_invocations: &[ToolInvocation],
    verdict: &ContaminationVerdict,
    verdict_reason: Option<&str>,
    thresholds_provisional: bool,
) -> (bool, Option<String>) {
    if let Some(failed) = tool_invocations.iter().find(|tool| is_tool_failure(tool)) {
        return (
            true,
            Some(format!(
                "{} exited with code {}",
                failed.name, failed.exit_code
            )),
        );
    }

    if thresholds_provisional {
        return (
            true,
            Some(format!(
                "contamination verdict {verdict:?} was reached against provisional (D-24, not \
                 yet calibrated) thresholds; see config/contamination-thresholds.json"
            )),
        );
    }

    match verdict {
        ContaminationVerdict::Clean => (false, None),
        ContaminationVerdict::Contaminated => (
            true,
            Some(
                verdict_reason
                    .unwrap_or("interference thresholds exceeded")
                    .to_string(),
            ),
        ),
        ContaminationVerdict::Uncalibrated => (
            true,
            Some("no calibrated contamination thresholds exist yet (D-17)".to_string()),
        ),
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
            breaktrace: None,
            measurements_root,
            thresholds: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../config/contamination-thresholds.json"),
            note: None,
            dry_run: false,
            allow_precondition_violation: false,
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
            cyclictest_path,
            hwlatdetect_path: PathBuf::from("hwlatdetect"),
        }
    }

    #[test]
    fn refusal_writes_nothing() {
        let scratch = tempfile::tempdir().expect("tempdir");
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
        let scratch = tempfile::tempdir().expect("tempdir");
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

    /// hwlatdetect exits with `(maxlatency > hardlimit)` and defaults `hardlimit` to the
    /// threshold, so a screen that observes anything above the threshold exits 1 by design.
    /// Treating that as a tool failure excluded both D-18 arms on 2026-09-05 with the reason
    /// "hwlatdetect exited with code 1", which reads as a broken capture rather than the
    /// finding it is. Those two manifests are published and D-12 forbids editing them; this
    /// stops it recurring.
    #[test]
    fn hwlatdetect_exit_one_is_a_finding_not_a_failure() {
        let (excluded, reason) = determine_exclusion(
            &[invocation("cyclictest", 0), invocation("hwlatdetect", 1)],
            &ContaminationVerdict::Clean,
            None,
            false,
        );
        assert!(
            !excluded,
            "exit 1 from hwlatdetect must not exclude the run: {reason:?}"
        );
        assert_eq!(reason, None);
    }

    /// Any other nonzero exit from hwlatdetect is still a real failure, and every nonzero
    /// exit from any other tool remains one.
    #[test]
    fn other_nonzero_exits_still_exclude() {
        let (excluded, reason) = determine_exclusion(
            &[invocation("hwlatdetect", 2)],
            &ContaminationVerdict::Clean,
            None,
            false,
        );
        assert!(excluded);
        assert!(reason.unwrap().contains("hwlatdetect exited with code 2"));

        let (excluded, reason) = determine_exclusion(
            &[invocation("cyclictest", 1)],
            &ContaminationVerdict::Clean,
            None,
            false,
        );
        assert!(excluded, "cyclictest has no such convention");
        assert!(reason.unwrap().contains("cyclictest exited with code 1"));
    }
}
