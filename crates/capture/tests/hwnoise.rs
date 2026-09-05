//! Tests for the `rtla hwnoise` parser, driven by the real probe committed in plan 01-22
//! (`docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt`, copied here byte-identical).

use nr_capture::hwnoise::{HwnoiseError, parse_hwnoise, parse_hwnoise_file};

const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/rtla-hwnoise-probe.txt"
);
const FIXTURE: &str = include_str!("fixtures/rtla-hwnoise-probe.txt");

const ISOLATED_CPUS: [u32; 6] = [6, 7, 8, 9, 10, 11];

#[test]
fn parses_the_committed_probe() {
    let run = parse_hwnoise(FIXTURE, &ISOLATED_CPUS).expect("the committed probe must parse");

    assert_eq!(run.rows.len(), 6, "one row per CPU named in -c 6-11");
    let cpus: Vec<u32> = run.rows.iter().map(|row| row.cpu).collect();
    assert_eq!(cpus, vec![6, 7, 8, 9, 10, 11]);

    // The final redraw's own values (period #59), named explicitly in
    // docs/rig/recon-2026-09-05/FINDINGS.md.
    let cpu6 = run.rows.iter().find(|row| row.cpu == 6).unwrap();
    assert_eq!(cpu6.period, 59);
    assert_eq!(cpu6.runtime_us, 44_250_000);
    assert_eq!(cpu6.noise_us, 3);
    assert_eq!(cpu6.max_noise_us, 2);
    assert_eq!(cpu6.max_single_us, 1);
    assert_eq!(cpu6.hw_count, 3);
    assert_eq!(cpu6.nmi_count, 0);

    let cpu9 = run.rows.iter().find(|row| row.cpu == 9).unwrap();
    assert_eq!(
        cpu9.hw_count, 7,
        "CPU 9's final HW count from the real probe"
    );

    for row in &run.rows {
        assert_eq!(
            row.max_single_us, 1,
            "every isolated CPU's final Max Single is 1us"
        );
        assert_eq!(row.nmi_count, 0, "NMI is zero on every isolated CPU");
        assert_eq!(
            row.runtime_us, 44_250_000,
            "every CPU shares one Runtime clock"
        );
    }

    assert_eq!(run.duration_seconds, Some(60.0));
    assert!(run.missing_cpus.is_empty());
    assert_eq!(run.observed_cpus, vec![6, 7, 8, 9, 10, 11]);

    // parse_hwnoise_file must agree with parsing the same text directly.
    let via_path = parse_hwnoise_file(std::path::Path::new(FIXTURE_PATH), &ISOLATED_CPUS)
        .expect("parse_hwnoise_file must parse the same fixture");
    assert_eq!(via_path.rows, run.rows);
}

#[test]
fn reports_every_requested_cpu() {
    // CPU 20 is not in the probe's -c list at all, so it must be reported as missing
    // rather than silently absent.
    let requested = [6, 7, 8, 9, 10, 11, 20];
    let run = parse_hwnoise(FIXTURE, &requested).expect("the committed probe must parse");

    assert_eq!(run.observed_cpus, vec![6, 7, 8, 9, 10, 11]);
    assert_eq!(run.missing_cpus, vec![20]);
}

#[test]
fn rejects_output_with_an_unexpected_header() {
    // Corrupt only the first header occurrence (line 10 of the real probe), leaving every
    // data row untouched; the committed fixture itself is never modified on disk.
    let corrupted = FIXTURE.replacen("NMI", "SMI", 1);

    let err = parse_hwnoise(&corrupted, &ISOLATED_CPUS)
        .expect_err("a header naming an unrecognised column must be rejected");

    match err {
        HwnoiseError::UnexpectedHeader { header } => {
            assert!(
                header.contains("SMI"),
                "error should carry the header it actually found, got: {header:?}"
            );
        }
        other => panic!("expected UnexpectedHeader, got {other:?}"),
    }
}

#[test]
fn parses_a_zero_noise_run() {
    let text = "\
duration:   0 00:00:01 | time is in us
CPU Period       Runtime        Noise  % CPU Aval   Max Noise   Max Single          HW          NMI
  6 #1            750000            0   100.00000           0            0           0            0
  7 #1            750000            0   100.00000           0            0           0            0
";
    let run = parse_hwnoise(text, &[6, 7]).expect("a clean, zero-noise probe must parse");

    assert_eq!(run.rows.len(), 2);
    for row in &run.rows {
        assert_eq!(row.noise_us, 0);
        assert_eq!(row.max_noise_us, 0);
        assert_eq!(row.max_single_us, 0);
        assert_eq!(row.hw_count, 0);
        assert_eq!(row.nmi_count, 0);
        assert!((row.cpu_avail_percent - 100.0).abs() < f64::EPSILON);
    }
    assert!(run.missing_cpus.is_empty());
    assert_eq!(run.duration_seconds, Some(1.0));
}
