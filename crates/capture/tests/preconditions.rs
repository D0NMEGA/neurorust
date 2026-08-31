//! Integration tests for the D-06 preconditions, driven entirely by fixture data so
//! they run identically on macOS and Linux (RESEARCH.md pitfall 5).

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

const ALL_CHECKS: [PreconditionCheck; 14] = [
    PreconditionCheck::NoActiveSshSessions,
    PreconditionCheck::SystemdDefaultTargetIsMultiUser,
    PreconditionCheck::DisplayManagerInactive,
    PreconditionCheck::NoGraphicalSession,
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
    assert_eq!(as_found_results.len(), 14, "got {as_found_results:#?}");

    let violated_facts = FixtureFacts::parse(VIOLATED);
    let violated_results = run_all(&violated_facts, &headline_spec());
    assert_eq!(violated_results.len(), 14, "got {violated_results:#?}");

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
