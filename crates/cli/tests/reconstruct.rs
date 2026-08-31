//! Integration coverage for `nrmeasure reconstruct` (D-16), driven against the real compiled
//! binary via `assert_cmd`.
//!
//! Every test drives against a temporary copy of `measurements/2026-08-28-precision3591/`,
//! never the real directory, so a test can never mutate a published artifact (plan 01-10 is
//! what reconstructs the real directory).

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use nr_manifest::RunManifest;
use serde_json::Value;

const EXCLUSION_REASON: &str = "SSH activity and an active GNOME session during the run";

fn nrmeasure() -> Command {
    Command::cargo_bin("nrmeasure").expect("nrmeasure binary is built")
}

/// Every required flag except `--verdict`, so individual tests add `--verdict` (and, when
/// contaminated, `--exclusion-reason`) themselves.
fn base_reconstruct_cmd(run_dir: &Path) -> Command {
    let mut cmd = nrmeasure();
    cmd.arg("reconstruct")
        .arg(run_dir)
        .args(["--rig-slug", "precision3591"])
        .args(["--run-class", "recon"])
        .args(["--instrument", "investigation"])
        .args(["--utc-start", "2026-08-28T20:36:21Z"])
        .args(["--utc-end", "2026-08-28T20:46:21Z"]);
    cmd
}

/// Copies every file from the real `measurements/2026-08-28-precision3591/` into a fresh temp
/// directory. Returns the `TempDir` guard (keep it alive for the duration of the test) and the
/// path to the copy.
fn copy_measurement_dir() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../measurements/2026-08-28-precision3591");
    let dst = temp.path().join("2026-08-28-precision3591");
    fs::create_dir_all(&dst).expect("mkdir dst");
    for entry in fs::read_dir(&src).expect("read src dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().expect("file has a name");
            fs::copy(&path, dst.join(name)).expect("copy file");
        }
    }
    (temp, dst)
}

fn read_manifest_json(run_dir: &Path) -> Value {
    let text = fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json");
    serde_json::from_str(&text).expect("manifest.json parses as JSON")
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn reconstruct_forces_reconstructed_tier() {
    let (_temp, run_dir) = copy_measurement_dir();
    let output = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "contaminated"])
        .args(["--exclusion-reason", EXCLUSION_REASON])
        .output()
        .expect("run reconstruct");
    assert_success(&output);

    let manifest = read_manifest_json(&run_dir);
    assert_eq!(manifest["provenance_tier"], "reconstructed");

    // No flag exists to set it to harness-generated.
    let help = nrmeasure()
        .args(["reconstruct", "--help"])
        .output()
        .expect("run reconstruct --help");
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(!help_text.to_ascii_lowercase().contains("harness-generated"));
    assert!(!help_text.contains("--provenance"));
}

#[test]
fn reconstruct_records_absent_fields() {
    let rig_txt = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../measurements/2026-08-28-precision3591/RIG.txt");
    let hist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist");

    let temp = tempfile::tempdir().expect("tempdir");
    let run_dir = temp.path().join("2026-08-28-precision3591");
    fs::create_dir_all(&run_dir).expect("mkdir run dir");
    fs::copy(&rig_txt, run_dir.join("RIG.txt")).expect("copy RIG.txt");
    fs::copy(&hist, run_dir.join("cyclictest-rt-isolated-idle-10m.hist")).expect("copy hist");
    // Deliberately no README.md: only a .hist and a RIG.txt.

    let output = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "uncalibrated"])
        .output()
        .expect("run reconstruct");
    assert_success(&output);

    let manifest = read_manifest_json(&run_dir);
    let absent_paths: Vec<&str> = manifest["absent_fields"]
        .as_array()
        .expect("absent_fields is an array")
        .iter()
        .map(|entry| {
            entry["field_path"]
                .as_str()
                .expect("field_path is a string")
        })
        .collect();

    assert!(
        absent_paths.contains(&"preconditions"),
        "absent_fields: {absent_paths:?}"
    );
    assert!(
        absent_paths.iter().any(|p| p.starts_with("interference")),
        "absent_fields: {absent_paths:?}"
    );
    // kernel.is_realtime can only come from the README, which is absent here.
    assert!(
        absent_paths.contains(&"kernel.is_realtime"),
        "absent_fields: {absent_paths:?}"
    );
}

#[test]
fn reconstruct_never_invents() {
    let (_temp, run_dir) = copy_measurement_dir();
    let rig_text = fs::read_to_string(run_dir.join("RIG.txt")).expect("read RIG.txt");
    let readme_text = fs::read_to_string(run_dir.join("README.md")).expect("read README.md");
    let source = format!("{rig_text}\n{readme_text}");

    let output = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "contaminated"])
        .args(["--exclusion-reason", EXCLUSION_REASON])
        .output()
        .expect("run reconstruct");
    assert_success(&output);

    let manifest = read_manifest_json(&run_dir);

    // Every populated field traces to a literal fragment of RIG.txt or the README; none is a
    // constant, a default, or copied from a different run.
    for pointer in [
        "/host/cpu_model",
        "/host/microcode",
        "/host/bios_version",
        "/kernel/release",
        "/os/distro",
        "/network/wired_iface",
    ] {
        let value = manifest
            .pointer(pointer)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{pointer} missing or not a string"));
        assert!(
            source.contains(value),
            "{pointer} = {value:?} is not traceable to RIG.txt or README.md"
        );
    }
}

