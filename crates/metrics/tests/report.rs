use nr_histogram::hist::{CyclictestRun, parse_hist_file};
use nr_manifest::RunManifest;
use nr_metrics::index::{RunSummary, render_index};
use nr_metrics::report::{Plat03Input, ReportError, render_plat03_verdict, render_run_report};

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

/// render_plat03_verdict with observed max 27 us and firmware floor 22 us renders both "total
/// observed maximum: 27 us against the 30 us gate" and "kernel contribution above the 22 us
/// firmware floor: 5 us". With observed max 45 us and firmware floor 22 us, the verdict line
/// reads "documented limitation" rather than "pass", and both numbers are still shown. A verdict
/// rendered without a firmware floor value returns Err(FirmwareFloorRequired) rather than
/// reporting only the total.
#[test]
fn plat03_report_decomposition() {
    let under_gate = render_plat03_verdict(&Plat03Input {
        observed_max_us: 27,
        gate_us: 30,
        firmware_floor_us: 22,
        firmware_floor_source_run_id: "2026-08-31-precision3591-firmware-floor-001".to_string(),
        p99_us: 9,
        p50_us: 2,
    })
    .expect("a fully populated Plat03Input renders");
    assert!(
        under_gate.contains("total observed maximum: 27 us against the 30 us gate"),
        "got: {under_gate}"
    );
    assert!(
        under_gate.contains("kernel contribution above the 22 us firmware floor: 5 us"),
        "got: {under_gate}"
    );
    assert!(under_gate.contains("verdict: under the 30 us gate"));

    let over_gate = render_plat03_verdict(&Plat03Input {
        observed_max_us: 45,
        gate_us: 30,
        firmware_floor_us: 22,
        firmware_floor_source_run_id: "2026-08-31-precision3591-firmware-floor-001".to_string(),
        p99_us: 9,
        p50_us: 2,
    })
    .expect("a fully populated Plat03Input renders");
    assert!(
        over_gate.contains("total observed maximum: 45 us against the 30 us gate"),
        "got: {over_gate}"
    );
    assert!(
        over_gate.contains("kernel contribution above the 22 us firmware floor: 23 us"),
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

    let missing_floor = render_plat03_verdict(&Plat03Input {
        observed_max_us: 27,
        gate_us: 30,
        firmware_floor_us: 0,
        firmware_floor_source_run_id: String::new(),
        p99_us: 9,
        p50_us: 2,
    });
    assert!(matches!(
        missing_floor,
        Err(ReportError::FirmwareFloorRequired)
    ));
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
            verdict: ContaminationVerdict::Clean,
            p99_us: 9,
            max_us: 27,
            in_series: true,
            reason: None,
        },
        RunSummary {
            date: "2026-08-24".to_string(),
            run_id: "run-contaminated-1".to_string(),
            run_class: RunClass::Weekly,
            instrument_class: InstrumentClass::HeadlineSeries,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            verdict: ContaminationVerdict::Contaminated,
            p99_us: 30,
            max_us: 3800,
            in_series: false,
            reason: Some("CAL IPI count exceeded threshold on cpu6".to_string()),
        },
        RunSummary {
            date: "2026-08-17".to_string(),
            run_id: "run-contaminated-2".to_string(),
            run_class: RunClass::Weekly,
            instrument_class: InstrumentClass::HeadlineSeries,
            provenance_tier: ProvenanceTier::HarnessGenerated,
            verdict: ContaminationVerdict::Contaminated,
            p99_us: 41,
            max_us: 4200,
            in_series: false,
            reason: Some("SSH session active during capture".to_string()),
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
        verdict: ContaminationVerdict::Contaminated,
        p99_us: 30,
        max_us: 3800,
        in_series: false,
        reason: Some("CAL IPI count exceeded threshold on cpu6".to_string()),
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
