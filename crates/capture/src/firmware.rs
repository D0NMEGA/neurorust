//! The standing check `docs/rig/firmware-floor-rt-vs-stock.md` calls for: assert which
//! CPUs a firmware capture's events actually name, not only its maximum.
//!
//! Checking the maximum alone hid the same defect twice. Three D-18 arms requested every
//! CPU and named only CPU 5 in their events; the 2026-08-28 P-core arm requested CPUs 0-11
//! and named only CPU 0. Both would have been caught immediately by asserting the event CPU
//! set against the requested one, which is what [`uncovered_cpus`] exists to make a
//! reusable, tested check rather than a fact restated in prose each time.
//!
//! This module is shared by both firmware instruments this project uses:
//! [`crate::hwnoise`] reports its own per-CPU rows directly (one osnoise thread per
//! requested CPU), while `hwlatdetect`'s single, possibly-unpinned kernel thread must be
//! checked by parsing which CPU each of its threshold-exceeding events actually landed on.

use std::collections::BTreeSet;

/// One threshold-exceeding record from `hwlatdetect`.
///
/// The line format is fixed by the tool:
///
/// ```text
/// ts: 1787951679.284904730, inner:17, outer:1, cpu:0
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct HwlatEvent {
    pub ts: f64,
    pub inner_us: u64,
    pub outer_us: u64,
    pub cpu: u32,
}

/// Parses every `ts: ..., inner:N, outer:N, cpu:N` event line in `text`. Every other line
/// (the summary block above the events, blank lines, anything else) is silently skipped:
/// this project only ever feeds this function real, already-committed `hwlatdetect`
/// captures, whose event lines are the only ones this exact shape describes.
pub fn parse_hwlatdetect_events(text: &str) -> Vec<HwlatEvent> {
    text.lines().filter_map(parse_event_line).collect()
}

fn parse_event_line(line: &str) -> Option<HwlatEvent> {
    let rest = line.trim().strip_prefix("ts:")?;

    let mut ts = None;
    let mut inner_us = None;
    let mut outer_us = None;
    let mut cpu = None;

    for (index, field) in rest.split(',').enumerate() {
        let field = field.trim();
        if index == 0 {
            ts = field.parse::<f64>().ok();
            continue;
        }
        let (key, value) = field.split_once(':')?;
        match key {
            "inner" => inner_us = value.parse().ok(),
            "outer" => outer_us = value.parse().ok(),
            "cpu" => cpu = value.parse().ok(),
            _ => {}
        }
    }

    Some(HwlatEvent {
        ts: ts?,
        inner_us: inner_us?,
        outer_us: outer_us?,
        cpu: cpu?,
    })
}

/// The set of CPUs that produced at least one event, sorted and deduplicated.
pub fn event_cpus(events: &[HwlatEvent]) -> Vec<u32> {
    events
        .iter()
        .map(|event| event.cpu)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The requested CPUs that produced no event (or, for [`crate::hwnoise`], no row) at all.
///
/// A firmware figure is about the CPUs its events name, not the CPUs the tool was pointed
/// at. Three D-18 arms requested every CPU and named only CPU 5; the 2026-08-28 P-core arm
/// requested 0-11 and named only CPU 0. Checking the maximum alone hid both for a week.
/// Assert which CPUs a firmware capture's events name, not only its maximum.
pub fn uncovered_cpus(requested: &[u32], observed: &[u32]) -> Vec<u32> {
    let observed: BTreeSet<u32> = observed.iter().copied().collect();
    requested
        .iter()
        .copied()
        .filter(|cpu| !observed.contains(cpu))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_real_event_line() {
        let events =
            parse_hwlatdetect_events("ts: 1787951679.284904730, inner:17, outer:1, cpu:0\n");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].cpu, 0);
        assert_eq!(events[0].inner_us, 17);
        assert_eq!(events[0].outer_us, 1);
        assert!((events[0].ts - 1_787_951_679.284_904_7).abs() < 1e-6);
    }

    #[test]
    fn ignores_non_event_lines() {
        let text = "Max Latency: 22us\nSamples recorded: 13\nSamples exceeding threshold: 13\n";
        assert!(parse_hwlatdetect_events(text).is_empty());
    }

    #[test]
    fn event_cpus_deduplicates_and_sorts() {
        let events = vec![
            HwlatEvent {
                ts: 1.0,
                inner_us: 1,
                outer_us: 1,
                cpu: 5,
            },
            HwlatEvent {
                ts: 2.0,
                inner_us: 1,
                outer_us: 1,
                cpu: 0,
            },
            HwlatEvent {
                ts: 3.0,
                inner_us: 1,
                outer_us: 1,
                cpu: 5,
            },
        ];
        assert_eq!(event_cpus(&events), vec![0, 5]);
    }

    #[test]
    fn uncovered_cpus_names_every_requested_cpu_missing_from_observed() {
        assert_eq!(
            uncovered_cpus(&[6, 7, 8, 9, 10, 11], &[5]),
            vec![6, 7, 8, 9, 10, 11]
        );
        assert!(uncovered_cpus(&[6, 7], &[6, 7]).is_empty());
    }
}
