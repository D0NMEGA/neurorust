//! Integration tests for the D-06 preconditions, driven entirely by fixture data so
//! they run identically on macOS and Linux (RESEARCH.md pitfall 5).

use nr_capture::environment;
use nr_capture::preconditions::{PreconditionSpec, refuse_on_violation, run_all};
use nr_capture::sources::FixtureFacts;
use nr_manifest::{InstrumentClass, PreconditionCheck, PreconditionResult, PreconditionStatus};

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
        target_cpus: TARGET_CPUS.to_vec(),
    }
}

fn investigation_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::Investigation,
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

    let snap = environment::snapshot(&facts, "test-rig").expect("snapshot succeeds");
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
