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
    /// What this run declared about its own thermal intent before it started. `None`
    /// on a manifest written before this field existed, or a reconstructed run, which
    /// never declared one. A harness-generated run always records `Some`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thermal_profile: Option<ThermalProfile>,
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
    /// Every firmware screen this run took, in execution order. Empty for a run that took none.
    #[serde(default)]
    pub firmware_screens: Vec<FirmwareScreen>,
    /// Per-CPU SMI counts bracketing the whole run. Absent on manifests written before this
    /// field existed and on machines where the register could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smi_counts: Option<SmiCounts>,
    /// D-16: fields a reconstructed manifest could not recover. Empty for a
    /// harness-generated run with nothing missing.
    pub absent_fields: Vec<AbsentField>,
    /// BENCH-06: a contaminated or otherwise non-headline run is still published, but
    /// excluded from the regression series. See `exclusion_reason`.
    pub excluded_from_series: bool,
    /// Required to be `Some` and non-empty when `excluded_from_series` is true; see
    /// `ValidationError::MissingExclusionReason`.
    pub exclusion_reason: Option<String>,
    /// Test-only fixture seams active for this run, by environment variable name.
    /// Empty for a real measurement.
    ///
    /// A fixture is one text read repeatedly, so every interference delta computed from
    /// one is exactly zero, which reads as a perfectly quiet machine rather than as a
    /// value that was never measured. Any entry here forces `excluded_from_series`.
    /// Finding 8 of `01-EXTERNAL-AUDIT.md`.
    #[serde(default)]
    pub fixtures_used: Vec<String>,
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

/// What a run declares about its own thermal intent, before it starts.
///
/// `HotScreen` exempts the run from [`PreconditionCheck::ThermalHeadroomAtStart`], because a
/// firmware screen saturates the machine on purpose and the 2026-08-28 screening it is
/// compared against ran at 91 to 95 C. The exemption used to key on `RunClass::Screen` and to
/// be evaluated only after the observed temperature had already exceeded the ceiling, so an
/// unintentionally hot idle screen was exempted too. Finding 5 of `01-EXTERNAL-AUDIT.md`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ThermalProfile {
    Normal,
    HotScreen,
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
    /// One entry per instrument, in execution order. Empty on manifests written before this
    /// field existed (every manifest committed before this plan). The whole-run `before`/
    /// `after`/`delta` above are retained as the outer bracket (reusing the first window's
    /// `before` and the last window's `after`); the D-15/D-24 verdict is computed from the
    /// cyclictest window's own measured elapsed time, not from the requested duration. Finding
    /// 7 of `01-EXTERNAL-AUDIT.md`.
    #[serde(default)]
    pub windows: Vec<InstrumentWindow>,
}

/// One instrument's own interference bracket.
///
/// The counters used to be sampled once before cyclictest and once after everything, while the
/// per-run-hour normalisation divided by the requested cyclictest duration alone. A 3600 s
/// cyclictest followed by a 900 s firmware screen therefore divided roughly 4500 s of counter
/// accumulation by 3600, and an early-terminated investigation run had the mismatch in the
/// other direction. Finding 7 of `01-EXTERNAL-AUDIT.md`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InstrumentWindow {
    /// The tool this window brackets, matching a `tools[].name`.
    pub instrument: String,
    /// Measured with `std::time::Instant`, from immediately before the process is spawned to
    /// immediately after it exits. Not the requested duration: a tool can exit early, late, or
    /// not at all.
    pub elapsed_seconds: f64,
    /// What the operator asked for, when the tool takes a duration. Recorded beside the
    /// measured value so a divergence is visible rather than absorbed.
    pub requested_seconds: Option<u64>,
    pub before: InterferenceSnapshot,
    pub after: InterferenceSnapshot,
    pub delta: InterferenceDelta,
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
    RtlaHwnoise,
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

