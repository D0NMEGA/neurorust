//! `nr-stop-harness`: the STOP-07 abort latency harness. Two subcommands, each wrapped in the
//! Phase 1 evidence machinery: `characterise` measures the clock's own read overhead and the
//! cross-core offset between two pinned threads (D-35, `RunClass::Recon`), and `abort-latency`
//! measures abort latency itself (D-31 through D-34, `RunClass::Headline`).
//!
//! Both subcommands share one admission sequence, in [`admit`]: gather facts, evaluate all
//! fifteen D-06 preconditions, and refuse (writing `ATTEMPT.json`) before any thread is pinned or
//! the clock is read. See `capture`'s module doc for the fixture-mode contract
//! [`capture::NR_STOP_FACTS_FIXTURE`] drives across facts, the clock, the interference snapshot,
//! and the scheduling requirement.

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};

use nr_capture::preconditions::PreconditionSpec;
use nr_capture::sources::SystemFacts;
use nr_manifest::{
    AttemptFailure, ContaminationVerdict, InstrumentClass, InterferenceSnapshot,
    InterferenceSnapshotPair, PreconditionResult, RequestedRun, RunClass, ThermalProfile,
    ToolInvocation,
};

use nr_stop_harness::characterise::{cross_core_offset_ns, current_clocksource, read_overhead_ns};
use nr_stop_harness::clock::{FixtureClock, MonotonicRawClock, RawClock};
use nr_stop_harness::trial::{TrialConfig, run_trials};
use nr_stop_harness::{capture, report, rundir};

#[derive(Debug, Parser)]
#[command(
    name = "nr-stop-harness",
    about = "STOP-07 abort latency harness for the neurorust reference rig"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Measures the clock's own read overhead and the cross-core offset between two pinned
    /// threads (D-35), wrapped in the D-06 preconditions and the D-12 manifest. A `RunClass::Recon`
    /// run: not a latency figure, but published beside one, so it is held to the same evidential
    /// bar.
    Characterise {
        /// Consecutive clock reads to time for the read-overhead sample.
        #[arg(long, default_value_t = 1_000_000)]
        iterations: usize,
        /// First cpu of the cross-core offset pair.
        #[arg(long, default_value_t = 7)]
        cpu_a: usize,
        /// Second cpu of the cross-core offset pair.
        #[arg(long, default_value_t = 8)]
        cpu_b: usize,
        /// Ping-pong rounds for the cross-core offset sample.
        #[arg(long, default_value_t = 1000)]
        rounds: usize,
        /// Root to read the running clocksource from. Override for testing; the real rig reads
        /// this from `/sys`.
        #[arg(long, default_value = "/sys")]
        sys_root: PathBuf,
        /// Rig identifier, e.g. precision3591.
        #[arg(long)]
        rig_slug: String,
        /// Root directory for run directories.
        #[arg(long, default_value = "measurements")]
        measurements_root: PathBuf,
    },
    /// Measures abort latency for one poll period (D-31 through D-34) and leaves a validating
    /// run directory: the raw per-trial capture, the D-06/D-14 manifest, a `StageMetrics` entry
    /// and a `REPORT.md`. Run once per period so each gets its own raw capture; plan 02-08 runs
    /// it against the two periods this project measures. A `RunClass::Headline` run.
    AbortLatency {
        /// The node iteration period this run measures, in nanoseconds. No default: the two
        /// periods this project publishes are 33_000 ns, one frame period at the 30 kHz target
        /// rate, and 1_000_000 ns, a 1 kHz node, and this subcommand runs once per period
        /// rather than looping over both internally, so the caller always states which one a
        /// given run and its raw capture belong to.
        #[arg(long)]
        period_ns: u64,
        /// Number of abort trials. A TrialRow renders to roughly 55 bytes of TSV; the in-repo
        /// capture-file limit is 25 MiB and the whole run-directory limit is 100 MiB, so a
        /// single capture file holds roughly 470,000 rows before the run directory logic would
        /// push it behind an external pointer. 200,000 keeps two periods comfortably inside one
        /// run directory with room for the manifest and the report; raise it with that trade in
        /// mind.
        #[arg(long, default_value_t = 200_000)]
        trials: usize,
        /// Core the hot (polling) thread pins to. 7 is inside the rig's isolated set 6-11.
        #[arg(long, default_value_t = 7)]
        hot_cpu: usize,
        /// Core the abort thread pins to. 8 is inside the rig's isolated set 6-11.
        #[arg(long, default_value_t = 8)]
        abort_cpu: usize,
        /// SCHED_FIFO priority for both threads. 80, not 99: the two threads sit on separate
        /// isolated cores and never contend, so nothing is bought by taking the top priority,
        /// and 99 stays free for anything that must preempt.
        #[arg(long, default_value_t = 80)]
        priority: u8,
        /// Seed for the abort-phase sequence, recorded so a run is reproducible.
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Rig identifier, e.g. precision3591.
        #[arg(long)]
        rig_slug: String,
        /// Root directory for run directories.
        #[arg(long, default_value = "measurements")]
        measurements_root: PathBuf,
        /// A prior `characterise` run's own `clock-characterisation.tsv`, read to populate this
        /// report's D-35 decomposition. When omitted, the report states both figures
        /// unavailable rather than a blank or a fabricated zero.
        #[arg(long)]
        characterisation_tsv: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Characterise {
            iterations,
            cpu_a,
            cpu_b,
            rounds,
            sys_root,
            rig_slug,
            measurements_root,
        } => run_characterise(
            iterations,
            cpu_a,
            cpu_b,
            rounds,
            &sys_root,
            &rig_slug,
            &measurements_root,
        ),
        Command::AbortLatency {
            period_ns,
            trials,
            hot_cpu,
            abort_cpu,
            priority,
            seed,
            rig_slug,
            measurements_root,
            characterisation_tsv,
        } => run_abort_latency(
            period_ns,
            trials,
            hot_cpu,
            abort_cpu,
            priority,
            seed,
            &rig_slug,
            &measurements_root,
            characterisation_tsv.as_deref(),
        ),
    }
}

