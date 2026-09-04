//! The D-06 precondition checks: assert and record, never enforce.
//!
//! Every check in [`run_all`] returns a [`nr_manifest::PreconditionResult`] whether it
//! passed or failed. [`refuse_on_violation`] is the only place a violation stops a
//! run; the checks themselves never touch system state (D-06, T-1-21). Because every
//! check reads through [`crate::sources::SystemFacts`] rather than the filesystem
//! directly, this module compiles and is fully tested on macOS as well as Linux
//! (RESEARCH.md pitfall 5).
//!
//! [`TracersQuiescent`](nr_manifest::PreconditionCheck::TracersQuiescent) mechanises
//! RESEARCH.md pattern 2 (the two-instrument split): a headline-class run is refused
//! when a tracer is armed, so investigation overhead can never leak into the
//! published series, even by mistake.

use nr_manifest::{
    InstrumentClass, PreconditionCheck, PreconditionResult, PreconditionStatus, RunClass,
};

use crate::sources::{
    SystemFacts, discover_ac_online, discover_cstates, discover_governed_cpu_count,
    discover_thermal_zones_c, format_cpu_ranges, parse_cpu_list,
};

/// Package temperature ceiling for [`PreconditionCheck::ThermalHeadroomAtStart`].
///
/// Derived from measurement, not assumed. The original 60 C value sat one degree above the
/// reference rig's own idle floor (58 to 59 C with fans at ~2200 RPM and a load average of
/// 0.26), which made every run a coin flip on a one-degree margin and blocked the D-17
/// calibration pair outright.
///
/// The evidence for 70 C, all from `hwlatdetect` at a 10 us threshold:
///
/// | Package temp | Condition                          | Events > 10 us | Max   |
/// |--------------|------------------------------------|----------------|-------|
/// | 59 C         | installed RT kernel, idle          | 0 / 900 s      | none  |
/// | 91 to 93 C   | stock kernel, 22 cores saturated   | 26 / 900 s     | 29 us |
/// | 93 to 95 C   | stock kernel, P-cores saturated    | 13 / 600 s     | 22 us |
///
/// The 59 C row is `docs/rig/d18-firmware-floor-rt-2026-09-01.txt`; the others are the
/// 2026-08-28 screening under `measurements/2026-08-28-precision3591/`. Firmware latency is
/// an SMI property and independent of which kernel is running, which the 59 C row confirms
/// by reproducing the live-USB idle result on the installed PREEMPT_RT system.
///
/// 70 C sits 11 C above the measured idle floor, so a run can actually start, and more than
/// 20 C below the lowest temperature at which any SMI has been observed on this hardware.
/// The band between 59 and 91 C remains unmeasured; narrowing it would require heating the
/// package to a series of setpoints and re-running `hwlatdetect` at each. Until that exists,
/// 70 C is a bounded extrapolation rather than a measured boundary, and it is stated as such.
///
/// This is a start-of-run gate only. A run that heats past this ceiling mid-flight is still
/// visible after the fact: the manifest records `temp_c_start`, `temp_c_end` and
/// `package_temp_c_max` per thermal zone, so thermal excursions are auditable rather than
/// silently absorbed.
const THERMAL_HEADROOM_CEILING_C: f32 = 70.0;

const DISPLAY_MANAGERS: [&str; 3] = ["gdm.service", "sddm.service", "lightdm.service"];
const TARGET_DEEP_CSTATES: [&str; 2] = ["C6", "C10"];
const PACKAGE_MANAGER_PROCESSES: [&str; 4] = ["apt", "dpkg", "unattended-upgrade", "snapd"];

/// What a run declares before its preconditions are evaluated: the instrument class
/// (RESEARCH.md pattern 2) and the CPU list the run intends to isolate work onto.
#[derive(Debug, Clone)]
pub struct PreconditionSpec {
    pub instrument_class: InstrumentClass,
    /// The run's cadence class. Only [`check_thermal_headroom_at_start`] consults it, to
    /// exempt a firmware screen whose whole purpose is to start hot.
    pub run_class: RunClass,
    pub target_cpus: Vec<u32>,
}

