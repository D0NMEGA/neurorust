//! Task 1 (the run directory, the preconditions, the refusal path) and task 2 (the manifest, the
//! metrics entry, the D-34 rendering) of plan 02-07: the evidence machinery wrapped around
//! plan 02-06's measurement core.

use std::path::Path;

use nr_capture::preconditions::PreconditionSpec;
use nr_capture::sources::FixtureFacts;
use nr_manifest::{
    ArtifactKind, AttemptFailure, AttemptRecord, AttemptStatus, InstrumentClass, PreconditionCheck,
    RequestedRun, RunClass, ThermalProfile,
};
use nr_stop_harness::{capture, report, rundir};
use time::OffsetDateTime;

/// The real, committed rig capture (untuned: every governor reads `powersave`). Reused from
/// `nr-capture`'s own fixtures rather than a synthetic guess, matching this project's own
/// established convention for fixture-driven tests.
const RIG_AS_FOUND: &str = include_str!("../../capture/tests/fixtures/probe-sysfs-tuning.txt");

fn utc(year: i32, month: time::Month, day: u8) -> OffsetDateTime {
    let date = time::Date::from_calendar_date(year, month, day).expect("valid date");
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
}

/// `target_cpus` is `[0, 1]` throughout this file, not `[7, 8]` (the harness's own CLI
/// defaults): `nr_capture::sources::FixtureFacts`'s cpuidle lookup only ever serves `cpu0`'s own
/// path (`crates/capture/src/sources.rs::lookup`), so a per-cpu-idle-state check against any
/// other single CPU always reads `Unavailable` under a fixture, refusing every `HeadlineSeries`
/// scenario this file needs to pass regardless of what the fixture text says. Using `cpu0` in
/// the target set is a test-construction choice; it does not change the harness's own real
/// defaults, which stay `7`/`8` on the real CLI.
const TARGET_CPUS: [u32; 2] = [0, 1];

fn headline_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::HeadlineSeries,
        run_class: RunClass::Headline,
        target_cpus: TARGET_CPUS.to_vec(),
        thermal_profile: ThermalProfile::Normal,
    }
}

/// A facts text that passes all fifteen preconditions under [`headline_spec`], derived from the
/// real, committed `RIG_AS_FOUND` capture plus the same governor/no_turbo/target/thermal
/// overrides `crates/cli/src/cmd/run.rs`'s own `tuned_facts_text` test helper applies, extended
/// with the handful of keys that helper leaves unset (session/tracing/isolcpus/rt-tuning
/// service): that helper is documented as producing "Unavailable, not Fail" for exactly those
/// fields and is used there only under `InstrumentClass::Investigation`, which tolerates
/// `Unavailable`. Every STOP-07 run is `HeadlineSeries`, which does not.
fn fully_passing_facts_text() -> String {
    let governor_and_target = RIG_AS_FOUND
        .replace(".governor=powersave", ".governor=performance")
        .replace(
            "intel_pstate.no_turbo=unavailable",
            "intel_pstate.no_turbo=1",
        )
        .replace(
            "systemd.default_target=graphical.target",
            "systemd.default_target=multi-user.target",
        );
    let with_thermal_margin: String = governor_and_target
        .lines()
        .map(|line| match line.split_once('=') {
            Some((key, _)) if key.starts_with("thermal.") => format!("{key}=40000"),
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{with_thermal_margin}\n\
         ssh.active_sessions=0\n\
         service.gdm.service.ActiveState=inactive\n\
         graphical.sessions=0\n\
         login.local_sessions=\n\
         cpu.isolated=0-1\n\
         service.rt-tuning.service.ActiveState=active\n"
    )
}

fn sample_requested() -> RequestedRun {
    RequestedRun {
        run_class: RunClass::Headline,
        instrument_class: InstrumentClass::HeadlineSeries,
        cpus: "0,1".to_string(),
        main_cpus: String::new(),
        duration_seconds: 1,
        with_hwlatdetect: false,
        hwlatdetect_duration_seconds: None,
    }
}

// -------------------------------------------------------------------------------------------
// Task 1: the run directory, the preconditions, the refusal path.
// -------------------------------------------------------------------------------------------

#[test]
fn run_id_follows_the_existing_convention() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let start = utc(2026, time::Month::September, 20);

    let run_dir = rundir::RunDir::create(tmp.path(), "precision3591", &RunClass::Headline, start)
        .expect("create succeeds");

    assert_eq!(run_dir.run_id, "2026-09-20-precision3591-headline");
    assert!(run_dir.path.is_dir());
}

