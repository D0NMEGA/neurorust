//! Everything the D-06 precondition checks and the D-14 environment snapshot need to
//! read, behind one trait.
//!
//! [`LiveFacts`] is the only Linux-gated type in this crate: it reads real sysfs and
//! procfs paths and shells out to `systemctl`/`loginctl`. [`FixtureFacts`] parses the
//! `key=value` text format plan 01-02's `probe-sysfs-tuning.txt` uses and serves the
//! same trait from committed fixture data, so every precondition check and every
//! environment-snapshot field has a real test on macOS as well as Linux
//! (RESEARCH.md pitfall 5).
//!
//! Both implementations answer the same generic questions (`read_text`, `path_exists`,
//! ...); neither exposes anything specific to one precondition. Values that need
//! pairing (a cpuidle state's name with its `disable` flag, a thermal zone's type with
//! its temperature) are synthesized from indexed sysfs paths
//! (`.../cpuidle/state{N}/{name,disable}`, `.../thermal_zone{N}/{type,temp}`) by both
//! implementations, rather than adding check-specific trait methods.

use std::collections::BTreeMap;
#[cfg(target_os = "linux")]
use std::path::Path;
#[cfg(target_os = "linux")]
use std::process::Command;

use thiserror::Error;

/// Everything the precondition checks and the environment snapshot need to read.
/// One live implementation (Linux, procfs and sysfs) and one fixture implementation
/// (parses the text files plan 01-02 captured on the rig). The fixture implementation
/// is what makes the macOS CI leg test real logic rather than nothing.
pub trait SystemFacts {
    fn read_text(&self, path: &str) -> Result<String, FactsError>;
    fn path_exists(&self, path: &str) -> bool;
    /// Names of entries directly under `path` (not recursive). Used to enumerate
    /// network interfaces under `/sys/class/net`, whose names are not a contiguous
    /// numeric sequence and so cannot be probed index-by-index like cpuidle states.
    fn list_dir(&self, path: &str) -> Result<Vec<String>, FactsError>;
    fn systemctl_show(
        &self,
        unit: &str,
        props: &[&str],
    ) -> Result<BTreeMap<String, String>, FactsError>;
    fn systemctl_get_default(&self) -> Result<String, FactsError>;
    fn active_ssh_sessions(&self) -> Result<u32, FactsError>;
    fn graphical_sessions(&self) -> Result<u32, FactsError>;
    /// Names every active login session of type `tty`, class `user`, on `seat0`: a
    /// login opened at the machine's own physical console (e.g. a text-mode login
    /// at `tty1`), as opposed to an SSH session (also `Type=tty` in `loginctl`, but
    /// attached to no seat) or an x11/wayland session (already covered by
    /// [`Self::graphical_sessions`]). Named per session, like
    /// [`Self::running_processes_matching`], so a violation can be attributed
    /// rather than only counted.
    fn local_login_sessions(&self) -> Result<Vec<String>, FactsError>;
    fn running_processes_matching(&self, names: &[&str]) -> Result<Vec<String>, FactsError>;
}

#[derive(Debug, Error)]
pub enum FactsError {
    #[error("{path} not found or unreadable: {reason}")]
    NotFound { path: String, reason: String },
    #[error("command {command} failed: {reason}")]
    Command { command: String, reason: String },
}

// ---------------------------------------------------------------------------------
// LiveFacts: the only Linux-gated type in this crate.
// ---------------------------------------------------------------------------------

/// Reads real sysfs/procfs paths and shells out to `systemctl`/`loginctl`. D-06: every
/// command here is read-only. No sysfs write, no `systemctl` call that changes state,
/// no `sudo`.
#[cfg(target_os = "linux")]
pub struct LiveFacts;

#[cfg(target_os = "linux")]
pub fn live() -> Result<LiveFacts, FactsError> {
    Ok(LiveFacts)
}

#[cfg(target_os = "linux")]
impl SystemFacts for LiveFacts {
    fn read_text(&self, path: &str) -> Result<String, FactsError> {
        std::fs::read_to_string(path).map_err(|source| FactsError::NotFound {
            path: path.to_string(),
            reason: source.to_string(),
        })
    }

