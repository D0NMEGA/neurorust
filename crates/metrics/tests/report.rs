use nr_histogram::hist::{CyclictestRun, parse_hist_file};
use nr_manifest::{
    AdmissionDisposition, AdmissionEvidence, AdmissionEvidenceSource, CpuExposure, FirmwareScreen,
    RunManifest, SeriesAdmission,
};
use nr_metrics::index::{RunOutcome, RunSummary, render_index};
use nr_metrics::report::{
    FirmwareObservation, Plat03Input, render_plat03_verdict, render_run_report,
};

/// A hand-built manifest, adapted from the reviewed worked example
/// (`crates/manifest/tests/fixtures/minimal-manifest.json`), covering every RunManifest field
/// with real project data (rig, kernel, tuning) rather than placeholders.
const SAMPLE_MANIFEST_JSON: &str = r##"{
  "schema_version": 1,
  "provenance_tier": "harness-generated",
  "run_id": "2026-08-31-precision3591-headline-test-001",
  "run_class": "headline",
  "instrument_class": "headline-series",
  "utc_start": "2026-08-31T06:00:00Z",
  "utc_end": "2026-08-31T06:10:00Z",
  "harness": {
    "version": "0.1.0",
    "git_sha": "0000000000000000000000000000000000000000",
    "git_dirty": false
  },
  "host": {
    "rig_slug": "precision3591",
    "system_vendor": "Dell Inc.",
    "system_model": "Precision 3591",
    "bios_version": "1.23.0",
    "bios_release_date": "2026-04-24",
    "cpu_model": "Intel(R) Core(TM) Ultra 9 185H",
    "microcode": "0x28",
    "logical_cpus": 22,
    "physical_cores": 16,
    "sockets": 1,
    "p_cores": "0-11",
    "e_cores": "12-21",
    "memory_gb": 30
  },
  "kernel": {
    "release": "7.0.0-30-realtime",
    "version_string": "#30-Ubuntu SMP PREEMPT_RT Fri Jul 31 18:22:54 UTC 2026",
    "is_realtime": true,
    "preempt_model": "PREEMPT_RT",
    "cmdline": "BOOT_IMAGE=/vmlinuz root=[redacted] ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 resume=[redacted]",
    "isolcpus": "6-11",
    "nohz_full": "6-11",
    "rcu_nocbs": "6-11",
    "irqaffinity": null
  },
  "os": {
    "distro": "Ubuntu",
    "version": "26.04.1 LTS",
    "session_kind": "installed",
    "systemd_default_target": "multi-user.target",
    "display_manager_active": false
  },
  "tuning": {
    "per_cpu_governor": [
      { "cpu": 6, "governor": "performance" },
      { "cpu": 7, "governor": "performance" }
    ],
    "no_turbo": true,
    "cstates": [
      { "cpu": 6, "name": "C1E", "disabled": false },
      { "cpu": 6, "name": "C6", "disabled": true }
    ],
    "rt_tuning_service": {
      "unit": "rt-tuning.service",
      "active_state": "active",
      "sub_state": "exited",
      "unit_file_state": "enabled"
    }
  },
  "power": {
    "ac_online": true,
    "battery_status": "Charging",
    "battery_percent": 85,
    "thermal_zones": [
      { "name": "x86_pkg_temp", "temp_c_start": 42.0, "temp_c_end": 45.5 }
    ],
    "package_temp_c_max": 45.5
  },
  "network": {
    "wired_iface": "enp0s31f6",
    "wired_driver": "e1000e",
    "wired_state": "down",
    "ptp_capabilities": "hardware-raw-clock",
    "wifi_iface": "wlp0s20f3",
    "wifi_driver": "iwlwifi",
    "wifi_state": "up"
  },
  "preconditions": [
    { "check": "no-active-ssh-sessions", "status": "pass", "observed": "0 sessions", "expected": "0 sessions" },
    { "check": "systemd-default-target-is-multi-user", "status": "pass", "observed": "multi-user.target", "expected": "multi-user.target" },
    { "check": "display-manager-inactive", "status": "pass", "observed": "inactive", "expected": "inactive" },
    { "check": "no-graphical-session", "status": "pass", "observed": "0 sessions", "expected": "0 sessions" },
    { "check": "governor-is-performance-on-all-cpus", "status": "pass", "observed": "performance", "expected": "performance" },
    { "check": "no-turbo-enabled", "status": "pass", "observed": "1", "expected": "1" },
    { "check": "deep-cstates-disabled", "status": "pass", "observed": "C6=off C10=off", "expected": "C6=off C10=off" },
    { "check": "isolcpus-covers-target-cpus", "status": "pass", "observed": "6-11", "expected": "6-11" },
    { "check": "kernel-is-realtime", "status": "pass", "observed": "1", "expected": "1" },
    { "check": "rt-tuning-service-active", "status": "pass", "observed": "active", "expected": "active" },
    { "check": "on-ac-power", "status": "pass", "observed": "AC=1", "expected": "AC=1" },
    { "check": "thermal-headroom-at-start", "status": "pass", "observed": "42.0 C", "expected": "< 80.0 C" },
    { "check": "no-package-manager-activity", "status": "pass", "observed": "no lock held", "expected": "no lock held" },
    { "check": "tracers-quiescent", "status": "pass", "observed": "nop", "expected": "nop" }
  ],
  "interference": {
    "before": {
      "isolated_cpus": [6, 7, 8, 9, 10, 11],
      "cal_ipis": [{ "cpu": 6, "count": 0 }],
      "tlb_ipis": [{ "cpu": 6, "count": 0 }],
      "context_switches": [{ "cpu": 6, "count": 100 }],
      "irqs": [{ "cpu": 6, "count": 5 }]
    },
    "after": {
      "isolated_cpus": [6, 7, 8, 9, 10, 11],
      "cal_ipis": [{ "cpu": 6, "count": 0 }],
      "tlb_ipis": [{ "cpu": 6, "count": 0 }],
      "context_switches": [{ "cpu": 6, "count": 102 }],
      "irqs": [{ "cpu": 6, "count": 5 }]
    },
    "delta": {
      "cal_ipis": [{ "cpu": 6, "count": 0 }],
      "tlb_ipis": [{ "cpu": 6, "count": 0 }],
      "context_switches": [{ "cpu": 6, "count": 2 }],
      "irqs": [{ "cpu": 6, "count": 0 }]
    },
    "tail_metrics": {
      "tail_excursion_ratio": 3.0,
      "thread_max_spread": 0.6,
      "overflow_rate_per_s": 0.0
    },
    "thresholds_provisional": false,
    "verdict": "clean"
  },
  "tools": [
    {
      "name": "cyclictest",
      "version": "2.80",
      "argv": [
        "cyclictest",
        "--mainaffinity=0,1",
        "--affinity=6-11",
        "--threads",
        "--mlockall",
        "--priority=99",
        "--interval=200",
        "--histofall=400",
        "--histfile=cyclictest-rt-isolated-idle-10m.hist"
      ],
      "exit_code": 0
    }
  ],
  "artifacts": [
    {
      "path": "cyclictest-rt-isolated-idle-10m.hist",
      "bytes": 23399,
      "blake3": "9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c9c",
      "kind": "cyclictest-hist",
      "stored": "in-repo"
    }
  ],
  "absent_fields": [],
  "excluded_from_series": false,
  "exclusion_reason": null,
  "notes": null
}"##;

