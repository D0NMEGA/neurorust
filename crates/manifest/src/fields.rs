//! The D-14 environment snapshot, typed.
//!
//! Every struct here derives `Serialize, Deserialize, JsonSchema, Debug, Clone,
//! PartialEq` and every struct (not enum) carries `#[serde(deny_unknown_fields)]`: an
//! unknown field on deserialization is schema drift and must fail loudly rather than be
//! silently dropped. Fields are required unless the value is genuinely absent on the
//! hardware (`Option<T>`), and no field here uses `#[serde(default)]` - required means
//! enforced, not aspirational (T-1-03).
//!
//! Every enum uses `#[serde(rename_all = "kebab-case")]` so the manifest reads as
//! `"harness-generated"`, `"calibration-contaminated"` and so on.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::provenance::{AbsentField, ProvenanceTier};

/// Current version of this schema. Bump on any breaking change to `RunManifest`.
pub const SCHEMA_VERSION: u32 = 1;

/// The D-12/D-14 run manifest: the full environment snapshot, provenance, checksums
/// and preconditions for one measurement run.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    /// Schema revision. Must equal [`SCHEMA_VERSION`]; see `validate::validate`.
    pub schema_version: u32,
    /// D-16: never `Option`, never defaulted. See [`crate::provenance`].
    pub provenance_tier: ProvenanceTier,
    /// Matches `^[a-z0-9][a-z0-9-]{0,63}$`; becomes a directory name under
    /// `measurements/`, so this is also the ASVS V12 boundary (T-1-16).
    pub run_id: String,
    pub run_class: RunClass,
    pub instrument_class: InstrumentClass,
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    pub utc_start: time::OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    pub utc_end: time::OffsetDateTime,
    pub harness: HarnessInfo,
    pub host: HostInfo,
    pub kernel: KernelInfo,
    pub os: OsInfo,
    pub tuning: TuningInfo,
    pub power: PowerInfo,
    pub network: NetworkInfo,
    pub preconditions: Vec<PreconditionResult>,
    pub interference: InterferenceSnapshotPair,
    pub tools: Vec<ToolInvocation>,
    pub artifacts: Vec<ArtifactRecord>,
    /// D-16: fields a reconstructed manifest could not recover. Empty for a
    /// harness-generated run with nothing missing.
    pub absent_fields: Vec<AbsentField>,
    /// BENCH-06: a contaminated or otherwise non-headline run is still published, but
    /// excluded from the regression series. See `exclusion_reason`.
    pub excluded_from_series: bool,
    /// Required to be `Some` and non-empty when `excluded_from_series` is true; see
    /// `ValidationError::MissingExclusionReason`.
    pub exclusion_reason: Option<String>,
    pub notes: Option<String>,
}

/// D-10 run cadence tag. Kept distinct so run classes are never averaged together.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum RunClass {
    Recon,
    Screen,
    CalibrationClean,
    CalibrationContaminated,
    Investigation,
    Headline,
    Weekly,
    Soak,
}

/// RESEARCH.md pattern 2: an investigation run carries its own tracer overhead and
/// must never feed the headline series, independently of `RunClass`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum InstrumentClass {
    HeadlineSeries,
    Investigation,
}

/// Ties a manifest to the exact harness build that produced it (T-1-10).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HarnessInfo {
    /// `env!("CARGO_PKG_VERSION")`.
    pub version: String,
    /// Full 40-character SHA of the harness commit.
    pub git_sha: String,
    pub git_dirty: bool,
}

/// The machine, per BENCH-04's rig-discipline requirement. `rig_slug` is a chosen
/// label rather than a network-derived machine name, and this snapshot deliberately
/// omits any network hardware address, wireless network identifier, or hardware
/// identification number (T-1-06).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HostInfo {
    /// e.g. `"precision3591"`. A chosen label, not a network-derived machine name.
    pub rig_slug: String,
    pub system_vendor: String,
    pub system_model: String,
    pub bios_version: String,
    pub bios_release_date: String,
    pub cpu_model: String,
    pub microcode: String,
    pub logical_cpus: u32,
    pub physical_cores: u32,
    pub sockets: u32,
    /// CPU list string, e.g. `"0-11"`.
    pub p_cores: String,
    /// CPU list string, e.g. `"12-21"`.
    pub e_cores: String,
    pub memory_gb: u32,
}