    fn path_exists(&self, path: &str) -> bool {
        Path::new(path).exists()
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, FactsError> {
        let entries = std::fs::read_dir(path).map_err(|source| FactsError::NotFound {
            path: path.to_string(),
            reason: source.to_string(),
        })?;
        let mut names: Vec<String> = entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        Ok(names)
    }

    fn systemctl_show(
        &self,
        unit: &str,
        props: &[&str],
    ) -> Result<BTreeMap<String, String>, FactsError> {
        let mut cmd = Command::new("systemctl");
        cmd.arg("show").arg(unit);
        for prop in props {
            cmd.arg(format!("--property={prop}"));
        }
        let output = cmd.output().map_err(|source| FactsError::Command {
            command: format!("systemctl show {unit}"),
            reason: source.to_string(),
        })?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut result = BTreeMap::new();
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                result.insert(key.to_string(), value.to_string());
            }
        }
        Ok(result)
    }

    fn systemctl_get_default(&self) -> Result<String, FactsError> {
        let output = Command::new("systemctl")
            .arg("get-default")
            .output()
            .map_err(|source| FactsError::Command {
                command: "systemctl get-default".to_string(),
                reason: source.to_string(),
            })?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn active_ssh_sessions(&self) -> Result<u32, FactsError> {
        // Established connections with the local (server-side) port 22, i.e.
        // incoming SSH sessions. Read-only (D-06): `ss` never changes state.
        let output = Command::new("ss")
            .args(["-Htn", "state", "established", "( sport = :22 )"])
            .output()
            .map_err(|source| FactsError::Command {
                command: "ss -Htn state established ( sport = :22 )".to_string(),
                reason: source.to_string(),
            })?;
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.lines().filter(|line| !line.trim().is_empty()).count() as u32)
    }

    fn graphical_sessions(&self) -> Result<u32, FactsError> {
        let list = Command::new("loginctl")
            .args(["list-sessions", "--no-legend"])
            .output()
            .map_err(|source| FactsError::Command {
                command: "loginctl list-sessions".to_string(),
                reason: source.to_string(),
            })?;
        let list_text = String::from_utf8_lossy(&list.stdout);
        let mut count = 0u32;
        for line in list_text.lines() {
            let Some(session_id) = line.split_whitespace().next() else {
                continue;
            };
            let show = Command::new("loginctl")
                .args(["show-session", session_id, "-p", "Type", "--value"])
                .output()
                .map_err(|source| FactsError::Command {
                    command: format!("loginctl show-session {session_id}"),
                    reason: source.to_string(),
                })?;
            let kind = String::from_utf8_lossy(&show.stdout).trim().to_string();
            if kind == "x11" || kind == "wayland" {
                count += 1;
            }
        }
        Ok(count)
    }

    fn local_login_sessions(&self) -> Result<Vec<String>, FactsError> {
        let list = Command::new("loginctl")
            .args(["list-sessions", "--no-legend"])
            .output()
            .map_err(|source| FactsError::Command {
                command: "loginctl list-sessions".to_string(),
                reason: source.to_string(),
            })?;
        let list_text = String::from_utf8_lossy(&list.stdout);
        let mut sessions = Vec::new();
        for line in list_text.lines() {
            let Some(session_id) = line.split_whitespace().next() else {
                continue;
            };
            // One call for all four properties, same convention as `systemctl_show`:
            // parse `Key=Value` lines rather than shelling out per property.
            let show = Command::new("loginctl")
                .args([
                    "show-session",
                    session_id,
                    "-p",
                    "Type",
                    "-p",
                    "Class",
                    "-p",
                    "Seat",
                    "-p",
                    "TTY",
                ])
                .output()
                .map_err(|source| FactsError::Command {
                    command: format!("loginctl show-session {session_id}"),
                    reason: source.to_string(),
                })?;
            let text = String::from_utf8_lossy(&show.stdout);
            let mut props = BTreeMap::new();
            for prop_line in text.lines() {
                if let Some((key, value)) = prop_line.split_once('=') {
                    props.insert(key.to_string(), value.to_string());
                }
            }

            // A local console login: a real seat (the physical keyboard/display),
            // not merely a tty-typed session, which an interactive SSH connection
            // also reports (see the trait doc comment). This is what keeps a single
            // SSH connection from tripping this check as well as
            // `NoActiveSshSessions`.
            let is_local_console_login = props.get("Type").map(String::as_str) == Some("tty")
                && props.get("Class").map(String::as_str) == Some("user")
                && props.get("Seat").map(String::as_str) == Some("seat0");
            if is_local_console_login {
                let tty = props.get("TTY").cloned().unwrap_or_default();
                sessions.push(if tty.is_empty() {
                    format!("session {session_id}")
                } else {
                    format!("{tty} (session {session_id})")
                });
            }
        }
        Ok(sessions)
    }

    fn running_processes_matching(&self, names: &[&str]) -> Result<Vec<String>, FactsError> {
        let mut found = Vec::new();
        let proc_entries = std::fs::read_dir("/proc").map_err(|source| FactsError::NotFound {
            path: "/proc".to_string(),
            reason: source.to_string(),
        })?;
        for entry in proc_entries.filter_map(|e| e.ok()) {
            let file_name = entry.file_name();
            let Some(pid_str) = file_name.to_str() else {
                continue;
            };
            if !pid_str.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let comm_path = entry.path().join("comm");
            let Ok(comm) = std::fs::read_to_string(&comm_path) else {
                continue;
            };
            let comm = comm.trim();
            if names.contains(&comm) {
                found.push(comm.to_string());
            }
        }
        Ok(found)
    }
}

