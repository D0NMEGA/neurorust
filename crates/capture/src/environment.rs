//! The D-14 environment snapshot: fills [`nr_manifest`]'s host/kernel/os/tuning/power/
//! network structs from real system sources, read through [`SystemFacts`] so this
//! module is fully tested on macOS as well as Linux.
//!
//! Every one of [`HostInfo`], [`KernelInfo`], [`OsInfo`], [`TuningInfo`],
//! [`PowerInfo`] and [`NetworkInfo`]'s fields is required by its type (`String`,
//! `u32`, `bool`, or a non-`Option` struct/enum) except the handful the schema
//! already marks `Option` (cmdline parameters, tuning/power readings that may
//! genuinely be absent on the hardware). D-16's "never guess" rule still applies to
//! the non-`Option` fields: when a source cannot be read, this module writes an
//! empty string (or `0`, or `false`) rather than a plausible-looking value, and
//! records the field in [`EnvironmentSnapshot::absent_fields`] so a reader can tell
//! "not recorded" from "recorded as empty".
//!
//! Never reads a hostname file, a machine-id file, a network hardware address, or a
//! wireless network name (T-1-06); `rig_slug` is supplied by the caller, not
//! discovered.

use std::collections::BTreeSet;

use nr_manifest::{
    AbsentField, CpuGovernor, CstateSetting, HostInfo, KernelInfo, NetworkInfo, OsInfo, PowerInfo,
    ServiceState, SessionKind, ThermalZone, TuningInfo,
};

use crate::sources::{
    FactsError, SystemFacts, discover_ac_online, discover_battery, discover_cstates,
    discover_governed_cpu_count, discover_thermal_zones_c,
};

pub struct EnvironmentSnapshot {
    pub host: HostInfo,
    pub kernel: KernelInfo,
    pub os: OsInfo,
    pub tuning: TuningInfo,
    pub power: PowerInfo,
    pub network: NetworkInfo,
    pub absent_fields: Vec<AbsentField>,
}

/// Fills the D-14 environment snapshot. `rig_slug` is a chosen label (e.g.
/// `"precision3591"`), never a network-derived machine name (T-1-06).
/// `thermal_start` is the thermal-zone reading taken BEFORE the instruments ran, when the
/// caller has one. Without it this function reads the zones once, at the moment it is called,
/// which for a live run is after both instruments have finished. Until 2026-09-04 that single
/// late reading was written into `temp_c_start` while `temp_c_end` was left permanently `None`,
/// so the field named "start" actually held the end temperature and `package_temp_c_max` was
/// one instant rather than a span. `reconstruct` legitimately has only one observation and
/// passes `None`; a live run must pass `Some`. See finding 5 of 01-EXTERNAL-AUDIT.md.
pub fn snapshot(
    facts: &dyn SystemFacts,
    rig_slug: &str,
    thermal_start: Option<&[(String, f32)]>,
) -> Result<EnvironmentSnapshot, FactsError> {
    let mut absent = Vec::new();

    let host = build_host(facts, rig_slug, &mut absent);
    let kernel = build_kernel(facts, &mut absent);
    let os = build_os(facts, &mut absent);
    let tuning = build_tuning(facts, &mut absent);
    let power = build_power(facts, &mut absent, thermal_start);
    let network = build_network(facts, &mut absent);

    Ok(EnvironmentSnapshot {
        host,
        kernel,
        os,
        tuning,
        power,
        network,
        absent_fields: absent,
    })
}

fn note_absent(absent: &mut Vec<AbsentField>, field_path: &str, reason: &str) {
    absent.push(AbsentField {
        field_path: field_path.to_string(),
        reason: reason.to_string(),
    });
}

/// Reads `path` and returns the trimmed text, or records `field_path` as absent and
/// returns `default`.
fn read_or_absent(
    facts: &dyn SystemFacts,
    path: &str,
    field_path: &str,
    absent: &mut Vec<AbsentField>,
) -> String {
    match facts.read_text(path) {
        Ok(text) => text.trim().to_string(),
        Err(_) => {
            note_absent(absent, field_path, &format!("{path} not present"));
            String::new()
        }
    }
}