/// The kernel and its real-time tuning. `Option` fields are cmdline parameters that
/// may genuinely be absent, not values the harness failed to read.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KernelInfo {
    /// `uname -r`, e.g. `"7.0.0-30-realtime"`.
    pub release: String,
    /// The full `uname -v` string.
    pub version_string: String,
    /// `/sys/kernel/realtime == 1`.
    pub is_realtime: bool,
    /// e.g. `"PREEMPT_RT"` or `"PREEMPT_DYNAMIC"`.
    pub preempt_model: String,
    /// `/proc/cmdline` with the `root=` and `resume=` values replaced by the
    /// literal token `[redacted]` (see [`KernelInfo::redact_cmdline`]); every
    /// other parameter, including `BOOT_IMAGE`, `isolcpus`, `nohz_full`,
    /// `rcu_nocbs` and `irqaffinity`, is verbatim. The root and swap
    /// filesystem UUIDs carry no reproduction value and are the only
    /// machine-instance identifiers on the command line (T-1-06); redaction
    /// is visible by design, never a silent drop, so a reader can tell a
    /// redacted field from one that was never recorded.
    pub cmdline: String,
    /// Parsed out of `cmdline`. `None` means the parameter was absent, not unread.
    pub isolcpus: Option<String>,
    pub nohz_full: Option<String>,
    pub rcu_nocbs: Option<String>,
    pub irqaffinity: Option<String>,
}

