//! Report rendering: the per-run `REPORT.md` (D-12) and the D-22 PLAT-03 decomposition.
//! Generated, never hand-authored, so the human-readable summary a reader sees can never
//! quietly disagree with the manifest and capture behind it.
//!
//! Every renderer here emits ASCII only, in the project's writing style: no em dashes, no
//! smart quotes, no emoji, sentence case headings.
//!
//! Publication decisions fixed here: histograms are generated on demand from the raw capture
//! and never committed as images, so the repository holds no binary plot files; the ASCII
//! histogram below is the in-repo visual form, since it diffs cleanly in review and needs no
//! plotting dependency.

use nr_histogram::hist::CyclictestRun;
use nr_histogram::percentiles::PercentileError;
use nr_manifest::{ContaminationVerdict, CpuCounter, RunManifest};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;

use crate::kebab;

/// Fixed display width of an ASCII histogram bar, in characters.
const HISTOGRAM_BAR_WIDTH: usize = 40;

/// Errors rendering a report.
#[derive(Debug, Error)]
pub enum ReportError {
    #[error("failed to compute percentiles: {0}")]
    Percentile(#[from] PercentileError),
    #[error("failed to format a timestamp: {0}")]
    Timestamp(#[from] time::error::Format),
}

/// The PLAT-03 verdict input.
///
/// The `hwlat` observation is `Option` on purpose. It used to be a required scalar, which
/// forced every caller to supply a "firmware floor" and made subtracting it look like the
/// intended use. A verdict is perfectly reportable without one: the scheduling maximum against
/// the gate stands on its own, and a missing hwlat observation is printed as missing rather
/// than defaulted to zero.
#[derive(Debug, Clone)]
pub struct Plat03Input {
    pub observed_max_us: u64,
    /// The gate PLAT-03 measures against. 30 in this project.
    pub gate_us: u64,
    /// An independent `hwlatdetect` observation, when a paired one exists. Reported alongside
    /// the scheduling maximum and never combined with it; see [`render_plat03_verdict`].
    pub hwlat: Option<HwlatObservation>,
    pub p99_us: u64,
    pub p50_us: u64,
}

/// A hardware-gap observation from `hwlatdetect`, carried so PLAT-03 can report it beside the
/// scheduling maximum while keeping the two visibly distinct.
#[derive(Debug, Clone)]
pub struct HwlatObservation {
    pub max_us: u64,
    /// The run directory this was observed in. A figure with no named source is not reportable.
    pub source_run_id: String,
    /// The conditions it was observed under, in the operator's words: CPU placement, load and
    /// thermal state. Without these the number cannot be compared to anything.
    pub conditions: String,
}

/// Renders the PLAT-03 verdict: the observed scheduling maximum against the gate, and separately
/// any paired `hwlatdetect` observation.
///
/// # Why there is no subtraction here
///
/// This function used to report `observed_max_us - firmware_floor_us` as "the kernel's
/// contribution", and plan 01-13 described that difference as "the largest the kernel's
/// contribution could be". It is not an upper bound, and it is not a lower bound either.
///
/// Take a 40 us scheduling maximum caused entirely by kernel activity, during which no firmware
/// interruption occurred at all. A separate `hwlatdetect` run observes a 22 us hardware gap.
/// Subtracting gives 18 us, while the kernel's actual contribution to that event was the full
/// 40 us. The subtraction understates it by half, and `saturating_sub` additionally turns any
/// hwlat maximum above the scheduling maximum into a flat zero.
///
/// The deeper problem is that the two figures are not commensurable. `cyclictest` measures
/// wakeup latency on the isolated CPUs; `hwlatdetect` counts sampling records whose polling
/// interval contained a gap, on whatever CPUs it was pointed at, under its own load and thermal
/// state. Firmware stalls and scheduling delay do not compose additively, and the two are
/// measured neither on the same CPUs nor at the same time.
///
/// So both numbers are printed, each attributed to the run and conditions it came from, and the
/// reader is told explicitly that they are not combined. Attribution of the residual to a named
/// cause belongs with a trace that supports it, not with arithmetic.
///
/// Recorded as finding 1 of `.planning/phases/01-trustworthy-measurement/01-EXTERNAL-AUDIT.md`.
pub fn render_plat03_verdict(input: &Plat03Input) -> Result<String, ReportError> {
    let jitter_us = input.p99_us.saturating_sub(input.p50_us);

    // At or above the gate is a miss. The project settled on the `samples_at_or_above`
    // convention on 2026-08-31, and a strict `<` here keeps the verdict consistent with it;
    // the previous `<=` reported a maximum of exactly the gate value as a pass.
    let under_gate = input.observed_max_us < input.gate_us;
    let verdict_word = if under_gate {
        format!("under the {} us gate", input.gate_us)
    } else {
        "documented limitation".to_string()
    };

    let mut out = String::new();
    out.push_str("## PLAT-03 verdict\n\n");
    out.push_str(&format!(
        "total observed maximum: {} us against the {} us gate\n",
        input.observed_max_us, input.gate_us
    ));
    out.push_str(&format!("jitter (p99 minus p50): {jitter_us} us\n"));
    match &input.hwlat {
        Some(h) => {
            out.push_str(&format!(
                "independent hwlatdetect maximum: {} us (run {}, conditions: {})\n",
                h.max_us, h.source_run_id, h.conditions
            ));
            out.push_str(
                "the two figures above are not subtracted: they come from different \
                 instruments, on different CPUs, under different conditions, and firmware \
                 stalls do not compose additively with scheduling delay\n",
            );
        }
        None => {
            out.push_str("independent hwlatdetect maximum: no paired hwlatdetect observation\n")
        }
    }
    out.push_str(&format!("verdict: {verdict_word}\n"));
    Ok(out)
}

/// Renders the per-run `REPORT.md`. Every section is derived from `manifest` and `run`; the
/// header names the manifest blake3 it was generated from, so a reader can tell the report and
/// the manifest can never quietly disagree (D-12).
pub fn render_run_report(
    manifest: &RunManifest,
    run: &CyclictestRun,
) -> Result<String, ReportError> {
    let mut out = String::new();
    out.push_str(&render_header(manifest)?);
    out.push_str(&render_rig_and_tuning(manifest));
    out.push_str(&render_preconditions(manifest));
    out.push_str(&render_contamination(manifest));
    out.push_str(&render_results(run)?);
    out.push_str(&render_histogram(run));
    out.push_str(&render_overflow_convention());
    out.push_str(&render_artifacts(manifest));
    Ok(out)
}

fn render_header(manifest: &RunManifest) -> Result<String, ReportError> {
    let mut out = String::new();
    out.push_str("# Run report\n\n");
    out.push_str(&format!("run id: {}\n", manifest.run_id));
    out.push_str(&format!("run class: {}\n", kebab(&manifest.run_class)));
    out.push_str(&format!(
        "instrument class: {}\n",
        kebab(&manifest.instrument_class)
    ));
    out.push_str(&format!(
        "provenance tier: {}\n",
        kebab(&manifest.provenance_tier)
    ));
    out.push_str(&format!(
        "utc start: {}\n",
        manifest.utc_start.format(&Rfc3339)?
    ));
    out.push_str(&format!(
        "utc end: {}\n",
        manifest.utc_end.format(&Rfc3339)?
    ));
    out.push_str(&format!(
        "manifest blake3: {}\n\n",
        manifest_blake3(manifest)
    ));
    Ok(out)
}

/// A content fingerprint of `manifest`, computed by the renderer itself rather than read from a
/// field. This is what makes "generated, not authored" mechanical (D-12): the header line
/// changes whenever the manifest's content changes, so the two cannot silently drift apart.
fn manifest_blake3(manifest: &RunManifest) -> String {
    let bytes = serde_json::to_vec(manifest).expect("a RunManifest always serialises");
    blake3::hash(&bytes).to_hex().to_string()
}

/// Rendered in the same field order `RIG.txt` uses, so a reader already familiar with that
/// artifact recognises this section.
fn render_rig_and_tuning(manifest: &RunManifest) -> String {
    let host = &manifest.host;
    let kernel = &manifest.kernel;
    let os = &manifest.os;
    let tuning = &manifest.tuning;
    let power = &manifest.power;
    let network = &manifest.network;

    let governor = tuning
        .per_cpu_governor
        .iter()
        .map(|g| format!("cpu{}={}", g.cpu, g.governor))
        .collect::<Vec<_>>()
        .join(" ");
    let cstates = tuning
        .cstates
        .iter()
        .map(|c| {
            format!(
                "cpu{}.{}={}",
                c.cpu,
                c.name,
                if c.disabled { "off" } else { "on" }
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let no_turbo = tuning
        .no_turbo
        .map(|v| if v { "1" } else { "0" }.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let ac_online = power
        .ac_online
        .map(|v| if v { "1" } else { "0" }.to_string())
        .unwrap_or_else(|| "unavailable".to_string());

    let mut out = String::new();
    out.push_str("## Rig and tuning\n\n");
    out.push_str("```\n");
    out.push_str(&format!(
        "system:       {} {}\n",
        host.system_vendor, host.system_model
    ));
    out.push_str(&format!(
        "bios:         {} released {}\n",
        host.bios_version, host.bios_release_date
    ));
    out.push_str(&format!("cpu:          {}\n", host.cpu_model));
    out.push_str(&format!(
        "topology:     {} logical / {} cores / {} socket\n",
        host.logical_cpus, host.physical_cores, host.sockets
    ));
    out.push_str(&format!("p_cores:      {}\n", host.p_cores));
    out.push_str(&format!("e_cores:      {}\n", host.e_cores));
    out.push_str(&format!("microcode:    {}\n", host.microcode));
    out.push_str(&format!(
        "kernel:       {} ({})\n",
        kernel.release, kernel.version_string
    ));
    out.push_str(&format!("os:           {} {}\n", os.distro, os.version));
    out.push_str(&format!(
        "session:      {} display_manager_active={}\n",
        kebab(&os.session_kind),
        os.display_manager_active
    ));
    out.push_str(&format!("memory_gb:    {}\n", host.memory_gb));
    out.push_str(&format!("cmdline:      {}\n", kernel.cmdline));
    out.push_str(&format!("governor:     {governor}\n"));
    out.push_str(&format!("no_turbo:     {no_turbo}\n"));
    out.push_str(&format!("cstates:      {cstates}\n"));
    out.push_str(&format!(
        "power:        AC={ac_online} battery={}\n",
        power.battery_status.as_deref().unwrap_or("unavailable")
    ));
    out.push_str(&format!(
        "nic_wired:    {} driver={} state={}\n",
        network.wired_iface.as_deref().unwrap_or("absent"),
        network.wired_driver.as_deref().unwrap_or("absent"),
        network.wired_state.as_deref().unwrap_or("absent")
    ));
    out.push_str(&format!(
        "nic_ptp:      {}\n",
        network.ptp_capabilities.as_deref().unwrap_or("absent")
    ));
    out.push_str(&format!(
        "nic_wifi:     {} driver={}\n",
        network.wifi_iface.as_deref().unwrap_or("absent"),
        network.wifi_driver.as_deref().unwrap_or("absent")
    ));
    out.push_str("```\n\n");
    out
}

/// All 15 D-06 precondition checks, always, whether they passed or failed.
fn render_preconditions(manifest: &RunManifest) -> String {
    let mut out = String::new();
    out.push_str("## Preconditions\n\n");
    out.push_str("| check | status | observed | expected |\n");
    out.push_str("|-------|--------|----------|----------|\n");
    for result in &manifest.preconditions {
        out.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            kebab(&result.check),
            kebab(&result.status),
            result.observed,
            result.expected
        ));
    }
    out.push('\n');
    out
}

fn counter_for(counters: &[CpuCounter], cpu: u32) -> u64 {
    counters
        .iter()
        .find(|counter| counter.cpu == cpu)
        .map(|counter| counter.count)
        .unwrap_or(0)
}

/// The D-15/D-24 contamination verdict: the D-24 tail metrics computed from the run's own
/// histogram (the primary signal), whether the thresholds behind the verdict are provisional,
/// and the `/proc/interrupts` counter deltas per isolated CPU (D-15's original signal, retained
/// as evidence), or the word "uncalibrated" and the reason when no thresholds exist yet (D-17).
fn render_contamination(manifest: &RunManifest) -> String {
    let pair = &manifest.interference;

    let mut out = String::new();
    out.push_str("## Contamination verdict\n\n");

    match pair.verdict {
        ContaminationVerdict::Uncalibrated => {
            out.push_str("verdict: uncalibrated\n");
            let reason = manifest
                .exclusion_reason
                .as_deref()
                .unwrap_or("no calibrated contamination thresholds exist yet (D-17)");
            out.push_str(&format!("reason: {reason}\n"));
        }
        ref verdict => {
            out.push_str(&format!("verdict: {}\n", kebab(verdict)));
            if let Some(reason) = &manifest.exclusion_reason {
                out.push_str(&format!("reason: {reason}\n"));
            }
        }
    }
    if pair.thresholds_provisional == Some(true) {
        out.push_str(
            "thresholds: provisional (derived from 2 runs, not a calibrated set; see \
             config/contamination-thresholds.json)\n",
        );
    }

    out.push_str("\ntail metrics (D-24):\n\n");
    match &pair.tail_metrics {
        Some(tail) => {
            out.push_str("| metric | value |\n|--------|-------|\n");
            out.push_str(&format!(
                "| tail excursion ratio (max / p99) | {:.1} |\n",
                tail.tail_excursion_ratio
            ));
            out.push_str(&format!(
                "| thread-max spread | {:.1}% |\n",
                tail.thread_max_spread * 100.0
            ));
            out.push_str(&format!(
                "| overflow rate | {:.4}/s |\n",
                tail.overflow_rate_per_s
            ));
        }
        None => out.push_str("not computed: this manifest predates D-24.\n"),
    }

    out.push_str("\ncounter deltas per isolated cpu:\n\n");
    out.push_str("| cpu | cal ipis | tlb ipis | context switches | irqs |\n");
    out.push_str("|-----|----------|----------|-------------------|------|\n");
    for &cpu in &pair.before.isolated_cpus {
        out.push_str(&format!(
            "| {cpu} | {} | {} | {} | {} |\n",
            counter_for(&pair.delta.cal_ipis, cpu),
            counter_for(&pair.delta.tlb_ipis, cpu),
            counter_for(&pair.delta.context_switches, cpu),
            counter_for(&pair.delta.irqs, cpu)
        ));
    }
    out.push('\n');
    out
}

/// A percentile table plus the sample count, the overflow count and the maximum. Percentiles
/// always come from the overflow-inclusive path: this module never reaches for the sibling,
/// overflow-excluding computation, because these are the numbers the project publishes.
fn render_results(run: &CyclictestRun) -> Result<String, ReportError> {
    let percentiles = run.percentiles(&[0.5, 0.95, 0.99, 0.999])?;

    let mut out = String::new();
    out.push_str("## Results\n\n");
    out.push_str("| percentile | latency (us) |\n");
    out.push_str("|------------|---------------|\n");
    let labels = ["p50", "p95", "p99", "p99.9"];
    for (label, &(_, value)) in labels.iter().zip(percentiles.values.iter()) {
        out.push_str(&format!("| {label} | {value} |\n"));
    }
    out.push('\n');
    out.push_str(&format!("sample count: {}\n", percentiles.total_samples));
    out.push_str(&format!(
        "overflow count: {}\n",
        percentiles.overflow_samples
    ));
    out.push_str(&format!("maximum: {} us\n\n", percentiles.max_us));
    Ok(out)
}

/// A fixed-width ASCII histogram, log scale on the count axis, one row per non-empty bin, per
/// OSADL's published convention (1 us resolution, logarithmic y axis).
fn render_histogram(run: &CyclictestRun) -> String {
    let bins: Vec<(u64, u64)> = run
        .to_bin_table()
        .into_iter()
        .filter(|&(_, count)| count > 0)
        .collect();

    let mut out = String::new();
    out.push_str("## Distribution\n\n");

    let Some(&(low, _)) = bins.first() else {
        out.push_str("no binned samples.\n\n");
        return out;
    };
    let (high, _) = *bins.last().expect("bins is non-empty, checked above");

    out.push_str(&format!(
        "ASCII histogram, bins {low} to {high} us, count axis on a log2 scale, one row per \
         non-empty bin.\n\n"
    ));
    out.push_str("```\n");

    let max_count = bins.iter().map(|&(_, count)| count).max().unwrap_or(1);
    let max_log = (max_count as f64 + 1.0).ln();
    for (bin_us, count) in &bins {
        let bar_len = if max_log > 0.0 {
            (((*count as f64 + 1.0).ln() / max_log) * HISTOGRAM_BAR_WIDTH as f64).round() as usize
        } else {
            HISTOGRAM_BAR_WIDTH
        };
        let bar_len = bar_len.clamp(1, HISTOGRAM_BAR_WIDTH);
        let bar = "#".repeat(bar_len);
        out.push_str(&format!("{bin_us:>6} us | {bar} {count}\n"));
    }
    out.push_str("```\n\n");
    out
}

fn render_overflow_convention() -> String {
    "## Overflow convention\n\n\
     Overflow samples are recorded at the histogram bound, which is a lower bound on their \
     true value. Percentiles at or above the overflow fraction are therefore conservative: the \
     reported value is no larger than the truth.\n\n"
        .to_string()
}

fn render_artifacts(manifest: &RunManifest) -> String {
    let mut out = String::new();
    out.push_str("## Artifacts\n\n");
    out.push_str("| path | bytes | blake3 |\n");
    out.push_str("|------|-------|--------|\n");
    for artifact in &manifest.artifacts {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            artifact.path, artifact.bytes, artifact.blake3
        ));
    }
    out
}