// ---------------------------------------------------------------------------------
// HostInfo
// ---------------------------------------------------------------------------------

struct CpuInfoSummary {
    model_name: Option<String>,
    microcode: Option<String>,
    logical_cpus: u32,
    physical_cores: u32,
    sockets: u32,
}

fn parse_cpuinfo(text: &str) -> CpuInfoSummary {
    let mut model_name = None;
    let mut microcode = None;
    let mut logical_cpus = 0u32;
    let mut cores: BTreeSet<(String, String)> = BTreeSet::new();
    let mut sockets: BTreeSet<String> = BTreeSet::new();
    let mut physical_id: Option<String> = None;
    let mut core_id: Option<String> = None;

    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim().to_string();
        match key {
            "processor" => {
                logical_cpus += 1;
                physical_id = None;
                core_id = None;
            }
            "model name" if model_name.is_none() => model_name = Some(value),
            "microcode" if microcode.is_none() => microcode = Some(value),
            "physical id" => {
                sockets.insert(value.clone());
                physical_id = Some(value);
            }
            "core id" => core_id = Some(value),
            _ => {}
        }
        if let (Some(p), Some(c)) = (&physical_id, &core_id) {
            cores.insert((p.clone(), c.clone()));
        }
    }

    CpuInfoSummary {
        model_name,
        microcode,
        logical_cpus,
        physical_cores: cores.len() as u32,
        sockets: sockets.len() as u32,
    }
}

/// Partitions CPUs into two groups by a per-CPU discriminating value (`cpu_capacity`
/// or, failing that, max frequency): the higher-valued group is P-cores, the
/// lower-valued group is E-cores. Any shape other than exactly two distinct values
/// cannot be called a clean P/E split, so it is not guessed.
fn partition_by_value(readings: &[(u32, u64)]) -> Option<(String, String)> {
    let distinct: BTreeSet<u64> = readings.iter().map(|(_, v)| *v).collect();
    if distinct.len() != 2 {
        return None;
    }
    let max_value = *distinct.iter().next_back()?;

    let mut high = Vec::new();
    let mut low = Vec::new();
    for &(cpu, value) in readings {
        if value == max_value {
            high.push(cpu);
        } else {
            low.push(cpu);
        }
    }

    Some((
        crate::sources::format_cpu_ranges(&high),
        crate::sources::format_cpu_ranges(&low),
    ))
}

fn discover_p_e_cores(facts: &dyn SystemFacts, cpu_count: u32) -> Option<(String, String)> {
    let capacities: Vec<(u32, u64)> = (0..cpu_count)
        .filter_map(|cpu| {
            facts
                .read_text(&format!("/sys/devices/system/cpu/cpu{cpu}/cpu_capacity"))
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map(|v| (cpu, v))
        })
        .collect();
    if capacities.len() == cpu_count as usize && !capacities.is_empty() {
        if let Some(split) = partition_by_value(&capacities) {
            return Some(split);
        }
    }

    let freqs: Vec<(u32, u64)> = (0..cpu_count)
        .filter_map(|cpu| {
            facts
                .read_text(&format!(
                    "/sys/devices/system/cpu/cpu{cpu}/cpufreq/cpuinfo_max_freq"
                ))
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map(|v| (cpu, v))
        })
        .collect();
    if freqs.len() == cpu_count as usize && !freqs.is_empty() {
        return partition_by_value(&freqs);
    }

    None
}