/// What [`admit`] hands back once a run is admitted: the facts source it read from (so the
/// caller need not re-derive fixture-vs-live), every one of the fifteen precondition results,
/// the interference snapshot taken immediately after admission (the "before" half of the D-15
/// bracket), the pre-run thermal reading (for the D-14 environment snapshot's `temp_c_start`),
/// and whether this run is fixture-driven (which the caller threads into the clock choice and
/// `TrialConfig::require_realtime_scheduling`).
struct Admission {
    facts: Box<dyn SystemFacts>,
    preconditions: Vec<PreconditionResult>,
    interference_before: InterferenceSnapshot,
    thermal_start: Vec<(String, f32)>,
    fixture_mode: bool,
}

/// Gathers facts, evaluates all fifteen D-06 preconditions, and refuses (writing `ATTEMPT.json`
/// into `run_dir` and returning `Err`) before anything else happens, on any violation or any
/// check that could not be evaluated. Shared by both subcommands so a refusal happens before any
/// thread is pinned or the clock is read, from either one.
fn admit(
    run_dir: &rundir::RunDir,
    fixture_text: Option<&str>,
    spec: &PreconditionSpec,
    requested: &RequestedRun,
    target_cpus: &[u32],
) -> anyhow::Result<Admission> {
    let facts = capture::gather_facts(fixture_text).context("failed to gather system facts")?;
    let thermal_start = nr_capture::sources::discover_thermal_zones_c(facts.as_ref());

    let results = nr_capture::preconditions::run_all(facts.as_ref(), spec);
    if let Err(refusal) =
        nr_capture::preconditions::refuse_on_violation(&results, &spec.instrument_class)
    {
        let message = format!(
            "{refusal}\n{}",
            capture::format_precondition_results(&results)
        );
        let failure = AttemptFailure {
            stage: "preconditions".to_string(),
            message,
        };
        capture::write_attempt(&run_dir.path, requested, &failure)
            .context("failed to write ATTEMPT.json for the refused run")?;
        anyhow::bail!(
            "run refused: {} of 15 precondition result(s) violated or unavailable; see {}",
            refusal.offenses.len(),
            run_dir.path.join("ATTEMPT.json").display()
        );
    }

    let fixture_mode = fixture_text.is_some();
    let interference_before = capture::take_interference_snapshot(target_cpus, fixture_mode)
        .context("failed to snapshot interference before the measurement")?;

    Ok(Admission {
        facts,
        preconditions: results,
        interference_before,
        thermal_start,
        fixture_mode,
    })
}

