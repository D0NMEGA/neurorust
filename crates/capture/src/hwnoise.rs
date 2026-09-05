//! Parses `rtla hwnoise` output into per-CPU rows.
//!
//! `rtla hwnoise` is the D-27 replacement for `hwlatdetect` on this project's isolated
//! cores (see `docs/rig/firmware-floor-rt-vs-stock.md`): given `-c 6-11`, it starts one
//! dedicated osnoise sampling thread per CPU in that list, so every requested CPU gets its
//! own thread instead of sharing a single migrating one. This is the property that answers
//! "did anything actually sample CPU 6" with a value rather than an inference from timing
//! gaps, which is exactly what `hwlatdetect`'s single, unpinned kernel thread could not do
//! under `isolcpus=6-11` (see [`crate::firmware`]).
//!
//! Every numeric time value in this output is in microseconds (the `duration:` header line
//! ends `| time is in us`). The nine columns, per row, are:
//!
//!   - `CPU`: the sampled CPU
//!   - `Period`: a redraw counter (`#N`), not a duration
//!   - `Runtime`: that CPU's own accumulated osnoise-thread execution time, in microseconds,
//!     since the run started. This, not the wall-clock `duration:` header, is the real
//!     per-CPU exposure figure: on the committed probe, `duration:` reads 60 seconds of
//!     wall clock while every CPU's final `Runtime` reads 44250000us (44.25s), because each
//!     osnoise thread is idle for a fixed fraction of every period. See
//!     `docs/rig/recon-2026-09-05/FINDINGS.md`.
//!   - `Noise`: hardware-related noise observed in this period, in microseconds
//!   - `% CPU Aval`: percentage of the period the CPU was available (not consumed by noise)
//!   - `Max Noise`: the largest noise value observed in this period, in microseconds
//!   - `Max Single`: the largest single noise event in this period, in microseconds
//!   - `HW`: count of hardware-noise events in this period
//!   - `NMI`: count of non-maskable interrupts in this period
//!
//! The default (non-`-q`) output redraws the whole table once per second, each redraw
//! preceded by a terminal-reset artifact (a lone `c` character) and its own repeated
//! header. This parser treats every occurrence of a valid header as the start of a fresh
//! block of rows and keeps the LAST row seen for each CPU, so the returned [`HwnoiseRun`]
//! reflects the run's final, cumulative state regardless of whether the input is a full
//! redraw capture (like the committed fixture, taken before `rtla hwnoise --help` documented
//! `-q/--quiet` to this project) or a single quiet-mode summary block.
//!
//! Columns are looked up by name, never by a hardcoded position: a header that does not
//! match the columns this module knows about is rejected with
//! [`HwnoiseError::UnexpectedHeader`] naming the header it found, rather than silently
//! mapping the wrong column. That silent mapping is the exact defect plan 01-04 found (and
//! fixed) in a different hand-rolled parser (`crates/histogram`); this module avoids it by
//! construction rather than by discipline.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HwnoiseError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("no 'rtla hwnoise' column header found in the input")]
    NoHeader,
    #[error("unrecognised rtla hwnoise header: {header:?}")]
    UnexpectedHeader { header: String },
}

/// One CPU's most recent row from an `rtla hwnoise` run. Every numeric time value is in
/// microseconds; see the module documentation for what each field counts.
#[derive(Debug, Clone, PartialEq)]
pub struct HwnoiseRow {
    pub cpu: u32,
    /// A redraw counter (`rtla`'s own `#N`), not a duration.
    pub period: u64,
    /// This CPU's own accumulated osnoise-thread execution time, in microseconds. The real
    /// per-CPU exposure figure; see the module documentation.
    pub runtime_us: u64,
    pub noise_us: u64,
    pub cpu_avail_percent: f64,
    pub max_noise_us: u64,
    pub max_single_us: u64,
    pub hw_count: u64,
    pub nmi_count: u64,
}

/// The whole parsed run: the last row seen for each observed CPU, plus which requested
/// CPUs did and did not appear at all.
#[derive(Debug, Clone, PartialEq)]
pub struct HwnoiseRun {
    /// One entry per CPU that produced at least one row, sorted by CPU number ascending.
    pub rows: Vec<HwnoiseRow>,
    /// The run's own reported wall-clock duration, in seconds, from its last `duration:`
    /// line. NOT per-CPU exposure; see [`HwnoiseRow::runtime_us`] for that.
    pub duration_seconds: Option<f64>,
    pub requested_cpus: Vec<u32>,
    /// Requested CPUs that produced at least one row, in the order given in
    /// `requested_cpus`.
    pub observed_cpus: Vec<u32>,
    /// Requested CPUs that produced no row at all, in the order given in `requested_cpus`.
    pub missing_cpus: Vec<u32>,
}

/// The nine columns this module understands, in the order every real `rtla hwnoise`
/// header this project has captured presents them. Three labels are themselves more than
/// one whitespace-separated word (`% CPU Aval`, `Max Noise`, `Max Single`), so a header is
/// validated as a whole token sequence rather than split naively and zipped index-for-index
/// with a data row's own single-word values.
const EXPECTED_COLUMNS: [&str; 9] = [
    "CPU",
    "Period",
    "Runtime",
    "Noise",
    "% CPU Aval",
    "Max Noise",
    "Max Single",
    "HW",
    "NMI",
];

