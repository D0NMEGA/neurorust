//! `MSR_SMI_COUNT` (register 0x34): an exact per-CPU count of system management interrupts
//! serviced on that CPU since boot.
//!
//! Read before and after a run, the difference is how many SMIs reached that CPU during it.
//! No sampling, no threshold, no inference from timing gaps, which is what three
//! `hwlatdetect` arms spent 45 minutes failing to establish. It answers how many, never how
//! long: a count is not a duration and must never be reported as one.
//!
//! Requires root and the `msr` kernel module. A failed read records its reason; a zero would
//! be indistinguishable from a CPU that genuinely took no SMIs, and those are opposite
//! findings, so no code path in this module ever substitutes one for the other.

use std::collections::BTreeMap;
#[cfg(target_os = "linux")]
use std::process::Command;

use nr_manifest::CpuCounter;
use thiserror::Error;

/// `MSR_SMI_COUNT`. The only register this module ever reads: `wrmsr` is never invoked, and
/// no caller can pass a different register in.
pub const SMI_COUNT_REGISTER: &str = "0x34";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SmiError {
    #[error("failed to spawn rdmsr: {0}")]
    Spawn(String),
    #[error("rdmsr exited {0} (is the msr module loaded and are we root?)")]
    ExitedNonZero(i32),
    #[error("could not parse rdmsr output {0:?} as a register value")]
    Unparseable(String),
    #[error("no reading recorded")]
    Missing,
}

/// Parses one `rdmsr -p <cpu> 0x34` reading: a bare value with no `0x` prefix. This module's
/// own invocation never passes `-d`, so a real reading is hexadecimal (`fa6`, confirmed
/// against `docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt`, where it equals 4006
/// decimal); an optional leading `0x`/`0X`, if one is ever present, is tolerated and stripped
/// rather than rejected.
pub fn parse_rdmsr_value(text: &str) -> Result<u64, SmiError> {
    let trimmed = text
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if trimmed.is_empty() {
        return Err(SmiError::Unparseable(text.to_string()));
    }
    u64::from_str_radix(trimmed, 16).map_err(|_| SmiError::Unparseable(text.to_string()))
}

/// Calls `read_one` for every CPU in `cpus`, keeping every CPU that succeeded and recording
/// only the FIRST failure's reason (naming the CPU and the error) rather than discarding
/// every successful reading alongside it. A CPU whose read fails contributes no entry to the
/// returned `Vec`: there is no code path here that turns a failed read into a `0`, since zero
/// SMIs and an unreadable register are opposite findings.
fn aggregate(
    cpus: &[u32],
    mut read_one: impl FnMut(u32) -> Result<u64, SmiError>,
) -> (Vec<CpuCounter>, Option<String>) {
    let mut counters = Vec::with_capacity(cpus.len());
    let mut reason: Option<String> = None;
    for &cpu in cpus {
        match read_one(cpu) {
            Ok(count) => counters.push(CpuCounter { cpu, count }),
            Err(err) => {
                if reason.is_none() {
                    reason = Some(format!("cpu {cpu}: {err}"));
                }
            }
        }
    }
    (counters, reason)
}

/// Parses one snapshot's worth of readings, in the exact form this project's own probe
/// records them (`docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt`): one
/// `cpu <N> 0x34 = <hex>` line per CPU. A requested CPU with no matching line contributes no
/// counter entry; the returned reason (if any) names the first such CPU rather than every
/// one, matching [`nr_manifest::SmiCounts::unavailable_reason`]'s own single-reason shape.
///
/// Used by the `NRMEASURE_SMI_FIXTURE` seam (one fixture text read for both the before and
/// after snapshot of a run, like every other fixture in this project) and by every test in
/// this module; aggregation is shared with the live path via [`aggregate`], never a second,
/// different rule for what counts as unreadable.
pub fn parse_smi_snapshot(text: &str, requested_cpus: &[u32]) -> (Vec<CpuCounter>, Option<String>) {
    let mut readings: BTreeMap<u32, Result<u64, SmiError>> = BTreeMap::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let [prefix, cpu_str, register, equals, value] = tokens.as_slice() else {
            continue;
        };
        if *prefix != "cpu" || *equals != "=" || *register != SMI_COUNT_REGISTER {
            continue;
        }
        let Ok(cpu) = cpu_str.parse::<u32>() else {
            continue;
        };
        readings.insert(cpu, parse_rdmsr_value(value));
    }

    aggregate(requested_cpus, |cpu| {
        readings.remove(&cpu).unwrap_or(Err(SmiError::Missing))
    })
}

/// Reads register 0x34 on every CPU in `cpus` by spawning `rdmsr -p <cpu> 0x34` directly (no
/// shell, T-1-09) once per CPU, keeping every CPU that succeeded and recording only the first
/// failure's reason (see [`aggregate`]). Linux-gated: `rdmsr` requires root and the `msr`
/// kernel module, like the register itself; a non-Linux host records why rather than
/// attempting a spawn that could never succeed.
#[cfg(target_os = "linux")]
pub fn read_smi_counts(cpus: &[u32]) -> (Vec<CpuCounter>, Option<String>) {
    aggregate(cpus, |cpu| {
        let output = Command::new("rdmsr")
            .args(["-p", &cpu.to_string(), SMI_COUNT_REGISTER])
            .output()
            .map_err(|source| SmiError::Spawn(source.to_string()))?;
        if !output.status.success() {
            return Err(SmiError::ExitedNonZero(output.status.code().unwrap_or(-1)));
        }
        parse_rdmsr_value(&String::from_utf8_lossy(&output.stdout))
    })
}

#[cfg(not(target_os = "linux"))]
pub fn read_smi_counts(_cpus: &[u32]) -> (Vec<CpuCounter>, Option<String>) {
    (
        Vec::new(),
        Some(
            "reading MSR_SMI_COUNT requires Linux; set NRMEASURE_SMI_FIXTURE to run against a \
             fixture on this host"
                .to_string(),
        ),
    )
}

/// `after` minus `before`, per CPU, saturating at zero. Only CPUs present in BOTH `before`
/// and `after` get an entry: a CPU that read successfully on only one side has no true delta
/// to report, and fabricating one (for example by treating the missing side as zero) is
/// exactly the "unreadable register read as a zero" confusion this module exists to prevent.
/// `before` and `after` each independently record every CPU that DID succeed on their own
/// read, so nothing is silently lost from the manifest as a whole, only from this one
/// computed field.
pub fn delta(before: &[CpuCounter], after: &[CpuCounter]) -> Vec<CpuCounter> {
    let before_map: BTreeMap<u32, u64> = before.iter().map(|c| (c.cpu, c.count)).collect();
    after
        .iter()
        .filter_map(|c| {
            before_map.get(&c.cpu).map(|&b| CpuCounter {
                cpu: c.cpu,
                count: c.count.saturating_sub(b),
            })
        })
        .collect()
}
