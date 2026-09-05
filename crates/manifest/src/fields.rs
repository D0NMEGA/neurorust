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
    /// The commit the binary was BUILT from, embedded at compile time by
    /// `crates/cli/build.rs`. Never re-derived from the working directory at run
    /// time: a stale executable running inside a newer checkout would otherwise
    /// inherit that checkout's identity, and a run launched by systemd-run with no
    /// working directory inside the checkout would record "unknown". Both happened;
    /// see finding 6 of `01-EXTERNAL-AUDIT.md`.
    pub git_sha: String,
    /// Whether the working tree was dirty when the binary was built. Meaningful only
    /// when `git_sha_source` is `build-time`.
    pub git_dirty: bool,
    /// How `git_sha` was obtained. Absent on manifests written before this field
    /// existed. `Unavailable` means the build could not read git at all, and
    /// `git_dirty` says nothing in that case.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_sha_source: Option<GitShaSource>,
    /// Lowercase hex blake3 of the executable that produced this manifest, read from
    /// `std::env::current_exe()`. The only field here that identifies the binary
    /// itself rather than a checkout. Deliberately NOT paired with the executable's
    /// absolute path: on the reference rig that path is a home directory
    /// (`/home/<user>/neurorust/target/release/nrmeasure`), manifests are published,
    /// and the blake3 plus byte count identify the binary without naming anyone's
    /// home directory (T-1-06).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_blake3: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executable_bytes: Option<u64>,
    /// The commit of the checkout the harness was INVOKED from, when one could be
    /// read. Recorded separately from `git_sha` so a stale executable running inside
    /// a newer checkout is visible rather than disguised. Absent when the process had
    /// no git checkout as its working directory, which is the normal case under
    /// systemd-run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invoked_from_git_sha: Option<String>,
}

/// How [`HarnessInfo::git_sha`] was obtained.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum GitShaSource {
    BuildTime,
    Unavailable,
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
///
/// `energy_performance_preference` is a second, independent tuning lever some HWP
/// (hardware P-state) backends drive alongside `governor`: on this project's own
/// Meteor Lake reference rig, `power-profiles-daemon`'s "performance" profile leaves
/// `governor` at `powersave` and expresses itself as
/// `energy_performance_preference=performance` instead
/// (`docs/rig/recon-2026-08-31/FINDINGS.md`, "The rt-tuning.service contradiction").
/// `None` means the hardware has no `energy_performance_preference` file at all
/// (non-HWP, non-Intel), never a stand-in for a read that failed. `governor` alone no
/// longer fully describes the tuning state on a machine like this one, which is why
/// the two are recorded side by side rather than one substituting for the other.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CpuGovernor {
    pub cpu: u32,
    pub governor: String,
    /// `/sys/devices/system/cpu/cpu{cpu}/cpufreq/energy_performance_preference`.
    pub energy_performance_preference: Option<String>,
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
    NoActiveLoginSessions,
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

impl PreconditionCheck {
    /// Every variant, in the same order [`crate::preconditions`] (via `nr_capture::
    /// preconditions::run_all`) evaluates them in a real run. `docs/measurement-
    /// protocol.md` is this list in prose; `crates/capture/tests/protocol_doc.rs`
    /// uses this constant to fail the moment the two drift apart, in either
    /// direction (a check added here without being documented, or a name in the
    /// document that names no real check).
    pub const ALL: &'static [PreconditionCheck] = &[
        PreconditionCheck::NoActiveSshSessions,
        PreconditionCheck::SystemdDefaultTargetIsMultiUser,
        PreconditionCheck::DisplayManagerInactive,
        PreconditionCheck::NoGraphicalSession,
        PreconditionCheck::NoActiveLoginSessions,
        PreconditionCheck::GovernorIsPerformanceOnAllCpus,
        PreconditionCheck::NoTurboEnabled,
        PreconditionCheck::DeepCstatesDisabled,
        PreconditionCheck::IsolcpusCoversTargetCpus,
        PreconditionCheck::KernelIsRealtime,
        PreconditionCheck::RtTuningServiceActive,
        PreconditionCheck::OnAcPower,
        PreconditionCheck::ThermalHeadroomAtStart,
        PreconditionCheck::NoPackageManagerActivity,
        PreconditionCheck::TracersQuiescent,
    ];
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