fn build_host(facts: &dyn SystemFacts, rig_slug: &str, absent: &mut Vec<AbsentField>) -> HostInfo {
    let system_vendor = read_or_absent(
        facts,
        "/sys/class/dmi/id/sys_vendor",
        "host.system_vendor",
        absent,
    );
    let system_model = read_or_absent(
        facts,
        "/sys/class/dmi/id/product_name",
        "host.system_model",
        absent,
    );
    let bios_version = read_or_absent(
        facts,
        "/sys/class/dmi/id/bios_version",
        "host.bios_version",
        absent,
    );
    let bios_release_date = read_or_absent(
        facts,
        "/sys/class/dmi/id/bios_date",
        "host.bios_release_date",
        absent,
    );

    let cpuinfo = facts.read_text("/proc/cpuinfo").ok();
    let summary = cpuinfo.as_deref().map(parse_cpuinfo);

    let cpu_model = summary
        .as_ref()
        .and_then(|s| s.model_name.clone())
        .unwrap_or_else(|| {
            note_absent(
                absent,
                "host.cpu_model",
                "/proc/cpuinfo has no 'model name'",
            );
            String::new()
        });
    let microcode = summary
        .as_ref()
        .and_then(|s| s.microcode.clone())
        .unwrap_or_else(|| {
            note_absent(absent, "host.microcode", "/proc/cpuinfo has no 'microcode'");
            String::new()
        });
    let logical_cpus = match &summary {
        Some(s) if s.logical_cpus > 0 => s.logical_cpus,
        _ => {
            note_absent(absent, "host.logical_cpus", "/proc/cpuinfo not readable");
            0
        }
    };
    let physical_cores = match &summary {
        Some(s) if s.physical_cores > 0 => s.physical_cores,
        _ => {
            note_absent(
                absent,
                "host.physical_cores",
                "no (physical id, core id) pairs found in /proc/cpuinfo",
            );
            0
        }
    };
    let sockets = match &summary {
        Some(s) if s.sockets > 0 => s.sockets,
        _ => {
            note_absent(
                absent,
                "host.sockets",
                "no 'physical id' found in /proc/cpuinfo",
            );
            0
        }
    };

    let (p_cores, e_cores) = match discover_p_e_cores(facts, discover_governed_cpu_count(facts)) {
        Some((p, e)) => (p, e),
        None => {
            note_absent(
                absent,
                "host.p_cores",
                "neither cpu_capacity nor cpuinfo_max_freq readable for every CPU",
            );
            note_absent(
                absent,
                "host.e_cores",
                "neither cpu_capacity nor cpuinfo_max_freq readable for every CPU",
            );
            (String::new(), String::new())
        }
    };

    let memory_gb = facts
        .read_text("/proc/meminfo")
        .ok()
        .and_then(|text| parse_mem_total_kb(&text))
        .map(|kb| (kb / (1024 * 1024)) as u32)
        .unwrap_or_else(|| {
            note_absent(absent, "host.memory_gb", "/proc/meminfo has no 'MemTotal'");
            0
        });

    HostInfo {
        rig_slug: rig_slug.to_string(),
        system_vendor,
        system_model,
        bios_version,
        bios_release_date,
        cpu_model,
        microcode,
        logical_cpus,
        physical_cores,
        sockets,
        p_cores,
        e_cores,
        memory_gb,
    }
}

fn parse_mem_total_kb(text: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        if parts.next()? != "MemTotal:" {
            return None;
        }
        parts.next()?.parse::<u64>().ok()
    })
}

// ---------------------------------------------------------------------------------
// KernelInfo
// ---------------------------------------------------------------------------------

fn extract_cmdline_param(cmdline: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    cmdline
        .split_whitespace()
        .find_map(|token| token.strip_prefix(prefix.as_str()).map(str::to_string))
}

