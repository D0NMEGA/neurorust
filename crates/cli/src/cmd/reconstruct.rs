//! `nrmeasure reconstruct`: build a manifest for a capture taken before the harness existed
//! (D-16).
//!
//! Three rules, each enforced:
//!
//!   1. `provenance_tier` is hardcoded to [`ProvenanceTier::Reconstructed`]. There is no flag
//!      to change it (RESEARCH.md pitfall 6).
//!   2. Every manifest field that cannot be traced to a value in `RIG.txt`, the README, or the
//!      run directory's own files is left at its zero value and recorded in `absent_fields`
//!      with a reason. Never copy a value from a different run; never substitute a plausible
//!      default.
//!   3. `--verdict` is required with no default, because a contamination verdict needs before
//!      and after interference snapshots a pre-harness capture does not have.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Args as ClapArgs, ValueEnum};
use nr_manifest::{
    AbsentField, ArtifactKind, ArtifactRecord, ContaminationVerdict, CpuGovernor, CstateSetting,
    GitShaSource, HarnessInfo, HostInfo, InterferenceDelta, InterferenceSnapshot,
    InterferenceSnapshotPair, KernelInfo, NetworkInfo, OsInfo, PowerInfo, PreconditionResult,
    ProvenanceTier, RunManifest, ServiceState, SessionKind, StorageLocation, ThermalZone,
    ToolInvocation, TuningInfo,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cmd::run::{InstrumentClassArg, RunClassArg};

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum VerdictArg {
    Clean,
    Contaminated,
    Uncalibrated,
}

impl From<VerdictArg> for ContaminationVerdict {
    fn from(value: VerdictArg) -> Self {
        match value {
            VerdictArg::Clean => ContaminationVerdict::Clean,
            VerdictArg::Contaminated => ContaminationVerdict::Contaminated,
            VerdictArg::Uncalibrated => ContaminationVerdict::Uncalibrated,
        }
    }
}

fn parse_rfc3339(s: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|err| err.to_string())
}

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// The existing capture directory to build a reconstructed manifest for.
    pub run_dir: PathBuf,

    #[arg(long = "rig-slug")]
    pub rig_slug: String,

    #[arg(long = "run-class", value_enum)]
    pub run_class: RunClassArg,

    #[arg(long = "instrument", value_enum)]
    pub instrument: InstrumentClassArg,

    /// Defaults to `<run-dir>/RIG.txt`.
    #[arg(long = "rig-txt")]
    pub rig_txt: Option<PathBuf>,

    /// Defaults to `<run-dir>/README.md`.
    #[arg(long = "readme")]
    pub readme: Option<PathBuf>,

    /// A contamination verdict cannot be computed without before/after interference
    /// snapshots, which a pre-harness capture does not have; this must be stated explicitly.
    #[arg(long = "verdict", value_enum)]
    pub verdict: VerdictArg,

    /// Required when `--verdict contaminated` is given.
    #[arg(long = "exclusion-reason")]
    pub exclusion_reason: Option<String>,

    #[arg(long = "utc-start", value_parser = parse_rfc3339)]
    pub utc_start: OffsetDateTime,

    #[arg(long = "utc-end", value_parser = parse_rfc3339)]
    pub utc_end: OffsetDateTime,

    #[arg(long)]
    pub note: Option<String>,

    /// Overwrite an existing manifest.json. Without this flag, reconstruct refuses to touch a
    /// run directory that already has one (D-06: the harness asserts and refuses, never
    /// mutates).
    #[arg(long)]
    pub force: bool,
}

/// RIG.txt keys this command knows how to map onto a manifest field. `captured_utc` is
/// deliberately absent from this list: the plan calls for it to be reported as unmapped, the
/// same as a genuinely unrecognised key, since it has no corresponding manifest field.
const MAPPED_RIG_KEYS: &[&str] = &[
    "system",
    "bios",
    "cpu",
    "topology",
    "p_cores",
    "e_cores",
    "microcode",
    "kernel",
    "os",
    "session",
    "memory_gb",
    "cmdline",
    "governor",
    "no_turbo",
    "cstates",
    "power",
    "nic_wired",
    "nic_ptp",
    "nic_wifi",
];

