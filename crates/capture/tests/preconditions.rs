//! Integration tests for the D-06 preconditions, driven entirely by fixture data so
//! they run identically on macOS and Linux (RESEARCH.md pitfall 5).

use nr_capture::environment;
use nr_capture::preconditions::{PreconditionSpec, refuse_on_violation, run_all};
use nr_capture::sources::FixtureFacts;
use nr_manifest::{
    InstrumentClass, PreconditionCheck, PreconditionResult, PreconditionStatus, RunClass,
    ThermalProfile,
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
        thermal_profile: ThermalProfile::Normal,
    }
}

fn investigation_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::Investigation,
        run_class: RunClass::Investigation,
        target_cpus: TARGET_CPUS.to_vec(),
        thermal_profile: ThermalProfile::Normal,
    }
}

/// A D-18 firmware screen that has NOT declared itself thermally exempt. The
/// exemption now follows the declared profile, not the run class (finding 5,
/// `01-EXTERNAL-AUDIT.md`), so a plain screen-class run behaves exactly like any
/// other `Normal`-profile run for this check; see `hot_screen_spec` for the
/// deliberately-hot D-18 arm.
fn screen_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::HeadlineSeries,
        run_class: RunClass::Screen,
        target_cpus: TARGET_CPUS.to_vec(),
        thermal_profile: ThermalProfile::Normal,
    }
}