impl KernelInfo {
    /// Redacts the `root=` and `resume=` parameters in a raw `/proc/cmdline`
    /// string, replacing each one's value with the literal token
    /// `[redacted]`. Every other parameter is preserved verbatim, because
    /// those are what a third party needs to reproduce the run; only the
    /// root and swap filesystem UUIDs are machine-instance identifiers with
    /// no reproduction value (T-1-06). `nr-capture` calls this when it reads
    /// `/proc/cmdline` at capture time, before the value is ever written to
    /// a manifest.
    pub fn redact_cmdline(raw: &str) -> String {
        raw.split_whitespace()
            .map(|token| match token.split_once('=') {
                Some(("root", _)) => "root=[redacted]",
                Some(("resume", _)) => "resume=[redacted]",
                _ => token,
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Whether the run happened on the installed system or a live-USB session, since the
/// two have different persistence and tuning guarantees.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionKind {
    Installed,
    LiveUsb,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OsInfo {
    pub distro: String,
    pub version: String,
    pub session_kind: SessionKind,
    pub systemd_default_target: String,
    pub display_manager_active: bool,
}

/// One CPU's scaling governor, as read per-core rather than assumed uniform.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CpuGovernor {
    pub cpu: u32,
    pub governor: String,
}

/// One CPU's C-state setting. `disabled` records whether that state is blocked, which
/// is what the `DeepCstatesDisabled` precondition and D-14's C-state field want.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CstateSetting {
    pub cpu: u32,
    pub name: String,
    pub disabled: bool,
}

/// systemd unit state for `rt-tuning.service`, the authoritative source for tuning
/// state (CONTEXT.md "Reusable assets": query it, do not re-derive tuning from sysfs).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ServiceState {
    pub unit: String,
    pub active_state: String,
    pub sub_state: String,
    pub unit_file_state: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TuningInfo {
    pub per_cpu_governor: Vec<CpuGovernor>,
    /// `None` when `intel_pstate/no_turbo` is absent, e.g. non-Intel hardware.
    pub no_turbo: Option<bool>,
    pub cstates: Vec<CstateSetting>,
    pub rt_tuning_service: ServiceState,
}

/// One thermal zone's temperature at the start and, once the run finishes, the end of
/// a run. `temp_c_end` is `None` until the run completes.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThermalZone {
    /// From `/sys/class/thermal`; names hardware, e.g. `"x86_pkg_temp"`.
    pub name: String,
    pub temp_c_start: f32,
    pub temp_c_end: Option<f32>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PowerInfo {
    pub ac_online: Option<bool>,
    /// e.g. `"Charging"`, `"Discharging"`, `"Full"`.
    pub battery_status: Option<String>,
    pub battery_percent: Option<u32>,
    pub thermal_zones: Vec<ThermalZone>,
    pub package_temp_c_max: Option<f32>,
}

/// Deliberately omits any network hardware address or wireless network identifier
/// (T-1-06).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NetworkInfo {
    /// e.g. `"enp0s31f6"`.
    pub wired_iface: Option<String>,
    /// e.g. `"e1000e"`.
    pub wired_driver: Option<String>,
    pub wired_state: Option<String>,
    /// e.g. `"hardware-raw-clock"`.
    pub ptp_capabilities: Option<String>,
    pub wifi_iface: Option<String>,
    pub wifi_driver: Option<String>,
    pub wifi_state: Option<String>,
}

/// D-06: every precondition check the harness knows about, asserted and recorded
/// rather than silently assumed. Defined once here as the single source of truth;
/// `nr-capture` implements the checks and `docs/measurement-protocol.md` documents
/// them.
///
/// `TracersQuiescent` mechanises RESEARCH.md pattern 2: for a `HeadlineSeries` run,
/// `/sys/kernel/tracing/current_tracer` must read `nop`, so an investigation run's
/// tracer overhead can never leak into the published series.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PreconditionCheck {
    NoActiveSshSessions,
    SystemdDefaultTargetIsMultiUser,
    DisplayManagerInactive,
    NoGraphicalSession,
    GovernorIsPerformanceOnAllCpus,
    NoTurboEnabled,
    DeepCstatesDisabled,
    IsolcpusCoversTargetCpus,
    KernelIsRealtime,
    RtTuningServiceActive,
    OnAcPower,
    ThermalHeadroomAtStart,
    NoPackageManagerActivity,
    TracersQuiescent,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PreconditionStatus {
    Pass,
    Fail,
    NotApplicable,
    Unavailable,
}

/// One precondition check's outcome. Recorded whether it passed or failed (D-06:
/// assert-and-record, never enforce beyond refusing the run), so the assertion list
/// doubles as PLAT-02's third-party reproduction checklist.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreconditionResult {
    pub check: PreconditionCheck,
    pub status: PreconditionStatus,
    /// What was actually seen, e.g. `"powersave"` or `"2 sessions"`.
    pub observed: String,
    /// What the check required, e.g. `"performance"`.
    pub expected: String,
}

/// One counter reading for one isolated CPU.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CpuCounter {
    pub cpu: u32,
    pub count: u64,
}

/// D-15's interference signal at one point in time (before or after a run), scoped to
/// the isolated CPUs rather than machine-wide. Tracks exactly the counters D-15 and the
/// 2026-08-28 post-mortem name as diagnostic: CAL and TLB IPIs, context switches, and
/// total IRQs per isolated CPU.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterferenceSnapshot {
    pub isolated_cpus: Vec<u32>,
    /// Function-call IPIs, per isolated CPU.
    pub cal_ipis: Vec<CpuCounter>,
    /// TLB shootdowns, per isolated CPU.
    pub tlb_ipis: Vec<CpuCounter>,
    pub context_switches: Vec<CpuCounter>,
    pub irqs: Vec<CpuCounter>,
}

/// The after-minus-before difference for each counter family in [`InterferenceSnapshot`].
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterferenceDelta {
    pub cal_ipis: Vec<CpuCounter>,
    pub tlb_ipis: Vec<CpuCounter>,
    pub context_switches: Vec<CpuCounter>,
    pub irqs: Vec<CpuCounter>,
}