#[test]
fn a_second_run_the_same_day_gets_a_suffix() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let start = utc(2026, time::Month::September, 20);

    let first = rundir::RunDir::create(tmp.path(), "precision3591", &RunClass::Headline, start)
        .expect("first run creates");
    std::fs::write(first.path.join("manifest.json"), "{}").expect("write manifest");

    let second = rundir::RunDir::create(tmp.path(), "precision3591", &RunClass::Headline, start)
        .expect("second run creates");
    assert_eq!(second.run_id, "2026-09-20-precision3591-headline-02");
    std::fs::write(second.path.join("manifest.json"), "{}").expect("write manifest");

    let third = rundir::RunDir::create(tmp.path(), "precision3591", &RunClass::Headline, start)
        .expect("third run creates");
    assert_eq!(third.run_id, "2026-09-20-precision3591-headline-03");
}

#[test]
fn an_invalid_rig_slug_fails_before_any_write() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let start = utc(2026, time::Month::September, 20);

    let err = rundir::RunDir::create(tmp.path(), "Precision-3591", &RunClass::Headline, start)
        .expect_err("uppercase rig slug must fail");
    assert!(matches!(err, rundir::RunDirError::InvalidRunId { .. }));

    let entries: Vec<_> = std::fs::read_dir(tmp.path()).expect("read_dir").collect();
    assert!(
        entries.is_empty(),
        "no directory must be created on refusal"
    );
}

#[test]
fn an_existing_manifest_refuses_rather_than_overwrites() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let populated = tmp.path().join("2026-09-20-precision3591-headline");
    std::fs::create_dir_all(&populated).expect("mkdir");
    std::fs::write(populated.join("manifest.json"), "{}").expect("write manifest");

    let err = rundir::refuse_if_manifest_exists(&populated)
        .expect_err("must refuse an existing manifest");
    assert!(matches!(err, rundir::RunDirError::AlreadyPopulated { .. }));
}

#[test]
fn all_fifteen_preconditions_are_evaluated() {
    // Deliberately untuned, unmodified real rig data: `run_all` returns every one of the
    // fifteen results regardless of how many pass, and records an unevaluable check rather than
    // omitting it.
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let spec = headline_spec();
    let results = nr_capture::preconditions::run_all(&facts, &spec);
    assert_eq!(results.len(), 15);
}

#[test]
fn a_failing_precondition_refuses_the_run() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let run_dir = rundir::RunDir::create(
        tmp.path(),
        "precision3591",
        &RunClass::Headline,
        utc(2026, time::Month::September, 20),
    )
    .expect("create run dir");

    // The real, untuned capture: every governor reads powersave.
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let spec = headline_spec();

    let refusal =
        capture::check_preconditions(&facts, &spec).expect_err("powersave governor must refuse");
    assert!(
        refusal
            .offenses
            .iter()
            .any(|r| r.check == PreconditionCheck::GovernorIsPerformanceOnAllCpus),
        "the governor check must be among the offenses: {:?}",
        refusal.offenses
    );

    let all_results = nr_capture::preconditions::run_all(&facts, &spec);
    let message = format!(
        "{refusal}\n{}",
        capture::format_precondition_results(&all_results)
    );
    let failure = AttemptFailure {
        stage: "preconditions".to_string(),
        message,
    };
    let requested = sample_requested();
    let attempt_path = capture::write_attempt(&run_dir.path, &requested, &failure)
        .expect("write_attempt succeeds");

    assert!(attempt_path.exists(), "ATTEMPT.json must exist");
    assert!(
        !run_dir.path.join("manifest.json").exists(),
        "a refused run must leave no manifest.json"
    );

    let attempt_text = std::fs::read_to_string(&attempt_path).expect("read ATTEMPT.json");
    let attempt: AttemptRecord = serde_json::from_str(&attempt_text).expect("parses");
    assert_eq!(attempt.status, AttemptStatus::Failed);
    assert!(!attempt.usable_for_numerical_analysis);
    let failure_text = attempt.failure.expect("failure recorded").message;
    assert!(
        failure_text.contains("GovernorIsPerformanceOnAllCpus"),
        "the failure message must name the offending check: {failure_text}"
    );
    // Both branches keep the full fifteen results: the written record names every check, not
    // only the ones that stopped the run.
    assert!(
        failure_text.matches("PreconditionCheck").count() >= 15
            || failure_text.lines().filter(|l| l.contains(':')).count() >= 15,
        "the failure message must record all fifteen results, not only the offenses"
    );
}