fn sample_manifest() -> RunManifest {
    serde_json::from_str(SAMPLE_MANIFEST_JSON).expect("the hand-built fixture manifest parses")
}

/// `sample_manifest()` with `firmware_screens` set to a single screen, for the tests below that
/// need a manifest carrying one. `SAMPLE_MANIFEST_JSON` predates `FirmwareScreen`/`SmiCounts`
/// entirely (no such keys in the JSON above), so `firmware_screens` and `smi_counts` deserialize
/// to their schema defaults (empty/`None`) before this helper sets the one field each test
/// actually varies.
fn manifest_with_firmware_screen(screen: FirmwareScreen) -> RunManifest {
    let mut manifest = sample_manifest();
    manifest.firmware_screens = vec![screen];
    manifest
}

/// `sample_manifest()` with `series_admission` set, for the two tests below that need a
/// manifest carrying one. `SAMPLE_MANIFEST_JSON` predates the field entirely (no such key in
/// the JSON above), so it deserializes to `None` before this helper sets it (D-28).
fn manifest_with_series_admission(admission: SeriesAdmission) -> RunManifest {
    let mut manifest = sample_manifest();
    manifest.series_admission = Some(admission);
    manifest
}

/// A realistic `rtla-hwnoise` screen: one osnoise sampling thread per requested cpu, matching
/// the committed 2026-09-05 probe plan 01-20 parses.
fn hwnoise_screen(requested_cpus: Vec<u32>, observed_cpus: Vec<u32>) -> FirmwareScreen {
    FirmwareScreen {
        instrument: "rtla-hwnoise".to_string(),
        tool_version: "7.0.12".to_string(),
        argv: vec![
            "rtla".to_string(),
            "hwnoise".to_string(),
            "-c".to_string(),
            "6-11".to_string(),
        ],
        requested_cpus,
        per_cpu_exposure_seconds: observed_cpus
            .iter()
            .map(|&cpu| CpuExposure {
                cpu,
                seconds: 44.25,
            })
            .collect(),
        observed_cpus,
        max_us: Some(1),
        max_population: "the largest Max Single value across the observed cpus".to_string(),
        events_recorded: 18,
    }
}

