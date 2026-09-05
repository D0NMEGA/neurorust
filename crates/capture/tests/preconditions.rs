//! Integration tests for the D-06 preconditions, driven entirely by fixture data so
//! they run identically on macOS and Linux (RESEARCH.md pitfall 5).

use nr_capture::environment;
use nr_capture::preconditions::{PreconditionSpec, refuse_on_violation, run_all};
use nr_capture::sources::FixtureFacts;
use nr_manifest::{
    InstrumentClass, PreconditionCheck, PreconditionResult, PreconditionStatus, RunClass,
};

/// The real D-06/D-14 tuning snapshot from the reference rig (plan 01-02), exactly as the
/// machine was found. It is NOT a passing baseline: the rig boots untuned, so its governor
/// reads `powersave` on all 22 CPUs and several checks legitimately fail or report
/// unavailable against it (see docs/rig/recon-2026-08-31/FINDINGS.md). A failing result
/// here is the precondition doing its job. Do not "fix" a check to make it pass against
/// this fixture; that would delete the phase's most important finding.
const RIG_AS_FOUND: &str = include_str!("fixtures/probe-sysfs-tuning.txt");
const VIOLATED: &str = include_str!("fixtures/violated-sysfs-tuning.txt");

/// Derived, NOT a rig capture (see fixtures/README.md): reproduces the exact
/// combination `docs/rig/recon-2026-08-31/FINDINGS.md`'s "The rt-tuning.service
/// contradiction" found on the reference rig on 2026-08-31 -
/// `scaling_governor=powersave` and `energy_performance_preference=performance` on
/// every CPU simultaneously, the state power-profiles-daemon's own "performance"
/// profile puts this Meteor Lake HWP backend into. `probe-sysfs-tuning.txt`'s own
/// probe script never captured the EPP value, which is why this fixture exists
/// rather than editing that one.
const EPP_PERFORMANCE: &str = include_str!("fixtures/epp-performance-sysfs-tuning.txt");

const TARGET_CPUS: [u32; 6] = [6, 7, 8, 9, 10, 11];

fn headline_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::HeadlineSeries,
        run_class: RunClass::Headline,
        target_cpus: TARGET_CPUS.to_vec(),
    }
}

fn investigation_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::Investigation,
        run_class: RunClass::Investigation,
        target_cpus: TARGET_CPUS.to_vec(),
    }
}

/// A D-18 firmware screen. It saturates the machine on purpose, so it is the one run
/// class that legitimately starts hot.
fn screen_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::HeadlineSeries,
        run_class: RunClass::Screen,
        target_cpus: TARGET_CPUS.to_vec(),
    }
}

const ALL_CHECKS: [PreconditionCheck; 15] = [
    PreconditionCheck::NoActiveSshSessions,
    PreconditionCheck::SystemdDefaultTargetIsMultiUser,
    PreconditionCheck::DisplayManagerInactive,
    PreconditionCheck::NoGraphicalSession,
    PreconditionCheck::NoActiveLoginSessions,
    PreconditionCheck::GovernorIsPerformanceOnAllCpus,
    PreconditionCheck::NoTurboEnabled,
    PreconditionCheck::DeepCstatesDisabled,
    PreconditionCheck::IsolcpusCoversTargetCpus,
    PreconditionCheck::KernelIsRealtime,
    PreconditionCheck::RtTuningServiceActive,
    PreconditionCheck::OnAcPower,
    PreconditionCheck::ThermalHeadroomAtStart,
    PreconditionCheck::NoPackageManagerActivity,
    PreconditionCheck::TracersQuiescent,
];

#[test]
fn all_precondition_results_recorded() {
    let as_found_facts = FixtureFacts::parse(RIG_AS_FOUND);
    let as_found_results = run_all(&as_found_facts, &headline_spec());
    assert_eq!(as_found_results.len(), 15, "got {as_found_results:#?}");

    let violated_facts = FixtureFacts::parse(VIOLATED);
    let violated_results = run_all(&violated_facts, &headline_spec());
    assert_eq!(violated_results.len(), 15, "got {violated_results:#?}");

    // A failing check produces a Fail result; it does not vanish from the list.
    assert!(
        violated_results
            .iter()
            .any(|r| r.status == PreconditionStatus::Fail),
        "expected at least one Fail among {violated_results:#?}"
    );
}