// ---------------------------------------------------------------------------------
// FixtureFacts: parses the key=value probe format for tests, on any platform.
// ---------------------------------------------------------------------------------

/// Serves [`SystemFacts`] from a `key=value` text fixture (plan 01-02's
/// `probe-sysfs-tuning.txt` format) plus a small set of extra conventions for data
/// that format does not carry (SSH/graphical session counts, running processes,
/// systemd unit properties, directory listings). Building a [`FixtureFacts`] from
/// real rig output, rather than a synthetic guess, is what makes a test against it
/// mean something.
#[derive(Debug, Clone, Default)]
pub struct FixtureFacts {
    data: BTreeMap<String, String>,
    /// cpu0 cpuidle states, in the order they appeared in the source text: `(name,
    /// disabled)`. Real sysfs numbers cpuidle states contiguously from `state0`, so
    /// file order is index order.
    cpu0_cstates: Vec<(String, bool)>,
    /// Thermal zones, in source order: `(type, millidegrees)`.
    thermal_zones: Vec<(String, i64)>,
    /// Directory listings a test has explicitly provided via [`FixtureFacts::with_dir`].
    dirs: BTreeMap<String, Vec<String>>,
}

/// A literal value meaning "the probe queried this path and it did not exist",
/// preserved verbatim in `probe-sysfs-tuning.txt` (see its fixtures README: the
/// `no_turbo` probe queried the wrong sysfs path, so this sentinel is genuine tool
/// output, not a placeholder this crate invented).
const UNAVAILABLE_SENTINEL: &str = "unavailable";

impl FixtureFacts {
    /// Parses the `key=value` fixture format, one assignment per line. Lines are
    /// split on the first `=` only, so a value may itself contain `=` or spaces (a
    /// real captured thermal zone type, `INT3400 Thermal`, has a space). Never
    /// fails: an unparsable line is silently skipped rather than rejected.
    pub fn parse(text: &str) -> Self {
        let mut data = BTreeMap::new();
        let mut cpu0_cstates = Vec::new();
        let mut thermal_zones = Vec::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            data.insert(key.to_string(), value.to_string());

            if let Some(name) = key.strip_prefix("cstate.") {
                // Value shape: "disable:0" or "disable:1".
                let disabled = value
                    .split_once(':')
                    .map(|(_, flag)| flag == "1")
                    .unwrap_or(false);
                cpu0_cstates.push((name.to_string(), disabled));
            } else if let Some(name) = key.strip_prefix("thermal.") {
                if let Ok(milli) = value.parse::<i64>() {
                    thermal_zones.push((name.to_string(), milli));
                }
            }
        }

