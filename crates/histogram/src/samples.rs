//! Percentiles over a flat slice of raw samples.
//!
//! `percentiles.rs` computes over a parsed `CyclictestRun`: per-thread columns, separately
//! reported overflows, a footer carrying exact minima and maxima. A measurement that is N trials
//! on one code path has none of that structure, so this module takes the samples directly.
//!
//! Same machinery, same philosophy. The percentiles come from `hdrhistogram`, exactly as
//! `CyclictestRun::percentiles` does, so the two cannot drift on rounding. The minimum and the
//! maximum are taken from the slice itself rather than from the histogram, matching the same
//! exactness rule already documented on the cyclictest run's own maximum accessor: the worst
//! case is exact, never a bound.

use hdrhistogram::Histogram;

use crate::percentiles::PercentileError;

/// hdrhistogram significant figures for raw sample statistics.
///
/// Five, not the three `percentiles.rs` uses. That constant is documented as keeping single digits
/// to low thousands of MICROSECONDS exact. These samples are nanoseconds: a 33 microsecond poll
/// period is 33_000 ns, where three significant figures would quantize to the nearest 33 ns and
/// five quantizes to the nearest 0.33 ns. The published figure is a worst case in the low tens of
/// microseconds, so the quantization must be far below the difference anyone would care about.
const SAMPLE_SIGFIG: u8 = 5;

/// Statistics over a flat slice of samples. Unit-agnostic on purpose: the caller knows whether
/// it passed nanoseconds or microseconds, and a field whose name implies one unit while holding
/// values in another is the kind of quiet mislabelling this project has already had to correct
/// once.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleStats {
    /// Number of samples. Equals the slice length; there is no overflow concept here.
    pub count: u64,
    /// `(quantile, value)` pairs, in the order requested. Quantized to `SAMPLE_SIGFIG`.
    pub values: Vec<(f64, u64)>,
    /// Exact, from the slice. Never quantized and never a bound.
    pub min: u64,
    /// Exact, from the slice. Never quantized and never a bound.
    pub max: u64,
}

/// Computes [`SampleStats`] over `samples` at the requested `quantiles`.
///
/// Returns [`PercentileError::EmptySamples`] on an empty slice rather than a zero-filled
/// result: a substituted zero is indistinguishable from a real observation, exactly the reason
/// `percentiles.rs` refuses the equivalent empty-run case.
pub fn stats_from_samples(
    samples: &[u64],
    quantiles: &[f64],
) -> Result<SampleStats, PercentileError> {
    if samples.is_empty() {
        return Err(PercentileError::EmptySamples);
    }

    let mut hist = Histogram::<u64>::new(SAMPLE_SIGFIG)?;
    for &sample in samples {
        hist.record(sample)?;
    }

    let values = quantiles
        .iter()
        .map(|&quantile| (quantile, hist.value_at_quantile(quantile)))
        .collect();

    Ok(SampleStats {
        count: samples.len() as u64,
        values,
        min: samples.iter().copied().min().unwrap_or(0),
        max: samples.iter().copied().max().unwrap_or(0),
    })
}