/// Runs every [`PreconditionCheck`] variant, in a stable order, and returns exactly
/// one [`PreconditionResult`] per variant. A check that could not be evaluated
/// returns [`PreconditionStatus::Unavailable`] naming the missing source; it is never
/// omitted, because a silently missing check is indistinguishable from one that
/// passed.
pub fn run_all(facts: &dyn SystemFacts, spec: &PreconditionSpec) -> Vec<PreconditionResult> {
    vec![
        check_no_active_ssh_sessions(facts),
        check_systemd_default_target(facts),
        check_display_manager_inactive(facts),
        check_no_graphical_session(facts),
        check_no_active_login_sessions(facts),
        check_governor_is_performance(facts),
        check_no_turbo_enabled(facts),
        check_deep_cstates_disabled(facts),
        check_isolcpus_covers_target_cpus(facts, &spec.target_cpus),
        check_kernel_is_realtime(facts),
        check_rt_tuning_service_active(facts),
        check_on_ac_power(facts),
        check_thermal_headroom_at_start(facts, &spec.run_class),
        check_no_package_manager_activity(facts),
        check_tracers_quiescent(facts, &spec.instrument_class),
    ]
}

/// Every offending check from a [`refuse_on_violation`] call, so an operator sees
/// every violation in one pass rather than fixing one and finding the next on a
/// second trip to the rig.
#[derive(Debug)]
pub struct RefusalError {
    pub offenses: Vec<PreconditionResult>,
}