pub fn run(args: Args) -> anyhow::Result<i32> {
    let manifest_path = args.run_dir.join("manifest.json");
    if manifest_path.exists() && !args.force {
        anyhow::bail!(
            "{} already exists; pass --force to overwrite",
            manifest_path.display()
        );
    }

    if matches!(args.verdict, VerdictArg::Contaminated) && args.exclusion_reason.is_none() {
        anyhow::bail!(
            "--exclusion-reason is required when --verdict is contaminated: a contamination \
             verdict without a stated reason hides exactly the judgement D-15 requires to be \
             visible"
        );
    }

    let rig_txt_path = args
        .rig_txt
        .clone()
        .unwrap_or_else(|| args.run_dir.join("RIG.txt"));
    let readme_path = args
        .readme
        .clone()
        .unwrap_or_else(|| args.run_dir.join("README.md"));

    // Both sources are read best-effort: a pre-harness capture with only a raw capture and one
    // of the two files must still produce an honest manifest, marking whatever the missing
    // source would have supplied as absent rather than refusing outright.
    let rig_text = std::fs::read_to_string(&rig_txt_path).unwrap_or_default();
    let readme_text = std::fs::read_to_string(&readme_path).unwrap_or_default();

    let rig = parse_rig_txt(&rig_text);
    report_unmapped_keys(&rig);

    let mut absent: Vec<AbsentField> = Vec::new();
    let mut auto_notes: Vec<String> = Vec::new();

    let host = build_host(&rig, args.rig_slug.clone(), &mut absent);
    let kernel = build_kernel(&rig, &readme_text, &mut absent);
    let os = build_os(&rig, &mut absent);
    let tuning = build_tuning(&rig, &mut absent, &mut auto_notes);
    let power = build_power(&rig, &mut absent);
    let network = build_network(&rig, &mut absent);

    absent.push(absent_field(
        "preconditions",
        "the harness did not exist when this capture was taken",
    ));
    absent.push(absent_field(
        "interference.before",
        "no /proc/interrupts snapshot exists for a pre-harness capture",
    ));
    absent.push(absent_field(
        "interference.after",
        "no /proc/interrupts snapshot exists for a pre-harness capture",
    ));
    absent.push(absent_field(
        "interference.delta",
        "no /proc/interrupts snapshot exists for a pre-harness capture",
    ));
    absent.push(absent_field(
        "interference.tail_metrics",
        "a reconstructed manifest is built from RIG.txt/README prose (D-16); the D-24 tail \
         metrics require parsing the run's own cyclictest histogram, which this command does \
         not do",
    ));
    absent.push(absent_field(
        "interference.thresholds_provisional",
        "no D-24 tail-metric verdict was computed for this reconstructed manifest",
    ));
    absent.push(absent_field(
        "tools",
        "RIG.txt and the README describe the run in prose but do not record a machine-readable \
         tool invocation (argv, exit code) for the pre-harness capture",
    ));
    absent.push(absent_field(
        "utc_start",
        "no pre-harness capture records a live run-start timestamp; --utc-start is the \
         operator's best defensible reconstruction from artifact modification times and other \
         evidence, not a value observed by the tool that ran",
    ));
    absent.push(absent_field(
        "utc_end",
        "no pre-harness capture records a live run-end timestamp; --utc-end is the operator's \
         best defensible reconstruction from artifact modification times and other evidence, \
         not a value observed by the tool that ran",
    ));

    let empty_snapshot = InterferenceSnapshot {
        isolated_cpus: Vec::new(),
        cal_ipis: Vec::new(),
        tlb_ipis: Vec::new(),
        rescheduling_ipis: Vec::new(),
        irqs: Vec::new(),
    };
    let interference = InterferenceSnapshotPair {
        before: empty_snapshot.clone(),
        after: empty_snapshot,
        delta: InterferenceDelta {
            cal_ipis: Vec::new(),
            tlb_ipis: Vec::new(),
            rescheduling_ipis: Vec::new(),
            irqs: Vec::new(),
        },
        tail_metrics: None,
        thresholds_provisional: None,
        verdict: args.verdict.into(),
        // A reconstructed manifest was never measured live: there is no per-instrument
        // timing to recover, only the absence itself.
        windows: Vec::new(),
    };

    let (excluded_from_series, exclusion_reason) = match args.verdict {
        VerdictArg::Clean => (false, None),
        VerdictArg::Contaminated => (true, args.exclusion_reason.clone()),
        VerdictArg::Uncalibrated => (
            true,
            Some(args.exclusion_reason.clone().unwrap_or_else(|| {
                "no calibrated contamination thresholds exist yet (D-17)".to_string()
            })),
        ),
    };

    let mut exclude_names: HashSet<String> = HashSet::new();
    exclude_names.insert("manifest.json".to_string());
    if let Some(name) = rig_txt_path.file_name() {
        exclude_names.insert(name.to_string_lossy().into_owned());
    }
    if let Some(name) = readme_path.file_name() {
        exclude_names.insert(name.to_string_lossy().into_owned());
    }
    let artifacts = collect_artifacts(&args.run_dir, &exclude_names).with_context(|| {
        format!(
            "failed to collect artifacts from {}",
            args.run_dir.display()
        )
    })?;

    let notes = build_notes(args.note.as_deref(), &auto_notes);

    let run_id = args
        .run_dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let manifest = RunManifest {
        schema_version: nr_manifest::SCHEMA_VERSION,
        provenance_tier: ProvenanceTier::Reconstructed,
        run_id,
        run_class: args.run_class.into(),
        instrument_class: args.instrument.into(),
        // A reconstructed run never declared a thermal profile: this concept did not
        // exist for it, so `None` is the honest answer rather than a guessed default.
        thermal_profile: None,
        utc_start: args.utc_start,
        utc_end: args.utc_end,
        harness: harness_info(),
        host,
        kernel,
        os,
        tuning,
        power,
        network,
        preconditions: Vec::<PreconditionResult>::new(),
        interference,
        tools: Vec::<ToolInvocation>::new(),
        artifacts,
        // Nothing in the reconstructed recon artifacts speaks to a firmware screen; a
        // reconstructed run records none rather than inferring one.
        firmware_screens: Vec::new(),
        smi_counts: None,
        // A reconstructed run never went through the live admission gate (D-28); there is no
        // evidence trail to recover, only the excluded_from_series/exclusion_reason pair the
        // operator already supplied above.
        series_admission: None,
        absent_fields: absent,
        excluded_from_series,
        exclusion_reason,
        // A reconstructed manifest was never driven live by any fixture seam; it is
        // rebuilt from committed recon artifacts instead.
        fixtures_used: Vec::new(),
        notes,
    };

    nr_manifest::validate(&args.run_dir, &manifest).map_err(|errors| {
        anyhow::anyhow!("reconstructed manifest failed its own validation: {errors:?}")
    })?;

    let manifest_json =
        serde_json::to_string_pretty(&manifest).context("failed to serialise the manifest")?;
    std::fs::write(&manifest_path, manifest_json)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;

    println!("wrote {}", manifest_path.display());
    Ok(0)
}

