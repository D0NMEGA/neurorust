//! Percentile computation for cyclictest runs, backed by `hdrhistogram`. Cyclictest's separately
//! reported overflow samples are counted by default; see [`CyclictestRun::percentiles`].
//!
//! Overflow samples are recorded at the histogram bound, which is a lower bound on their true
//! value. Percentiles at or above the overflow fraction are therefore conservative: the reported
//! value is no larger than the truth. The reported maximum comes from the cyclictest footer and
//! is exact.

use hdrhistogram::Histogram;
use thiserror::Error;

use crate::hist::CyclictestRun;

/// hdrhistogram significant figures. 3 keeps every latency value this project expects to see
/// (single digits to low thousands of microseconds) exact; see the precision note in
/// `crates/histogram/tests/percentiles.rs`.
const SIGFIG: u8 = 3;

/// Errors computing percentiles or building the underlying `hdrhistogram::Histogram`s.
#[derive(Debug, Error)]
pub enum PercentileError {
    #[error("cannot compute percentiles over a run with zero samples")]
    EmptyRun,
    #[error("cannot compute statistics over zero samples")]
    EmptySamples,
    #[error("failed to create hdrhistogram: {0}")]
    Creation(#[from] hdrhistogram::CreationError),
    #[error("failed to record a sample: {0}")]
    Record(#[from] hdrhistogram::RecordError),
    #[error("failed to merge per-thread histograms: {0}")]
    Addition(#[from] hdrhistogram::AdditionError),
}

/// The result of computing percentiles (and related totals) over a [`CyclictestRun`].
#[derive(Debug, Clone)]
pub struct Percentiles {
    /// Binned samples plus overflow samples, if overflows were included in this computation.
    pub total_samples: u64,
    /// Samples present in the histogram body.
    pub binned_samples: u64,
    /// Samples cyclictest reported as overflows; 0 if this computation excluded them.
    pub overflow_samples: u64,
    /// `(quantile, value_us)` pairs, in the order requested.
    pub values: Vec<(f64, u64)>,
    /// Exact, from the cyclictest footer maxima. Never the overflow bound.
    pub max_us: u64,
    /// Exact, from the cyclictest footer minima.
    pub min_us: u64,
}

impl CyclictestRun {
    /// Percentiles over every sample, overflows included at the histogram bound.
    /// This is the correct computation and the only one used by nr-metrics.
    pub fn percentiles(&self, quantiles: &[f64]) -> Result<Percentiles, PercentileError> {
        self.compute_percentiles(quantiles, true)
    }

    /// Percentiles over binned samples only, ignoring the separately reported overflows.
    /// Present so tests can reproduce the figure the 2026-08-28 README published, and so the
    /// D-23 correction can show both numbers side by side. Do not use it for a new figure.
    pub fn percentiles_excluding_overflows(
        &self,
        quantiles: &[f64],
    ) -> Result<Percentiles, PercentileError> {
        self.compute_percentiles(quantiles, false)
    }

    /// Count of samples at or above `gate_us`, overflows included. This is the form a
    /// published gate figure must use: PLAT-03 requires worst-case latency "brought under
    /// 30 us", so a sample landing exactly on the gate has not met it. Prefer this over
    /// [`Self::samples_above`] for any new figure.
    pub fn samples_at_or_above(&self, gate_us: u64) -> u64 {
        self.count_samples(gate_us, true, true)
    }

    /// Count of samples strictly above `gate_us`, overflows included. Retained because the
    /// 2026-08-28 README used the strict boundary; see [`Self::samples_at_or_above`] for the
    /// form a new figure should use.
    pub fn samples_above(&self, gate_us: u64) -> u64 {
        self.count_samples(gate_us, true, false)
    }

    /// Count of samples strictly above `gate_us`, overflows excluded. Reproduces the
    /// previously published (incorrect) figure exactly; same caveat as
    /// [`Self::percentiles_excluding_overflows`].
    pub fn samples_above_excluding_overflows(&self, gate_us: u64) -> u64 {
        self.count_samples(gate_us, false, false)
    }

    /// Global maximum across per-thread footer maxima. Exact; never the overflow bound.
    pub fn max_us(&self) -> u64 {
        self.max_us.iter().copied().max().unwrap_or(0)
    }

    /// Bin index (microseconds) to sample count, summed across threads.
    pub fn to_bin_table(&self) -> Vec<(u64, u64)> {
        self.bins
            .iter()
            .map(|(&bin_us, counts)| (bin_us, counts.iter().sum()))
            .collect()
    }

    fn compute_percentiles(
        &self,
        quantiles: &[f64],
        include_overflows: bool,
    ) -> Result<Percentiles, PercentileError> {
        let merged = self.merged_histogram(include_overflows)?;
        if merged.is_empty() {
            return Err(PercentileError::EmptyRun);
        }

        let values = quantiles
            .iter()
            .map(|&q| (q, merged.value_at_quantile(q)))
            .collect();
        let overflow_samples = if include_overflows {
            self.overflows.iter().sum()
        } else {
            0
        };

        Ok(Percentiles {
            total_samples: merged.len(),
            binned_samples: self.binned_sample_count(),
            overflow_samples,
            values,
            max_us: self.max_us(),
            min_us: self.min_us.iter().copied().min().unwrap_or(0),
        })
    }

    fn count_samples(&self, gate_us: u64, include_overflows: bool, at_or_above: bool) -> u64 {
        let binned: u64 = self
            .bins
            .iter()
            .filter(|&(&bin_us, _)| {
                if at_or_above {
                    bin_us >= gate_us
                } else {
                    bin_us > gate_us
                }
            })
            .map(|(_, counts)| counts.iter().sum::<u64>())
            .sum();
        if include_overflows {
            binned + self.overflows.iter().sum::<u64>()
        } else {
            binned
        }
    }

    fn binned_sample_count(&self) -> u64 {
        self.bins.values().flat_map(|counts| counts.iter()).sum()
    }

    fn per_thread_histograms(
        &self,
        include_overflows: bool,
    ) -> Result<Vec<Histogram<u64>>, PercentileError> {
        let mut hists: Vec<Histogram<u64>> = (0..self.threads)
            .map(|_| Histogram::<u64>::new(SIGFIG))
            .collect::<Result<_, _>>()?;

        for (&bin_us, counts) in &self.bins {
            for (hist, &count) in hists.iter_mut().zip(counts.iter()) {
                if count > 0 {
                    hist.record_n(bin_us, count)?;
                }
            }
        }

        if include_overflows {
            for (hist, &overflow_count) in hists.iter_mut().zip(self.overflows.iter()) {
                // cyclictest reports overflows separately from the bins. They are samples at or
                // above the histogram bound and they must be recorded, or every percentile,
                // every over-gate count and every total is wrong. This is the exact error
                // corrected under D-23.
                for _ in 0..overflow_count {
                    hist.record(self.overflow_bound_us)?;
                }
            }
        }

        Ok(hists)
    }

    fn merged_histogram(&self, include_overflows: bool) -> Result<Histogram<u64>, PercentileError> {
        let per_thread = self.per_thread_histograms(include_overflows)?;
        let mut merged = Histogram::<u64>::new(SIGFIG)?;
        for hist in &per_thread {
            merged.add(hist)?;
        }
        Ok(merged)
    }
}