#[test]
fn reconstruct_parses_rig_txt() {
    let (_temp, run_dir) = copy_measurement_dir();
    let output = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "contaminated"])
        .args(["--exclusion-reason", EXCLUSION_REASON])
        .output()
        .expect("run reconstruct");
    assert_success(&output);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unmapped RIG.txt key: captured_utc"),
        "stderr should report the one deliberately unmapped key: {stderr}"
    );

    let manifest = read_manifest_json(&run_dir);
    assert_eq!(
        manifest["host"]["cpu_model"],
        "Intel(R) Core(TM) Ultra 9 185H"
    );
    assert_eq!(manifest["host"]["microcode"], "0x28");
    assert_eq!(manifest["host"]["p_cores"], "0-11");
    assert_eq!(manifest["host"]["e_cores"], "12-21");
    assert_eq!(manifest["host"]["logical_cpus"], 22);
    assert_eq!(manifest["host"]["physical_cores"], 16);
    assert_eq!(manifest["host"]["sockets"], 1);
    assert_eq!(manifest["host"]["memory_gb"], 30);
    assert_eq!(manifest["kernel"]["release"], "7.0.0-30-generic");
    assert_eq!(manifest["kernel"]["preempt_model"], "PREEMPT_DYNAMIC");
    assert_eq!(manifest["kernel"]["is_realtime"], true);
    assert_eq!(manifest["os"]["distro"], "Ubuntu");
    assert_eq!(manifest["os"]["version"], "26.04.1 LTS");
    assert_eq!(manifest["os"]["session_kind"], "live-usb");
    assert_eq!(manifest["tuning"]["no_turbo"], true);
    assert_eq!(manifest["power"]["ac_online"], true);
    assert_eq!(manifest["power"]["battery_status"], "Charging");
    assert_eq!(manifest["power"]["battery_percent"], 85);
    assert_eq!(manifest["network"]["wired_iface"], "enp0s31f6");
    assert_eq!(manifest["network"]["wired_driver"], "e1000e");
    assert_eq!(manifest["network"]["wired_state"], "down");
    assert_eq!(
        manifest["network"]["ptp_capabilities"],
        "hardware-raw-clock"
    );
    assert_eq!(manifest["network"]["wifi_iface"], "wlp0s20f3");
    assert_eq!(manifest["network"]["wifi_driver"], "iwlwifi");
}

#[test]
fn reconstruct_checksums_artifacts() {
    let (_temp, run_dir) = copy_measurement_dir();
    let output = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "contaminated"])
        .args(["--exclusion-reason", EXCLUSION_REASON])
        .output()
        .expect("run reconstruct");
    assert_success(&output);

    let manifest = read_manifest_json(&run_dir);
    let artifacts = manifest["artifacts"]
        .as_array()
        .expect("artifacts is an array");
    let paths: Vec<&str> = artifacts
        .iter()
        .map(|a| a["path"].as_str().expect("path is a string"))
        .collect();

    for expected in [
        "cyclictest-rt-isolated-idle-10m.hist",
        "hwlatdetect-stock-15m.txt",
        "hwlatdetect-tuned-15m.txt",
        "hwlatdetect-tuned-underload-15m.txt",
        "hwlatdetect-pcore-underload-10m.txt",
        "partition-table-AFTER-resize.txt",
        "partition-table-BEFORE-resize.txt",
    ] {
        assert!(
            paths.contains(&expected),
            "missing artifact {expected}: {paths:?}"
        );
    }
    assert!(
        !paths.contains(&"RIG.txt") && !paths.contains(&"README.md"),
        "RIG.txt and README.md must not themselves be recorded as artifacts: {paths:?}"
    );

    for artifact in artifacts {
        let path = run_dir.join(artifact["path"].as_str().unwrap());
        let bytes = fs::read(&path).expect("read artifact file");
        let expected_hash = blake3::hash(&bytes).to_hex().to_string();
        assert_eq!(
            artifact["blake3"], expected_hash,
            "checksum mismatch for {path:?}"
        );
    }
}

#[test]
fn reconstruct_output_validates() {
    let (_temp, run_dir) = copy_measurement_dir();
    let output = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "contaminated"])
        .args(["--exclusion-reason", EXCLUSION_REASON])
        .output()
        .expect("run reconstruct");
    assert_success(&output);

    let manifest_text =
        fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json");
    let manifest: RunManifest = serde_json::from_str(&manifest_text).expect("manifest.json parses");
    nr_manifest::validate(&run_dir, &manifest).expect("reconstructed manifest must validate");
}

#[test]
fn reconstruct_refuses_to_overwrite() {
    let (_temp, run_dir) = copy_measurement_dir();
    fs::write(run_dir.join("manifest.json"), "{}").expect("seed a manifest.json");

    let output = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "contaminated"])
        .args(["--exclusion-reason", EXCLUSION_REASON])
        .output()
        .expect("run reconstruct");
    assert!(
        !output.status.success(),
        "must refuse to overwrite without --force"
    );

    let output_forced = base_reconstruct_cmd(&run_dir)
        .args(["--verdict", "contaminated"])
        .args(["--exclusion-reason", EXCLUSION_REASON])
        .arg("--force")
        .output()
        .expect("run reconstruct --force");
    assert_success(&output_forced);
}

#[test]
fn reconstruct_requires_verdict_argument() {
    let (_temp, run_dir) = copy_measurement_dir();
    let output = base_reconstruct_cmd(&run_dir)
        .output()
        .expect("run reconstruct");

    assert!(!output.status.success(), "must refuse without --verdict");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_ascii_lowercase().contains("verdict"),
        "stderr should name the missing --verdict argument: {stderr}"
    );
}