/// A realistic `hwlatdetect` screen: one non-migrating tracer thread, so per-CPU exposure has
/// no basis and is reported as empty, exactly like every committed `hwlatdetect*.txt` capture.
fn hwlatdetect_screen() -> FirmwareScreen {
    FirmwareScreen {
        instrument: "hwlatdetect".to_string(),
        tool_version: "2.80".to_string(),
        argv: vec!["hwlatdetect".to_string(), "--duration=900".to_string()],
        requested_cpus: vec![6, 7, 8, 9, 10, 11],
        observed_cpus: vec![5],
        per_cpu_exposure_seconds: vec![],
        max_us: Some(22),
        max_population: "the observed maximum across all reported events".to_string(),
        events_recorded: 13,
    }
}

/// The committed 2026-08-28 fixture: 6 threads, 17,994,956 binned samples, 888 overflows,
/// 3,806 us maximum. The same fixture nr-histogram's own test suite is built against.
fn sample_run() -> CyclictestRun {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist"
    );
    parse_hist_file(std::path::Path::new(path), None)
        .expect("the committed 2026-08-28 fixture parses")
}

/// PLAT-03 reports the observed maximum against the gate and, separately, any independent
/// hwlatdetect observation. It does NOT subtract one from the other.
///
/// The subtraction this replaces claimed to yield "the largest the kernel's contribution could
/// be". It does not. A 40 us scheduling maximum caused entirely by kernel activity, with no
/// firmware interruption during that event, minus a 22 us hwlat maximum observed in a different
/// run on different CPUs under a different load, yields 18 us and understates the real kernel
/// contribution by half. An observed hardware gap is neither a fixed delay charged to every
/// wakeup nor a guaranteed floor under the worst scheduling event, and the two figures come
/// from different instruments measuring different things. See 01-EXTERNAL-AUDIT.md finding 1.
#[test]
fn plat03_reports_both_figures_without_subtracting_them() {
    let under_gate = render_plat03_verdict(&Plat03Input {
        observed_max_us: 27,
        gate_us: 30,
        firmware: Some(FirmwareObservation {
            instrument: "hwlatdetect".to_string(),
            max_us: Some(22),
            max_population: "the observed maximum across all reported events".to_string(),
            observed_cpus: vec![0],
            source_run_id: "2026-08-31-precision3591-firmware-floor-001".to_string(),
            conditions: "P-cores 0-11, 22 logical CPUs loaded, package 93 to 95 C".to_string(),
        }),
        p99_us: 9,
        p50_us: 2,
    })
    .expect("renders");
    assert!(
        under_gate.contains("total observed maximum: 27 us against the 30 us gate"),
        "got: {under_gate}"
    );
    assert!(
        under_gate.contains("verdict: under the 30 us gate"),
        "got: {under_gate}"
    );

    // The hwlat figure appears, attributed and condition-qualified, but never subtracted.
    assert!(under_gate.contains("22 us"), "got: {under_gate}");
    assert!(
        under_gate.contains("2026-08-31-precision3591-firmware-floor-001"),
        "got: {under_gate}"
    );
    assert!(
        under_gate.contains("not subtracted"),
        "the report must say plainly that the two are not combined: {under_gate}"
    );
    assert!(
        !under_gate.contains("kernel contribution"),
        "the invalid decomposition must be gone: {under_gate}"
    );
    // 27 - 22 = 5 must not appear as a derived quantity.
    assert!(!under_gate.contains(": 5 us"), "got: {under_gate}");

    let over_gate = render_plat03_verdict(&Plat03Input {
        observed_max_us: 45,
        gate_us: 30,
        firmware: None,
        p99_us: 9,
        p50_us: 2,
    })
    .expect("a verdict renders without any firmware observation");
    assert!(
        over_gate.contains("total observed maximum: 45 us against the 30 us gate"),
        "got: {over_gate}"
    );
    assert!(
        over_gate.contains("verdict: documented limitation"),
        "got: {over_gate}"
    );
    assert!(
        !over_gate.contains("under the 30 us gate"),
        "got: {over_gate}"
    );
    assert!(
        over_gate.contains("no paired firmware observation"),
        "a missing observation is stated, not silently omitted: {over_gate}"
    );
}

