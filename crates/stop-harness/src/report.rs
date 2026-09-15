//! `REPORT.md` for a STOP-07 abort-latency run: D-34's side-by-side rendering of the observed
//! total and its decomposition, following `render_plat03_verdict`'s own shape
//! (`crates/metrics/src/report.rs`) and its own not-combined statement, because D-34 is D-22
//! applied to this phase.
//!
//! Prose style matches the rest of this project: ASCII only, no em dashes, no emoji, sentence
//! case headings.

use nr_histogram::samples::SampleStats;

/// A summary of one D-35 characterisation figure (read overhead or cross-core offset): min, max
/// and mean over the raw signed nanosecond readings, plus the run it came from. Deliberately not
/// built over `nr_histogram::samples::stats_from_samples`: that machinery is `hdrhistogram`-backed
/// and cannot represent a negative value, and a cross-core offset is signed by construction
/// (`crates/stop-harness/src/characterise.rs::offset_estimate_ns` returns `i64`). A min/max/mean
/// summary over the raw slice needs no such assumption and is adequate for a supplementary
/// figure; the headline abort-latency figure is what gets full percentile treatment.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterisationFigure {
    pub min_ns: i64,
    pub max_ns: i64,
    pub mean_ns: f64,
    pub count: usize,
    pub source_run_id: String,
}

/// Summarises `samples` (already read from a prior characterisation run's own
/// `clock-characterisation.tsv`), or `None` for an empty slice: an empty summary is not
/// reportable and must not render as a fabricated zero.
pub fn summarise_characterisation(
    samples: &[i64],
    source_run_id: &str,
) -> Option<CharacterisationFigure> {
    if samples.is_empty() {
        return None;
    }
    let min_ns = *samples.iter().min().expect("checked non-empty above");
    let max_ns = *samples.iter().max().expect("checked non-empty above");
    let mean_ns = samples.iter().sum::<i64>() as f64 / samples.len() as f64;
    Some(CharacterisationFigure {
        min_ns,
        max_ns,
        mean_ns,
        count: samples.len(),
        source_run_id: source_run_id.to_string(),
    })
}

/// Everything [`render_abort_latency_report`] needs. Independent of [`nr_manifest::RunManifest`]
/// on purpose, matching `render_plat03_verdict`'s own precedent: a focused input type is easier
/// to construct in a test than a full manifest, and the renderer's own contract stays narrow.
#[derive(Debug, Clone)]
pub struct AbortLatencyReportInput {
    pub run_id: String,
    pub period_ns: u64,
    pub trials: usize,
    /// End-to-end abort observation latency, in nanoseconds: the D-31 interval, over a uniformly
    /// random abort phase within one poll period (D-34).
    pub latency_stats: SampleStats,
    /// D-35's cross-core propagation component, from the characterisation run. `None` when no
    /// characterisation run was supplied (`--characterisation-tsv` was not given).
    pub cross_core_offset: Option<CharacterisationFigure>,
    /// D-35's clock read overhead, from the same characterisation run. `None` for the same
    /// reason.
    pub read_overhead: Option<CharacterisationFigure>,
    /// The running clocksource, from the characterisation run, when known.
    pub clocksource: Option<String>,
}

/// The five caveats task 2's own action text requires, each its own sentence.
fn caveats() -> [String; 5] {
    [
        "The measured hot path is the D-33 stand-in: a single thread on one isolated core whose \
         loop body is only the latch poll, nothing else. Phase 6 re-measures against the real \
         DAG."
            .to_string(),
        "This measurement predates the Phase 3 substrate: no mlockall, no preallocated pools, no \
         allocator hook (D-36)."
            .to_string(),
        "Thread interleavings are unverified until CHAN-06 brings loom into CI in Phase 4 \
         (D-50); this figure says nothing about them."
            .to_string(),
        "The PLAT-01 approximately 3.8 ms stall is unexplained and not reproduced. This figure \
         carries it as an explicit limitation; see \
         docs/rig/plat01-stall-investigation.md for the exposure it was not observed over."
            .to_string(),
        "The cross-core offset estimate assumes symmetric propagation delay in both directions \
         of the ping-pong exchange."
            .to_string(),
    ]
}