fn build_kernel(facts: &dyn SystemFacts, absent: &mut Vec<AbsentField>) -> KernelInfo {
    let release = read_or_absent(
        facts,
        "/proc/sys/kernel/osrelease",
        "kernel.release",
        absent,
    );
    let version_string = read_or_absent(
        facts,
        "/proc/sys/kernel/version",
        "kernel.version_string",
        absent,
    );

    let is_realtime = match facts.read_text("/sys/kernel/realtime") {
        Ok(text) => text.trim() == "1",
        Err(_) => {
            note_absent(
                absent,
                "kernel.is_realtime",
                "/sys/kernel/realtime not present",
            );
            false
        }
    };

    let preempt_model = if version_string.contains("PREEMPT_RT") {
        "PREEMPT_RT".to_string()
    } else if version_string.contains("PREEMPT_DYNAMIC") {
        "PREEMPT_DYNAMIC".to_string()
    } else {
        note_absent(
            absent,
            "kernel.preempt_model",
            "neither PREEMPT_RT nor PREEMPT_DYNAMIC found in the version string",
        );
        String::new()
    };

    let raw_cmdline = read_or_absent(facts, "/proc/cmdline", "kernel.cmdline", absent);
    let cmdline = KernelInfo::redact_cmdline(&raw_cmdline);

    let isolcpus = extract_cmdline_param(&raw_cmdline, "isolcpus");
    if isolcpus.is_none() {
        note_absent(absent, "kernel.isolcpus", "isolcpus not present on cmdline");
    }
    let nohz_full = extract_cmdline_param(&raw_cmdline, "nohz_full");
    if nohz_full.is_none() {
        note_absent(
            absent,
            "kernel.nohz_full",
            "nohz_full not present on cmdline",
        );
    }
    let rcu_nocbs = extract_cmdline_param(&raw_cmdline, "rcu_nocbs");
    if rcu_nocbs.is_none() {
        note_absent(
            absent,
            "kernel.rcu_nocbs",
            "rcu_nocbs not present on cmdline",
        );
    }
    let irqaffinity = extract_cmdline_param(&raw_cmdline, "irqaffinity");
    if irqaffinity.is_none() {
        note_absent(
            absent,
            "kernel.irqaffinity",
            "irqaffinity not present on cmdline",
        );
    }

    KernelInfo {
        release,
        version_string,
        is_realtime,
        preempt_model,
        cmdline,
        isolcpus,
        nohz_full,
        rcu_nocbs,
        irqaffinity,
    }
}

// ---------------------------------------------------------------------------------
// OsInfo
// ---------------------------------------------------------------------------------

fn parse_os_release_field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (k, v) = line.split_once('=')?;
        if k.trim() != key {
            return None;
        }
        Some(v.trim().trim_matches('"').to_string())
    })
}

fn any_display_manager_active(facts: &dyn SystemFacts) -> Option<bool> {
    const DISPLAY_MANAGERS: [&str; 3] = ["gdm.service", "sddm.service", "lightdm.service"];
    let mut any_found = false;
    let mut active = false;
    for unit in DISPLAY_MANAGERS {
        if let Ok(props) = facts.systemctl_show(unit, &["ActiveState"]) {
            if let Some(state) = props.get("ActiveState") {
                any_found = true;
                if state == "active" {
                    active = true;
                }
            }
        }
    }
    any_found.then_some(active)
}

fn build_os(facts: &dyn SystemFacts, absent: &mut Vec<AbsentField>) -> OsInfo {
    let os_release = facts.read_text("/etc/os-release").ok();
    let distro = os_release
        .as_deref()
        .and_then(|t| parse_os_release_field(t, "NAME"))
        .unwrap_or_else(|| {
            note_absent(absent, "os.distro", "/etc/os-release has no NAME");
            String::new()
        });
    let version = os_release
        .as_deref()
        .and_then(|t| parse_os_release_field(t, "VERSION"))
        .unwrap_or_else(|| {
            note_absent(absent, "os.version", "/etc/os-release has no VERSION");
            String::new()
        });

    // A live-USB session overlays its root filesystem at /cow (casper/live-boot).
    let session_kind = if facts.path_exists("/cow") {
        SessionKind::LiveUsb
    } else {
        SessionKind::Installed
    };

    let systemd_default_target = match facts.systemctl_get_default() {
        Ok(target) => target,
        Err(_) => {
            note_absent(
                absent,
                "os.systemd_default_target",
                "systemctl get-default not available",
            );
            String::new()
        }
    };

    let display_manager_active = any_display_manager_active(facts).unwrap_or_else(|| {
        note_absent(
            absent,
            "os.display_manager_active",
            "no display-manager unit state available",
        );
        false
    });

    OsInfo {
        distro,
        version,
        session_kind,
        systemd_default_target,
        display_manager_active,
    }
}