/// A D-18 firmware screen that has declared `--thermal-profile hot-screen`: the one
/// combination the exemption actually protects. It saturates the machine on purpose,
/// so it is the one declared profile that legitimately starts hot.
fn hot_screen_spec() -> PreconditionSpec {
    PreconditionSpec {
        instrument_class: InstrumentClass::HeadlineSeries,
        run_class: RunClass::Screen,
        target_cpus: TARGET_CPUS.to_vec(),
        thermal_profile: ThermalProfile::HotScreen,
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
    // VIOLATED only ever captured current_tracer; the other three controls, the
    // instances directory and the process list are genuinely absent from this
    // fixture and read back as unavailable or none, not as a second, fabricated
    // violation.
    assert_eq!(
        tracer_result.observed,
        "current_tracer=timerlat events/enable=unavailable set_event=unavailable \
         tracing_on=unavailable instances=unavailable samplers=none"
    );
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
    // NotApplicable still records what was observed, now including the two signals
    // this plan adds (instances=, samplers=) alongside current_tracer.
    assert_eq!(
        tracer_result.observed,
        "timerlat instances=unavailable samplers=none"
    );
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
/// starting thermally throttled, which is a different question, so a run that has
/// DECLARED itself a hot screen (`--thermal-profile hot-screen`) must not be refused
/// for starting hot. Discovered the hard way: with the check applied unconditionally,
/// arm 2 was refused at 78 C and the arms were impossible to take through the harness
/// at all, even though the 70 C ceiling's own justification table is built from those
/// very under-load rows.
///
/// The exemption follows the DECLARATION, not the run class (finding 5,
/// `01-EXTERNAL-AUDIT.md`): it used to key on `RunClass::Screen` alone, which exempted
/// an unintentionally hot idle screen just as readily as a deliberately saturated one.
#[test]
fn hot_screen_profile_exempts_a_hot_start() {
    let hot = RIG_AS_FOUND.replace("thermal.x86_pkg_temp=64000", "thermal.x86_pkg_temp=88000");
    let facts = FixtureFacts::parse(&hot);

    let result = thermal_result(&run_all(&facts, &hot_screen_spec()));
    assert_eq!(
        result.status,
        PreconditionStatus::NotApplicable,
        "a declared hot screen must not be refused for starting hot: {result:#?}"
    );
    assert_eq!(
        result.observed, "88.0 C",
        "NotApplicable still records the real observed temperature"
    );
}

/// A `screen`-class run that never declared `--thermal-profile hot-screen` gets no
/// exemption at all: the class alone used to be sufficient, and an unintentionally hot
/// idle screen was exempted right along with a deliberately saturated one. This is the
/// behaviour finding 5 asked for.
#[test]
fn normal_profile_refuses_a_hot_screen() {
    let hot = RIG_AS_FOUND.replace("thermal.x86_pkg_temp=64000", "thermal.x86_pkg_temp=88000");
    let facts = FixtureFacts::parse(&hot);

    let result = thermal_result(&run_all(&facts, &screen_spec()));
    assert_eq!(
        result.status,
        PreconditionStatus::Fail,
        "a screen run that never declared hot-screen must not be exempted just for being \
         --class screen: {result:#?}"
    );
    assert_eq!(result.observed, "88.0 C");
}

/// A declared hot screen is exempt even when the observation itself is cool: the
/// `HotScreen` arm performs no temperature comparison at all, so applicability is
/// settled entirely by the declaration, before anything is read.
#[test]
fn hot_screen_profile_still_records_a_cool_start() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let result = thermal_result(&run_all(&facts, &hot_screen_spec()));
    assert_eq!(
        result.status,
        PreconditionStatus::NotApplicable,
        "{result:#?}"
    );
    assert_eq!(
        result.observed, "69.1 C",
        "NotApplicable still records the real observed temperature, even when it is cool"
    );
}

/// The normal profile (every class other than a declared hot screen) behaves exactly
/// as the unconditional check did before this plan: passes at or below the ceiling,
/// fails above it.
#[test]
fn normal_profile_is_the_default() {
    let cool = FixtureFacts::parse(RIG_AS_FOUND);
    let cool_result = thermal_result(&run_all(&cool, &headline_spec()));
    assert_eq!(
        cool_result.status,
        PreconditionStatus::Pass,
        "{cool_result:#?}"
    );

    let hot_text = RIG_AS_FOUND.replace("thermal.x86_pkg_temp=64000", "thermal.x86_pkg_temp=92000");
    let hot = FixtureFacts::parse(&hot_text);
    let hot_result = thermal_result(&run_all(&hot, &headline_spec()));
    assert_eq!(
        hot_result.status,
        PreconditionStatus::Fail,
        "{hot_result:#?}"
    );
    assert_eq!(hot_result.observed, "92.0 C");
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

/// Extends a fixture with a real per-CPU cpuidle registration (POLL, C1E; no C6/C10)
/// on every target CPU, matching the fact that `intel_idle.max_cstate=1` is a global
/// boot parameter rather than a per-CPU setting: every core on this rig registers the
/// same states. `RIG_AS_FOUND`'s own probe only ever read cpu0 (exactly the limitation
/// this plan closes), so a test that needs a genuine "registered but absent" reading
/// on the isolated cores, rather than "no data was read at all", supplies this
/// explicitly.
fn cstates_present_on_every_target_cpu() -> String {
    let mut text = String::new();
    for cpu in TARGET_CPUS {
        text.push_str(&format!(
            "/sys/devices/system/cpu/cpu{cpu}/cpuidle/state0/name=POLL\n\
             /sys/devices/system/cpu/cpu{cpu}/cpuidle/state0/disable=0\n\
             /sys/devices/system/cpu/cpu{cpu}/cpuidle/state1/name=C1E\n\
             /sys/devices/system/cpu/cpu{cpu}/cpuidle/state1/disable=0\n"
        ));
    }
    text
}

/// No target CPU registers C6 or C10 at all (only POLL and C1E, the real
/// `intel_idle.max_cstate=1` rationale), so every target CPU passes by absence, and
/// the observed string still names every CPU checked, collapsed to a range since
/// every one of them agrees.
#[test]
fn deep_cstates_absent_states_still_pass() {
    let text = format!("{RIG_AS_FOUND}\n{}", cstates_present_on_every_target_cpu());
    let facts = FixtureFacts::parse(&text);
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

/// A fixture where the four top-level controls all read quiescent, so anything this
/// test catches is caught by the two new signals alone. Named for the real capture it
/// exists because of: `measurements/2026-09-06-precision3591-screen-02`, taken
/// 2026-09-06 after a killed `rtla` left an osnoise instance running on cpus 6-11.
/// That run's cyclictest lost 75 percent of its cycles on every isolated thread
/// (thread 0, cpu 6: 75055 of an expected 300000 cycles; thread 5, cpu 11: 75051),
/// every thread's maximum stalled at roughly 750021us, which is
/// `/sys/kernel/tracing/osnoise/runtime_us` (750000 of a 1000000 `period_us`). All
/// fifteen preconditions passed on that run.
fn quiescent_top_level_facts() -> nr_capture::sources::FixtureFacts {
    FixtureFacts::parse(RIG_AS_FOUND)
        .with("/sys/kernel/tracing/events/enable", "0")
        .with("/sys/kernel/tracing/set_event", "")
        .with("/sys/kernel/tracing/tracing_on", "0")
}

#[test]
fn orphaned_osnoise_kthread_on_a_target_cpu_fails() {
    let facts = quiescent_top_level_facts().with("processes.running", "osnoise/6");
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
    assert!(
        result.observed.contains("osnoise/6"),
        "observed should name the orphaned kthread: {}",
        result.observed
    );
}

#[test]
fn orphaned_timerlat_kthread_on_a_target_cpu_fails() {
    let facts = quiescent_top_level_facts().with("processes.running", "timerlat/9");
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
    assert!(
        result.observed.contains("timerlat/9"),
        "observed should name the orphaned kthread: {}",
        result.observed
    );
}

/// A sampler thread on a CPU this run is not measuring is not this check's business:
/// target_cpus is 6-11, and cpu 3 is a housekeeping core.
#[test]
fn a_sampling_thread_on_a_non_target_cpu_does_not_fail() {
    let facts = quiescent_top_level_facts().with("processes.running", "osnoise/3");
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Pass, "{result:#?}");
    assert!(
        result.observed.contains("samplers=none"),
        "cpu 3 is not a target cpu, so no sampler should be reported: {}",
        result.observed
    );
}

#[test]
fn a_live_tracing_instance_fails() {
    let facts = quiescent_top_level_facts()
        .with_dir("/sys/kernel/tracing/instances", &["osnoise_top"])
        .with(
            "/sys/kernel/tracing/instances/osnoise_top/current_tracer",
            "osnoise",
        );
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Fail, "{result:#?}");
    assert!(
        result.observed.contains("osnoise_top"),
        "observed should name the live instance: {}",
        result.observed
    );
}

#[test]
fn an_empty_instances_directory_passes() {
    let facts = quiescent_top_level_facts().with_dir("/sys/kernel/tracing/instances", &[]);
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Pass, "{result:#?}");
    assert!(
        result.observed.contains("instances=none"),
        "an empty, readable instances directory is not unavailable: {}",
        result.observed
    );
}

