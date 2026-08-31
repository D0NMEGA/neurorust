//! Parses cyclictest `.hist` output (the `-h`/`--histogram` and `-H`/`--histofall` layouts) into
//! a typed [`CyclictestRun`], including the footer block where per-thread overflow counts live.
//!
//! This module only parses. It does not compute any derived statistics; that lives in
//! `crate::percentiles`.
//!
//! Column-count rule, confirmed against real captures from this project's reference rig
//! (`docs/rig/recon-2026-08-31/FINDINGS.md`, "Column counts"): `-h` writes one column per thread
//! everywhere. `-H` adds one extra summary column on the right, but only to the histogram body,
//! `# Max Latencies:`, and `# Histogram Overflows:`; `# Min Latencies:` and `# Avg Latencies:`
//! stay per-thread-only in both layouts. Thread count is therefore derived from
//! `# Min Latencies:` here, not from `# Max Latencies:`, and the two lines that can carry a
//! summary column are truncated back to the thread count wherever they do.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Where the overflow bound (the value overflow samples are recorded at) came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowBoundSource {
    /// The caller passed the `--histogram`/`--histofall` bound explicitly.
    Declared,
    /// Derived as the highest bin present in the file, plus one.
    Derived,
}

/// A single cyclictest `.hist` capture, parsed and typed.
#[derive(Debug, Clone)]
pub struct CyclictestRun {
    pub threads: usize,
    pub summary_column_present: bool,
    /// bin_us -> per-thread counts, `threads` long, in thread order.
    pub bins: BTreeMap<u64, Vec<u64>>,
    /// Per thread.
    pub min_us: Vec<u64>,
    /// Per thread, as cyclictest reported it (integer).
    pub avg_us: Vec<u64>,
    /// Per thread.
    pub max_us: Vec<u64>,
    /// Per thread.
    pub overflows: Vec<u64>,
    /// Per thread.
    pub overflow_cycles: Vec<Vec<u64>>,
    pub overflow_bound_us: u64,
    pub overflow_bound_source: OverflowBoundSource,
}

/// Errors parsing a `.hist` file. A capture file is untrusted input: it may be truncated,
/// hand-edited, or paired with the wrong run, so this module rejects rather than coerces
/// (ASVS V5; see the plan's threat model, T-1-17).
#[derive(Debug, Error)]
pub enum HistError {
    #[error("missing required header line '# Histogram'")]
    MissingHeader,
    #[error("line {line} has fewer columns than the width established earlier in the file")]
    RaggedColumns { line: usize },
    #[error(
        "column count {found} matches neither the thread count ({expected}) nor thread count + 1"
    )]
    ColumnCountMismatch { expected: usize, found: usize },
    #[error("missing footer line: {which}")]
    MissingFooterLine { which: &'static str },
    #[error("failed to parse integer on line {line}: {source}")]
    ParseInt {
        line: usize,
        #[source]
        source: std::num::ParseIntError,
    },
    #[error(
        "thread {thread} declares {declared} overflows but lists {found} overflow cycle numbers"
    )]
    OverflowCycleCountMismatch {
        thread: usize,
        declared: u64,
        found: usize,
    },
    #[error("failed to read {path}: {source}", path = .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse --json output: {0}")]
    JsonDeserialize(#[from] serde_json::Error),
    #[error("--json summary and .hist run disagree on {field}: json={json}, hist={hist}")]
    SummaryDisagreement {
        field: &'static str,
        json: String,
        hist: String,
    },
}

/// Parses the `# <label>:` prefixed line at `footer_lines[idx]`, returning its line number and
/// the text after the prefix.
fn expect_footer_line<'a>(
    footer_lines: &'a [(usize, &'a str)],
    idx: usize,
    prefix: &'static str,
) -> Result<(usize, &'a str), HistError> {
    let (line_no, line) = footer_lines
        .get(idx)
        .ok_or(HistError::MissingFooterLine { which: prefix })?;
    let rest = line
        .strip_prefix(prefix)
        .ok_or(HistError::MissingFooterLine { which: prefix })?;
    Ok((*line_no, rest))
}

/// Splits `rest` on whitespace and parses each token as a `u64`.
fn parse_u64_list(rest: &str, line_no: usize) -> Result<Vec<u64>, HistError> {
    rest.split_whitespace()
        .map(|token| {
            token.parse::<u64>().map_err(|source| HistError::ParseInt {
                line: line_no,
                source,
            })
        })
        .collect()
}

/// A footer line is either exactly `threads` wide (both `-h` and `-H`) or `threads + 1` wide
/// (`-H` only, for the two footer lines that carry a summary column). Drops the summary value
/// when present so every returned `Vec` is `threads` long.
fn drop_optional_summary_column(values: Vec<u64>, threads: usize) -> Result<Vec<u64>, HistError> {
    let found = values.len();
    if found == threads {
        Ok(values)
    } else if found == threads + 1 {
        Ok(values[..threads].to_vec())
    } else {
        Err(HistError::ColumnCountMismatch {
            expected: threads,
            found,
        })
    }
}