// ---------------------------------------------------------------------------------
// TuningInfo
// ---------------------------------------------------------------------------------

fn build_tuning(facts: &dyn SystemFacts, absent: &mut Vec<AbsentField>) -> TuningInfo {
    let cpu_count = discover_governed_cpu_count(facts);
    let per_cpu_governor: Vec<CpuGovernor> = (0..cpu_count)
        .map(|cpu| {
            let governor = facts
                .read_text(&format!(
                    "/sys/devices/system/cpu/cpu{cpu}/cpufreq/scaling_governor"
                ))
                .unwrap_or_default()
                .trim()
                .to_string();
            // Optional (D-16): absent on non-HWP and non-Intel hardware, and on
            // this crate's own RIG_AS_FOUND fixture, whose probe script never
            // captured it (docs/rig/recon-2026-08-31/FINDINGS.md). A second,
            // independent tuning lever alongside `governor`, never a substitute
            // for a read that failed.
            let energy_performance_preference = facts
                .read_text(&format!(
                    "/sys/devices/system/cpu/cpu{cpu}/cpufreq/energy_performance_preference"
                ))
                .ok()
                .map(|text| text.trim().to_string());
            CpuGovernor {
                cpu,
                governor,
                energy_performance_preference,
            }
        })
        .collect();
    if per_cpu_governor.is_empty() {
        note_absent(
            absent,
            "tuning.per_cpu_governor",
            "no scaling_governor files found",
        );
    } else if per_cpu_governor
        .iter()
        .all(|c| c.energy_performance_preference.is_none())
    {
        note_absent(
            absent,
            "tuning.per_cpu_governor.energy_performance_preference",
            "no energy_performance_preference files found",
        );
    }

    let no_turbo = match facts.read_text("/sys/devices/system/cpu/intel_pstate/no_turbo") {
        Ok(text) => Some(text.trim() == "1"),
        Err(_) => {
            note_absent(
                absent,
                "tuning.no_turbo",
                "intel_pstate/no_turbo not present",
            );
            None
        }
    };

    let cstates: Vec<CstateSetting> = discover_cstates(facts, 0)
        .into_iter()
        .map(|(name, disabled)| CstateSetting {
            cpu: 0,
            name,
            disabled,
        })
        .collect();
    if cstates.is_empty() {
        note_absent(absent, "tuning.cstates", "no cpuidle states found on cpu0");
    }

    let rt_tuning_service = match facts.systemctl_show(
        "rt-tuning.service",
        &["ActiveState", "SubState", "UnitFileState"],
    ) {
        Ok(props) if !props.is_empty() => ServiceState {
            unit: "rt-tuning.service".to_string(),
            active_state: props.get("ActiveState").cloned().unwrap_or_default(),
            sub_state: props.get("SubState").cloned().unwrap_or_default(),
            unit_file_state: props.get("UnitFileState").cloned().unwrap_or_default(),
        },
        _ => {
            note_absent(
                absent,
                "tuning.rt_tuning_service",
                "rt-tuning.service state not available",
            );
            ServiceState {
                unit: "rt-tuning.service".to_string(),
                active_state: String::new(),
                sub_state: String::new(),
                unit_file_state: String::new(),
            }
        }
    };

    TuningInfo {
        per_cpu_governor,
        no_turbo,
        cstates,
        rt_tuning_service,
    }
}

// ---------------------------------------------------------------------------------
// PowerInfo
// ---------------------------------------------------------------------------------

