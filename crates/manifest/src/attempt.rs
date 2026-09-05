//! `ATTEMPT.json`: the durable record of one execution of `nrmeasure run`, written
//! before the first instrument is spawned and updated on every exit path.
//!
//! Raw output used to live in a `tempfile::tempdir()` until parsing, reconciliation
//! and the contamination verdict had all succeeded, so any failure among them
//! deleted the capture; an hour of rig time then survived only as a line on
//! stderr. Finding 7 of `01-EXTERNAL-AUDIT.md`.
//!
//! [`AttemptRecord`] is deliberately a separate root type from [`crate::RunManifest`],
//! never a variant of it: a manifest is the record of a measurement, an attempt
//! record is the record of an execution, and conflating the two would mean a failed
//! attempt produced something shaped like a measurement. A directory may hold a
//! `manifest.json`, or an `ATTEMPT.json` with `status: failed`, but never both
//! (`crates/cli/src/cmd/verify.rs`'s check 1 rejects that combination as a defect).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::fields::{ArtifactRecord, HarnessInfo, InstrumentClass, RunClass, ToolInvocation};

/// Current version of this schema. Bump on any breaking change to [`AttemptRecord`].
pub const ATTEMPT_SCHEMA_VERSION: u32 = 1;

/// The durable record of one execution of `nrmeasure run`, written before the first
/// instrument is spawned and updated on every exit path.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecord {
    pub schema_version: u32,
    pub run_id: String,
    #[serde(with = "time::serde::rfc3339")]
    #[schemars(with = "String")]
    pub utc_start: OffsetDateTime,
    /// `None` while `status` is `in-progress`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    #[schemars(with = "Option<String>")]
    pub utc_end: Option<OffsetDateTime>,
    pub status: AttemptStatus,
    pub harness: HarnessInfo,
    /// What the operator asked for: run class, instrument class, CPU list, and each
    /// instrument's requested duration. Recorded up front so a failed attempt still
    /// says what it was trying to do.
    pub requested: RequestedRun,
    /// Every tool that was spawned, with its exit code, as far as execution got.
    pub tools: Vec<ToolInvocation>,
    /// Every file kept from this attempt, checksummed. On a failure this is the
    /// partial output and the captured stderr; on success it is the same set the
    /// manifest lists.
    pub preserved: Vec<ArtifactRecord>,
    pub failure: Option<AttemptFailure>,
    /// False whenever the numbers could not be produced. A reader must be able to
    /// tell a run that measured nothing from a run that measured zero.
    pub usable_for_numerical_analysis: bool,
}

/// The lifecycle of one [`AttemptRecord`]. `InProgress` is a strict-verification
/// failure by design (`crates/cli/src/cmd/verify.rs`, T-1-60): a run that never
/// finished is not a committable state.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AttemptStatus {
    InProgress,
    Completed,
    Failed,
}

/// Where in the pipeline an attempt failed and why, recorded verbatim rather than
/// inferred after the fact.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttemptFailure {
    /// Which step failed, one of: `tool-spawn`, `tool-exit`, `parse`, `reconcile`,
    /// `verdict`, `environment-snapshot`, `manifest-write`.
    pub stage: String,
    /// The error as reported, verbatim (including its full `anyhow` context chain).
    pub message: String,
}

/// What the operator asked `nrmeasure run` to do, captured before any instrument
/// starts so a failed attempt still says what it was trying to measure.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RequestedRun {
    pub run_class: RunClass,
    pub instrument_class: InstrumentClass,
    /// The measurement CPU list, e.g. `"6-11"`.
    pub cpus: String,
    /// The cyclictest main thread affinity, e.g. `"0,1"`.
    pub main_cpus: String,
    pub duration_seconds: u64,
    pub with_hwlatdetect: bool,
    /// `Some` only when `with_hwlatdetect` is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hwlatdetect_duration_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fields::GitShaSource;

    fn sample_harness() -> HarnessInfo {
        HarnessInfo {
            version: "0.1.0".to_string(),
            git_sha: "0".repeat(40),
            git_dirty: false,
            git_sha_source: Some(GitShaSource::BuildTime),
            executable_blake3: None,
            executable_bytes: None,
            invoked_from_git_sha: None,
        }
    }

    fn sample_requested() -> RequestedRun {
        RequestedRun {
            run_class: RunClass::Recon,
            instrument_class: InstrumentClass::HeadlineSeries,
            cpus: "6-11".to_string(),
            main_cpus: "0,1".to_string(),
            duration_seconds: 60,
            with_hwlatdetect: false,
            hwlatdetect_duration_seconds: None,
        }
    }

    #[test]
    fn in_progress_record_round_trips_with_no_end_time_and_no_failure() {
        let record = AttemptRecord {
            schema_version: ATTEMPT_SCHEMA_VERSION,
            run_id: "2026-09-06-precision3591-recon".to_string(),
            utc_start: OffsetDateTime::now_utc(),
            utc_end: None,
            status: AttemptStatus::InProgress,
            harness: sample_harness(),
            requested: sample_requested(),
            tools: Vec::new(),
            preserved: Vec::new(),
            failure: None,
            usable_for_numerical_analysis: false,
        };

        let json = serde_json::to_string_pretty(&record).expect("serialises");
        assert!(
            !json.contains("\"utc_end\""),
            "utc_end must be omitted, not null, while in progress: {json}"
        );
        let restored: AttemptRecord = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(restored, record);
    }

    #[test]
    fn failed_record_round_trips_with_a_named_stage() {
        let mut record = AttemptRecord {
            schema_version: ATTEMPT_SCHEMA_VERSION,
            run_id: "2026-09-06-precision3591-recon".to_string(),
            utc_start: OffsetDateTime::now_utc(),
            utc_end: Some(OffsetDateTime::now_utc()),
            status: AttemptStatus::Failed,
            harness: sample_harness(),
            requested: sample_requested(),
            tools: Vec::new(),
            preserved: Vec::new(),
            failure: Some(AttemptFailure {
                stage: "parse".to_string(),
                message: "failed to parse cyclictest's .hist output".to_string(),
            }),
            usable_for_numerical_analysis: false,
        };

        let json = serde_json::to_string_pretty(&record).expect("serialises");
        let restored: AttemptRecord = serde_json::from_str(&json).expect("deserialises");
        assert_eq!(restored, record);

        // deny_unknown_fields: a stray key must be rejected, matching every other
        // manifest-family type's contract.
        record.schema_version += 1;
        let mut value: serde_json::Value = serde_json::from_str(&json).expect("parses as JSON");
        value["unexpected"] = serde_json::Value::Bool(true);
        assert!(
            serde_json::from_value::<AttemptRecord>(value).is_err(),
            "an unknown field must be rejected"
        );
    }
}