/// Parses cyclictest `.hist` output.
///
/// `declared_bound_us` is the `--histogram`/`--histofall` bound the caller invoked cyclictest
/// with, if known. When `None`, the bound is derived as the highest bin present in the file plus
/// one, and [`OverflowBoundSource::Derived`] is recorded so a reader knows it was inferred rather
/// than declared.
pub fn parse_hist(input: &str, declared_bound_us: Option<u64>) -> Result<CyclictestRun, HistError> {
    let mut all_lines = input.lines().enumerate().map(|(i, line)| (i + 1, line));

    let (_, first_line) = all_lines.next().ok_or(HistError::MissingHeader)?;
    if first_line.trim_end() != "# Histogram" {
        return Err(HistError::MissingHeader);
    }

    let mut data_lines: Vec<(usize, &str)> = Vec::new();
    let mut footer_lines: Vec<(usize, &str)> = Vec::new();
    let mut in_footer = false;
    for (line_no, line) in all_lines {
        if line.starts_with('#') {
            in_footer = true;
        }
        if in_footer {
            footer_lines.push((line_no, line));
        } else if !line.trim().is_empty() {
            data_lines.push((line_no, line));
        }
    }

    // Footer first: thread count is authoritative from "# Min Latencies:", the one line that
    // never gains a `-H` summary column.
    let (min_line_no, min_rest) = expect_footer_line(&footer_lines, 0, "# Min Latencies:")?;
    let min_us = parse_u64_list(min_rest, min_line_no)?;
    let threads = min_us.len();

    let (avg_line_no, avg_rest) = expect_footer_line(&footer_lines, 1, "# Avg Latencies:")?;
    let avg_us = parse_u64_list(avg_rest, avg_line_no)?;
    if avg_us.len() != threads {
        return Err(HistError::ColumnCountMismatch {
            expected: threads,
            found: avg_us.len(),
        });
    }

    let (max_line_no, max_rest) = expect_footer_line(&footer_lines, 2, "# Max Latencies:")?;
    let max_us = drop_optional_summary_column(parse_u64_list(max_rest, max_line_no)?, threads)?;

    let (overflow_line_no, overflow_rest) =
        expect_footer_line(&footer_lines, 3, "# Histogram Overflows:")?;
    let overflows =
        drop_optional_summary_column(parse_u64_list(overflow_rest, overflow_line_no)?, threads)?;

    expect_footer_line(&footer_lines, 4, "# Histogram Overflow at cycle number:")?;

    let mut overflow_cycles: Vec<Vec<u64>> = Vec::with_capacity(threads);
    for (thread, &declared) in overflows.iter().enumerate() {
        let prefix = format!("# Thread {thread}:");
        let (line_no, line) = footer_lines
            .get(5 + thread)
            .ok_or(HistError::MissingFooterLine {
                which: "Thread overflow cycle line",
            })?;
        let rest = line
            .strip_prefix(prefix.as_str())
            .ok_or(HistError::MissingFooterLine {
                which: "Thread overflow cycle line",
            })?;
        let cycles = parse_u64_list(rest, *line_no)?;
        if cycles.len() as u64 != declared {
            return Err(HistError::OverflowCycleCountMismatch {
                thread,
                declared,
                found: cycles.len(),
            });
        }
        overflow_cycles.push(cycles);
    }

    // Data body: decide once, from the first data line, whether columns are `threads` or
    // `threads + 1` wide, then hold every later line to that same width.
    let mut bins: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut established_width: Option<usize> = None;
    let mut summary_column_present = false;

    for (line_no, raw_line) in &data_lines {
        let fields: Vec<&str> = raw_line.split_whitespace().collect();
        let width = fields.len();

        match established_width {
            None => {
                let count_columns = width.saturating_sub(1);
                if count_columns == threads {
                    summary_column_present = false;
                } else if count_columns == threads + 1 {
                    summary_column_present = true;
                } else {
                    return Err(HistError::ColumnCountMismatch {
                        expected: threads,
                        found: count_columns,
                    });
                }
                established_width = Some(width);
            }
            Some(expected) if width < expected => {
                return Err(HistError::RaggedColumns { line: *line_no });
            }
            Some(expected) if width != expected => {
                return Err(HistError::ColumnCountMismatch {
                    expected,
                    found: width,
                });
            }
            Some(_) => {}
        }

        let bin_us = fields[0]
            .parse::<u64>()
            .map_err(|source| HistError::ParseInt {
                line: *line_no,
                source,
            })?;
        let mut counts: Vec<u64> = fields[1..]
            .iter()
            .map(|token| {
                token.parse::<u64>().map_err(|source| HistError::ParseInt {
                    line: *line_no,
                    source,
                })
            })
            .collect::<Result<_, _>>()?;
        if summary_column_present {
            counts.truncate(threads);
        }
        bins.insert(bin_us, counts);
    }

    let (overflow_bound_us, overflow_bound_source) = match declared_bound_us {
        Some(bound) => (bound, OverflowBoundSource::Declared),
        None => {
            let highest_bin = bins.keys().next_back().copied().unwrap_or(0);
            (highest_bin + 1, OverflowBoundSource::Derived)
        }
    };

    Ok(CyclictestRun {
        threads,
        summary_column_present,
        bins,
        min_us,
        avg_us,
        max_us,
        overflows,
        overflow_cycles,
        overflow_bound_us,
        overflow_bound_source,
    })
}

/// Reads and parses a `.hist` file from disk. See [`parse_hist`] for the format and the
/// `declared_bound_us` argument.
pub fn parse_hist_file(
    path: &Path,
    declared_bound_us: Option<u64>,
) -> Result<CyclictestRun, HistError> {
    let input = std::fs::read_to_string(path).map_err(|source| HistError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_hist(&input, declared_bound_us)
}