/// Renders `REPORT.md` for one abort-latency run. Presents, as separately attributed figures:
/// the end-to-end worst case and percentiles over a uniformly random abort phase, the trial
/// count and poll period that produced them; the poll period itself, named as its own term; the
/// cross-core propagation component and the clock read overhead from the characterisation run,
/// each with its method named; then an explicit sentence that none of this is combined, netted
/// or subtracted. A missing characterisation figure is printed as an explicit "unavailable" line,
/// never a blank cell or a substituted zero: finding 10 of the external audit corrected exactly
/// that error once already.
pub fn render_abort_latency_report(input: &AbortLatencyReportInput) -> String {
    let mut out = String::new();
    out.push_str("# STOP-07 abort latency report\n\n");
    out.push_str(&format!("run id: {}\n", input.run_id));
    out.push_str(&format!("poll period: {} ns\n", input.period_ns));
    out.push_str(&format!("trials: {}\n\n", input.trials));

    out.push_str("## Abort observation latency (end to end)\n\n");
    out.push_str(
        "Measured from the timestamp the aborting thread takes immediately before the latch \
         store to the timestamp the hot-path thread takes at the iteration where it first \
         observes the gate closed (D-31), over a uniformly random abort phase within one poll \
         period (D-34). Nothing sits between either timestamp read and the call it brackets.\n\n",
    );
    out.push_str("| statistic | value (ns) |\n|-----------|------------|\n");
    for (quantile, value) in &input.latency_stats.values {
        out.push_str(&format!("| p{} | {value} |\n", format_quantile(*quantile)));
    }
    out.push_str(&format!(
        "| maximum (worst case) | {} |\n",
        input.latency_stats.max
    ));
    out.push_str(&format!(
        "\nsample count: {}\n\n",
        input.latency_stats.count
    ));

    out.push_str("## Decomposition (D-34)\n\n");
    out.push_str(&format!(
        "poll period: {} ns, stated as its own term. The hot path checks the latch once per \
         iteration at a fixed point in the loop (D-32), so the end-to-end observation latency \
         above is bounded by one iteration plus cross-core propagation.\n\n",
        input.period_ns
    ));

    render_characterisation_figure(
        &mut out,
        "cross-core propagation",
        "a Cristian's-algorithm round-trip estimate between the two measurement cores \
         (crates/stop-harness/src/characterise.rs::offset_estimate_ns)",
        &input.cross_core_offset,
    );
    render_characterisation_figure(
        &mut out,
        "clock read overhead",
        "the consecutive-delta cost of reading CLOCK_MONOTONIC_RAW on the measurement host \
         (crates/stop-harness/src/characterise.rs::read_overhead_ns)",
        &input.read_overhead,
    );
    match &input.clocksource {
        Some(source) => out.push_str(&format!("running clocksource: {source}\n\n")),
        None => out.push_str(
            "running clocksource: unavailable. No characterisation run was supplied to this \
             report.\n\n",
        ),
    }

    out.push_str(
        "These figures are reported side by side and are not combined, netted or subtracted \
         from one another. The end-to-end worst case above already includes whatever cross-core \
         propagation and poll-period delay actually occurred in each trial; the decomposition \
         names the terms that compose it, and arithmetic between the two would double-count or \
         discard part of what was actually measured.\n\n",
    );

    out.push_str("## Caveats\n\n");
    for caveat in caveats() {
        out.push_str(&format!("- {caveat}\n"));
    }
    out.push('\n');

    out
}

fn render_characterisation_figure(
    out: &mut String,
    label: &str,
    method: &str,
    figure: &Option<CharacterisationFigure>,
) {
    match figure {
        Some(figure) => {
            out.push_str(&format!(
                "{label}: min {} ns, mean {:.1} ns, max {} ns, from {} ({} samples). method: \
                 {method}\n\n",
                figure.min_ns, figure.mean_ns, figure.max_ns, figure.source_run_id, figure.count
            ));
        }
        None => {
            out.push_str(&format!(
                "{label}: unavailable. No characterisation run was supplied to this report \
                 (--characterisation-tsv was not given). Stated explicitly rather than left \
                 blank or printed as zero, which would read as a real observation of zero.\n\n"
            ));
        }
    }
}

/// Renders a quantile as a percentile label, e.g. `0.5` -> `"50"`, `0.999` -> `"99.9"`.
fn format_quantile(quantile: f64) -> String {
    let percent = quantile * 100.0;
    if (percent.round() - percent).abs() < f64::EPSILON {
        format!("{percent:.0}")
    } else {
        format!("{percent}")
    }
}

/// Everything [`render_characterisation_report`] needs.
#[derive(Debug, Clone)]
pub struct CharacterisationReportInput {
    pub run_id: String,
    pub read_overhead: Option<CharacterisationFigure>,
    pub cross_core_offset: Option<CharacterisationFigure>,
    pub clocksource: Option<String>,
}

/// Renders `REPORT.md` for a D-35 clock characterisation run (`RunClass::Recon`). Simpler than
/// [`render_abort_latency_report`]: this run measures the clock itself, not the D-33 stand-in
/// hot path, so none of that renderer's abort-latency-specific caveats apply here. Published
/// beside the abort-latency figure per D-35, not folded into it.
pub fn render_characterisation_report(input: &CharacterisationReportInput) -> String {
    let mut out = String::new();
    out.push_str("# STOP-07 clock characterisation report (D-35)\n\n");
    out.push_str(&format!("run id: {}\n\n", input.run_id));
    out.push_str(
        "This run measures the clock used by the abort-latency figure, not the abort-latency \
         figure itself: the read cost of CLOCK_MONOTONIC_RAW, and the cross-core offset between \
         the two measurement cores. Published beside the abort-latency report per D-35, never \
         combined with it.\n\n",
    );

    render_characterisation_figure(
        &mut out,
        "clock read overhead",
        "the consecutive-delta cost of reading CLOCK_MONOTONIC_RAW on the measurement host",
        &input.read_overhead,
    );
    render_characterisation_figure(
        &mut out,
        "cross-core propagation",
        "a Cristian's-algorithm round-trip estimate between the two measurement cores, which \
         assumes symmetric propagation delay in both directions of the exchange",
        &input.cross_core_offset,
    );
    match &input.clocksource {
        Some(source) => out.push_str(&format!("running clocksource: {source}\n\n")),
        None => out.push_str("running clocksource: unavailable\n\n"),
    }

    out
}