        Self {
            data,
            cpu0_cstates,
            thermal_zones,
            dirs: BTreeMap::new(),
        }
    }

    /// Overrides (or adds) a single `key=value` pair. Does not affect the cstate or
    /// thermal-zone ordering, both fixed at parse time from the source text.
    #[must_use]
    pub fn with(mut self, key: &str, value: &str) -> Self {
        self.data.insert(key.to_string(), value.to_string());
        self
    }

    /// Provides a directory listing for [`SystemFacts::list_dir`]. Not derived from
    /// `probe-sysfs-tuning.txt`, which carries no network data at all; a test that
    /// needs `NetworkInfo` fields populated supplies one explicitly.
    #[must_use]
    pub fn with_dir(mut self, path: &str, entries: &[&str]) -> Self {
        self.dirs.insert(
            path.to_string(),
            entries.iter().map(|s| s.to_string()).collect(),
        );
        self
    }

    fn get_key(&self, key: &str, path: &str) -> Result<String, FactsError> {
        match self.data.get(key) {
            Some(v) if v != UNAVAILABLE_SENTINEL => Ok(v.clone()),
            _ => Err(FactsError::NotFound {
                path: path.to_string(),
                reason: format!("fixture key {key:?} absent or unavailable"),
            }),
        }
    }

    /// Translates a real sysfs/procfs path into this fixture's flat key namespace, or
    /// into the ordered cstate/thermal side lists for indexed paths.
    fn lookup(&self, path: &str) -> Result<String, FactsError> {
        if let Some(rest) = path
            .strip_prefix("/sys/devices/system/cpu/cpu")
            .and_then(|r| r.strip_suffix("/cpufreq/scaling_governor"))
        {
            return self.get_key(&format!("cpu{rest}.governor"), path);
        }

        if let Some(rest) = path
            .strip_prefix("/sys/devices/system/cpu/cpu")
            .and_then(|r| r.strip_suffix("/cpufreq/energy_performance_preference"))
        {
            return self.get_key(&format!("cpu{rest}.energy_performance_preference"), path);
        }

        if path == "/sys/devices/system/cpu/intel_pstate/no_turbo" {
            return self.get_key("intel_pstate.no_turbo", path);
        }

        if let Some(rest) = path.strip_prefix("/sys/devices/system/cpu/cpu0/cpuidle/state") {
            if let Some((idx_str, field)) = rest.split_once('/') {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    let Some((name, disabled)) = self.cpu0_cstates.get(idx) else {
                        return Err(FactsError::NotFound {
                            path: path.to_string(),
                            reason: "no cpuidle state at this index".to_string(),
                        });
                    };
                    return match field {
                        "name" => Ok(name.clone()),
                        "disable" => Ok(if *disabled { "1" } else { "0" }.to_string()),
                        _ => Err(FactsError::NotFound {
                            path: path.to_string(),
                            reason: "unrecognised cpuidle field".to_string(),
                        }),
                    };
                }
            }
        }

        if path == "/sys/kernel/realtime" {
            return self.get_key("kernel.realtime", path);
        }

        if path == "/sys/devices/system/cpu/isolated" {
            return self.get_key("cpu.isolated", path);
        }

        if path == "/sys/kernel/tracing/current_tracer" {
            return self.get_key("tracing.current_tracer", path);
        }

        if path.starts_with("/sys/class/power_supply/") && path.ends_with("/online") {
            return self.get_key("power.ac_online", path);
        }
        if path.starts_with("/sys/class/power_supply/") && path.contains("BAT") {
            if path.ends_with("/status") {
                return self.get_key("power.battery_status", path);
            }
            if path.ends_with("/capacity") {
                return self.get_key("power.battery_capacity", path);
            }
        }

        if let Some(rest) = path.strip_prefix("/sys/class/thermal/thermal_zone") {
            if let Some((idx_str, field)) = rest.split_once('/') {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    let Some((name, milli)) = self.thermal_zones.get(idx) else {
                        return Err(FactsError::NotFound {
                            path: path.to_string(),
                            reason: "no thermal zone at this index".to_string(),
                        });
                    };
                    return match field {
                        "type" => Ok(name.clone()),
                        "temp" => Ok(milli.to_string()),
                        _ => Err(FactsError::NotFound {
                            path: path.to_string(),
                            reason: "unrecognised thermal field".to_string(),
                        }),
                    };
                }
            }
        }

        // /proc/cpuinfo, /proc/meminfo, /proc/cmdline, /etc/os-release and similar
        // whole-file text blobs are stored under their literal path.
        self.get_key(path, path)
    }
}