/// D-15's automatic contamination call. A `Contaminated` run is still published per
/// BENCH-06, but excluded from the headline series.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ContaminationVerdict {
    Clean,
    Contaminated,
    /// No calibrated thresholds exist yet to render a verdict against (D-17).
    Uncalibrated,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterferenceSnapshotPair {
    pub before: InterferenceSnapshot,
    pub after: InterferenceSnapshot,
    pub delta: InterferenceDelta,
    pub verdict: ContaminationVerdict,
}

/// One external tool the harness shelled out to, e.g. `cyclictest`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ToolInvocation {
    pub name: String,
    /// As reported by the tool itself.
    pub version: String,
    /// The exact argument vector, for reproduction. Any output-file path in
    /// `argv` is recorded relative to the run directory, not as an absolute
    /// path, so the command a third party pastes actually runs (the rewrite
    /// happens where argv is captured, in `nr-capture`, plan 01-05).
    /// Everything else in `argv` is verbatim.
    pub argv: Vec<String>,
    pub exit_code: i32,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    CyclictestHist,
    CyclictestJson,
    HwlatdetectText,
    FtraceMarkers,
    RtlaTimerlat,
    Other,
}

/// Where an artifact's bytes actually live. D-12: raw captures stay byte-identical to
/// what the tool emitted, so this only ever points at a file, never embeds one.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum StorageLocation {
    InRepo,
    External { url: String },
}

/// One raw capture, checksummed (D-12). `path` is validated by `validate::validate`
/// against traversal and absolute paths before it is ever joined to a run directory
/// (ASVS V12, T-1-16).
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRecord {
    /// Relative to the run directory. No `..`, no leading `/`.
    pub path: String,
    pub bytes: u64,
    /// Lowercase hex blake3 digest.
    pub blake3: String,
    pub kind: ArtifactKind,
    pub stored: StorageLocation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_cmdline_redacts_root_and_resume_uuids_only() {
        let raw = "BOOT_IMAGE=/vmlinuz-7.0.0-30-realtime root=UUID=4c9a2f1e-8b3d-4a6f-9c2e-1a2b3c4d5e6f ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5 intel_pstate=no_hwp resume=UUID=9f8e7d6c-5b4a-3928-1706-f5e4d3c2b1a0 resume_offset=53248";

        let redacted = KernelInfo::redact_cmdline(raw);

        assert_eq!(
            redacted,
            "BOOT_IMAGE=/vmlinuz-7.0.0-30-realtime root=[redacted] ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5 intel_pstate=no_hwp resume=[redacted] resume_offset=53248"
        );

        // Redaction is visible, never a silent drop.
        assert!(redacted.contains("root=[redacted]"));
        assert!(redacted.contains("resume=[redacted]"));

        // No real UUID pattern survives redaction.
        assert!(!redacted.contains("4c9a2f1e-8b3d-4a6f-9c2e-1a2b3c4d5e6f"));
        assert!(!redacted.contains("9f8e7d6c-5b4a-3928-1706-f5e4d3c2b1a0"));
        assert!(!redacted.to_ascii_lowercase().contains("uuid="));

        // Every reproduction-relevant parameter survives verbatim, including
        // the resume_offset near-miss, which must not be matched as `resume=`.
        for verbatim in [
            "BOOT_IMAGE=/vmlinuz-7.0.0-30-realtime",
            "isolcpus=6-11",
            "nohz_full=6-11",
            "rcu_nocbs=6-11",
            "irqaffinity=0-5",
            "intel_pstate=no_hwp",
            "resume_offset=53248",
        ] {
            assert!(
                redacted.contains(verbatim),
                "expected {verbatim:?} to survive, got {redacted:?}"
            );
        }
    }

    #[test]
    fn redact_cmdline_is_a_no_op_without_root_or_resume() {
        let raw = "BOOT_IMAGE=/vmlinuz ro quiet splash isolcpus=6-11";
        assert_eq!(KernelInfo::redact_cmdline(raw), raw);
    }
}