/// D-24: the tail-based contamination metrics, computed from a run's own cyclictest
/// histogram rather than from `/proc/interrupts`. See
/// `crates/capture/src/interference.rs` for how each field is computed and why the
/// interference counters below were found insufficient on their own: the D-17
/// calibration pair (`measurements/2026-09-01-precision3591-calibration-clean` and
/// `measurements/2026-09-02-precision3591-calibration-contaminated`) did not separate on
/// the interference counters at any usable magnitude, while producing a worst case 50x
/// higher. An earlier version of this note said the contaminated run recorded FEWER
/// interrupts; that was wrong, and is corrected in `nr_capture::interference`'s module
/// documentation. Only device IRQs invert; per run hour CAL, TLB and RES are all higher
/// on the contaminated arm.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TailMetrics {
    /// Global maximum divided by p99. A clean run and a globally-stalled run can share
    /// nearly identical percentiles up to p99.99 (contamination here adds rare
    /// catastrophic excursions rather than shifting the bulk distribution), so this
    /// ratio is where the two separate: 8.7 on the D-17 clean arm, 428.4 on its
    /// contaminated twin.
    pub tail_excursion_ratio: f64,
    /// `(max(per-thread max) - min(per-thread max)) / max(per-thread max)`. A single
    /// global stall (for example `stop_machine()`, or a system-wide TLB shootdown)
    /// halts every isolated thread at nearly the same instant, so their per-thread
    /// maxima cluster tightly (3.6% on the D-17 contaminated arm) where independent,
    /// per-core noise scatters them (76.9% on its clean twin). Low spread alone is not
    /// evidence of contamination: a clean run whose six per-thread maxima happen to sit
    /// close together also scores low. See `tail_excursion_ratio`.
    pub thread_max_spread: f64,
    /// Overflow samples (recorded at the histogram bound) divided by run duration in
    /// seconds. Coarser than the other two fields, since it depends on the histogram
    /// bound (400 us in this project); recorded as evidence, never the primary signal.
    pub overflow_rate_per_s: f64,
}

/// D-15/D-24: the full contamination evidence for one run. `before`/`after`/`delta` are
/// the `/proc/interrupts` interference snapshot (D-15's original signal, retained as
/// evidence but no longer sufficient as the sole detector, see `TailMetrics`);
/// `tail_metrics` is the D-24 tail-based signal computed from the run's own histogram;
/// `verdict` is the resulting call. `tail_metrics` and `thresholds_provisional` are
/// `Option` only so a manifest captured before D-24 (2026-09-02 and earlier) still
/// deserializes; every manifest `nrmeasure run` produces from D-24 onward always
/// populates both with `Some`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InterferenceSnapshotPair {
    pub before: InterferenceSnapshot,
    pub after: InterferenceSnapshot,
    pub delta: InterferenceDelta,
    pub tail_metrics: Option<TailMetrics>,
    /// True when `verdict` was computed against provisional (not yet calibrated)
    /// tail-metric thresholds; see `Thresholds::Provisional` in
    /// `crates/capture/src/interference.rs`. Never conflate a provisional `Clean` with
    /// a calibrated one: `nr-cli`'s `determine_exclusion` checks this before ever
    /// admitting a run to the headline series on the strength of `verdict` alone.
    pub thresholds_provisional: Option<bool>,
    pub verdict: ContaminationVerdict,
}

/// Maps a path that appeared in an executed argv to the artifact it became in the run
/// directory.
///
/// The argv is recorded exactly as executed, which means it names the scratch directory
/// the tool actually wrote into (`/tmp/.tmpEb5CdX/cyclictest.hist`) and that directory is
/// gone by the time anyone reads the manifest. Rewriting the argv to point at the
/// committed file was tried and was worse: it produced a command line that was never run,
/// and because the rewrite matched against the run directory while the tool wrote into
/// `/tmp`, it silently did nothing at all. Finding 6 of `01-EXTERNAL-AUDIT.md`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPathMapping {
    /// The path exactly as it appeared in `argv`.
    pub executed_path: String,
    /// The matching entry in this manifest's own `artifacts` array, e.g. `cyclictest.hist`.
    pub artifact_path: String,
}

