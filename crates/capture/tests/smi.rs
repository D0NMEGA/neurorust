//! `nr_capture::smi`: `MSR_SMI_COUNT` parsing and aggregation, tested with no rig (plan
//! 01-21). The live, root-and-msr-module-requiring reader is Linux-gated and untestable
//! here, exactly like `interference::snapshot`; every test below exercises the shared,
//! text-driven parsing and aggregation the live path also calls, which is where the
//! "keep every success, name only the first failure" rule actually lives.

use nr_capture::smi::{parse_rdmsr_value, parse_smi_snapshot};
use nr_manifest::CpuCounter;

/// The probe's own hex reading, `docs/rig/recon-2026-09-05/probe-rdmsr-smi-count.txt`:
/// `cpu 0  0x34 = fa6`, confirmed there to equal 4006 decimal by the same probe's own
/// `rdmsr -d` reading of the same register on cpu 6 (`cpu 6  0x34 = 4006`).
#[test]
fn parses_a_hex_rdmsr_reading() {
    assert_eq!(parse_rdmsr_value("fa6").expect("parses"), 4006);
    assert_eq!(
        parse_rdmsr_value("fa6\n").expect("parses with the trailing newline real stdout has"),
        4006
    );
}

/// A CPU absent from the input yields no counter entry at all, and the reason names it
/// rather than the read being silently treated as a zero SMI count.
#[test]
fn unreadable_register_records_a_reason_not_a_zero() {
    // cpu 7 is deliberately absent: this is what a fixture (or a live read that could not
    // reach that one CPU) looks like when it cannot say anything about it.
    let text = "cpu 6  0x34 = fa6\n";
    let (counters, reason) = parse_smi_snapshot(text, &[6, 7]);

    assert_eq!(
        counters,
        vec![CpuCounter {
            cpu: 6,
            count: 4006
        }]
    );
    assert!(
        !counters.iter().any(|c| c.cpu == 7),
        "cpu 7 must have no counter entry at all: {counters:?}"
    );
    let reason = reason.expect("a missing cpu must record a reason, never a substituted zero");
    assert!(
        reason.contains("cpu 7"),
        "reason should name cpu 7: {reason}"
    );
}