#[test]
fn preconditions_refuse_on_violation() {
    // The real rig fixture already carries a real violation (every CPU governor
    // reads powersave; docs/rig/recon-2026-08-31/FINDINGS.md).
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let results = run_all(&facts, &headline_spec());
    let err = refuse_on_violation(&results, &InstrumentClass::HeadlineSeries)
        .expect_err("a fixture with a real Fail must refuse");
    assert!(!err.offenses.is_empty());
    let message = err.to_string();
    assert!(
        message.contains("GovernorIsPerformanceOnAllCpus"),
        "refusal message should name the offending check: {message}"
    );

    // An all-Pass set returns Ok.
    let all_pass: Vec<PreconditionResult> = ALL_CHECKS
        .into_iter()
        .map(|check| PreconditionResult {
            check,
            status: PreconditionStatus::Pass,
            observed: "ok".to_string(),
            expected: "ok".to_string(),
        })
        .collect();
    assert!(refuse_on_violation(&all_pass, &InstrumentClass::HeadlineSeries).is_ok());
}

#[test]
fn unavailable_is_not_a_pass() {
    // no_turbo's fixture value is the literal "unavailable" sentinel: a genuine
    // false negative from the probe querying the wrong sysfs path (see FINDINGS.md).
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let results = run_all(&facts, &headline_spec());
    let no_turbo = results
        .iter()
        .find(|r| r.check == PreconditionCheck::NoTurboEnabled)
        .expect("NoTurboEnabled result present");
    assert_eq!(no_turbo.status, PreconditionStatus::Unavailable);

    let err = refuse_on_violation(&results, &InstrumentClass::HeadlineSeries)
        .expect_err("Unavailable must refuse a headline-class run");
    assert!(
        err.offenses
            .iter()
            .any(|r| r.status == PreconditionStatus::Unavailable)
    );
}

#[test]
fn tracers_quiescent_refuses_headline_run() {
    let facts = FixtureFacts::parse(VIOLATED); // tracing.current_tracer=timerlat
    let results = run_all(&facts, &headline_spec());
    let tracer_result = results
        .iter()
        .find(|r| r.check == PreconditionCheck::TracersQuiescent)
        .expect("TracersQuiescent result present");
    assert_eq!(tracer_result.status, PreconditionStatus::Fail);
    assert_eq!(tracer_result.observed, "timerlat");
}

#[test]
fn tracers_quiescent_allows_investigation_run() {
    let facts = FixtureFacts::parse(VIOLATED);
    let results = run_all(&facts, &investigation_spec());
    let tracer_result = results
        .iter()
        .find(|r| r.check == PreconditionCheck::TracersQuiescent)
        .expect("TracersQuiescent result present");
    assert_eq!(tracer_result.status, PreconditionStatus::NotApplicable);
    // NotApplicable still records what was observed.
    assert_eq!(tracer_result.observed, "timerlat");
}

/// `FixtureFacts::default()` answers every other fact with `Unavailable`; only
/// `login.local_sessions` is set, so this exercises `NoActiveLoginSessions` in
/// isolation rather than requiring a full rig snapshot.
#[test]
fn no_active_login_sessions_passes_with_no_sessions() {
    let facts = FixtureFacts::default().with("login.local_sessions", "");
    let results = run_all(&facts, &headline_spec());
    let result = results
        .iter()
        .find(|r| r.check == PreconditionCheck::NoActiveLoginSessions)
        .expect("NoActiveLoginSessions result present");
    assert_eq!(result.status, PreconditionStatus::Pass);
    assert_eq!(result.observed, "none");
}

#[test]
fn no_active_login_sessions_fails_and_names_the_session() {
    let facts = FixtureFacts::default().with("login.local_sessions", "tty1 (session 3)");
    let results = run_all(&facts, &headline_spec());
    let result = results
        .iter()
        .find(|r| r.check == PreconditionCheck::NoActiveLoginSessions)
        .expect("NoActiveLoginSessions result present");
    assert_eq!(result.status, PreconditionStatus::Fail);
    assert!(
        result.observed.contains("tty1"),
        "observed should name the offending session: {}",
        result.observed
    );
}