/// A maximum exactly equal to the gate is NOT under it. The project counts samples at or above
/// the boundary (`samples_at_or_above`, settled 2026-08-31), so 30 us against a 30 us gate is a
/// miss. The previous `<=` made exactly-30 read as a pass.
#[test]
fn plat03_gate_boundary_is_at_or_above() {
    let exactly_at_gate = render_plat03_verdict(&Plat03Input {
        observed_max_us: 30,
        gate_us: 30,
        firmware: None,
        p99_us: 9,
        p50_us: 2,
    })
    .expect("renders");
    assert!(
        !exactly_at_gate.contains("under the 30 us gate"),
        "exactly 30 us must not report as under a 30 us gate: {exactly_at_gate}"
    );
    assert!(
        exactly_at_gate.contains("verdict: documented limitation"),
        "got: {exactly_at_gate}"
    );

    let just_under = render_plat03_verdict(&Plat03Input {
        observed_max_us: 29,
        gate_us: 30,
        firmware: None,
        p99_us: 9,
        p50_us: 2,
    })
    .expect("renders");
    assert!(
        just_under.contains("verdict: under the 30 us gate"),
        "got: {just_under}"
    );
}

/// render_run_report emits a fixed-width ASCII histogram with a log scale on the count axis and
/// no non-ASCII characters.
#[test]
fn report_renders_ascii_histogram() {
    let report = render_run_report(&sample_manifest(), &sample_run()).expect("renders");

    assert!(report.contains("## Distribution"));
    assert!(report.is_ascii(), "report must be ASCII only");
    assert!(
        report.contains('#'),
        "expected at least one ASCII histogram bar character"
    );
}

/// The rendered report contains the sentence explaining that overflow samples are recorded at
/// the histogram bound and that high percentiles are therefore conservative.
#[test]
fn report_states_overflow_convention() {
    let report = render_run_report(&sample_manifest(), &sample_run()).expect("renders");

    assert!(report.contains("recorded at the histogram bound"));
    assert!(report.contains("therefore conservative"));
}