#[test]
fn an_unavailable_check_refuses_a_headline_run() {
    // Unmodified real rig data. It has no ssh.active_sessions, cpu.isolated, or
    // rt-tuning.service ActiveState keys at all, so several checks read Unavailable; under
    // HeadlineSeries that refuses on its own, independent of anything that outright Fails.
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let spec = headline_spec();

    let refusal =
        capture::check_preconditions(&facts, &spec).expect_err("an unavailable check must refuse");
    assert!(
        refusal
            .offenses
            .iter()
            .any(|r| r.status == nr_manifest::PreconditionStatus::Unavailable),
        "at least one offense must be Unavailable, not only Fail: {:?}",
        refusal.offenses
    );
}

// -------------------------------------------------------------------------------------------
// Task 2: the manifest, the metrics entry, the D-34 rendering.
// -------------------------------------------------------------------------------------------

/// Everything a "completed run" test needs, built once and shared across this section's
/// assertions.
struct CompletedRun {
    _tmp: tempfile::TempDir,
    run_dir: rundir::RunDir,
    manifest: nr_manifest::RunManifest,
    metrics_entry: nr_metrics::series::StageMetrics,
    capture_path: std::path::PathBuf,
}

fn build_completed_run() -> CompletedRun {
    let tmp = tempfile::tempdir().expect("tempdir");
    let run_dir = rundir::RunDir::create(
        tmp.path(),
        "precision3591",
        &RunClass::Headline,
        utc(2026, time::Month::September, 20),
    )
    .expect("create run dir");

    let facts_text = fully_passing_facts_text();
    let facts = FixtureFacts::parse(&facts_text);
    let spec = headline_spec();
    let preconditions =
        capture::check_preconditions(&facts, &spec).expect("all fifteen preconditions pass");

    let interference_before =
        capture::take_interference_snapshot(&TARGET_CPUS, true).expect("fixture interference");
    let interference_after =
        capture::take_interference_snapshot(&TARGET_CPUS, true).expect("fixture interference");
    let interference = nr_manifest::InterferenceSnapshotPair {
        delta: capture::interference_delta(&interference_before, &interference_after),
        before: interference_before,
        after: interference_after,
        tail_metrics: None,
        thresholds_provisional: None,
        verdict: nr_manifest::ContaminationVerdict::Uncalibrated,
        windows: Vec::new(),
    };

    // A small, realistic raw capture: five trial rows, matching abort-latency's real TSV shape.
    let capture_path = run_dir.path.join("abort-latency-1000000ns.tsv");
    std::fs::write(
        &capture_path,
        "trial\tphase_ns\tabort_raw_ns\tobserved_raw_ns\tlatency_ns\n\
         0\t100\t1000\t1030\t30\n\
         1\t200\t2000\t2041\t41\n\
         2\t300\t3000\t3028\t28\n\
         3\t400\t4000\t4055\t55\n\
         4\t500\t5000\t5012\t12\n",
    )
    .expect("write raw capture");
    let artifact =
        capture::record_artifact(&capture_path, &run_dir.path).expect("checksum raw capture");

    let env_snapshot = nr_capture::environment::snapshot(&facts, "precision3591", Some(&[]))
        .expect("environment snapshot");
    let harness = capture::harness_info();
    let tools = vec![nr_manifest::ToolInvocation {
        name: "nr-stop-harness".to_string(),
        version: harness.version.clone(),
        argv: vec!["nr-stop-harness".to_string(), "abort-latency".to_string()],
        exit_code: 0,
        artifact_paths: Vec::new(),
    }];

    let manifest = capture::build_manifest(capture::ManifestInputs {
        run_id: run_dir.run_id.clone(),
        run_class: RunClass::Headline,
        utc_start: run_dir_start(),
        utc_end: OffsetDateTime::now_utc(),
        harness,
        preconditions,
        interference,
        tools,
        artifacts: vec![artifact.clone()],
        env: env_snapshot,
        fixtures_used: vec![capture::NR_STOP_FACTS_FIXTURE.to_string()],
    });

    nr_manifest::validate(&run_dir.path, &manifest)
        .map_err(|errors| format!("{errors:?}"))
        .expect("manifest validates");
    let manifest_json = serde_json::to_string_pretty(&manifest).expect("serialise manifest");
    std::fs::write(run_dir.path.join("manifest.json"), manifest_json).expect("write manifest.json");

    let latencies: Vec<u64> = vec![30, 41, 28, 55, 12];
    let stats = nr_histogram::samples::stats_from_samples(&latencies, &[0.5, 0.95, 0.99, 0.999])
        .expect("stats over five samples");
    let metrics_entry = capture::build_metrics_entry(capture::MetricsEntryInputs {
        run_id: run_dir.run_id.clone(),
        run_class: RunClass::Headline,
        utc_start: run_dir_start(),
        rig_slug: "precision3591".to_string(),
        stats: &stats,
        population: format!(
            "abort observation latency, converted from nanosecond samples in {}",
            artifact.path
        ),
        manifest_blake3: capture::manifest_blake3(&manifest),
    });
    let metrics_json = serde_json::to_string_pretty(&metrics_entry).expect("serialise entry");
    std::fs::write(run_dir.path.join("metrics-entry.json"), metrics_json)
        .expect("write metrics-entry.json");

    std::fs::write(run_dir.path.join("REPORT.md"), "# placeholder\n").expect("write REPORT.md");

    CompletedRun {
        _tmp: tmp,
        run_dir,
        manifest,
        metrics_entry,
        capture_path,
    }
}