/// RIG_AS_FOUND is the 2026-08-31 recon capture, which predates this check: it
/// never queried per-session Type/Class/Seat data at all, so the fixture key is
/// genuinely absent here rather than a simulated failure (the same reason
/// `NoActiveSshSessions`, `NoGraphicalSession`, `DisplayManagerInactive` and
/// `RtTuningServiceActive` are also `Unavailable` against this same fixture).
/// Honest absence must report `Unavailable`, never a silent `Pass` (design point
/// 4: record, do not guess).
#[test]
fn no_active_login_sessions_unavailable_when_loginctl_data_absent() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let results = run_all(&facts, &headline_spec());
    let result = results
        .iter()
        .find(|r| r.check == PreconditionCheck::NoActiveLoginSessions)
        .expect("NoActiveLoginSessions result present");
    assert_eq!(result.status, PreconditionStatus::Unavailable);
}

#[test]
fn observed_value_is_recorded() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let results = run_all(&facts, &headline_spec());
    let governor_result = results
        .iter()
        .find(|r| r.check == PreconditionCheck::GovernorIsPerformanceOnAllCpus)
        .expect("GovernorIsPerformanceOnAllCpus result present");

    assert_eq!(governor_result.status, PreconditionStatus::Fail);
    assert_ne!(governor_result.observed, "failed");
    assert!(
        governor_result.observed.starts_with("cpu"),
        "observed should name a CPU: {}",
        governor_result.observed
    );
    assert!(
        governor_result.observed.contains("powersave"),
        "observed should name the offending governor: {}",
        governor_result.observed
    );
}

/// The whole point of recording `energy_performance_preference` alongside
/// `governor` (docs/measurement-protocol.md, "The governor operating point"):
/// power-profiles-daemon's "performance" profile on this HWP backend never writes
/// the literal `scaling_governor=performance` value this protocol requires, so the
/// precondition must still fail, while the D-14 snapshot records both values so a
/// reader can tell this operating point apart from one where neither lever is set.
#[test]
fn epp_and_governor_are_recorded_as_distinct_operating_points() {
    let facts = FixtureFacts::parse(EPP_PERFORMANCE);

    let results = run_all(&facts, &headline_spec());
    let governor_result = results
        .iter()
        .find(|r| r.check == PreconditionCheck::GovernorIsPerformanceOnAllCpus)
        .expect("GovernorIsPerformanceOnAllCpus result present");
    assert_eq!(
        governor_result.status,
        PreconditionStatus::Fail,
        "energy_performance_preference must never substitute for the literal \
         scaling_governor=performance value this check requires: {governor_result:#?}"
    );

    let snap = environment::snapshot(&facts, "test-rig", None).expect("snapshot succeeds");
    assert!(!snap.tuning.per_cpu_governor.is_empty());
    for cpu_governor in &snap.tuning.per_cpu_governor {
        assert_eq!(
            cpu_governor.governor, "powersave",
            "cpu{}: {cpu_governor:?}",
            cpu_governor.cpu
        );
        assert_eq!(
            cpu_governor.energy_performance_preference.as_deref(),
            Some("performance"),
            "cpu{}: {cpu_governor:?}",
            cpu_governor.cpu
        );
    }
}