/// A rendered report for a manifest carrying `series_admission` shows the admitted verdict
/// and one table row per evidence source consulted (D-28), as two statements separate from
/// the D-24 contamination verdict.
#[test]
fn series_admission_section_shows_admitted_and_every_evidence_source() {
    let admission = SeriesAdmission {
        admitted: true,
        exclusions: vec![],
        evidence: vec![
            AdmissionEvidence {
                source: AdmissionEvidenceSource::Preconditions,
                observed: "14 pass, 0 fail, 0 not-applicable, 0 unavailable".to_string(),
                disposition: AdmissionDisposition::Clean,
            },
            AdmissionEvidence {
                source: AdmissionEvidenceSource::ToolExitCodes,
                observed: "cyclictest=0".to_string(),
                disposition: AdmissionDisposition::Clean,
            },
        ],
    };
    let manifest = manifest_with_series_admission(admission);
    let report = render_run_report(&manifest, &sample_run()).expect("renders");

    assert!(report.contains("## Series admission"), "got:\n{report}");
    assert!(report.contains("admitted: yes"), "got:\n{report}");
    assert!(
        report.contains(
            "| preconditions | 14 pass, 0 fail, 0 not-applicable, 0 unavailable | clean |"
        ),
        "expected a table row for the preconditions evidence source: {report}"
    );
    assert!(
        report.contains("| tool-exit-codes | cyclictest=0 | clean |"),
        "expected a table row for the tool exit codes evidence source: {report}"
    );
}

/// A rendered report for a manifest with no `series_admission` (every manifest committed
/// before plan 01-24) states plainly that it predates the record, and synthesises no verdict
/// from `excluded_from_series`: a reconstructed verdict is not an observed one (D-16).
#[test]
fn series_admission_section_states_absence_plainly() {
    let report = render_run_report(&sample_manifest(), &sample_run()).expect("renders");

    assert!(report.contains("## Series admission"), "got:\n{report}");
    assert!(
        report.contains("not recorded: this manifest predates the admission record (D-28)"),
        "got:\n{report}"
    );
    assert!(
        !report.contains("admitted: yes") && !report.contains("admitted: no"),
        "no verdict may be synthesised for a manifest that predates the record: {report}"
    );
}

/// The report contains a header line naming the manifest blake3 it was generated from.
#[test]
fn report_is_generated_not_authored() {
    let report = render_run_report(&sample_manifest(), &sample_run()).expect("renders");

    assert!(
        report
            .lines()
            .any(|line| line.starts_with("manifest blake3: ")),
        "expected a 'manifest blake3: ' header line, got:\n{report}"
    );
}

/// render_index over three run summaries, one Clean and two Contaminated, emits three rows; the
/// contaminated rows carry their exclusion reason in the reason column.
#[test]
fn losing_config_rendered() {
    use nr_manifest::{ContaminationVerdict, InstrumentClass, ProvenanceTier, RunClass};

    let summaries = vec![
        RunSummary {
            date: "2026-08-31".to_string(),
            run_id: "run-clean".to_string(),
            run_class: RunClass::Weekly,
            instrument_class: InstrumentClass::HeadlineSeries,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            verdict: Some(ContaminationVerdict::Clean),
            p99_us: Some(9),
            max_us: Some(27),
            in_series: true,
            reason: None,
            outcome: RunOutcome::Measured,
        },
        RunSummary {
            date: "2026-08-24".to_string(),
            run_id: "run-contaminated-1".to_string(),
            run_class: RunClass::Weekly,
            instrument_class: InstrumentClass::HeadlineSeries,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            verdict: Some(ContaminationVerdict::Contaminated),
            p99_us: Some(30),
            max_us: Some(3800),
            in_series: false,
            reason: Some("CAL IPI count exceeded threshold on cpu6".to_string()),
            outcome: RunOutcome::Measured,
        },
        RunSummary {
            date: "2026-08-17".to_string(),
            run_id: "run-contaminated-2".to_string(),
            run_class: RunClass::Weekly,
            instrument_class: InstrumentClass::HeadlineSeries,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            verdict: Some(ContaminationVerdict::Contaminated),
            p99_us: Some(41),
            max_us: Some(4200),
            in_series: false,
            reason: Some("SSH session active during capture".to_string()),
            outcome: RunOutcome::Measured,
        },
    ];

    let index = render_index(&summaries);
    let row_count = index
        .lines()
        .filter(|line| line.starts_with("| 2026-"))
        .count();
    assert_eq!(row_count, 3, "expected three rows, got:\n{index}");
    assert!(index.contains("CAL IPI count exceeded threshold on cpu6"));
    assert!(index.contains("SSH session active during capture"));
}