/// Validates `header` against [`EXPECTED_COLUMNS`] and returns each column name's position
/// among a data row's own whitespace-split values. An unrecognised header - any column
/// added, removed, renamed or reordered - is rejected outright rather than parsed by
/// guessing which position moved where.
fn column_positions(header: &str) -> Result<HashMap<&'static str, usize>, HwnoiseError> {
    let expected_tokens: Vec<&str> = EXPECTED_COLUMNS
        .iter()
        .flat_map(|name| name.split_whitespace())
        .collect();
    let found_tokens: Vec<&str> = header.split_whitespace().collect();

    if found_tokens != expected_tokens {
        return Err(HwnoiseError::UnexpectedHeader {
            header: header.to_string(),
        });
    }

    Ok(EXPECTED_COLUMNS
        .iter()
        .enumerate()
        .map(|(position, &name)| (name, position))
        .collect())
}

/// Reads one data row's columns by name via `positions`. Returns `None` for a line that
/// happens to split into the right number of tokens but is not a real row: this module's
/// scanner is intentionally permissive about what non-data content it skips over, e.g. the
/// embedded `rtla hwnoise --help` text a raw capture like the committed fixture carries
/// after the run itself.
fn parse_row(tokens: &[&str], positions: &HashMap<&'static str, usize>) -> Option<HwnoiseRow> {
    let value = |name: &str| -> Option<&str> { tokens.get(*positions.get(name)?).copied() };

    Some(HwnoiseRow {
        cpu: value("CPU")?.parse().ok()?,
        period: value("Period")?.trim_start_matches('#').parse().ok()?,
        runtime_us: value("Runtime")?.parse().ok()?,
        noise_us: value("Noise")?.parse().ok()?,
        cpu_avail_percent: value("% CPU Aval")?.parse().ok()?,
        max_noise_us: value("Max Noise")?.parse().ok()?,
        max_single_us: value("Max Single")?.parse().ok()?,
        hw_count: value("HW")?.parse().ok()?,
        nmi_count: value("NMI")?.parse().ok()?,
    })
}

/// Parses a `duration:   <days> <HH:MM:SS> | time is in us` line's value (the text after
/// the `duration:` prefix) into total seconds. `None` for anything that does not match this
/// exact shape, which the caller treats as "no duration recorded" rather than a hard error:
/// this line is provenance, not the figure this module exists to produce.
fn parse_duration_seconds(rest: &str) -> Option<f64> {
    let mut tokens = rest.split_whitespace();
    let days: f64 = tokens.next()?.parse().ok()?;
    let mut hms = tokens.next()?.split(':');
    let hours: f64 = hms.next()?.parse().ok()?;
    let minutes: f64 = hms.next()?.parse().ok()?;
    let seconds: f64 = hms.next()?.parse().ok()?;
    Some(days * 86_400.0 + hours * 3_600.0 + minutes * 60.0 + seconds)
}

/// Parses `text` as `rtla hwnoise` output. `requested_cpus` is the `-c` list the run was
/// invoked with; it decides what [`HwnoiseRun::observed_cpus`] and
/// [`HwnoiseRun::missing_cpus`] report and is otherwise not required to match anything in
/// the text.
pub fn parse_hwnoise(text: &str, requested_cpus: &[u32]) -> Result<HwnoiseRun, HwnoiseError> {
    let mut positions: Option<HashMap<&'static str, usize>> = None;
    let mut rows: BTreeMap<u32, HwnoiseRow> = BTreeMap::new();
    let mut duration_seconds: Option<f64> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("duration:") {
            if let Some(seconds) = parse_duration_seconds(rest) {
                duration_seconds = Some(seconds);
            }
            continue;
        }

        if line.split_whitespace().next() == Some("CPU") {
            // Every real header starts with the literal token "CPU"; validate the whole
            // line before trusting anything about its shape.
            positions = Some(column_positions(line)?);
            continue;
        }

        let Some(cols) = positions.as_ref() else {
            // Banner lines, comments and the embedded --help text before any header has
            // been seen at all are not data; nothing to do with them yet.
            continue;
        };

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != EXPECTED_COLUMNS.len() {
            continue;
        }
        if let Some(row) = parse_row(&tokens, cols) {
            rows.insert(row.cpu, row);
        }
    }

    if positions.is_none() {
        return Err(HwnoiseError::NoHeader);
    }

    let rows: Vec<HwnoiseRow> = rows.into_values().collect();
    let observed: BTreeSet<u32> = rows.iter().map(|row| row.cpu).collect();
    let observed_cpus: Vec<u32> = requested_cpus
        .iter()
        .copied()
        .filter(|cpu| observed.contains(cpu))
        .collect();
    let missing_cpus: Vec<u32> = requested_cpus
        .iter()
        .copied()
        .filter(|cpu| !observed.contains(cpu))
        .collect();

    Ok(HwnoiseRun {
        rows,
        duration_seconds,
        requested_cpus: requested_cpus.to_vec(),
        observed_cpus,
        missing_cpus,
    })
}

/// Reads `path` and parses it the same way as [`parse_hwnoise`].
pub fn parse_hwnoise_file(path: &Path, requested_cpus: &[u32]) -> Result<HwnoiseRun, HwnoiseError> {
    let text = std::fs::read_to_string(path).map_err(|source| HwnoiseError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_hwnoise(&text, requested_cpus)
}
