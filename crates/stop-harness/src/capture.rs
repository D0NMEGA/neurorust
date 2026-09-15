//! The run-admission half of the STOP-07 pipeline (preconditions, the run directory, and the
//! refusal path), extended with the evidence half (harness identity, artifact checksums, the
//! manifest, and the metrics entry).
//!
//! Every check here reuses `nr_capture::preconditions::run_all`, not a second list: a STOP-07
//! run is refused on exactly the same fifteen D-06 checks `nrmeasure run` evaluates, over the two
//! cores this run actually uses rather than the whole isolated set (a stricter, more honest
//! statement; see [`PreconditionSpec`] construction in `main.rs`).
//!
//! ## The fixture seam
//!
//! [`NR_STOP_FACTS_FIXTURE`] is the one environment variable that switches this harness from the
//! real rig to a deterministic fixture, parallel to `nrmeasure run`'s own `NRMEASURE_FACTS_FIXTURE`
//! (`crates/cli/src/cmd/run.rs`). Setting it is a single, coherent switch, not four independent
//! ones: `main.rs` also reads it to choose [`crate::clock::FixtureClock`] over
//! [`crate::clock::RawClock`], a deterministic zero-delta interference snapshot over a live
//! `/proc/interrupts` read, and `TrialConfig::require_realtime_scheduling: false` over `true` (the
//! D-36 guarantee 02-06 already made unconditional and unreachable from any CLI flag on every
//! other path). A run taken this way can never be mistaken for a rig capture through two
//! independent, always-applied mechanisms: `gather_facts` never reads the real machine while a
//! fixture path is supplied (the guard: the live path this function would otherwise take is a
//! platform-gated, Linux-only read that a fixture-driven call never reaches, so a fixture run
//! never touches, and therefore never risks misreporting, real sysfs state), and
//! [`RunManifest::fixtures_used`] always names this variable when it drove the run (the record).
//! The guard and the record are both needed: the guard is what stops a fixture text from being
//! silently blended with a real observation inside one run, and the record is what keeps a
//! fixture-driven manifest honest about its own provenance even if a future caller relaxes
//! something else. Operationally, `scripts/nr-run-measurement` (this plan's task 3) never exports
//! this variable, so the sanctioned rig entry point can never set it by accident; only this
//! crate's own test suite and a deliberate manual invocation ever do.
use std::path::{Path, PathBuf};

use nr_capture::interference;
use nr_capture::preconditions::{self, PreconditionSpec, RefusalError};
use nr_capture::sources::{FixtureFacts, SystemFacts};
use nr_manifest::{
    ArtifactKind, ArtifactRecord, AttemptFailure, AttemptRecord, AttemptStatus, ChecksumError,
    ContaminationVerdict, GitShaSource, HarnessInfo, InstrumentClass, InterferenceDelta,
    InterferenceSnapshot, InterferenceSnapshotPair, PreconditionResult, ProvenanceTier,
    RequestedRun, RunClass, RunManifest, StorageLocation, ThermalProfile, ToolInvocation,
};
use nr_metrics::series::StageMetrics;
use thiserror::Error;
use time::OffsetDateTime;

/// Parallel to `nrmeasure run`'s `NRMEASURE_FACTS_FIXTURE`. Holds a path to a fixture facts text
/// file; see the module doc for the full fixture-mode contract this variable's presence drives.
pub const NR_STOP_FACTS_FIXTURE: &str = "NR_STOP_FACTS_FIXTURE";