fn run_dir_start() -> OffsetDateTime {
    utc(2026, time::Month::September, 20)
}

/// Behaviors: the_raw_capture_is_written_before_it_is_checksummed,
/// every_file_in_the_run_directory_is_listed_as_an_artifact, the_manifest_validates,
/// the_metrics_entry_carries_the_committed_stage_and_tool_strings,
/// percentiles_are_present_and_the_max_is_exact.
#[test]
fn a_completed_run_leaves_validating_evidence() {
    let run = build_completed_run();

    // the_raw_capture_is_written_before_it_is_checksummed: re-read the file independently and
    // recompute its digest; it must match the recorded artifact exactly.
    let artifact = run
        .manifest
        .artifacts
        .iter()
        .find(|a| a.kind == ArtifactKind::Other)
        .expect("the raw capture is recorded");
    let recomputed = nr_manifest::blake3_file(&run.capture_path).expect("recompute blake3");
    assert_eq!(artifact.blake3, recomputed);
    assert_eq!(
        artifact.bytes,
        std::fs::metadata(&run.capture_path).expect("stat").len()
    );

    // every_file_in_the_run_directory_is_listed_as_an_artifact: manifest.json, metrics-entry.json
    // and REPORT.md are not captures and are not listed; every other file is.
    let known_non_capture = ["manifest.json", "metrics-entry.json", "REPORT.md"];
    for entry in std::fs::read_dir(&run.run_dir.path).expect("read_dir") {
        let entry = entry.expect("entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        if known_non_capture.contains(&name.as_str()) {
            continue;
        }
        assert!(
            run.manifest.artifacts.iter().any(|a| a.path == name),
            "{name} exists on disk but is not listed in manifest.artifacts"
        );
    }

    // the_manifest_validates.
    assert!(nr_manifest::validate(&run.run_dir.path, &run.manifest).is_ok());

    // the_metrics_entry_carries_the_committed_stage_and_tool_strings: the exact strings
    // crates/metrics/tests/series.rs::stage_names_are_open already round-trips.
    assert_eq!(run.metrics_entry.stage, "emergency_stop.abort_latency");
    assert_eq!(run.metrics_entry.tool, "nr-stop-harness");

    // percentiles_are_present_and_the_max_is_exact.
    assert!(run.metrics_entry.p50_us.is_some());
    assert!(run.metrics_entry.p95_us.is_some());
    assert!(run.metrics_entry.p99_us.is_some());
    assert!(run.metrics_entry.p999_us.is_some());
    // The five sample latencies were 30, 41, 28, 55, 12 ns; the exact maximum is 55 ns, which
    // truncates to 0 us. Asserted against the literal conversion, not re-derived, so a future
    // change to the truncation convention is caught here.
    assert_eq!(run.metrics_entry.max_us, 55 / 1_000);
    assert_eq!(run.metrics_entry.overflow_count, 0);
}