fn build_power(
    facts: &dyn SystemFacts,
    absent: &mut Vec<AbsentField>,
    thermal_start: Option<&[(String, f32)]>,
) -> PowerInfo {
    let ac_online = match discover_ac_online(facts) {
        Some(value) => Some(value == "1"),
        None => {
            note_absent(absent, "power.ac_online", "no AC power_supply entry found");
            None
        }
    };

    let (battery_status, battery_percent) = match discover_battery(facts) {
        Some((_, status, capacity)) => (Some(status), capacity.parse::<u32>().ok()),
        None => {
            note_absent(
                absent,
                "power.battery_status",
                "no BAT power_supply entry found",
            );
            note_absent(
                absent,
                "power.battery_percent",
                "no BAT power_supply entry found",
            );
            (None, None)
        }
    };

    let zones_now = discover_thermal_zones_c(facts);
    let thermal_zones: Vec<ThermalZone> = match thermal_start {
        Some(start) => start
            .iter()
            .map(|(name, start_c)| ThermalZone {
                name: name.clone(),
                temp_c_start: *start_c,
                temp_c_end: zones_now
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, end_c)| *end_c),
            })
            .collect(),
        None => zones_now
            .iter()
            .map(|(name, temp_c)| ThermalZone {
                name: name.clone(),
                temp_c_start: *temp_c,
                temp_c_end: None,
            })
            .collect(),
    };
    // The maximum across every reading actually taken. This is two points per zone on a live
    // run, not a sampled trace, so it bounds only what was observed at the ends of the run; a
    // spike in between is still invisible here. Stated rather than implied, because the 70 C
    // gate's justification once leaned on this field being a run maximum.
    let package_temp_c_max = if thermal_zones.is_empty() {
        note_absent(absent, "power.package_temp_c_max", "no thermal zones found");
        None
    } else {
        Some(
            thermal_zones
                .iter()
                .flat_map(|z| std::iter::once(z.temp_c_start).chain(z.temp_c_end))
                .fold(f32::MIN, f32::max),
        )
    };

    PowerInfo {
        ac_online,
        battery_status,
        battery_percent,
        thermal_zones,
        package_temp_c_max,
    }
}

// ---------------------------------------------------------------------------------
// NetworkInfo
// ---------------------------------------------------------------------------------