/// Reads `capture::NR_STOP_FACTS_FIXTURE`, if set, as a path to a fixture facts text file, and
/// returns its content. `None` when the variable is unset: the normal, real-rig path.
fn read_fixture_text() -> anyhow::Result<Option<String>> {
    let Some(path) = std::env::var(capture::NR_STOP_FACTS_FIXTURE).ok() else {
        return Ok(None);
    };
    let text = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read {} at {path}",
            capture::NR_STOP_FACTS_FIXTURE
        )
    })?;
    Ok(Some(text))
}

fn fixtures_used(fixture_mode: bool) -> Vec<String> {
    if fixture_mode {
        vec![capture::NR_STOP_FACTS_FIXTURE.to_string()]
    } else {
        Vec::new()
    }
}

/// This invocation's own argv, home-directory-redacted (T-1-06), for `tools` (matching the
/// explicit-argv, never-a-shell-string pattern `crates/cli/src/tools.rs` established).
fn self_invocation_argv(run_dir: &Path) -> Vec<String> {
    std::env::args()
        .map(|arg| capture::redact_home_prefix(&arg, run_dir))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn run_characterise(
    iterations: usize,
    cpu_a: usize,
    cpu_b: usize,
    rounds: usize,
    sys_root: &Path,
    rig_slug: &str,
    measurements_root: &Path,
) -> anyhow::Result<()> {
    let target_cpus = vec![cpu_a as u32, cpu_b as u32];
    let run_class = RunClass::Recon;
    let spec = PreconditionSpec {
        instrument_class: InstrumentClass::HeadlineSeries,
        run_class: run_class.clone(),
        target_cpus: target_cpus.clone(),
        thermal_profile: ThermalProfile::Normal,
    };
    let requested = RequestedRun {
        run_class: run_class.clone(),
        instrument_class: InstrumentClass::HeadlineSeries,
        cpus: format!("{cpu_a},{cpu_b}"),
        main_cpus: String::new(),
        duration_seconds: 0,
        with_hwlatdetect: false,
        hwlatdetect_duration_seconds: None,
    };

    let utc_start = time::OffsetDateTime::now_utc();
    let run_dir = rundir::RunDir::create(measurements_root, rig_slug, &run_class, utc_start)
        .context("failed to create the run directory")?;

    let fixture_text = read_fixture_text()?;
    let admission = admit(
        &run_dir,
        fixture_text.as_deref(),
        &spec,
        &requested,
        &target_cpus,
    )?;

    let (overhead_ns, offset_ns) = if admission.fixture_mode {
        let clock = FixtureClock::generated(1_000);
        measure_characterisation(&clock, iterations, cpu_a, cpu_b, rounds)?
    } else {
        let clock = RawClock;
        measure_characterisation(&clock, iterations, cpu_a, cpu_b, rounds)?
    };
    let clocksource = current_clocksource(sys_root);

    let tsv_path = run_dir.path.join("clock-characterisation.tsv");
    std::fs::write(
        &tsv_path,
        format_characterisation_tsv(&overhead_ns, &offset_ns, clocksource.as_deref()),
    )
    .context("failed to write clock-characterisation.tsv")?;
    let artifact = capture::record_artifact(&tsv_path, &run_dir.path)
        .context("failed to checksum clock-characterisation.tsv")?;

    let interference_after =
        capture::take_interference_snapshot(&target_cpus, admission.fixture_mode)
            .context("failed to snapshot interference after the measurement")?;
    let interference = InterferenceSnapshotPair {
        delta: capture::interference_delta(&admission.interference_before, &interference_after),
        before: admission.interference_before,
        after: interference_after,
        tail_metrics: None,
        thresholds_provisional: None,
        verdict: ContaminationVerdict::Uncalibrated,
        windows: Vec::new(),
    };

    let env_snapshot = nr_capture::environment::snapshot(
        admission.facts.as_ref(),
        rig_slug,
        Some(&admission.thermal_start),
    )
    .context("failed to capture the environment snapshot")?;
    let utc_end = time::OffsetDateTime::now_utc();
    let harness = capture::harness_info();
    let tools = vec![ToolInvocation {
        name: "nr-stop-harness".to_string(),
        version: harness.version.clone(),
        argv: self_invocation_argv(&run_dir.path),
        exit_code: 0,
        artifact_paths: Vec::new(),
    }];

    let manifest = capture::build_manifest(capture::ManifestInputs {
        run_id: run_dir.run_id.clone(),
        run_class,
        utc_start,
        utc_end,
        harness,
        preconditions: admission.preconditions,
        interference,
        tools,
        artifacts: vec![artifact],
        env: env_snapshot,
        fixtures_used: fixtures_used(admission.fixture_mode),
    });
    write_manifest(&run_dir.path, &manifest)?;

    let report_text =
        report::render_characterisation_report(&report::CharacterisationReportInput {
            run_id: run_dir.run_id.clone(),
            read_overhead: report::summarise_characterisation(
                &overhead_ns.iter().map(|&v| v as i64).collect::<Vec<_>>(),
                &run_dir.run_id,
            ),
            cross_core_offset: report::summarise_characterisation(&offset_ns, &run_dir.run_id),
            clocksource,
        });
    std::fs::write(run_dir.path.join("REPORT.md"), report_text)
        .context("failed to write REPORT.md")?;

    println!("run directory: {}", run_dir.path.display());
    Ok(())
}

fn measure_characterisation<C: MonotonicRawClock + Sync>(
    clock: &C,
    iterations: usize,
    cpu_a: usize,
    cpu_b: usize,
    rounds: usize,
) -> anyhow::Result<(Vec<u64>, Vec<i64>)> {
    let overhead = read_overhead_ns(clock, iterations).context("measuring clock read overhead")?;
    let offsets =
        cross_core_offset_ns(clock, cpu_a, cpu_b, rounds).context("measuring cross-core offset")?;
    Ok((overhead, offsets))
}

/// `clock-characterisation.tsv`'s exact on-disk format: a `metric\tvalue` header, one row per
/// `read_overhead_ns` sample, one row per `cross_core_offset_ns` sample, then one
/// `current_clocksource` row. Shared in spirit with the labels `main.rs` already printed to
/// stdout before this plan (02-06); a reader of the raw capture and a reader of the journal see
/// the same vocabulary.
fn format_characterisation_tsv(
    overhead_ns: &[u64],
    offset_ns: &[i64],
    clocksource: Option<&str>,
) -> String {
    let mut out = String::from("metric\tvalue\n");
    for sample in overhead_ns {
        out.push_str(&format!("read_overhead_ns\t{sample}\n"));
    }
    for sample in offset_ns {
        out.push_str(&format!("cross_core_offset_ns\t{sample}\n"));
    }
    out.push_str(&format!(
        "current_clocksource\t{}\n",
        clocksource.unwrap_or("unavailable")
    ));
    out
}

/// `(read_overhead_ns samples, cross_core_offset_ns samples, clocksource, source run id)`, named
/// so [`read_characterisation_tsv`]'s signature stays under clippy's type-complexity lint.
type CharacterisationTsvData = (Vec<i64>, Vec<i64>, Option<String>, String);

/// Reads a prior `characterise` run's own `clock-characterisation.tsv`, following the format
/// [`format_characterisation_tsv`] writes. The source run id is the file's own parent directory
/// name, so the report can attribute the figures without a second flag naming it separately.
fn read_characterisation_tsv(path: &Path) -> anyhow::Result<CharacterisationTsvData> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read --characterisation-tsv {}", path.display()))?;
    let mut overhead = Vec::new();
    let mut offset = Vec::new();
    let mut clocksource = None;
    for line in text.lines().skip(1) {
        let Some((metric, value)) = line.split_once('\t') else {
            continue;
        };
        match metric {
            "read_overhead_ns" => {
                if let Ok(v) = value.parse::<i64>() {
                    overhead.push(v);
                }
            }
            "cross_core_offset_ns" => {
                if let Ok(v) = value.parse::<i64>() {
                    offset.push(v);
                }
            }
            "current_clocksource" => clocksource = Some(value.to_string()),
            _ => {}
        }
    }
    let source_run_id = path
        .parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok((overhead, offset, clocksource, source_run_id))
}