#[test]
fn harness_mutates_nothing() {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let write_patterns = ["fs::write", "File::create", "OpenOptions::new"];
    let path_prefixes = ["/sys", "/proc", "/etc"];

    let mut offending = Vec::new();
    for entry in std::fs::read_dir(&src_dir).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let contents = std::fs::read_to_string(&path).expect("read source file");
        for (line_no, line) in contents.lines().enumerate() {
            let has_write = write_patterns.iter().any(|p| line.contains(p));
            let has_path = path_prefixes.iter().any(|p| line.contains(p));
            if has_write && has_path {
                offending.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    line_no + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offending.is_empty(),
        "found system-mutating writes: {offending:#?}"
    );
}

/// The D-18 under-load arms saturate 22 cores and hold the package in the low 90s C by
/// design: `hwlatdetect` is screening for load-triggered SMIs, and the 2026-08-28
/// screening this re-run is compared against reported 91 to 93 C whole-machine and 93 to
/// 95 C P-core-only. `ThermalHeadroomAtStart` exists to stop a *latency* run from
/// starting thermally throttled, which is a different question, so it must not refuse a
/// firmware screen for being hot. Discovered the hard way: with the check applied
/// unconditionally, arm 2 was refused at 78 C and the arms were impossible to take
/// through the harness at all, even though the 70 C ceiling's own justification table is
/// built from those very under-load rows.
#[test]
fn thermal_headroom_does_not_refuse_a_firmware_screen() {
    let hot = RIG_AS_FOUND.replace("thermal.x86_pkg_temp=64000", "thermal.x86_pkg_temp=92000");
    let facts = FixtureFacts::parse(&hot);

    let screen = thermal_result(&run_all(&facts, &screen_spec()));
    assert_eq!(
        screen.status,
        PreconditionStatus::NotApplicable,
        "a screen run must not be refused for starting hot"
    );
    assert_eq!(
        screen.observed, "92.0 C",
        "NotApplicable still records the real observed temperature"
    );

    // The gate stays intact for every class that measures latency.
    let headline = thermal_result(&run_all(&facts, &headline_spec()));
    assert_eq!(headline.status, PreconditionStatus::Fail);
    assert_eq!(headline.observed, "92.0 C");
}

/// A screen run that starts cold still reports a real Pass, not a blanket exemption.
///
/// The observed value is 69.1 C rather than the fixture's `thermal.x86_pkg_temp=64000`
/// because the check reports the hottest zone across all of them, not the package zone
/// alone. That leaves this fixture sitting 0.9 C under the ceiling, which is worth
/// knowing: it is the same one-degree margin that made the old 60 C ceiling a coin flip.
#[test]
fn thermal_headroom_still_passes_a_cold_screen() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let screen = thermal_result(&run_all(&facts, &screen_spec()));
    assert_eq!(screen.status, PreconditionStatus::Pass);
    assert_eq!(screen.observed, "69.1 C");
}

fn thermal_result(results: &[PreconditionResult]) -> PreconditionResult {
    results
        .iter()
        .find(|r| r.check == PreconditionCheck::ThermalHeadroomAtStart)
        .expect("ThermalHeadroomAtStart result present")
        .clone()
}

/// The thermal record must describe the run, not the instant the snapshot happened to run.
///
/// Until 2026-09-04 `environment::snapshot` was called once, after both instruments finished,
/// and wrote that single reading into `temp_c_start` while leaving `temp_c_end` permanently
/// `None`. So the field named "start" held the END temperature, and `package_temp_c_max` was
/// the maximum across zones at one instant rather than across the run. The 70 C thermal gate's
/// own justification leaned on those fields making mid-run heating auditable, which they did
/// not. See finding 5 of 01-EXTERNAL-AUDIT.md.
#[test]
fn thermal_record_carries_both_ends_of_the_run() {
    let cold = FixtureFacts::parse(RIG_AS_FOUND);
    let start = nr_capture::sources::discover_thermal_zones_c(&cold);
    assert!(!start.is_empty(), "fixture must have thermal zones");

    // The run heats up: the same zones read hotter when the snapshot is taken at the end.
    let hot_text = RIG_AS_FOUND.replace("thermal.x86_pkg_temp=64000", "thermal.x86_pkg_temp=92000");
    let hot = FixtureFacts::parse(&hot_text);

    let snap = environment::snapshot(&hot, "test-rig", Some(&start)).expect("snapshot succeeds");
    let pkg = snap
        .power
        .thermal_zones
        .iter()
        .find(|z| z.name == "x86_pkg_temp")
        .expect("x86_pkg_temp zone present");

    assert_eq!(pkg.temp_c_start, 64.0, "start must be the pre-run reading");
    assert_eq!(
        pkg.temp_c_end,
        Some(92.0),
        "end must be populated, not left None"
    );
    assert_eq!(
        snap.power.package_temp_c_max,
        Some(92.0),
        "the max must span both readings, not just the snapshot instant"
    );
}

/// With no pre-run reading (reconstruct, which has only one observation), the end is absent
/// rather than fabricated, and start carries the single reading that does exist.
#[test]
fn thermal_record_without_a_pre_run_reading_leaves_the_end_absent() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let snap = environment::snapshot(&facts, "test-rig", None).expect("snapshot succeeds");
    let pkg = snap
        .power
        .thermal_zones
        .iter()
        .find(|z| z.name == "x86_pkg_temp")
        .expect("x86_pkg_temp zone present");
    assert_eq!(pkg.temp_c_start, 64.0);
    assert_eq!(pkg.temp_c_end, None);
}