impl std::fmt::Display for RefusalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "run refused: {} precondition(s) violated",
            self.offenses.len()
        )?;
        for offense in &self.offenses {
            writeln!(
                f,
                "  {:?}: observed {:?}, expected {:?} ({:?})",
                offense.check, offense.observed, offense.expected, offense.status
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for RefusalError {}

/// Returns `Err` when any result is [`PreconditionStatus::Fail`], or when any result
/// is [`PreconditionStatus::Unavailable`] and `instrument_class` is
/// [`InstrumentClass::HeadlineSeries`] (a check that could not be evaluated is never
/// treated as a pass for the series that gets published). An `Investigation` run may
/// proceed with an `Unavailable` result, since it never feeds the published series.
pub fn refuse_on_violation(
    results: &[PreconditionResult],
    instrument_class: &InstrumentClass,
) -> Result<(), RefusalError> {
    let offenses: Vec<PreconditionResult> = results
        .iter()
        .filter(|r| {
            r.status == PreconditionStatus::Fail
                || (r.status == PreconditionStatus::Unavailable
                    && *instrument_class == InstrumentClass::HeadlineSeries)
        })
        .cloned()
        .collect();

    if offenses.is_empty() {
        Ok(())
    } else {
        Err(RefusalError { offenses })
    }
}

fn unavailable(
    check: PreconditionCheck,
    observed: impl Into<String>,
    expected: impl Into<String>,
) -> PreconditionResult {
    PreconditionResult {
        check,
        status: PreconditionStatus::Unavailable,
        observed: observed.into(),
        expected: expected.into(),
    }
}

fn check_no_active_ssh_sessions(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "0";
    match facts.active_ssh_sessions() {
        Ok(n) => PreconditionResult {
            check: PreconditionCheck::NoActiveSshSessions,
            status: if n == 0 {
                PreconditionStatus::Pass
            } else {
                PreconditionStatus::Fail
            },
            observed: n.to_string(),
            expected: expected.to_string(),
        },
        Err(_) => unavailable(
            PreconditionCheck::NoActiveSshSessions,
            "ssh session count not available",
            expected,
        ),
    }
}

fn check_systemd_default_target(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "multi-user.target";
    match facts.systemctl_get_default() {
        Ok(target) => PreconditionResult {
            check: PreconditionCheck::SystemdDefaultTargetIsMultiUser,
            status: if target == expected {
                PreconditionStatus::Pass
            } else {
                PreconditionStatus::Fail
            },
            observed: target,
            expected: expected.to_string(),
        },
        Err(_) => unavailable(
            PreconditionCheck::SystemdDefaultTargetIsMultiUser,
            "systemctl get-default not available",
            expected,
        ),
    }
}

fn check_display_manager_inactive(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "inactive";
    let mut observed_parts = Vec::new();
    let mut any_found = false;
    let mut all_inactive = true;

    for unit in DISPLAY_MANAGERS {
        if let Ok(props) = facts.systemctl_show(unit, &["ActiveState"]) {
            if let Some(state) = props.get("ActiveState") {
                any_found = true;
                if state != "inactive" {
                    all_inactive = false;
                }
                observed_parts.push(format!("{unit}={state}"));
            }
        }
    }

    if !any_found {
        return unavailable(
            PreconditionCheck::DisplayManagerInactive,
            "no display-manager unit state available",
            expected,
        );
    }

    PreconditionResult {
        check: PreconditionCheck::DisplayManagerInactive,
        status: if all_inactive {
            PreconditionStatus::Pass
        } else {
            PreconditionStatus::Fail
        },
        observed: observed_parts.join(", "),
        expected: expected.to_string(),
    }
}

fn check_no_graphical_session(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "0";
    match facts.graphical_sessions() {
        Ok(n) => PreconditionResult {
            check: PreconditionCheck::NoGraphicalSession,
            status: if n == 0 {
                PreconditionStatus::Pass
            } else {
                PreconditionStatus::Fail
            },
            observed: n.to_string(),
            expected: expected.to_string(),
        },
        Err(_) => unavailable(
            PreconditionCheck::NoGraphicalSession,
            "graphical session count not available",
            expected,
        ),
    }
}

/// Detects a login session opened at the machine's own physical console, e.g. an
/// operator sitting down and logging in at `tty1` and getting a shell. Found on the
/// reference rig on 2026-09-01: a console login mid-run triggered
/// `update-motd.d`'s own apt and fwupd queries, and none of `NoActiveSshSessions`
/// (established SSH connections only), `NoGraphicalSession` (x11/wayland sessions
/// only) or `DisplayManagerInactive` (gdm/sddm/lightdm unit state only) could see
/// it: a text-mode login at the physical keyboard falls through all three.
///
/// Scoped to sessions of type `tty`, class `user`, on `seat0` specifically so a
/// single incoming SSH connection is never counted here as well as by
/// `NoActiveSshSessions`: an SSH session with an allocated pty also reports
/// `Type=tty` in `loginctl`, but is attached to no seat (a seat names a physical
/// console's own keyboard, display and input devices, which a remote session never
/// has), so `Seat=seat0` is what tells the two apart without double-counting one
/// SSH connection as two violations. A `systemd-run` transient unit (how this
/// protocol has `nrmeasure` launched; see docs/measurement-protocol.md) opens no
/// login session at all, so the harness's own normal invocation never trips this
/// check.
fn check_no_active_login_sessions(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "no local console login session";
    match facts.local_login_sessions() {
        Ok(sessions) => PreconditionResult {
            check: PreconditionCheck::NoActiveLoginSessions,
            status: if sessions.is_empty() {
                PreconditionStatus::Pass
            } else {
                PreconditionStatus::Fail
            },
            observed: if sessions.is_empty() {
                "none".to_string()
            } else {
                sessions.join(", ")
            },
            expected: expected.to_string(),
        },
        Err(_) => unavailable(
            PreconditionCheck::NoActiveLoginSessions,
            "loginctl session list not available",
            expected,
        ),
    }
}

/// Reads every CPU's governor directly from `cpufreq/scaling_governor` rather than
/// trusting `rt-tuning.service`'s `ActiveState`. `docs/rig/recon-2026-08-31/
/// FINDINGS.md` ("The rt-tuning.service contradiction") found the service can report
/// healthy while every governor still reads `powersave`, because
/// `power-profiles-daemon` wins a boot-time race and expresses "performance" as
/// `scaling_governor=powersave` plus `energy_performance_preference=performance` on
/// this Meteor Lake HWP backend. A service reporting healthy tells you it ran, not
/// that its effect survived; this check is what actually verifies the effect.
fn check_governor_is_performance(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "performance on all CPUs";
    let cpu_count = discover_governed_cpu_count(facts);
    if cpu_count == 0 {
        return unavailable(
            PreconditionCheck::GovernorIsPerformanceOnAllCpus,
            "no scaling_governor files found",
            expected,
        );
    }

    let mut offending: Option<(u32, String)> = None;
    for cpu in 0..cpu_count {
        let path = format!("/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor");
        let governor = facts
            .read_text(&path)
            .unwrap_or_default()
            .trim()
            .to_string();
        if governor != "performance" {
            offending = Some((cpu, governor));
            break;
        }
    }

    match offending {
        None => PreconditionResult {
            check: PreconditionCheck::GovernorIsPerformanceOnAllCpus,
            status: PreconditionStatus::Pass,
            observed: format!("performance on {cpu_count} CPUs"),
            expected: expected.to_string(),
        },
        Some((cpu, governor)) => PreconditionResult {
            check: PreconditionCheck::GovernorIsPerformanceOnAllCpus,
            status: PreconditionStatus::Fail,
            observed: format!("cpu{cpu}={governor}"),
            expected: expected.to_string(),
        },
    }
}

fn check_no_turbo_enabled(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "1";
    match facts.read_text("/sys/devices/system/cpu/intel_pstate/no_turbo") {
        Ok(text) => {
            let observed = text.trim().to_string();
            PreconditionResult {
                check: PreconditionCheck::NoTurboEnabled,
                status: if observed == expected {
                    PreconditionStatus::Pass
                } else {
                    PreconditionStatus::Fail
                },
                observed,
                expected: expected.to_string(),
            }
        }
        Err(_) => unavailable(
            PreconditionCheck::NoTurboEnabled,
            "intel_pstate/no_turbo not present",
            expected,
        ),
    }
}

/// A deep C-state that was never registered by the cpuidle driver (e.g. under
/// `intel_idle.max_cstate=1`) cannot fire, so its absence counts as satisfied rather
/// than as a failure to read a `disable` flag that does not exist. See
/// `docs/rig/recon-2026-08-31/FINDINGS.md`, "Downstream implication for plan 01-05".
fn check_deep_cstates_disabled(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "C6 disabled, C10 disabled";
    let states = discover_cstates(facts, 0);
    if states.is_empty() {
        return unavailable(
            PreconditionCheck::DeepCstatesDisabled,
            "no cpuidle states found on cpu0",
            expected,
        );
    }

    let mut observed_parts = Vec::new();
    let mut violated = false;
    for target in TARGET_DEEP_CSTATES {
        match states.iter().find(|(name, _)| name == target) {
            Some((_, disabled)) => {
                observed_parts.push(format!(
                    "{target}={}",
                    if *disabled { "disabled" } else { "enabled" }
                ));
                if !disabled {
                    violated = true;
                }
            }
            None => observed_parts.push(format!("{target} not present")),
        }
    }

    PreconditionResult {
        check: PreconditionCheck::DeepCstatesDisabled,
        status: if violated {
            PreconditionStatus::Fail
        } else {
            PreconditionStatus::Pass
        },
        observed: observed_parts.join(", "),
        expected: expected.to_string(),
    }
}

fn check_isolcpus_covers_target_cpus(
    facts: &dyn SystemFacts,
    target_cpus: &[u32],
) -> PreconditionResult {
    let expected = format_cpu_ranges(target_cpus);
    match facts.read_text("/sys/devices/system/cpu/isolated") {
        Ok(text) => {
            let observed = text.trim().to_string();
            let isolated = parse_cpu_list(&observed);
            let covers = target_cpus.iter().all(|cpu| isolated.contains(cpu));
            PreconditionResult {
                check: PreconditionCheck::IsolcpusCoversTargetCpus,
                status: if covers {
                    PreconditionStatus::Pass
                } else {
                    PreconditionStatus::Fail
                },
                observed,
                expected,
            }
        }
        Err(_) => unavailable(
            PreconditionCheck::IsolcpusCoversTargetCpus,
            "/sys/devices/system/cpu/isolated not present",
            expected,
        ),
    }
}

fn check_kernel_is_realtime(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "1";
    match facts.read_text("/sys/kernel/realtime") {
        Ok(text) => {
            let observed = text.trim().to_string();
            PreconditionResult {
                check: PreconditionCheck::KernelIsRealtime,
                status: if observed == expected {
                    PreconditionStatus::Pass
                } else {
                    PreconditionStatus::Fail
                },
                observed,
                expected: expected.to_string(),
            }
        }
        Err(_) => unavailable(
            PreconditionCheck::KernelIsRealtime,
            "/sys/kernel/realtime not present",
            expected,
        ),
    }
}

fn check_rt_tuning_service_active(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "active";
    let props = facts.systemctl_show("rt-tuning.service", &["ActiveState"]);
    match props.ok().and_then(|p| p.get("ActiveState").cloned()) {
        Some(state) => PreconditionResult {
            check: PreconditionCheck::RtTuningServiceActive,
            status: if state == expected {
                PreconditionStatus::Pass
            } else {
                PreconditionStatus::Fail
            },
            observed: state,
            expected: expected.to_string(),
        },
        None => unavailable(
            PreconditionCheck::RtTuningServiceActive,
            "rt-tuning.service ActiveState not available",
            expected,
        ),
    }
}

/// The 2026-08-28 README names charge controllers as a known SMI source, which is
/// why AC power is asserted here rather than assumed.
fn check_on_ac_power(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "AC=1";
    match discover_ac_online(facts) {
        Some(value) => PreconditionResult {
            check: PreconditionCheck::OnAcPower,
            status: if value == "1" {
                PreconditionStatus::Pass
            } else {
                PreconditionStatus::Fail
            },
            observed: format!("AC={value}"),
            expected: expected.to_string(),
        },
        None => unavailable(
            PreconditionCheck::OnAcPower,
            "no AC power_supply entry found",
            expected,
        ),
    }
}

/// Refuses a run that starts thermally loaded, except for [`RunClass::Screen`].
///
/// The ceiling protects a *latency* measurement from starting throttled. A firmware
/// screen asks a different question: `hwlatdetect` under D-18 saturates 22 cores on
/// purpose to provoke load-triggered SMIs, and the 2026-08-28 screening it is compared
/// against ran at 91 to 93 C whole-machine and 93 to 95 C P-core-only. Applying the
/// ceiling to a screen made those arms impossible to take through the harness at all:
/// arm 2 was refused at 78 C on 2026-09-04, even though this very constant's evidence
/// table is built from the under-load rows it was forbidding.
///
/// A screen still records its real observed temperature as
/// [`PreconditionStatus::NotApplicable`], never a silent omission and never a fake pass,
/// and the manifest keeps `temp_c_start`, `temp_c_end` and `package_temp_c_max` per zone
/// regardless. Every other class, including `Headline` and `Weekly`, is unchanged.
fn check_thermal_headroom_at_start(
    facts: &dyn SystemFacts,
    run_class: &RunClass,
) -> PreconditionResult {
    let expected = format!("package temp <= {THERMAL_HEADROOM_CEILING_C} C");
    let zones = discover_thermal_zones_c(facts);
    if zones.is_empty() {
        return unavailable(
            PreconditionCheck::ThermalHeadroomAtStart,
            "no thermal zones found",
            expected,
        );
    }

    let max_c = zones.iter().map(|(_, c)| *c).fold(f32::MIN, f32::max);
    let status = match run_class {
        RunClass::Screen if max_c > THERMAL_HEADROOM_CEILING_C => PreconditionStatus::NotApplicable,
        _ if max_c <= THERMAL_HEADROOM_CEILING_C => PreconditionStatus::Pass,
        _ => PreconditionStatus::Fail,
    };
    PreconditionResult {
        check: PreconditionCheck::ThermalHeadroomAtStart,
        status,
        observed: format!("{max_c:.1} C"),
        expected,
    }
}

/// Ubuntu runs `unattended-upgrades` on a timer, and it is a real contamination
/// source on a machine that measures for an hour.
fn check_no_package_manager_activity(facts: &dyn SystemFacts) -> PreconditionResult {
    let expected = "no apt, dpkg, unattended-upgrade or snapd process";
    match facts.running_processes_matching(&PACKAGE_MANAGER_PROCESSES) {
        Ok(found) => PreconditionResult {
            check: PreconditionCheck::NoPackageManagerActivity,
            status: if found.is_empty() {
                PreconditionStatus::Pass
            } else {
                PreconditionStatus::Fail
            },
            observed: if found.is_empty() {
                "none".to_string()
            } else {
                found.join(", ")
            },
            expected: expected.to_string(),
        },
        Err(_) => unavailable(
            PreconditionCheck::NoPackageManagerActivity,
            "process list not available",
            expected,
        ),
    }
}

/// RESEARCH.md pattern 2, mechanised: a headline-class run is refused when a tracer
/// is armed, so an investigation run's overhead can never leak into the published
/// series, even by mistake.
fn check_tracers_quiescent(
    facts: &dyn SystemFacts,
    instrument_class: &InstrumentClass,
) -> PreconditionResult {
    let expected = "nop";
    let observed = facts
        .read_text("/sys/kernel/tracing/current_tracer")
        .map(|t| t.trim().to_string());

    match instrument_class {
        InstrumentClass::HeadlineSeries => match observed {
            Ok(tracer) => PreconditionResult {
                check: PreconditionCheck::TracersQuiescent,
                status: if tracer == expected {
                    PreconditionStatus::Pass
                } else {
                    PreconditionStatus::Fail
                },
                observed: tracer,
                expected: expected.to_string(),
            },
            Err(_) => unavailable(
                PreconditionCheck::TracersQuiescent,
                "current_tracer not available",
                expected,
            ),
        },
        InstrumentClass::Investigation => PreconditionResult {
            check: PreconditionCheck::TracersQuiescent,
            status: PreconditionStatus::NotApplicable,
            observed: observed.unwrap_or_else(|_| "unavailable".to_string()),
            expected: expected.to_string(),
        },
    }
}