/// One firmware screen: which instrument ran, exactly how, and what it observed per CPU.
///
/// Recorded per CPU because the alternative is what happened on 2026-08-28 and again on
/// 2026-09-05: a single maximum published as a machine-wide firmware floor, when every event
/// behind it named one CPU that was not among the isolated cores. See
/// `docs/rig/firmware-floor-rt-vs-stock.md`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FirmwareScreen {
    /// The instrument, e.g. `rtla-hwnoise` or `hwlatdetect`. Two instruments measure two
    /// different things and their numbers are not interchangeable.
    pub instrument: String,
    pub tool_version: String,
    /// The full argument vector as executed.
    pub argv: Vec<String>,
    /// The CPUs the instrument was asked to sample.
    pub requested_cpus: Vec<u32>,
    /// The CPUs that actually produced a row or an event. A requested CPU absent from this
    /// list was not sampled, or was sampled and observed nothing; the instrument's own
    /// output decides which, and `per_cpu_exposure_seconds` is what distinguishes them.
    pub observed_cpus: Vec<u32>,
    /// Real sampling time on each CPU, when the instrument reports it. Wall-clock duration
    /// is not exposure: a single migrating thread polling across N CPUs gives each roughly
    /// 1/N of its own polling time, and `hwlatdetect` in `mode=none` gives all of it to one.
    /// Left empty for an instrument whose own output gives no basis to compute this per CPU
    /// (never filled by dividing wall-clock duration; see `nr_capture::hwnoise`).
    pub per_cpu_exposure_seconds: Vec<CpuExposure>,
    /// The maximum the instrument reported, in microseconds, and the population it is the
    /// maximum of. `None` when the run observed nothing above threshold.
    pub max_us: Option<u64>,
    /// What `max_us` is a maximum over, in words, e.g.
    /// `threshold-exceeding sampling records on the listed CPUs`.
    pub max_population: String,
    pub events_recorded: u64,
}

/// One CPU's real sampling time within a [`FirmwareScreen`], in seconds.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CpuExposure {
    pub cpu: u32,
    pub seconds: f64,
}