fn deep_cstates_result(results: &[PreconditionResult]) -> PreconditionResult {
    results
        .iter()
        .find(|r| r.check == PreconditionCheck::DeepCstatesDisabled)
        .expect("DeepCstatesDisabled result present")
        .clone()
}

/// A fixture where cpu7 (one of the target CPUs) has C6 registered and enabled, while
/// every other target CPU (like cpu0's own tree in `RIG_AS_FOUND`) registers no C6 or
/// C10 at all. The old check read cpu0's cpuidle tree alone and would have reported
/// `Pass`, even though cpu0 is not one of the CPUs the run isolates. Finding 8 of
/// `01-EXTERNAL-AUDIT.md`.
#[test]
fn deep_cstates_checks_every_target_cpu() {
    let text = format!(
        "{RIG_AS_FOUND}\n\
         /sys/devices/system/cpu/cpu7/cpuidle/state0/name=C6\n\
         /sys/devices/system/cpu/cpu7/cpuidle/state0/disable=0\n"
    );
    let facts = FixtureFacts::parse(&text);
    let result = deep_cstates_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
    assert!(
        result.observed.contains("cpu7"),
        "observed should name cpu7: {}",
        result.observed
    );
}

/// Every target CPU appears in the observed string by name once any of them disagrees,
/// so a reader can see which CPU was actually read rather than trusting a summary.
#[test]
fn deep_cstates_reports_per_cpu_observation() {
    let text = format!(
        "{RIG_AS_FOUND}\n\
         /sys/devices/system/cpu/cpu7/cpuidle/state0/name=C6\n\
         /sys/devices/system/cpu/cpu7/cpuidle/state0/disable=0\n"
    );
    let facts = FixtureFacts::parse(&text);
    let result = deep_cstates_result(&run_all(&facts, &headline_spec()));
    for cpu in TARGET_CPUS {
        assert!(
            result.observed.contains(&format!("cpu{cpu}")),
            "observed should name cpu{cpu}: {}",
            result.observed
        );
    }
}

/// No target CPU registers C6 or C10 at all (the real `intel_idle.max_cstate=1`
/// rationale `RIG_AS_FOUND` already reflects for cpu0), so every target CPU passes by
/// absence, and the observed string still names every CPU checked, collapsed to a
/// range since every one of them agrees.
#[test]
fn deep_cstates_absent_states_still_pass() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let result = deep_cstates_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Pass, "{result:#?}");
    assert_eq!(
        result.observed,
        "C6 not present, C10 not present on cpus 6-11"
    );
}

fn tracer_result(results: &[PreconditionResult]) -> PreconditionResult {
    results
        .iter()
        .find(|r| r.check == PreconditionCheck::TracersQuiescent)
        .expect("TracersQuiescent result present")
        .clone()
}

/// `current_tracer=nop` alone (RIG_AS_FOUND's own value) used to satisfy
/// `TracersQuiescent`, even with event tracing armed through the independent
/// `events/enable` control. Finding 8 of `01-EXTERNAL-AUDIT.md`.
#[test]
fn tracers_quiescent_fails_on_enabled_events() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND)
        .with("/sys/kernel/tracing/events/enable", "1")
        .with("/sys/kernel/tracing/set_event", "")
        .with("/sys/kernel/tracing/tracing_on", "0");
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
}

#[test]
fn tracers_quiescent_fails_on_set_event() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND)
        .with("/sys/kernel/tracing/events/enable", "0")
        .with("/sys/kernel/tracing/set_event", "sched:sched_switch")
        .with("/sys/kernel/tracing/tracing_on", "0");
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
}

#[test]
fn tracers_quiescent_fails_on_tracing_on() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND)
        .with("/sys/kernel/tracing/events/enable", "0")
        .with("/sys/kernel/tracing/set_event", "")
        .with("/sys/kernel/tracing/tracing_on", "1");
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
}

/// The observed string names the offending control by path, not only that something
/// somewhere is armed.
#[test]
fn tracers_quiescent_reports_which_control_is_armed() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND)
        .with("/sys/kernel/tracing/events/enable", "1")
        .with("/sys/kernel/tracing/set_event", "")
        .with("/sys/kernel/tracing/tracing_on", "0");
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
    assert!(
        result.observed.contains("events/enable=1"),
        "observed should name the armed control: {}",
        result.observed
    );
}