#[allow(clippy::too_many_arguments)]
fn run_abort_latency(
    period_ns: u64,
    trials: usize,
    hot_cpu: usize,
    abort_cpu: usize,
    priority: u8,
    seed: u64,
    rig_slug: &str,
    measurements_root: &Path,
    characterisation_tsv: Option<&Path>,
) -> anyhow::Result<()> {
    let target_cpus = vec![hot_cpu as u32, abort_cpu as u32];
    let run_class = RunClass::Headline;
    let spec = PreconditionSpec {
        instrument_class: InstrumentClass::HeadlineSeries,
        run_class: run_class.clone(),
        target_cpus: target_cpus.clone(),
        thermal_profile: ThermalProfile::Normal,
    };
    let requested = RequestedRun {
        run_class: run_class.clone(),
        instrument_class: InstrumentClass::HeadlineSeries,
        cpus: format!("{hot_cpu},{abort_cpu}"),
        main_cpus: String::new(),
        duration_seconds: trials as u64 * period_ns / 1_000_000_000,
        with_hwlatdetect: false,
        hwlatdetect_duration_seconds: None,
    };

    let utc_start = time::OffsetDateTime::now_utc();
    let run_dir = rundir::RunDir::create(measurements_root, rig_slug, &run_class, utc_start)
        .context("failed to create the run directory")?;

    let fixture_text = read_fixture_text()?;
    let admission = admit(
        &run_dir,
        fixture_text.as_deref(),
        &spec,
        &requested,
        &target_cpus,
    )?;

    let outcome = if admission.fixture_mode {
        let clock = FixtureClock::generated(1_000);
        let config = TrialConfig {
            period_ns,
            trials,
            hot_cpu,
            abort_cpu,
            priority,
            seed,
            // Fixture mode already means the facts, the clock and the interference snapshot are
            // all synthetic; requiring a real SCHED_FIFO pin here as well would only make the
            // fixture path unusable on a dev host, never a real guarantee (the same reasoning
            // 02-06 already applied to this field, here extended to the harness's own fixture
            // switch rather than to a bare test flag).
            require_realtime_scheduling: false,
        };
        run_trials(&clock, &config)
    } else {
        let clock = RawClock;
        let config = TrialConfig {
            period_ns,
            trials,
            hot_cpu,
            abort_cpu,
            priority,
            seed,
            // Real invocation: a placement- or scheduling-sensitive figure taken without a
            // guaranteed pin and SCHED_FIFO is not defensible under this project's own rig
            // discipline (D-36), so a failure to obtain either refuses the whole run.
            require_realtime_scheduling: true,
        };
        run_trials(&clock, &config)
    }
    .context("running abort-latency trials")?;

    let tsv_path = run_dir
        .path
        .join(format!("abort-latency-{period_ns}ns.tsv"));
    std::fs::write(&tsv_path, format_trial_tsv(&outcome.rows))
        .with_context(|| format!("failed to write {}", tsv_path.display()))?;
    let artifact = capture::record_artifact(&tsv_path, &run_dir.path)
        .with_context(|| format!("failed to checksum {}", tsv_path.display()))?;

    let latencies_ns: Vec<u64> = outcome.rows.iter().map(|row| row.latency_ns).collect();
    let latency_stats =
        nr_histogram::samples::stats_from_samples(&latencies_ns, &[0.5, 0.95, 0.99, 0.999])
            .context("computing abort-latency percentiles")?;

    let interference_after =
        capture::take_interference_snapshot(&target_cpus, admission.fixture_mode)
            .context("failed to snapshot interference after the measurement")?;
    let interference = InterferenceSnapshotPair {
        delta: capture::interference_delta(&admission.interference_before, &interference_after),
        before: admission.interference_before,
        after: interference_after,
        tail_metrics: None,
        thresholds_provisional: None,
        verdict: ContaminationVerdict::Uncalibrated,
        windows: Vec::new(),
    };

    let env_snapshot = nr_capture::environment::snapshot(
        admission.facts.as_ref(),
        rig_slug,
        Some(&admission.thermal_start),
    )
    .context("failed to capture the environment snapshot")?;
    let utc_end = time::OffsetDateTime::now_utc();
    let harness = capture::harness_info();
    let tools = vec![ToolInvocation {
        name: "nr-stop-harness".to_string(),
        version: harness.version.clone(),
        argv: self_invocation_argv(&run_dir.path),
        exit_code: 0,
        artifact_paths: Vec::new(),
    }];

    let manifest = capture::build_manifest(capture::ManifestInputs {
        run_id: run_dir.run_id.clone(),
        run_class: run_class.clone(),
        utc_start,
        utc_end,
        harness,
        preconditions: admission.preconditions,
        interference,
        tools,
        artifacts: vec![artifact.clone()],
        env: env_snapshot,
        fixtures_used: fixtures_used(admission.fixture_mode),
    });
    let manifest_blake3 = capture::manifest_blake3(&manifest);
    write_manifest(&run_dir.path, &manifest)?;

    let metrics_entry = capture::build_metrics_entry(capture::MetricsEntryInputs {
        run_id: run_dir.run_id.clone(),
        run_class,
        utc_start,
        rig_slug: rig_slug.to_string(),
        stats: &latency_stats,
        population: format!(
            "abort observation latency (D-31), converted from nanosecond samples in {}; \
             population is the {trials} abort trials in this run, at a {period_ns} ns poll \
             period",
            artifact.path
        ),
        manifest_blake3,
    });
    let metrics_json = serde_json::to_string_pretty(&metrics_entry)
        .context("failed to serialise metrics-entry.json")?;
    std::fs::write(run_dir.path.join("metrics-entry.json"), metrics_json)
        .context("failed to write metrics-entry.json")?;

    let (read_overhead, cross_core_offset, clocksource) = match characterisation_tsv {
        Some(path) => {
            let (overhead, offset, clocksource, source_run_id) = read_characterisation_tsv(path)?;
            (
                report::summarise_characterisation(&overhead, &source_run_id),
                report::summarise_characterisation(&offset, &source_run_id),
                clocksource,
            )
        }
        None => (None, None, None),
    };
    let report_text = report::render_abort_latency_report(&report::AbortLatencyReportInput {
        run_id: run_dir.run_id.clone(),
        period_ns,
        trials,
        latency_stats,
        cross_core_offset,
        read_overhead,
        clocksource,
    });
    std::fs::write(run_dir.path.join("REPORT.md"), report_text)
        .context("failed to write REPORT.md")?;

    println!("run directory: {}", run_dir.path.display());
    Ok(())
}

fn format_trial_tsv(rows: &[nr_stop_harness::trial::TrialRow]) -> String {
    let mut out = String::from("trial\tphase_ns\tabort_raw_ns\tobserved_raw_ns\tlatency_ns\n");
    for row in rows {
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            row.trial, row.phase_ns, row.abort_raw_ns, row.observed_raw_ns, row.latency_ns
        ));
    }
    out
}

fn write_manifest(run_dir: &Path, manifest: &nr_manifest::RunManifest) -> anyhow::Result<()> {
    nr_manifest::validate(run_dir, manifest).map_err(|errors| {
        anyhow::anyhow!("generated manifest failed its own validation: {errors:?}")
    })?;
    let json =
        serde_json::to_string_pretty(manifest).context("failed to serialise manifest.json")?;
    std::fs::write(run_dir.join("manifest.json"), json).context("failed to write manifest.json")
}