fn build_network(facts: &dyn SystemFacts, absent: &mut Vec<AbsentField>) -> NetworkInfo {
    struct IfaceFacts {
        iface: String,
        driver: Option<String>,
        state: Option<String>,
    }

    let mut wired: Option<IfaceFacts> = None;
    let mut wifi: Option<IfaceFacts> = None;

    match facts.list_dir("/sys/class/net") {
        Ok(ifaces) => {
            for iface in ifaces {
                if iface == "lo" {
                    continue;
                }
                let is_wireless = facts.path_exists(&format!("/sys/class/net/{iface}/wireless"));
                let driver = facts
                    .read_text(&format!("/sys/class/net/{iface}/device/uevent"))
                    .ok()
                    .and_then(|text| {
                        text.lines()
                            .find_map(|l| l.strip_prefix("DRIVER=").map(str::to_string))
                    });
                let state = facts
                    .read_text(&format!("/sys/class/net/{iface}/operstate"))
                    .ok()
                    .map(|s| s.trim().to_string());
                let entry = IfaceFacts {
                    iface: iface.clone(),
                    driver,
                    state,
                };

                if is_wireless {
                    wifi.get_or_insert(entry);
                } else {
                    wired.get_or_insert(entry);
                }
            }
        }
        Err(_) => {
            note_absent(
                absent,
                "network.wired_iface",
                "/sys/class/net not enumerable",
            );
            note_absent(
                absent,
                "network.wifi_iface",
                "/sys/class/net not enumerable",
            );
        }
    }

    if wired.is_none() {
        note_absent(
            absent,
            "network.wired_iface",
            "no non-wireless interface found",
        );
    }
    if wifi.is_none() {
        note_absent(absent, "network.wifi_iface", "no wireless interface found");
    }

    // Best-effort: a populated `.../device/ptp` directory under the wired interface
    // is treated as hardware timestamping capability. This does not query the exact
    // capability set `ethtool -T` reports; it is a presence check only.
    let ptp_capabilities = wired.as_ref().and_then(|w| {
        facts
            .list_dir(&format!("/sys/class/net/{}/device/ptp", w.iface))
            .ok()
            .filter(|entries| !entries.is_empty())
            .map(|_| "hardware-clock-present".to_string())
    });
    if ptp_capabilities.is_none() {
        note_absent(
            absent,
            "network.ptp_capabilities",
            "no ptp device found under the wired interface",
        );
    }

    NetworkInfo {
        wired_iface: wired.as_ref().map(|w| w.iface.clone()),
        wired_driver: wired.as_ref().and_then(|w| w.driver.clone()),
        wired_state: wired.as_ref().and_then(|w| w.state.clone()),
        ptp_capabilities,
        wifi_iface: wifi.as_ref().map(|w| w.iface.clone()),
        wifi_driver: wifi.as_ref().and_then(|w| w.driver.clone()),
        wifi_state: wifi.as_ref().and_then(|w| w.state.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::FixtureFacts;

    const PROBE_SYSFS_TUNING: &str = include_str!("../tests/fixtures/probe-sysfs-tuning.txt");

    const SAMPLE_CPUINFO: &str = "processor\t: 0\n\
vendor_id\t: GenuineIntel\n\
model name\t: Intel(R) Core(TM) Ultra 9 185H\n\
physical id\t: 0\n\
core id\t\t: 0\n\
microcode\t: 0x28\n\
\n\
processor\t: 1\n\
vendor_id\t: GenuineIntel\n\
model name\t: Intel(R) Core(TM) Ultra 9 185H\n\
physical id\t: 0\n\
core id\t\t: 1\n\
microcode\t: 0x28\n";

    fn base_facts() -> FixtureFacts {
        FixtureFacts::parse(PROBE_SYSFS_TUNING)
            .with("/proc/cpuinfo", SAMPLE_CPUINFO)
            .with("/proc/meminfo", "MemTotal:       31457280 kB\n")
            .with(
                "/proc/cmdline",
                "BOOT_IMAGE=/vmlinuz root=UUID=1111-2222 ro quiet splash \
                 isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11",
            )
            .with("/proc/sys/kernel/osrelease", "7.0.0-30-realtime")
            .with(
                "/proc/sys/kernel/version",
                "#30-Ubuntu SMP PREEMPT_RT Fri Jul 31 18:22:54 UTC 2026",
            )
            .with(
                "/etc/os-release",
                "NAME=\"Ubuntu\"\nVERSION=\"26.04.1 LTS\"\n",
            )
    }

    #[test]
    fn snapshot_fills_every_required_field() {
        let facts = base_facts();
        let snap = snapshot(&facts, "precision3591", None).expect("snapshot succeeds");

        assert_eq!(snap.host.rig_slug, "precision3591");
        assert_eq!(snap.host.cpu_model, "Intel(R) Core(TM) Ultra 9 185H");
        assert_eq!(snap.host.logical_cpus, 2);
        assert_eq!(snap.kernel.release, "7.0.0-30-realtime");
        assert_eq!(snap.kernel.preempt_model, "PREEMPT_RT");
        assert!(snap.kernel.cmdline.contains("root=[redacted]"));
        assert_eq!(snap.os.distro, "Ubuntu");
        assert!(!snap.tuning.per_cpu_governor.is_empty());

        let serialised = serde_json::to_string(&snap.host).expect("HostInfo serialises");
        assert!(serialised.contains("precision3591"));
    }

    #[test]
    fn absent_source_becomes_absent_field() {
        // probe-sysfs-tuning.txt's no_turbo line is the literal "unavailable"
        // sentinel (a real false-negative from the wrong sysfs path; see
        // docs/rig/recon-2026-08-31/FINDINGS.md).
        let facts = base_facts();
        let snap = snapshot(&facts, "precision3591", None).expect("snapshot succeeds");

        assert_eq!(snap.tuning.no_turbo, None);
        assert!(
            snap.absent_fields
                .iter()
                .any(|f| f.field_path == "tuning.no_turbo"),
            "expected an absent_fields entry for tuning.no_turbo, got {:?}",
            snap.absent_fields
        );
    }

    #[test]
    fn epp_absent_from_rig_as_found_fixture_is_recorded_honestly() {
        // probe-sysfs-tuning.txt has no energy_performance_preference lines at all;
        // the probe script that produced it never captured the value (see this
        // crate's tests/fixtures/README.md). Every CPU's EPP must come back None,
        // not an empty string or a guessed value, and the absence must be recorded
        // rather than silently dropped (D-16).
        let facts = base_facts();
        let snap = snapshot(&facts, "precision3591", None).expect("snapshot succeeds");

        assert!(!snap.tuning.per_cpu_governor.is_empty());
        assert!(
            snap.tuning
                .per_cpu_governor
                .iter()
                .all(|c| c.energy_performance_preference.is_none()),
            "expected every CPU's EPP to read None from a fixture with no EPP lines, got {:?}",
            snap.tuning.per_cpu_governor
        );
        assert!(
            snap.absent_fields
                .iter()
                .any(|f| f.field_path == "tuning.per_cpu_governor.energy_performance_preference"),
            "expected an absent_fields entry for missing EPP, got {:?}",
            snap.absent_fields
        );
    }

    #[test]
    fn cmdline_params_are_parsed() {
        let facts = base_facts();
        let snap = snapshot(&facts, "precision3591", None).expect("snapshot succeeds");

        assert_eq!(snap.kernel.isolcpus.as_deref(), Some("6-11"));
        assert_eq!(snap.kernel.nohz_full.as_deref(), Some("6-11"));
        assert_eq!(snap.kernel.rcu_nocbs.as_deref(), Some("6-11"));
    }

    #[test]
    fn cmdline_param_absent_is_none() {
        let facts = base_facts();
        let snap = snapshot(&facts, "precision3591", None).expect("snapshot succeeds");

        assert_eq!(snap.kernel.irqaffinity, None);
        assert!(
            snap.absent_fields
                .iter()
                .any(|f| f.field_path == "kernel.irqaffinity"),
            "expected an absent_fields entry for kernel.irqaffinity, got {:?}",
            snap.absent_fields
        );
    }

    #[test]
    fn rig_slug_is_not_hostname() {
        let facts = base_facts();
        let snap_a = snapshot(&facts, "precision3591", None).expect("snapshot succeeds");
        let snap_b = snapshot(&facts, "some-other-rig", None).expect("snapshot succeeds");

        assert_eq!(snap_a.host.rig_slug, "precision3591");
        assert_eq!(snap_b.host.rig_slug, "some-other-rig");
    }

    #[test]
    fn no_host_identifiers_collected() {
        // Patterns are built at runtime so this check's own source does not contain
        // the literal substrings it scans for (docs/rig/recon-2026-08-31/FINDINGS.md
        // hit exactly this self-referential trap once already, in prose).
        let source = include_str!("environment.rs");
        let hostname_path = ["/etc/", "hostname"].concat();
        let machine_id_path = ["/etc/", "machine-id"].concat();
        let address_suffix = ["/addr", "ess"].concat();
        let wireless_name_token = ["ss", "id"].concat();

        for pattern in [
            &hostname_path,
            &machine_id_path,
            &address_suffix,
            &wireless_name_token,
        ] {
            assert_eq!(
                source.matches(pattern.as_str()).count(),
                0,
                "environment.rs must not reference {pattern:?}"
            );
        }
    }
}