/// One external tool the harness shelled out to, e.g. `cyclictest`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ToolInvocation {
    pub name: String,
    /// As reported by the tool itself.
    pub version: String,
    /// The exact argument vector as executed, for reproduction. A previous
    /// convention rewrote any output-file path to be relative to the run
    /// directory; that rewrite ran against the wrong directory (the tool
    /// actually wrote into a scratch `tempfile::tempdir()`, not the final run
    /// directory), silently did nothing, and left every committed manifest's
    /// argv naming a `/tmp` directory that no longer exists (finding 6 of
    /// `01-EXTERNAL-AUDIT.md`). `argv` is now recorded byte for byte as
    /// executed; use `artifact_paths` to map a path in `argv` to the artifact
    /// it became. A home directory prefix in any element is replaced by the
    /// literal token `[redacted]` (T-1-06), following the same visible
    /// convention as `KernelInfo::redact_cmdline`.
    pub argv: Vec<String>,
    pub exit_code: i32,
    /// Every path in `argv` that became a committed artifact, paired with the
    /// artifact's name in this manifest. Empty for a tool that wrote no file
    /// (`hwlatdetect` writes its report to stdout). Defaults to empty so
    /// manifests written before this field existed keep parsing.
    #[serde(default)]
    pub artifact_paths: Vec<ArtifactPathMapping>,
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

    /// `energy_performance_preference` must round-trip when present, when
    /// explicitly `None`, and when the key is omitted entirely from the input
    /// (this crate's Option-field convention, D-16): a manifest predating this
    /// field, or hardware without the sysfs knob, must still deserialize.
    #[test]
    fn cpu_governor_energy_performance_preference_round_trips() {
        let with_epp = CpuGovernor {
            cpu: 6,
            governor: "powersave".to_string(),
            energy_performance_preference: Some("performance".to_string()),
        };
        let json = serde_json::to_string(&with_epp).expect("CpuGovernor must serialize");
        assert!(json.contains("\"energy_performance_preference\":\"performance\""));
        let restored: CpuGovernor =
            serde_json::from_str(&json).expect("CpuGovernor must deserialize");
        assert_eq!(restored, with_epp);

        let without_epp = CpuGovernor {
            cpu: 6,
            governor: "powersave".to_string(),
            energy_performance_preference: None,
        };
        let json_absent =
            serde_json::to_string(&without_epp).expect("CpuGovernor must serialize when absent");
        let restored_absent: CpuGovernor =
            serde_json::from_str(&json_absent).expect("CpuGovernor must deserialize when absent");
        assert_eq!(restored_absent, without_epp);

        let minimal_json = r#"{"cpu":6,"governor":"powersave"}"#;
        let restored_from_minimal: CpuGovernor =
            serde_json::from_str(minimal_json).expect("a missing optional key must not error");
        assert_eq!(restored_from_minimal.energy_performance_preference, None);
    }

    /// `PreconditionCheck::ALL` is a hand-written list next to a hand-written enum;
    /// nothing in the type system keeps the two in sync when a variant is added.
    /// This is the only thing that would catch a forgotten entry before
    /// `crates/capture/tests/protocol_doc.rs` (which trusts `ALL` as the ground
    /// truth) silently stopped checking the new variant against the document.
    #[test]
    fn all_contains_every_variant_exactly_once() {
        let all = [
            PreconditionCheck::NoActiveSshSessions,
            PreconditionCheck::SystemdDefaultTargetIsMultiUser,
            PreconditionCheck::DisplayManagerInactive,
            PreconditionCheck::NoGraphicalSession,
            PreconditionCheck::NoActiveLoginSessions,
            PreconditionCheck::GovernorIsPerformanceOnAllCpus,
            PreconditionCheck::NoTurboEnabled,
            PreconditionCheck::DeepCstatesDisabled,
            PreconditionCheck::IsolcpusCoversTargetCpus,
            PreconditionCheck::KernelIsRealtime,
            PreconditionCheck::RtTuningServiceActive,
            PreconditionCheck::OnAcPower,
            PreconditionCheck::ThermalHeadroomAtStart,
            PreconditionCheck::NoPackageManagerActivity,
            PreconditionCheck::TracersQuiescent,
        ];
        assert_eq!(PreconditionCheck::ALL.len(), 15);
        for variant in &all {
            assert!(
                PreconditionCheck::ALL.contains(variant),
                "PreconditionCheck::ALL is missing {variant:?}"
            );
        }
    }
}