impl SystemFacts for FixtureFacts {
    fn read_text(&self, path: &str) -> Result<String, FactsError> {
        self.lookup(path)
    }

    fn path_exists(&self, path: &str) -> bool {
        self.lookup(path).is_ok() || self.dirs.contains_key(path)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, FactsError> {
        self.dirs
            .get(path)
            .cloned()
            .ok_or_else(|| FactsError::NotFound {
                path: path.to_string(),
                reason: "no fixture directory listing provided".to_string(),
            })
    }

    fn systemctl_show(
        &self,
        unit: &str,
        props: &[&str],
    ) -> Result<BTreeMap<String, String>, FactsError> {
        let mut result = BTreeMap::new();
        for prop in props {
            if let Some(value) = self.data.get(&format!("service.{unit}.{prop}")) {
                result.insert((*prop).to_string(), value.clone());
            }
        }
        Ok(result)
    }

    fn systemctl_get_default(&self) -> Result<String, FactsError> {
        self.get_key("systemd.default_target", "systemctl get-default")
    }

    fn active_ssh_sessions(&self) -> Result<u32, FactsError> {
        self.get_key("ssh.active_sessions", "active_ssh_sessions")
            .and_then(|v| {
                v.parse().map_err(|_| FactsError::NotFound {
                    path: "ssh.active_sessions".to_string(),
                    reason: "not a number".to_string(),
                })
            })
    }

    fn graphical_sessions(&self) -> Result<u32, FactsError> {
        self.get_key("graphical.sessions", "graphical_sessions")
            .and_then(|v| {
                v.parse().map_err(|_| FactsError::NotFound {
                    path: "graphical.sessions".to_string(),
                    reason: "not a number".to_string(),
                })
            })
    }

    /// `login.local_sessions`: a comma-separated list of named local console
    /// sessions, empty for none. Absent (or the `unavailable` sentinel) means the
    /// fixture never captured this data, same convention as every other
    /// `get_key`-backed field.
    fn local_login_sessions(&self) -> Result<Vec<String>, FactsError> {
        let raw = self.get_key("login.local_sessions", "local_login_sessions")?;
        Ok(raw
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect())
    }

    fn running_processes_matching(&self, names: &[&str]) -> Result<Vec<String>, FactsError> {
        let running = self
            .data
            .get("processes.running")
            .map(|v| v.split(',').map(str::trim).collect::<Vec<_>>())
            .unwrap_or_default();
        Ok(running
            .into_iter()
            .filter(|proc_name| names.contains(proc_name))
            .map(str::to_string)
            .collect())
    }
}

// ---------------------------------------------------------------------------------
// Shared discovery helpers, reused by preconditions.rs and environment.rs. Both
// operate purely through `SystemFacts`, so both are exercised on macOS via
// `FixtureFacts`.
// ---------------------------------------------------------------------------------

/// Upper bound for index-probing loops (cpuidle states, thermal zones, CPUs). Real
/// hardware never approaches this; it exists only to give a probing loop a
/// termination guarantee independent of `path_exists` always returning `true`.
const MAX_PROBE_INDEX: usize = 256;

/// Counts contiguously-numbered `cpuN` directories with a `cpufreq/scaling_governor`
/// file, starting from `cpu0`. This is the enumeration the
/// `GovernorIsPerformanceOnAllCpus` precondition iterates; it is deliberately
/// independent of `/proc/cpuinfo`'s own processor count (a machine without cpufreq
/// on some core is a real, if unusual, configuration this should not paper over).
pub fn discover_governed_cpu_count(facts: &dyn SystemFacts) -> u32 {
    (0..MAX_PROBE_INDEX as u32)
        .take_while(|n| {
            facts.path_exists(&format!(
                "/sys/devices/system/cpu/cpu{n}/cpufreq/scaling_governor"
            ))
        })
        .count() as u32
}

/// Reads `cpu{cpu}`'s per-cpuidle-state name and `disable` flag by probing
/// `.../cpuidle/state{N}/{name,disable}` for contiguously increasing `N`, starting at
/// 0, stopping at the first absent index. A cpuidle driver that never registered a
/// deep C-state (e.g. `intel_idle.max_cstate=1`) simply has fewer states here; that
/// absence, not a `disable=0` reading, is the signal `DeepCstatesDisabled` treats as
/// a pass for the missing state.
pub fn discover_cstates(facts: &dyn SystemFacts, cpu: u32) -> Vec<(String, bool)> {
    let mut states = Vec::new();
    for idx in 0..MAX_PROBE_INDEX {
        let name_path = format!("/sys/devices/system/cpu/cpu{cpu}/cpuidle/state{idx}/name");
        let Ok(name) = facts.read_text(&name_path) else {
            break;
        };
        let disable_path = format!("/sys/devices/system/cpu/cpu{cpu}/cpuidle/state{idx}/disable");
        let disabled = facts
            .read_text(&disable_path)
            .map(|v| v.trim() == "1")
            .unwrap_or(false);
        states.push((name.trim().to_string(), disabled));
    }
    states
}

/// Reads every thermal zone's type and temperature (Celsius) by probing
/// `/sys/class/thermal/thermal_zone{N}/{type,temp}` for contiguously increasing `N`.
pub fn discover_thermal_zones_c(facts: &dyn SystemFacts) -> Vec<(String, f32)> {
    let mut zones = Vec::new();
    for idx in 0..MAX_PROBE_INDEX {
        let type_path = format!("/sys/class/thermal/thermal_zone{idx}/type");
        let Ok(name) = facts.read_text(&type_path) else {
            break;
        };
        let temp_path = format!("/sys/class/thermal/thermal_zone{idx}/temp");
        if let Ok(milli) = facts
            .read_text(&temp_path)
            .and_then(|v| parse_milli(&v, &temp_path))
        {
            zones.push((name.trim().to_string(), milli as f32 / 1000.0));
        }
    }
    zones
}

fn parse_milli(text: &str, path: &str) -> Result<i64, FactsError> {
    text.trim()
        .parse::<i64>()
        .map_err(|_| FactsError::NotFound {
            path: path.to_string(),
            reason: "not an integer".to_string(),
        })
}

/// Candidate sysfs names for the AC adapter's `power_supply` entry; vendors do not
/// agree on one name.
const AC_CANDIDATES: [&str; 4] = ["AC", "AC0", "ADP0", "ADP1"];

/// Reads the AC-online flag by trying each of [`AC_CANDIDATES`] in turn, returning
/// the first that resolves.
pub fn discover_ac_online(facts: &dyn SystemFacts) -> Option<String> {
    AC_CANDIDATES.iter().find_map(|name| {
        facts
            .read_text(&format!("/sys/class/power_supply/{name}/online"))
            .ok()
            .map(|v| v.trim().to_string())
    })
}

/// Candidate sysfs names for the battery's `power_supply` entry.
const BATTERY_CANDIDATES: [&str; 2] = ["BAT0", "BAT1"];

pub fn discover_battery(facts: &dyn SystemFacts) -> Option<(String, String, String)> {
    BATTERY_CANDIDATES.iter().find_map(|name| {
        let status = facts
            .read_text(&format!("/sys/class/power_supply/{name}/status"))
            .ok()?;
        let capacity = facts
            .read_text(&format!("/sys/class/power_supply/{name}/capacity"))
            .ok()?;
        Some((
            (*name).to_string(),
            status.trim().to_string(),
            capacity.trim().to_string(),
        ))
    })
}

/// Parses a Linux CPU-list string (`"6-11"`, `"0,2,4-6"`) into the individual CPU
/// numbers it names.
pub fn parse_cpu_list(s: &str) -> Vec<u32> {
    let mut out = Vec::new();
    for part in s.trim().split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((a, b)) = part.split_once('-') {
            if let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) {
                out.extend(a..=b);
            }
        } else if let Ok(n) = part.parse::<u32>() {
            out.push(n);
        }
    }
    out
}