/// render_index over a summary marked excluded_from_series still emits its row.
#[test]
fn index_never_omits() {
    use nr_manifest::{ContaminationVerdict, InstrumentClass, ProvenanceTier, RunClass};

    let excluded = RunSummary {
        date: "2026-08-24".to_string(),
        run_id: "run-excluded".to_string(),
        run_class: RunClass::Weekly,
        instrument_class: InstrumentClass::HeadlineSeries,
        provenance_tier: ProvenanceTier::HarnessGenerated,
        verdict: Some(ContaminationVerdict::Contaminated),
        p99_us: Some(30),
        max_us: Some(3800),
        in_series: false,
        reason: Some("CAL IPI count exceeded threshold on cpu6".to_string()),
        outcome: RunOutcome::Measured,
    };

    let index = render_index(&[excluded]);
    assert!(index.contains("run-excluded"));
    assert!(index.contains("| no |"));
}

/// Snapshots one full headline report rendered from the hand-built manifest plus the committed
/// 2026-08-28 fixture, so any change to the published shape shows up as a reviewable diff.
#[test]
fn headline_report_snapshot() {
    let report = render_run_report(&sample_manifest(), &sample_run()).expect("renders");
    insta::assert_snapshot!("headline_report", report);
}

/// Every line in a rendered report's firmware section that opens with the bullet marker this
/// module writes the three finding-3 statements with.
fn extract_caveats(report: &str) -> Vec<&str> {
    report
        .lines()
        .filter(|line| line.starts_with("- "))
        .collect()
}

/// A report rendered from a manifest carrying an `rtla-hwnoise` screen states all three
/// finding-3 statements (`01-EXTERNAL-AUDIT.md` finding 3), written for that instrument.
#[test]
fn firmware_section_states_all_three_caveats() {
    let manifest = manifest_with_firmware_screen(hwnoise_screen(
        vec![6, 7, 8, 9, 10, 11],
        vec![6, 7, 8, 9, 10, 11],
    ));
    let report = render_run_report(&manifest, &sample_run()).expect("renders");

    assert!(report.contains("## Firmware screen"), "got:\n{report}");
    assert!(
        report.contains("not uniquely identified SMIs"),
        "statement 1 (execution gaps are not uniquely identified SMIs) is missing: {report}"
    );
    assert!(
        report.contains("one osnoise sampling thread per CPU"),
        "statement 2 (per-CPU figures come from one sampling thread each) is missing: {report}"
    );
    assert!(
        report.contains("per-CPU exposure above is what the tool reports"),
        "statement 3 (exposure is not a divided wall-clock duration) is missing: {report}"
    );
}