/// The stage string `crates/metrics/tests/series.rs::stage_names_are_open` already round-trips
/// under D-04. Not `emergency_stop.clock_characterisation` or similar: the characterisation run
/// carries no percentile-shaped latency figure of its own and gets no metrics entry (D-35 is
/// published beside the abort-latency figure, not as a second stage in the series).
pub const METRICS_STAGE: &str = "emergency_stop.abort_latency";
/// The tool string the same committed test round-trips.
pub const METRICS_TOOL: &str = "nr-stop-harness";

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error(
        "live system facts are only available on Linux; set {NR_STOP_FACTS_FIXTURE} to run \
         against a fixture on this host"
    )]
    LinuxOnly,
    #[error(transparent)]
    Facts(#[from] nr_capture::sources::FactsError),
    #[error(transparent)]
    Interference(#[from] interference::InterferenceError),
    #[error("failed to serialise {what}: {source}")]
    Serialize {
        what: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("failed to write {path}: {source}", path = .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to stat {path}: {source}", path = .path.display())]
    Stat {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to checksum {path}: {source}", path = .path.display())]
    Checksum {
        path: PathBuf,
        #[source]
        source: ChecksumError,
    },
}

// ---------------------------------------------------------------------------------------------
// Run admission: facts, preconditions, the refusal path.
// ---------------------------------------------------------------------------------------------

/// With `None`, reads the real machine (Linux only). With `Some(text)`, parses `text` as a
/// fixture facts document (plan 01-02's `key=value` format); `text` is the fixture file's own
/// content, already read by the caller, not a path.
pub fn gather_facts(fixture: Option<&str>) -> Result<Box<dyn SystemFacts>, CaptureError> {
    match fixture {
        Some(text) => Ok(Box::new(FixtureFacts::parse(text))),
        None => live_facts(),
    }
}

#[cfg(target_os = "linux")]
fn live_facts() -> Result<Box<dyn SystemFacts>, CaptureError> {
    Ok(Box::new(nr_capture::sources::live()?))
}

#[cfg(not(target_os = "linux"))]
fn live_facts() -> Result<Box<dyn SystemFacts>, CaptureError> {
    Err(CaptureError::LinuxOnly)
}

/// Runs all fifteen D-06 checks and refuses on any violation, or (for a `HeadlineSeries`
/// instrument class, which every STOP-07 run declares) any check that could not be evaluated.
/// The `Ok` and the `Err` both carry evidence rather than a bare verdict: `Ok` returns every
/// result, and an `Err`'s [`RefusalError::offenses`] names only what actually violated or could
/// not be read, following `nr_capture::preconditions::refuse_on_violation`'s own contract
/// unchanged. A caller that also wants the full fifteen alongside a refusal (so a written record
/// shows what the harness saw, not only what stopped it) calls
/// [`nr_capture::preconditions::run_all`] itself, exactly as [`format_precondition_results`]'s
/// callers do.
pub fn check_preconditions(
    facts: &dyn SystemFacts,
    spec: &PreconditionSpec,
) -> Result<Vec<PreconditionResult>, RefusalError> {
    let results = preconditions::run_all(facts, spec);
    preconditions::refuse_on_violation(&results, &spec.instrument_class)?;
    Ok(results)
}

/// Renders every one of `results` (all fifteen, whatever their status), not only the offending
/// ones: a refused run's own failure record names what the harness saw, matching this task's own
/// required behavior that both the pass and the refusal branch keep the full set.
pub fn format_precondition_results(results: &[PreconditionResult]) -> String {
    let mut out = String::from("all fifteen precondition results (not only the violations):\n");
    for result in results {
        out.push_str(&format!(
            "  {:?}: {:?} (observed {:?}, expected {:?})\n",
            result.check, result.status, result.observed, result.expected
        ));
    }
    out
}

/// Writes `ATTEMPT.json` into `run_root` (an already-created run directory) for a refused or
/// otherwise-failed run, following `nr_manifest::attempt`'s shape exactly: `run_id` is `run_root`'s
/// own directory name, `utc_start`/`utc_end` are both the moment of the failure (a refusal or an
/// early error has no separate measurement interval to bracket), `status` is `Failed`, `tools`
/// and `preserved` are empty (nothing was invoked or captured yet), and
/// `usable_for_numerical_analysis` is `false`.
pub fn write_attempt(
    run_root: &Path,
    requested: &RequestedRun,
    failure: &AttemptFailure,
) -> Result<PathBuf, CaptureError> {
    let run_id = run_root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let now = OffsetDateTime::now_utc();
    let record = AttemptRecord {
        schema_version: nr_manifest::ATTEMPT_SCHEMA_VERSION,
        run_id,
        utc_start: now,
        utc_end: Some(now),
        status: AttemptStatus::Failed,
        harness: harness_info(),
        requested: requested.clone(),
        tools: Vec::new(),
        preserved: Vec::new(),
        failure: Some(failure.clone()),
        usable_for_numerical_analysis: false,
    };
    let path = run_root.join("ATTEMPT.json");
    let json = serde_json::to_string_pretty(&record).map_err(|source| CaptureError::Serialize {
        what: "ATTEMPT.json",
        source,
    })?;
    std::fs::write(&path, json).map_err(|source| CaptureError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

// ---------------------------------------------------------------------------------------------
// Harness identity (T-1-10, copying the rule `crates/cli` follows).
// ---------------------------------------------------------------------------------------------

/// Identifies this build by its executable's blake3 and byte count, never by its absolute path:
/// the rig's own path sits under a home directory and manifests are published (T-1-06). The
/// three `env!` values are embedded at compile time by `build.rs` (a declared copy of
/// `crates/cli/build.rs`); on the rig, `git_sha_source` will read `pushed-stamp` (D-29), because
/// the rig's checkout is an rsync mirror with no `.git` for a build there to read.
pub fn harness_info() -> HarnessInfo {
    let (executable_blake3, executable_bytes) = match std::env::current_exe() {
        Ok(path) => (
            nr_manifest::blake3_file(&path).ok(),
            std::fs::metadata(&path).ok().map(|metadata| metadata.len()),
        ),
        Err(_) => (None, None),
    };
    HarnessInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: env!("NR_BUILD_GIT_SHA").to_string(),
        git_dirty: env!("NR_BUILD_GIT_DIRTY") == "true",
        git_sha_source: Some(GitShaSource::from_build_env(env!(
            "NR_BUILD_GIT_SHA_SOURCE"
        ))),
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

// ---------------------------------------------------------------------------------------------
// Interference snapshot (D-15's evidence, not a contamination verdict this harness computes).
// ---------------------------------------------------------------------------------------------

/// A deterministic, always-valid `/proc/interrupts`-shaped text: a header naming 24 CPU columns
/// (more than the rig's 22 logical CPUs, so any plausible `--hot-cpu`/`--abort-cpu` or
/// `--cpu-a`/`--cpu-b` value is in range) and zero counts on every tracked row. Used for both the
/// before and the after snapshot in fixture mode, so the resulting delta is exactly zero by
/// construction, following the same convention `nrmeasure run`'s own `NRMEASURE_INTERRUPTS_FIXTURE`
/// seam uses (`crates/cli/src/cmd/run.rs`): a fixture-derived zero must never be read as an
/// observation of a quiet machine.
fn fixture_interrupts_text() -> String {
    const CPU_COUNT: usize = 24;
    let header = format!(
        "    {}",
        (0..CPU_COUNT)
            .map(|cpu| format!("CPU{cpu}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    let zero_row =
        |label: &str| -> String { format!("{label}: {}", vec!["0"; CPU_COUNT].join(" ")) };
    format!(
        "{header}\n{}\n{}\n{}\n",
        zero_row("CAL"),
        zero_row("TLB"),
        zero_row("RES")
    )
}

/// Takes an interference snapshot over `cpus`: the fixture text above when `fixture_mode` is
/// true (every platform), the live `/proc/interrupts` read otherwise (Linux only).
pub fn take_interference_snapshot(
    cpus: &[u32],
    fixture_mode: bool,
) -> Result<InterferenceSnapshot, CaptureError> {
    if fixture_mode {
        Ok(interference::snapshot_from_text(
            &fixture_interrupts_text(),
            cpus,
        )?)
    } else {
        live_interference_snapshot(cpus)
    }
}

#[cfg(target_os = "linux")]
fn live_interference_snapshot(cpus: &[u32]) -> Result<InterferenceSnapshot, CaptureError> {
    Ok(interference::snapshot(cpus)?)
}

#[cfg(not(target_os = "linux"))]
fn live_interference_snapshot(_cpus: &[u32]) -> Result<InterferenceSnapshot, CaptureError> {
    Err(CaptureError::LinuxOnly)
}

/// The after-minus-before difference for each counter family, saturating at zero. A local
/// duplicate of `nr_capture::interference`'s own private `compute_delta`, for the same reason
/// `crates/cli/src/cmd/run.rs::window_delta` already is one: the real function is private to
/// that crate's own `verdict` computation, and this is the only caller outside it.
pub fn interference_delta(
    before: &InterferenceSnapshot,
    after: &InterferenceSnapshot,
) -> InterferenceDelta {
    fn diff(
        before: &[nr_manifest::CpuCounter],
        after: &[nr_manifest::CpuCounter],
    ) -> Vec<nr_manifest::CpuCounter> {
        let before_map: std::collections::BTreeMap<u32, u64> = before
            .iter()
            .map(|counter| (counter.cpu, counter.count))
            .collect();
        after
            .iter()
            .map(|counter| nr_manifest::CpuCounter {
                cpu: counter.cpu,
                count: counter
                    .count
                    .saturating_sub(before_map.get(&counter.cpu).copied().unwrap_or(0)),
            })
            .collect()
    }
    InterferenceDelta {
        cal_ipis: diff(&before.cal_ipis, &after.cal_ipis),
        tlb_ipis: diff(&before.tlb_ipis, &after.tlb_ipis),
        rescheduling_ipis: diff(&before.rescheduling_ipis, &after.rescheduling_ipis),
        irqs: diff(&before.irqs, &after.irqs),
    }
}

// ---------------------------------------------------------------------------------------------
// Artifacts.
// ---------------------------------------------------------------------------------------------

/// Checksums a capture file already written at its final location inside `run_dir` (this
/// harness generates every capture directly into place; there is no scratch directory to copy
/// out of) and returns its [`ArtifactRecord`]. Always `kind: ArtifactKind::Other`: every raw
/// capture this harness produces (the abort-latency trial TSV, the clock-characterisation TSV)
/// is this harness's own format, not one of the cyclictest/hwlatdetect/rtla shapes the other
/// variants name, and `ArtifactKind::Other` already exists for exactly this case, so no variant
/// is added to `nr-manifest`. Always `StorageLocation::InRepo`: the STOP-07 captures measured in
/// 02-06 (roughly 52 bytes per trial row) stay far under the 25 MiB/100 MiB in-repo policy
/// `crates/cli/src/rundir.rs` enforces for the larger ftrace captures that policy exists for, so
/// this harness does not replicate that branch.
pub fn record_artifact(path: &Path, run_dir: &Path) -> Result<ArtifactRecord, CaptureError> {
    let kind = ArtifactKind::Other;
    let bytes = std::fs::metadata(path)
        .map_err(|source| CaptureError::Stat {
            path: path.to_path_buf(),
            source,
        })?
        .len();
    let blake3 = nr_manifest::blake3_file(path).map_err(|source| CaptureError::Checksum {
        path: path.to_path_buf(),
        source,
    })?;
    let relative = path.strip_prefix(run_dir).unwrap_or(path);
    Ok(ArtifactRecord {
        path: relative.to_string_lossy().into_owned(),
        bytes,
        blake3,
        kind,
        stored: StorageLocation::InRepo,
    })
}

// ---------------------------------------------------------------------------------------------
// The manifest.
// ---------------------------------------------------------------------------------------------

/// Everything [`build_manifest`] needs, gathered by `main.rs` from the pipeline's own stages.
pub struct ManifestInputs {
    pub run_id: String,
    pub run_class: RunClass,
    pub utc_start: OffsetDateTime,
    pub utc_end: OffsetDateTime,
    pub harness: HarnessInfo,
    pub preconditions: Vec<PreconditionResult>,
    pub interference: InterferenceSnapshotPair,
    pub tools: Vec<ToolInvocation>,
    pub artifacts: Vec<ArtifactRecord>,
    pub env: nr_capture::environment::EnvironmentSnapshot,
    pub fixtures_used: Vec<String>,
}

/// D-37's exclusion reason, in the manifest's own words: this is a statement about scope, never
/// about this run's quality. `excluded_from_series` means "not admitted to the headline
/// regression series in this phase", not "this result looks bad".
pub fn exclusion_reason_text() -> String {
    "excluded_from_series: true per D-37. The emergency_stop.abort_latency stage is not wired \
     into the weekly series or the regression baseline in Phase 2, because the measured hot path \
     is the D-33 stand-in rather than the real runtime and a weekly series taken against the \
     stand-in would track the stand-in, not the runtime. Wiring this stage into the weekly job is \
     deferred to Phase 5 or 6, when the hot path is real. This is a statement about scope, not \
     about this run's quality."
        .to_string()
}

/// Assembles a completed run's [`RunManifest`]. `instrument_class` is always `HeadlineSeries`
/// (D-06: it is what makes an `Unavailable` precondition a refusal rather than a pass, for both
/// run classes this harness produces). `thermal_profile` is always `Normal`: this harness never
/// takes a declared hot screen. `firmware_screens` is empty and `smi_counts` is `None`: this run
/// takes neither, and an empty/absent record is the honest statement of that, not a gap.
/// `series_admission` is `None`; `nr_manifest::validate` only checks it when `Some`, so a
/// harness that computes no D-28 admission record (this one has no weekly-series or
/// regression-baseline concept to admit into at all) leaves it unset rather than fabricating one.
pub fn build_manifest(inputs: ManifestInputs) -> RunManifest {
    RunManifest {
        schema_version: nr_manifest::SCHEMA_VERSION,
        provenance_tier: ProvenanceTier::HarnessGenerated,
        run_id: inputs.run_id,
        run_class: inputs.run_class,
        instrument_class: InstrumentClass::HeadlineSeries,
        thermal_profile: Some(ThermalProfile::Normal),
        utc_start: inputs.utc_start,
        utc_end: inputs.utc_end,
        harness: inputs.harness,
        host: inputs.env.host,
        kernel: inputs.env.kernel,
        os: inputs.env.os,
        tuning: inputs.env.tuning,
        power: inputs.env.power,
        network: inputs.env.network,
        preconditions: inputs.preconditions,
        interference: inputs.interference,
        tools: inputs.tools,
        artifacts: inputs.artifacts,
        firmware_screens: Vec::new(),
        smi_counts: None,
        series_admission: None,
        absent_fields: inputs.env.absent_fields,
        excluded_from_series: true,
        exclusion_reason: Some(exclusion_reason_text()),
        fixtures_used: inputs.fixtures_used,
        notes: None,
    }
}

/// A content fingerprint of `manifest`, matching `nr_metrics::report`'s own `manifest_blake3`
/// computation exactly, so the metrics entry's `manifest_blake3` field and a reader independently
/// hashing the committed `manifest.json` always agree.
pub fn manifest_blake3(manifest: &RunManifest) -> String {
    let bytes = serde_json::to_vec(manifest).expect("a RunManifest always serialises");
    blake3::hash(&bytes).to_hex().to_string()
}

// ---------------------------------------------------------------------------------------------
// The metrics entry.
// ---------------------------------------------------------------------------------------------

/// Everything [`build_metrics_entry`] needs. `nanosecond_percentiles` are the raw D-31 abort
/// observation latencies in nanoseconds, and this function performs the unit conversion to
/// microseconds explicitly (truncating, not rounding: `StageMetrics`'s fields are documented as
/// exact worst cases and exact percentiles, and a truncating `ns / 1000` is the simplest
/// convention that never reports a microsecond value larger than what was actually observed).
pub struct MetricsEntryInputs<'a> {
    pub run_id: String,
    pub run_class: RunClass,
    pub utc_start: OffsetDateTime,
    pub rig_slug: String,
    pub stats: &'a nr_histogram::samples::SampleStats,
    pub population: String,
    pub manifest_blake3: String,
}

fn us_from_ns(ns: u64) -> u64 {
    ns / 1_000
}

fn quantile_us(stats: &nr_histogram::samples::SampleStats, quantile: f64) -> Option<u64> {
    stats
        .values
        .iter()
        .find(|(q, _)| (*q - quantile).abs() < f64::EPSILON)
        .map(|(_, value)| us_from_ns(*value))
}

/// `crates/metrics/tests/series.rs::stage_names_are_open` already round-trips this stage and
/// tool string; this is that test's real producer. `overflow_count` is `0`: there is no overflow
/// concept for a fixed-size trial array (unlike a cyclictest histogram with a fixed bound), so
/// `0` is the true count, not a substitution. `contamination_verdict` is `Uncalibrated`: this
/// harness computes no D-24 tail-metric verdict of its own (that machinery is built over a
/// `CyclictestRun`, which this measurement has none of), and `Uncalibrated` is the schema's own
/// "no verdict exists yet" value rather than a fabricated `Clean`.
pub fn build_metrics_entry(inputs: MetricsEntryInputs<'_>) -> StageMetrics {
    StageMetrics {
        run_id: inputs.run_id,
        run_class: inputs.run_class,
        instrument_class: InstrumentClass::HeadlineSeries,
        utc_start: inputs.utc_start,
        iso_week: iso_week_label(inputs.utc_start),
        rig_slug: inputs.rig_slug,
        tool: METRICS_TOOL.to_string(),
        stage: METRICS_STAGE.to_string(),
        sample_count: inputs.stats.count,
        overflow_count: 0,
        p50_us: quantile_us(inputs.stats, 0.5),
        p95_us: quantile_us(inputs.stats, 0.95),
        p99_us: quantile_us(inputs.stats, 0.99),
        p999_us: quantile_us(inputs.stats, 0.999),
        max_us: us_from_ns(inputs.stats.max),
        population: inputs.population,
        contamination_verdict: ContaminationVerdict::Uncalibrated,
        excluded_from_series: true,
        exclusion_reason: Some(exclusion_reason_text()),
        manifest_blake3: inputs.manifest_blake3,
    }
}

/// `"<year>-W<week>"`, matching `crates/cli/src/cmd/series.rs::iso_week_label`'s own convention
/// exactly (that function is private to `nr-cli`, so this is a small, local duplicate rather than
/// a widened public API for one external caller).
fn iso_week_label(dt: OffsetDateTime) -> String {
    let (year, week, _) = dt.to_iso_week_date();
    format!("{year}-W{week:02}")
}

// ---------------------------------------------------------------------------------------------
// Home-directory redaction (T-1-06), for this harness's own recorded invocation.
// ---------------------------------------------------------------------------------------------

/// Replaces a leading `/home/<user>` or `/Users/<user>` prefix in `value` with the literal token
/// `[redacted]`, following `KernelInfo::redact_cmdline`'s visible-redaction convention. `run_dir`
/// (this run's own, already-known path) is walked for the conventional shape rather than trusting
/// `$HOME` alone: `scripts/nr-run-measurement` (task 3) launches this binary as a root
/// `systemd-run` transient unit with no `User=`/`PAMName=`, whose minimal environment does not
/// reliably set `HOME` to the operator's real home directory. That exact gap let
/// `/home/<user>/...` reach three committed `nrmeasure` manifests before it was caught (plan
/// 01-23, T-1-95, `crates/cli/src/cmd/run.rs::redaction_prefixes`); this harness is deployed the
/// same way and would reproduce the same gap if it trusted `$HOME` alone.
pub fn redact_home_prefix(value: &str, run_dir: &Path) -> String {
    let mut prefixes: Vec<String> = Vec::new();
    if let Some(conventional) = conventional_home_prefix(run_dir) {
        prefixes.push(conventional);
    }
    if let Some(home) = std::env::home_dir() {
        let home = home.to_string_lossy().into_owned();
        if !home.is_empty() && !prefixes.contains(&home) {
            prefixes.push(home);
        }
    }
    prefixes.sort_by_key(|prefix| std::cmp::Reverse(prefix.len()));

    let (flag, rest) = match value.split_once('=') {
        Some((flag, rest)) if flag.starts_with('-') => (Some(flag), rest),
        _ => (None, value),
    };
    let redacted_rest = prefixes
        .iter()
        .find_map(|prefix| rest.strip_prefix(prefix.as_str()))
        .map(|suffix| format!("[redacted]{suffix}"))
        .unwrap_or_else(|| rest.to_string());
    match flag {
        Some(flag) => format!("{flag}={redacted_rest}"),
        None => redacted_rest,
    }
}

/// Walks `path`'s ancestors for one whose own immediate parent is literally named `home` or
/// `Users`, i.e. a path shaped `/home/<user>` or `/Users/<user>`.
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