/// Collapses a list of CPU numbers into range notation (`"6-11"`, `"0,2,4-6"`), the
/// inverse of [`parse_cpu_list`].
pub fn format_cpu_ranges(cpus: &[u32]) -> String {
    let mut sorted = cpus.to_vec();
    sorted.sort_unstable();
    sorted.dedup();

    let mut ranges = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let start = sorted[i];
        let mut end = start;
        while i + 1 < sorted.len() && sorted[i + 1] == end + 1 {
            end = sorted[i + 1];
            i += 1;
        }
        ranges.push(if start == end {
            start.to_string()
        } else {
            format!("{start}-{end}")
        });
        i += 1;
    }
    ranges.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cpu_list_handles_ranges_and_singletons() {
        assert_eq!(parse_cpu_list("6-11"), vec![6, 7, 8, 9, 10, 11]);
        assert_eq!(parse_cpu_list("0,2,4-6"), vec![0, 2, 4, 5, 6]);
        assert_eq!(parse_cpu_list(""), Vec::<u32>::new());
    }

    #[test]
    fn format_cpu_ranges_round_trips_parse_cpu_list() {
        assert_eq!(format_cpu_ranges(&[6, 7, 8, 9, 10, 11]), "6-11");
        assert_eq!(format_cpu_ranges(&[0, 2, 4, 5, 6]), "0,2,4-6");
    }

    #[test]
    fn fixture_facts_translates_governor_path() {
        let facts = FixtureFacts::parse("cpu3.governor=performance\n");
        assert_eq!(
            facts
                .read_text("/sys/devices/system/cpu/cpu3/cpufreq/scaling_governor")
                .unwrap(),
            "performance"
        );
        assert!(
            facts
                .read_text("/sys/devices/system/cpu/cpu4/cpufreq/scaling_governor")
                .is_err()
        );
    }

    #[test]
    fn fixture_facts_translates_energy_performance_preference_path() {
        let facts = FixtureFacts::parse("cpu3.energy_performance_preference=performance\n");
        assert_eq!(
            facts
                .read_text("/sys/devices/system/cpu/cpu3/cpufreq/energy_performance_preference")
                .unwrap(),
            "performance"
        );
        // Absent on a CPU this fixture never mentions, not a guessed value.
        assert!(
            facts
                .read_text("/sys/devices/system/cpu/cpu4/cpufreq/energy_performance_preference")
                .is_err()
        );
    }

    #[test]
    fn fixture_facts_unavailable_sentinel_is_not_found() {
        let facts = FixtureFacts::parse("intel_pstate.no_turbo=unavailable\n");
        assert!(
            facts
                .read_text("/sys/devices/system/cpu/intel_pstate/no_turbo")
                .is_err()
        );
        assert!(!facts.path_exists("/sys/devices/system/cpu/intel_pstate/no_turbo"));
    }

    #[test]
    fn fixture_facts_with_overrides_a_key() {
        let facts =
            FixtureFacts::parse("cpu3.governor=performance\n").with("cpu3.governor", "powersave");
        assert_eq!(
            facts
                .read_text("/sys/devices/system/cpu/cpu3/cpufreq/scaling_governor")
                .unwrap(),
            "powersave"
        );
    }

    #[test]
    fn fixture_facts_cstates_absent_beyond_recorded_states() {
        let facts = FixtureFacts::parse("cstate.POLL=disable:0\ncstate.C1E=disable:0\n");
        let states = discover_cstates(&facts, 0);
        assert_eq!(
            states,
            vec![("POLL".to_string(), false), ("C1E".to_string(), false)]
        );
    }
}