/// Behavior: the_metrics_entry_is_not_appended_to_the_series. The real, committed
/// `metrics/latency-series.json` and `metrics/baseline.json` are byte-identical before and after
/// building a completed run entirely inside a tempdir: nothing in this harness's own pipeline
/// opens either file for writing.
#[test]
fn completing_a_run_never_touches_the_committed_series() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("resolve repo root");
    let series_path = repo_root.join("metrics/latency-series.json");
    let baseline_path = repo_root.join("metrics/baseline.json");
    let series_before = std::fs::read(&series_path).expect("read latency-series.json");
    let baseline_before = std::fs::read(&baseline_path).expect("read baseline.json");

    let _run = build_completed_run();

    let series_after = std::fs::read(&series_path).expect("read latency-series.json again");
    let baseline_after = std::fs::read(&baseline_path).expect("read baseline.json again");
    assert_eq!(
        series_before, series_after,
        "latency-series.json must be untouched"
    );
    assert_eq!(
        baseline_before, baseline_after,
        "baseline.json must be untouched"
    );
}

/// Behaviors: the_report_states_the_total_and_the_decomposition_separately, and
/// a_missing_characterisation_is_stated_rather_than_omitted.
#[test]
fn the_report_separates_total_from_decomposition_and_never_fabricates_a_missing_figure() {
    let stats = nr_histogram::samples::stats_from_samples(
        &[30u64, 41, 28, 55, 12],
        &[0.5, 0.95, 0.99, 0.999],
    )
    .expect("stats");

    // With no characterisation run supplied at all.
    let without_characterisation =
        report::render_abort_latency_report(&report::AbortLatencyReportInput {
            run_id: "2026-09-20-precision3591-headline".to_string(),
            period_ns: 1_000_000,
            trials: 5,
            latency_stats: stats.clone(),
            cross_core_offset: None,
            read_overhead: None,
            clocksource: None,
        });
    assert!(without_characterisation.contains("not combined"));
    assert!(without_characterisation.contains("poll period"));
    assert!(
        without_characterisation.contains("55"),
        "the exact maximum must appear"
    );
    assert!(
        without_characterisation
            .to_lowercase()
            .contains("unavailable"),
        "a missing characterisation figure must say so explicitly"
    );
    assert!(
        !without_characterisation.contains("cross-core propagation: 0 ns")
            && !without_characterisation.contains("clock read overhead: 0 ns"),
        "a missing figure must never render as a fabricated zero"
    );

    // With a characterisation run supplied.
    let figure =
        report::summarise_characterisation(&[10, 12, 9, 11], "2026-09-20-precision3591-recon")
            .expect("non-empty samples summarise");
    let with_characterisation =
        report::render_abort_latency_report(&report::AbortLatencyReportInput {
            run_id: "2026-09-20-precision3591-headline".to_string(),
            period_ns: 1_000_000,
            trials: 5,
            latency_stats: stats,
            cross_core_offset: Some(figure.clone()),
            read_overhead: Some(figure),
            clocksource: Some("tsc".to_string()),
        });
    assert!(with_characterisation.contains("not combined"));
    assert!(with_characterisation.contains("2026-09-20-precision3591-recon"));
    assert!(with_characterisation.contains("tsc"));
}