/// `RIG_AS_FOUND` never calls `with_dir` for `/sys/kernel/tracing/instances`, so
/// `list_dir` genuinely fails here, the same "no with_dir call" shape every other
/// fixture-driven test in this suite uses to represent a path the probe never
/// captured. A machine where this directory cannot be read (permissions, or a
/// kernel with no instances support) must not be refused a run on that basis alone.
#[test]
fn an_unreadable_instances_directory_is_not_a_violation() {
    let facts = quiescent_top_level_facts();
    let result = tracer_result(&run_all(&facts, &headline_spec()));
    assert_eq!(result.status, PreconditionStatus::Pass, "{result:#?}");
    assert!(
        result.observed.contains("instances=unavailable"),
        "observed should record the unreadable instances directory: {}",
        result.observed
    );
}

#[test]
fn investigation_records_both_new_signals_without_failing() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND).with("processes.running", "osnoise/6");
    let result = tracer_result(&run_all(&facts, &investigation_spec()));
    assert_eq!(
        result.status,
        PreconditionStatus::NotApplicable,
        "{result:#?}"
    );
    assert!(
        result.observed.contains("osnoise/6"),
        "observed should still name the kthread on an investigation run: {}",
        result.observed
    );
}

#[test]
fn all_fifteen_preconditions_are_still_evaluated() {
    let facts = FixtureFacts::parse(RIG_AS_FOUND);
    let results = run_all(&facts, &headline_spec());
    assert_eq!(results.len(), 15, "got {results:#?}");
    for check in ALL_CHECKS {
        assert!(
            results.iter().any(|r| r.check == check),
            "missing a result for {check:?}"
        );
    }
}