fn absent_field(field_path: &str, reason: &str) -> AbsentField {
    AbsentField {
        field_path: field_path.to_string(),
        reason: reason.to_string(),
    }
}

fn build_notes(operator_note: Option<&str>, auto_notes: &[String]) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(note) = operator_note {
        if !note.trim().is_empty() {
            parts.push(note.to_string());
        }
    }
    parts.extend(auto_notes.iter().cloned());
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}

/// Ties the manifest to the exact harness build that ran the reconstruction (T-1-10).
/// Mirrors `cmd::run::harness_info` exactly, including reading the same
/// `crates/cli/build.rs`-embedded constants and hashing the same running
/// executable, kept as a small standalone copy here since that function is private
/// to its own module.
fn harness_info() -> HarnessInfo {
    let (executable_blake3, executable_bytes) = match std::env::current_exe() {
        Ok(path) => (
            nr_manifest::blake3_file(&path).ok(),
            std::fs::metadata(&path).ok().map(|m| m.len()),
        ),
        Err(_) => (None, None),
    };
    HarnessInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: env!("NR_BUILD_GIT_SHA").to_string(),
        git_dirty: env!("NR_BUILD_GIT_DIRTY") == "true",
        git_sha_source: Some(GitShaSource::from_build_env(env!(
            "NR_BUILD_GIT_SHA_SOURCE"
        ))),
        executable_blake3,
        executable_bytes,
        invoked_from_git_sha: git_output(&["rev-parse", "HEAD"]),
    }
}