/// The three statements rendered for `hwlatdetect` and for `rtla-hwnoise` are not identical
/// strings: copying one instrument's caveats onto the other would either understate or
/// overstate what its own output actually supports. The hwnoise exposure sentence specifically
/// names one sampling thread per CPU, the exact difference from hwlatdetect's single
/// non-migrating tracer thread on this rig.
#[test]
fn firmware_caveats_differ_by_instrument() {
    let hwlat_report = render_run_report(
        &manifest_with_firmware_screen(hwlatdetect_screen()),
        &sample_run(),
    )
    .expect("renders");
    let hwnoise_report = render_run_report(
        &manifest_with_firmware_screen(hwnoise_screen(
            vec![6, 7, 8, 9, 10, 11],
            vec![6, 7, 8, 9, 10, 11],
        )),
        &sample_run(),
    )
    .expect("renders");

    let hwlat_caveats = extract_caveats(&hwlat_report);
    let hwnoise_caveats = extract_caveats(&hwnoise_report);
    assert_eq!(hwlat_caveats.len(), 3, "got:\n{hwlat_report}");
    assert_eq!(hwnoise_caveats.len(), 3, "got:\n{hwnoise_report}");
    assert_ne!(
        hwlat_caveats, hwnoise_caveats,
        "the two instruments' three statements must not be the same strings"
    );
    assert!(
        hwnoise_caveats
            .iter()
            .any(|line| line.contains("one osnoise sampling thread per CPU")),
        "the hwnoise exposure sentence must name one sampling thread per cpu: {hwnoise_caveats:?}"
    );
    assert!(
        hwlat_caveats
            .iter()
            .any(|line| line.contains("wall-clock duration is not per-CPU exposure")),
        "the hwlatdetect exposure sentence must state that duration is not exposure: \
         {hwlat_caveats:?}"
    );
}

/// The section prints both the requested and the observed CPU lists, and warns when a
/// requested CPU produced nothing: the exact shape of defect that cost three D-18 arms before
/// plan 01-20's coverage test caught it in code.
#[test]
fn firmware_section_names_observed_cpus() {
    let manifest = manifest_with_firmware_screen(hwnoise_screen(
        vec![6, 7, 8, 9, 10, 11, 12],
        vec![6, 7, 8, 9, 10, 11],
    ));
    let report = render_run_report(&manifest, &sample_run()).expect("renders");

    assert!(
        report.contains("requested cpus: 6,7,8,9,10,11,12"),
        "got:\n{report}"
    );
    assert!(
        report.contains("observed cpus: 6,7,8,9,10,11"),
        "got:\n{report}"
    );
    assert!(
        report.contains("warning: requested but not observed: 12"),
        "expected a warning naming the uncovered cpu 12: {report}"
    );
}

/// A cyclictest-only manifest (no firmware screen; `sample_manifest()`'s underlying JSON
/// predates `FirmwareScreen` entirely) renders no firmware section at all, not an empty one
/// with just a heading.
#[test]
fn report_omits_the_section_when_no_screen_ran() {
    let report = render_run_report(&sample_manifest(), &sample_run()).expect("renders");
    assert!(
        !report.contains("Firmware screen"),
        "a cyclictest-only run must render no firmware section: {report}"
    );
}

/// `render_plat03_verdict` prints the instrument name alongside its observation, and still
/// states plainly that the two figures are not combined (finding 1, `01-EXTERNAL-AUDIT.md`).
#[test]
fn plat03_observation_names_its_instrument() {
    let report = render_plat03_verdict(&Plat03Input {
        observed_max_us: 27,
        gate_us: 30,
        firmware: Some(FirmwareObservation {
            instrument: "rtla-hwnoise".to_string(),
            max_us: Some(1),
            max_population: "the largest Max Single value across the observed cpus".to_string(),
            observed_cpus: vec![6, 7, 8, 9, 10, 11],
            source_run_id: "2026-09-05-precision3591-screen".to_string(),
            conditions: "cpus 6-11, 22 logical cpus loaded, package 85 to 87 C".to_string(),
        }),
        p99_us: 9,
        p50_us: 2,
    })
    .expect("renders");

    assert!(
        report.contains("independent rtla-hwnoise maximum"),
        "the instrument name must appear with the observation: {report}"
    );
    assert!(report.contains("1 us"), "got: {report}");
    assert!(
        report.contains("not subtracted"),
        "the report must still say the two figures are not combined: {report}"
    );
}