/// `MSR_SMI_COUNT` (register 0x34) read on each CPU before and after a run.
///
/// An exact count of system management interrupts serviced on that CPU, with no sampling,
/// no threshold and no inference from timing gaps. It answers how many, never how long: a
/// count is not a duration and must never be reported as one.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SmiCounts {
    /// The literal register, `"0x34"`.
    pub register: String,
    pub before: Vec<CpuCounter>,
    pub after: Vec<CpuCounter>,
    pub delta: Vec<CpuCounter>,
    /// Why the read failed, when it did. `rdmsr` needs root and the `msr` module; a run on a
    /// machine without either records the reason rather than a zero.
    pub unavailable_reason: Option<String>,
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

    /// A manifest carrying a `FirmwareScreen` serialises and deserialises unchanged.
    #[test]
    fn firmware_screen_roundtrips() {
        let screen = FirmwareScreen {
            instrument: "rtla-hwnoise".to_string(),
            tool_version: "7.0.12".to_string(),
            argv: vec![
                "rtla".to_string(),
                "hwnoise".to_string(),
                "-c".to_string(),
                "6-11".to_string(),
                "-H".to_string(),
                "0-5".to_string(),
                "-P".to_string(),
                "f:99".to_string(),
                "-q".to_string(),
                "-d".to_string(),
                "900s".to_string(),
            ],
            requested_cpus: vec![6, 7, 8, 9, 10, 11],
            observed_cpus: vec![6, 7, 8, 9, 10, 11],
            per_cpu_exposure_seconds: vec![
                CpuExposure {
                    cpu: 6,
                    seconds: 44.25,
                },
                CpuExposure {
                    cpu: 7,
                    seconds: 44.25,
                },
            ],
            max_us: Some(2),
            max_population: "threshold-exceeding sampling records on the listed CPUs".to_string(),
            events_recorded: 7,
        };

        let json = serde_json::to_string(&screen).expect("FirmwareScreen must serialize");
        let restored: FirmwareScreen =
            serde_json::from_str(&json).expect("FirmwareScreen must deserialize");
        assert_eq!(restored, screen);
    }

    /// Before, after and delta per CPU serialise and deserialise unchanged, including the
    /// `unavailable_reason` case where the register could not be read at all (no root, no
    /// `msr` module).
    #[test]
    fn smi_counts_roundtrip() {
        let counts = SmiCounts {
            register: "0x34".to_string(),
            before: vec![CpuCounter {
                cpu: 6,
                count: 4006,
            }],
            after: vec![CpuCounter {
                cpu: 6,
                count: 4006,
            }],
            delta: vec![CpuCounter { cpu: 6, count: 0 }],
            unavailable_reason: None,
        };
        let json = serde_json::to_string(&counts).expect("SmiCounts must serialize");
        let restored: SmiCounts = serde_json::from_str(&json).expect("SmiCounts must deserialize");
        assert_eq!(restored, counts);

        let unavailable = SmiCounts {
            register: "0x34".to_string(),
            before: vec![],
            after: vec![],
            delta: vec![],
            unavailable_reason: Some("rdmsr: not running as root".to_string()),
        };
        let json_unavailable =
            serde_json::to_string(&unavailable).expect("SmiCounts must serialize when unavailable");
        let restored_unavailable: SmiCounts = serde_json::from_str(&json_unavailable)
            .expect("SmiCounts must deserialize when unavailable");
        assert_eq!(restored_unavailable, unavailable);
    }

    /// The eight manifests committed before plan 01-20 added `firmware_screens` and
    /// `smi_counts` to the schema, named explicitly rather than discovered by globbing
    /// `measurements/`: plan 01-23 committed the first manifests that legitimately DO carry
    /// both fields (real `rtla hwnoise` captures), so a live directory scan would now sweep
    /// those up too and fail on them for doing exactly what they are supposed to do. Pinning
    /// the list keeps this test meaningful forever, not just until the next real capture
    /// lands: it proves `#[serde(default)]` still parses these eight specific, historical,
    /// pre-fields manifests, which is the only thing this test ever claimed to check.
    /// Counted independently on 2026-09-05 (see plan 01-20's own `<interfaces>` block); a
    /// future plan must not add a ninth name here, since every manifest committed after
    /// plan 01-20 is expected to carry these fields.
    const MANIFESTS_PREDATING_FIRMWARE_FIELDS: &[&str] = &[
        "2026-08-28-precision3591",
        "2026-09-01-precision3591-calibration-clean",
        "2026-09-02-precision3591-calibration-contaminated",
        "2026-09-03-precision3591-calibration-clean",
        "2026-09-03-precision3591-calibration-contaminated",
        "2026-09-05-precision3591-screen",
        "2026-09-05-precision3591-screen-02",
        "2026-09-05-precision3591-screen-03",
    ];

    #[test]
    fn committed_manifests_parse_without_firmware_fields() {
        let measurements_dir =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../measurements"));
        for run_id in MANIFESTS_PREDATING_FIRMWARE_FIELDS {
            let manifest_path = measurements_dir.join(run_id).join("manifest.json");
            let text = std::fs::read_to_string(&manifest_path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", manifest_path.display()));
            assert!(
                !text.contains("firmware_screens") && !text.contains("smi_counts"),
                "{}: expected to predate the firmware fields this test is about; if it now \
                 carries them, remove it from MANIFESTS_PREDATING_FIRMWARE_FIELDS instead",
                manifest_path.display()
            );
            let manifest: RunManifest = serde_json::from_str(&text).unwrap_or_else(|err| {
                panic!("{} failed to deserialize: {err}", manifest_path.display())
            });
            assert!(
                manifest.firmware_screens.is_empty(),
                "{} predates firmware_screens and must default to empty",
                manifest_path.display()
            );
            assert!(
                manifest.smi_counts.is_none(),
                "{} predates smi_counts and must default to absent",
                manifest_path.display()
            );
        }
    }

    /// The mirror image of [`committed_manifests_parse_without_firmware_fields`], added by
    /// plan 01-23: every manifest committed after plan 01-20 (the D-18 re-take arms) DOES
    /// carry both fields, on disk, not defaulted. A manifest satisfying neither list would
    /// be a real gap in this pair's coverage; `firmware_cpu_coverage.rs`'s
    /// `hwnoise_captures_cover_the_isolated_cores` separately checks what those fields say,
    /// not merely that they are present.
    #[test]
    fn newer_manifests_carry_firmware_fields_on_disk() {
        let measurements_dir =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../measurements"));
        let predates: std::collections::HashSet<&str> = MANIFESTS_PREDATING_FIRMWARE_FIELDS
            .iter()
            .copied()
            .collect();
        let mut checked = 0;
        for entry in std::fs::read_dir(measurements_dir)
            .expect("measurements/ must exist for this test to mean anything")
        {
            let entry = entry.expect("readable directory entry");
            let path = entry.path();
            let Some(run_id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if predates.contains(run_id) {
                continue;
            }
            let manifest_path = path.join("manifest.json");
            let Ok(text) = std::fs::read_to_string(&manifest_path) else {
                continue; // a failed-attempt directory has no manifest.json at all
            };
            assert!(
                text.contains("firmware_screens") || text.contains("smi_counts"),
                "{}: not in MANIFESTS_PREDATING_FIRMWARE_FIELDS but also carries neither \
                 field on disk; add it to that list if it genuinely predates plan 01-20, or \
                 investigate why a post-01-20 run has neither",
                manifest_path.display()
            );
            checked += 1;
        }
        assert!(
            checked >= 4,
            "expected at least the four post-01-20 manifests plan 01-23 committed; found {checked}"
        );
    }
}