fn git_output(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

// ---------------------------------------------------------------------------------
// RIG.txt parsing
// ---------------------------------------------------------------------------------

/// Parses `key: value` lines. Splits on the *first* `:` only, so a value that itself contains a
/// colon (`captured_utc`'s RFC3339 timestamp) survives intact.
fn parse_rig_txt(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|line| {
            if line.trim().is_empty() {
                return None;
            }
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

/// A key present in RIG.txt with no corresponding manifest field, including `captured_utc`,
/// which the mapping table explicitly calls out as unmapped rather than silently dropped.
fn report_unmapped_keys(rig: &BTreeMap<String, String>) {
    for key in rig.keys() {
        if !MAPPED_RIG_KEYS.contains(&key.as_str()) {
            eprintln!("unmapped RIG.txt key: {key}");
        }
    }
}

/// Splits at the first run of two or more consecutive spaces, mirroring the visual field
/// separator RIG.txt's own `bios:` and `kernel:` lines use (confirmed against the real file:
/// both carry a literal double space where a single-space value boundary would be ambiguous).
fn split_double_space(value: &str) -> Option<(&str, &str)> {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b' ' && bytes[i + 1] == b' ' {
            let start = i;
            let mut end = i;
            while end < bytes.len() && bytes[end] == b' ' {
                end += 1;
            }
            return Some((&value[..start], &value[end..]));
        }
        i += 1;
    }
    None
}

/// `nr_capture::environment`'s own `extract_cmdline_param`, duplicated here (a four-line, pure
/// string helper) rather than exposed cross-crate for this one caller.
fn extract_cmdline_param(cmdline: &str, name: &str) -> Option<String> {
    let prefix = format!("{name}=");
    cmdline
        .split_whitespace()
        .find_map(|token| token.strip_prefix(prefix.as_str()).map(str::to_string))
}

/// Finds the literal marker `/sys/kernel/realtime` in `readme` and reads the `= 0` / `= 1` that
/// follows it (allowing for the surrounding backticks/parentheses the real README uses:
/// `` (`/sys/kernel/realtime` = 1) ``). `None` when the marker is not present at all.
fn parse_is_realtime_from_readme(readme: &str) -> Option<bool> {
    const MARKER: &str = "/sys/kernel/realtime";
    let idx = readme.find(MARKER)?;
    let after = &readme[idx + MARKER.len()..];
    let after = after.trim_start_matches(['`', ' ', '\t']);
    let after = after.strip_prefix('=')?.trim_start();
    if after.starts_with('1') {
        Some(true)
    } else if after.starts_with('0') {
        Some(false)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------------
// Field builders, one per manifest sub-struct.
// ---------------------------------------------------------------------------------

fn build_host(
    rig: &BTreeMap<String, String>,
    rig_slug: String,
    absent: &mut Vec<AbsentField>,
) -> HostInfo {
    let (system_vendor, system_model) = match rig.get("system") {
        Some(value) => match value.split_once(' ') {
            Some((vendor, model)) => (vendor.to_string(), model.to_string()),
            None => (value.clone(), String::new()),
        },
        None => {
            absent.push(absent_field(
                "host.system_vendor",
                "RIG.txt has no system: line",
            ));
            absent.push(absent_field(
                "host.system_model",
                "RIG.txt has no system: line",
            ));
            (String::new(), String::new())
        }
    };

    let (bios_version, bios_release_date) =
        match rig.get("bios").and_then(|v| split_double_space(v)) {
            Some((version, rest)) => {
                let date = rest.trim().strip_prefix("released ").unwrap_or(rest.trim());
                (version.trim().to_string(), date.trim().to_string())
            }
            None => {
                absent.push(absent_field(
                    "host.bios_version",
                    "RIG.txt has no bios: line, or it has no double-space separator",
                ));
                absent.push(absent_field(
                    "host.bios_release_date",
                    "RIG.txt has no bios: line, or it has no double-space separator",
                ));
                (String::new(), String::new())
            }
        };

    let cpu_model = rig.get("cpu").cloned().unwrap_or_else(|| {
        absent.push(absent_field("host.cpu_model", "RIG.txt has no cpu: line"));
        String::new()
    });

    let (logical_cpus, physical_cores, sockets) =
        match rig.get("topology").and_then(|v| parse_topology(v)) {
            Some(triple) => triple,
            None => {
                for field in ["host.logical_cpus", "host.physical_cores", "host.sockets"] {
                    absent.push(absent_field(
                    field,
                    "RIG.txt has no topology: line in the \"N logical / N cores / N socket\" form",
                ));
                }
                (0, 0, 0)
            }
        };

    let p_cores = rig.get("p_cores").cloned().unwrap_or_else(|| {
        absent.push(absent_field("host.p_cores", "RIG.txt has no p_cores: line"));
        String::new()
    });
    let e_cores = rig.get("e_cores").cloned().unwrap_or_else(|| {
        absent.push(absent_field("host.e_cores", "RIG.txt has no e_cores: line"));
        String::new()
    });
    let microcode = rig.get("microcode").cloned().unwrap_or_else(|| {
        absent.push(absent_field(
            "host.microcode",
            "RIG.txt has no microcode: line",
        ));
        String::new()
    });
    let memory_gb = match rig.get("memory_gb").and_then(|v| v.parse::<u32>().ok()) {
        Some(value) => value,
        None => {
            absent.push(absent_field(
                "host.memory_gb",
                "RIG.txt has no memory_gb: line, or it is not a plain integer",
            ));
            0
        }
    };

    HostInfo {
        rig_slug,
        system_vendor,
        system_model,
        bios_version,
        bios_release_date,
        cpu_model,
        microcode,
        logical_cpus,
        physical_cores,
        sockets,
        p_cores,
        e_cores,
        memory_gb,
    }
}

fn parse_topology(value: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = value.split('/').map(str::trim).collect();
    if parts.len() != 3 {
        return None;
    }
    let leading_int = |s: &str| s.split_whitespace().next()?.parse::<u32>().ok();
    Some((
        leading_int(parts[0])?,
        leading_int(parts[1])?,
        leading_int(parts[2])?,
    ))
}

fn build_kernel(
    rig: &BTreeMap<String, String>,
    readme_text: &str,
    absent: &mut Vec<AbsentField>,
) -> KernelInfo {
    let (release, version_string, preempt_model) =
        match rig.get("kernel").and_then(|v| split_double_space(v)) {
            Some((release, rest)) => {
                let version_string = rest
                    .trim()
                    .trim_start_matches('(')
                    .trim_end_matches(')')
                    .to_string();
                let preempt_model = if version_string.contains("PREEMPT_RT") {
                    "PREEMPT_RT".to_string()
                } else if version_string.contains("PREEMPT_DYNAMIC") {
                    "PREEMPT_DYNAMIC".to_string()
                } else {
                    absent.push(absent_field(
                        "kernel.preempt_model",
                        "neither PREEMPT_RT nor PREEMPT_DYNAMIC found in RIG.txt's kernel: line",
                    ));
                    String::new()
                };
                (release.trim().to_string(), version_string, preempt_model)
            }
            None => {
                for field in [
                    "kernel.release",
                    "kernel.version_string",
                    "kernel.preempt_model",
                ] {
                    absent.push(absent_field(
                        field,
                        "RIG.txt has no kernel: line, or it has no double-space separator",
                    ));
                }
                (String::new(), String::new(), String::new())
            }
        };

    let is_realtime = match parse_is_realtime_from_readme(readme_text) {
        Some(value) => value,
        None => {
            absent.push(absent_field(
                "kernel.is_realtime",
                "neither RIG.txt nor the README states /sys/kernel/realtime's value",
            ));
            false
        }
    };

    let raw_cmdline = rig.get("cmdline").cloned().unwrap_or_else(|| {
        absent.push(absent_field(
            "kernel.cmdline",
            "RIG.txt has no cmdline: line",
        ));
        String::new()
    });
    let cmdline = KernelInfo::redact_cmdline(&raw_cmdline);

    let isolcpus = extract_cmdline_param(&raw_cmdline, "isolcpus");
    if isolcpus.is_none() {
        absent.push(absent_field(
            "kernel.isolcpus",
            "isolcpus not present on RIG.txt's cmdline: value",
        ));
    }
    let nohz_full = extract_cmdline_param(&raw_cmdline, "nohz_full");
    if nohz_full.is_none() {
        absent.push(absent_field(
            "kernel.nohz_full",
            "nohz_full not present on RIG.txt's cmdline: value",
        ));
    }
    let rcu_nocbs = extract_cmdline_param(&raw_cmdline, "rcu_nocbs");
    if rcu_nocbs.is_none() {
        absent.push(absent_field(
            "kernel.rcu_nocbs",
            "rcu_nocbs not present on RIG.txt's cmdline: value",
        ));
    }
    let irqaffinity = extract_cmdline_param(&raw_cmdline, "irqaffinity");
    if irqaffinity.is_none() {
        absent.push(absent_field(
            "kernel.irqaffinity",
            "irqaffinity not present on RIG.txt's cmdline: value",
        ));
    }

    KernelInfo {
        release,
        version_string,
        is_realtime,
        preempt_model,
        cmdline,
        isolcpus,
        nohz_full,
        rcu_nocbs,
        irqaffinity,
    }
}

fn build_os(rig: &BTreeMap<String, String>, absent: &mut Vec<AbsentField>) -> OsInfo {
    let (distro, version) = match rig.get("os").and_then(|v| v.split_once(' ')) {
        Some((distro, version)) => (distro.to_string(), version.to_string()),
        None => {
            absent.push(absent_field("os.distro", "RIG.txt has no os: line"));
            absent.push(absent_field("os.version", "RIG.txt has no os: line"));
            (String::new(), String::new())
        }
    };

    let session_kind = match rig.get("session") {
        Some(value) if value.to_ascii_lowercase().contains("live usb") => SessionKind::LiveUsb,
        Some(_) => SessionKind::Installed,
        None => {
            absent.push(absent_field(
                "os.session_kind",
                "RIG.txt has no session: line; defaulting to installed would be a guess",
            ));
            SessionKind::Installed
        }
    };

    absent.push(absent_field(
        "os.systemd_default_target",
        "neither RIG.txt nor the README records the systemd default target",
    ));
    absent.push(absent_field(
        "os.display_manager_active",
        "neither RIG.txt nor the README records display manager state",
    ));

    OsInfo {
        distro,
        version,
        session_kind,
        systemd_default_target: String::new(),
        display_manager_active: false,
    }
}

fn build_tuning(
    rig: &BTreeMap<String, String>,
    absent: &mut Vec<AbsentField>,
    auto_notes: &mut Vec<String>,
) -> TuningInfo {
    // RIG.txt records one machine-wide governor value, not a per-CPU list; the shape
    // `TuningInfo::per_cpu_governor` needs cannot be filled from it without guessing which
    // CPUs it applies to. Record the observed value in the manifest's notes instead.
    if let Some(value) = rig.get("governor") {
        auto_notes.push(format!(
            "RIG.txt recorded a single scaling governor value: {value}"
        ));
    }
    absent.push(absent_field(
        "tuning.per_cpu_governor",
        "RIG.txt records a single governor value, not a per-CPU list",
    ));
    absent.push(absent_field(
        "tuning.per_cpu_governor.energy_performance_preference",
        "this field was added to the manifest schema on 2026-08-31 (commit d73cc27), after \
         this capture was taken, and was never recorded for it; the 2026-08-28 runs were taken \
         on a live-USB stock kernel, a different machine state from the installed PREEMPT_RT \
         system this field now describes, so no value can be inferred from the rig's current \
         state",
    ));

    let no_turbo = match rig.get("no_turbo").map(String::as_str) {
        Some("1") => Some(true),
        Some("0") => Some(false),
        _ => {
            absent.push(absent_field(
                "tuning.no_turbo",
                "RIG.txt's no_turbo: value is absent or not a plain 0/1",
            ));
            None
        }
    };

    let cstates = rig
        .get("cstates")
        .map(|value| {
            value
                .split_whitespace()
                .filter_map(|token| {
                    let (name, state) = token.split_once('=')?;
                    Some(CstateSetting {
                        cpu: 0,
                        name: name.to_string(),
                        disabled: state == "off",
                    })
                })
                .collect()
        })
        .unwrap_or_else(|| {
            absent.push(absent_field(
                "tuning.cstates",
                "RIG.txt has no cstates: line",
            ));
            Vec::new()
        });

    absent.push(absent_field(
        "tuning.rt_tuning_service",
        "RIG.txt predates rt-tuning.service; the README does not record its state either",
    ));

    TuningInfo {
        per_cpu_governor: Vec::<CpuGovernor>::new(),
        no_turbo,
        cstates,
        rt_tuning_service: ServiceState {
            unit: "rt-tuning.service".to_string(),
            active_state: String::new(),
            sub_state: String::new(),
            unit_file_state: String::new(),
        },
    }
}

fn build_power(rig: &BTreeMap<String, String>, absent: &mut Vec<AbsentField>) -> PowerInfo {
    let (ac_online, battery_status, battery_percent) = match rig.get("power") {
        Some(value) => {
            let mut ac = None;
            let mut status = None;
            let mut percent = None;
            for token in value.split_whitespace() {
                if let Some(v) = token.strip_prefix("AC=") {
                    ac = Some(v == "1");
                } else if let Some(v) = token.strip_prefix("battery=") {
                    status = Some(v.to_string());
                } else if let Some(pct) = token.strip_suffix('%') {
                    percent = pct.parse().ok();
                }
            }
            (ac, status, percent)
        }
        None => {
            absent.push(absent_field(
                "power.ac_online",
                "RIG.txt has no power: line",
            ));
            absent.push(absent_field(
                "power.battery_status",
                "RIG.txt has no power: line",
            ));
            absent.push(absent_field(
                "power.battery_percent",
                "RIG.txt has no power: line",
            ));
            (None, None, None)
        }
    };

    absent.push(absent_field(
        "power.thermal_zones",
        "RIG.txt's power: line carries AC/battery state only, no thermal readings",
    ));
    absent.push(absent_field(
        "power.package_temp_c_max",
        "RIG.txt's power: line carries AC/battery state only, no thermal readings",
    ));

    PowerInfo {
        ac_online,
        battery_status,
        battery_percent,
        thermal_zones: Vec::<ThermalZone>::new(),
        package_temp_c_max: None,
    }
}

/// Parses an `iface [key=value ...]` RIG.txt line (`nic_wired:`/`nic_wifi:`'s shape): the first
/// whitespace token (no `=`) is the interface name, every later token is a `key=value` pair.
fn parse_iface_line(value: &str) -> (Option<String>, BTreeMap<String, String>) {
    let mut tokens = value.split_whitespace();
    let iface = tokens.next().map(str::to_string);
    let mut kvs = BTreeMap::new();
    for token in tokens {
        if let Some((key, val)) = token.split_once('=') {
            kvs.insert(key.to_string(), val.to_string());
        }
    }
    (iface, kvs)
}

fn build_network(rig: &BTreeMap<String, String>, absent: &mut Vec<AbsentField>) -> NetworkInfo {
    let (wired_iface, wired_kvs) = match rig.get("nic_wired") {
        Some(value) => parse_iface_line(value),
        None => {
            absent.push(absent_field(
                "network.wired_iface",
                "RIG.txt has no nic_wired: line",
            ));
            (None, BTreeMap::new())
        }
    };
    let wired_driver = wired_kvs.get("driver").cloned();
    let wired_state = wired_kvs.get("state").cloned();
    if wired_driver.is_none() {
        absent.push(absent_field(
            "network.wired_driver",
            "no driver= in RIG.txt's nic_wired: line",
        ));
    }
    if wired_state.is_none() {
        absent.push(absent_field(
            "network.wired_state",
            "no state= in RIG.txt's nic_wired: line",
        ));
    }

    let ptp_capabilities = rig
        .get("nic_ptp")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    if ptp_capabilities.is_none() {
        absent.push(absent_field(
            "network.ptp_capabilities",
            "RIG.txt has no nic_ptp: line",
        ));
    }

    let (wifi_iface, wifi_kvs) = match rig.get("nic_wifi") {
        Some(value) => parse_iface_line(value),
        None => {
            absent.push(absent_field(
                "network.wifi_iface",
                "RIG.txt has no nic_wifi: line",
            ));
            (None, BTreeMap::new())
        }
    };
    let wifi_driver = wifi_kvs.get("driver").cloned();
    if wifi_driver.is_none() {
        absent.push(absent_field(
            "network.wifi_driver",
            "no driver= in RIG.txt's nic_wifi: line",
        ));
    }
    absent.push(absent_field(
        "network.wifi_state",
        "RIG.txt's nic_wifi: line carries no state= key",
    ));

    NetworkInfo {
        wired_iface,
        wired_driver,
        wired_state,
        ptp_capabilities,
        wifi_iface,
        wifi_driver,
        wifi_state: None,
    }
}

// ---------------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------------

fn infer_artifact_kind(filename: &str) -> ArtifactKind {
    if filename.ends_with(".hist") {
        ArtifactKind::CyclictestHist
    } else if filename.starts_with("cyclictest") && filename.ends_with(".json") {
        ArtifactKind::CyclictestJson
    } else if filename.starts_with("hwlatdetect") {
        ArtifactKind::HwlatdetectText
    } else if filename.starts_with("trace-") || filename.ends_with(".trace.dat") {
        ArtifactKind::FtraceMarkers
    } else if filename.starts_with("timerlat") {
        ArtifactKind::RtlaTimerlat
    } else {
        ArtifactKind::Other
    }
}

/// Every file in `run_dir` except the names in `exclude` (`manifest.json`, the RIG.txt and
/// README paths actually used) becomes an `ArtifactRecord`. A file whose kind cannot be
/// inferred is recorded as `Other`, never skipped.
fn collect_artifacts(
    run_dir: &Path,
    exclude: &HashSet<String>,
) -> anyhow::Result<Vec<ArtifactRecord>> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(run_dir)
        .with_context(|| format!("failed to read {}", run_dir.display()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    entries.sort();

    let mut artifacts = Vec::new();
    for path in entries {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if exclude.contains(&name) {
            continue;
        }
        let bytes = std::fs::metadata(&path)
            .with_context(|| format!("failed to stat {}", path.display()))?
            .len();
        let blake3 = nr_manifest::blake3_file(&path)
            .map_err(|source| anyhow::anyhow!("failed to checksum {}: {source}", path.display()))?;
        artifacts.push(ArtifactRecord {
            path: name.clone(),
            bytes,
            blake3,
            kind: infer_artifact_kind(&name),
            stored: StorageLocation::InRepo,
        });
    }
    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rig_txt_preserves_colons_inside_values() {
        let rig =
            parse_rig_txt("captured_utc: 2026-08-28T20:36:21Z\nsystem: Dell Inc. Precision 3591\n");
        assert_eq!(rig.get("captured_utc").unwrap(), "2026-08-28T20:36:21Z");
        assert_eq!(rig.get("system").unwrap(), "Dell Inc. Precision 3591");
    }

    #[test]
    fn split_double_space_finds_the_first_run() {
        assert_eq!(
            split_double_space("1.23.0  released 04/24/2026"),
            Some(("1.23.0", "released 04/24/2026"))
        );
        assert_eq!(split_double_space("no double space here"), None);
    }

    #[test]
    fn parse_topology_extracts_three_leading_integers() {
        assert_eq!(
            parse_topology("22 logical / 16 cores / 1 socket"),
            Some((22, 16, 1))
        );
        assert_eq!(parse_topology("not topology shaped"), None);
    }

    #[test]
    fn parse_is_realtime_from_readme_reads_the_stated_value() {
        let readme = "Kernel 7.0.0-30-realtime (`/sys/kernel/realtime` = 1), isolcpus=6-11";
        assert_eq!(parse_is_realtime_from_readme(readme), Some(true));
        assert_eq!(parse_is_realtime_from_readme("no marker here"), None);
    }

    #[test]
    fn parse_iface_line_splits_name_from_key_value_pairs() {
        let (iface, kvs) = parse_iface_line("enp0s31f6 driver=e1000e state=down");
        assert_eq!(iface.as_deref(), Some("enp0s31f6"));
        assert_eq!(kvs.get("driver").map(String::as_str), Some("e1000e"));
        assert_eq!(kvs.get("state").map(String::as_str), Some("down"));
    }

    #[test]
    fn infer_artifact_kind_matches_the_documented_shapes() {
        assert_eq!(
            infer_artifact_kind("cyclictest.hist"),
            ArtifactKind::CyclictestHist
        );
        assert_eq!(
            infer_artifact_kind("cyclictest.json"),
            ArtifactKind::CyclictestJson
        );
        assert_eq!(
            infer_artifact_kind("hwlatdetect-15m.txt"),
            ArtifactKind::HwlatdetectText
        );
        assert_eq!(
            infer_artifact_kind("partition-table-BEFORE.txt"),
            ArtifactKind::Other
        );
    }
}
