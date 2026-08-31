//! The `measurements/` directory layout: deterministic naming, disambiguation, and
//! the in-repo versus external-pointer size policy.
//!
//! Directory name: `<YYYY-MM-DD>-<rig-slug>-<run-class>[-NN]`. The date is the UTC
//! date of the run start. `-NN` is a two-digit sequence appended only from the
//! second run of the same date, rig and class onward, starting at `-02`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use nr_manifest::RunClass;
use thiserror::Error;
use time::OffsetDateTime;
use time::macros::format_description;

/// A capture file over this size is recorded as `StorageLocation::External` rather
/// than copied into the run directory. `cyclictest`/`hwlatdetect` captures never
/// approach this; the threshold exists for the larger ftrace/trace-cmd captures a
/// PLAT-01 investigation run can produce.
pub const MAX_IN_REPO_FILE_BYTES: u64 = 25 * 1024 * 1024;

/// A run directory whose total in-repo bytes would exceed this after adding a file
/// records that file externally instead, even when the file itself is under
/// [`MAX_IN_REPO_FILE_BYTES`].
pub const MAX_IN_REPO_RUN_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum RunDirError {
    #[error("invalid run id {run_id:?}: {reason}")]
    InvalidRunId { run_id: String, reason: String },
    #[error("{path} already contains a manifest.json", path = .path.display())]
    AlreadyPopulated { path: PathBuf },
    #[error("failed to create directory {path}: {source}", path = .path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// A resolved, created run directory. `run_id` equals the directory's own name and
/// satisfies `nr_manifest`'s run_id charset rule (`^[a-z0-9][a-z0-9-]{0,63}$`).
#[derive(Debug)]
pub struct RunDir {
    pub path: PathBuf,
    pub run_id: String,
}

impl RunDir {
    /// Computes the deterministic name (disambiguating against sibling directories
    /// already present under `root`), validates it against `nr_manifest`'s run_id
    /// charset rule, creates the directory, and returns a handle. A bad rig slug
    /// therefore fails before any filesystem write.
    pub fn create(
        root: &Path,
        rig_slug: &str,
        run_class: &RunClass,
        utc_start: OffsetDateTime,
    ) -> Result<RunDir, RunDirError> {
        let base = base_name(rig_slug, run_class, utc_start);
        let existing = existing_names(root);
        let run_id = next_available_name(&base, &existing);

        validate_run_id(&run_id)?;

        let path = root.join(&run_id);
        // Belt and suspenders: `next_available_name` only ever proposes a name not
        // already present in `existing`, so this should be unreachable outside a
        // race between two concurrent invocations. Checked directly rather than
        // assumed, since a silently overwritten prior run is exactly the kind of
        // mistake D-12 (raw captures stay byte-identical) exists to prevent.
        refuse_if_manifest_exists(&path)?;

        std::fs::create_dir_all(&path).map_err(|source| RunDirError::Io {
            path: path.clone(),
            source,
        })?;

        Ok(RunDir { path, run_id })
    }
}

fn base_name(rig_slug: &str, run_class: &RunClass, utc_start: OffsetDateTime) -> String {
    let date_format = format_description!("[year]-[month]-[day]");
    let date = utc_start
        .date()
        .format(&date_format)
        .unwrap_or_else(|_| "0000-00-00".to_string());
    format!("{date}-{rig_slug}-{}", run_class_slug(run_class))
}

/// Renders a [`RunClass`] the same way its JSON form does (kebab-case), e.g.
/// `RunClass::CalibrationClean` -> `"calibration-clean"`, so a directory name and
/// the manifest's own `run_class` field always agree.
pub fn run_class_slug(run_class: &RunClass) -> String {
    match serde_json::to_value(run_class) {
        Ok(serde_json::Value::String(s)) => s,
        other => format!("{other:?}"),
    }
}

fn existing_names(root: &Path) -> BTreeSet<String> {
    std::fs::read_dir(root)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The first of `base`, `base-02`, `base-03`, ... not already present in `existing`.
/// Disambiguation is driven by directory existence alone: a directory occupying a
/// name, even one left behind by an interrupted run with no manifest yet, is never
/// silently reused, so an in-flight or crashed run can never be clobbered.
fn next_available_name(base: &str, existing: &BTreeSet<String>) -> String {
    if !existing.contains(base) {
        return base.to_string();
    }
    let mut n = 2u32;
    loop {
        let candidate = format!("{base}-{n:02}");
        if !existing.contains(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// `nr_manifest`'s run_id charset rule, checked by hand rather than with a regex
/// dependency, matching `nr_manifest::validate`'s own approach.
fn validate_run_id(run_id: &str) -> Result<(), RunDirError> {
    let mut chars = run_id.chars();
    let ok = match chars.next() {
        Some(first) if first.is_ascii_lowercase() || first.is_ascii_digit() => {
            chars.clone().count() <= 63
                && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        }
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err(RunDirError::InvalidRunId {
            run_id: run_id.to_string(),
            reason: "must match ^[a-z0-9][a-z0-9-]{0,63}$".to_string(),
        })
    }
}

/// Refuses if `dir` already contains a `manifest.json`. Called defensively inside
/// [`RunDir::create`], and again by `cmd::run` immediately before the final manifest
/// write (manifest.json is written last), closing the gap between directory
/// creation and that final write.
pub fn refuse_if_manifest_exists(dir: &Path) -> Result<(), RunDirError> {
    if dir.join("manifest.json").exists() {
        return Err(RunDirError::AlreadyPopulated {
            path: dir.to_path_buf(),
        });
    }
    Ok(())
}

/// Whether a file this size should be recorded as `StorageLocation::External` rather
/// than copied into the run directory: over the per-file limit, or pushing the
/// running per-run total (bytes already placed in this run directory) over the
/// per-run limit.
pub fn exceeds_in_repo_limit(file_bytes: u64, run_bytes_so_far: u64) -> bool {
    file_bytes > MAX_IN_REPO_FILE_BYTES || run_bytes_so_far + file_bytes > MAX_IN_REPO_RUN_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc_midnight(year: i32, month: time::Month, day: u8) -> OffsetDateTime {
        let date = time::Date::from_calendar_date(year, month, day).expect("valid date");
        OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
    }

    #[test]
    fn run_dir_name_is_deterministic() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let start = utc_midnight(2026, time::Month::September, 6);

        let run_dir = RunDir::create(tmp.path(), "precision3591", &RunClass::Weekly, start)
            .expect("create succeeds");

        assert_eq!(run_dir.run_id, "2026-09-06-precision3591-weekly");
        assert!(run_dir.path.is_dir());
    }

    #[test]
    fn run_dir_name_disambiguates() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let start = utc_midnight(2026, time::Month::September, 6);

        let first = RunDir::create(tmp.path(), "precision3591", &RunClass::Weekly, start)
            .expect("first run creates");
        std::fs::write(first.path.join("manifest.json"), "{}").expect("write manifest");

        let second = RunDir::create(tmp.path(), "precision3591", &RunClass::Weekly, start)
            .expect("second run creates");
        assert_eq!(second.run_id, "2026-09-06-precision3591-weekly-02");
        std::fs::write(second.path.join("manifest.json"), "{}").expect("write manifest");

        let third = RunDir::create(tmp.path(), "precision3591", &RunClass::Weekly, start)
            .expect("third run creates");
        assert_eq!(third.run_id, "2026-09-06-precision3591-weekly-03");
    }

    #[test]
    fn run_dir_refuses_existing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let populated = tmp.path().join("2026-09-06-precision3591-weekly");
        std::fs::create_dir_all(&populated).expect("mkdir");
        std::fs::write(populated.join("manifest.json"), "{}").expect("write manifest");

        let err = refuse_if_manifest_exists(&populated).expect_err("must refuse");
        assert!(matches!(err, RunDirError::AlreadyPopulated { .. }));
    }

    #[test]
    fn run_id_matches_directory() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let start = utc_midnight(2026, time::Month::September, 6);

        let run_dir = RunDir::create(tmp.path(), "precision3591", &RunClass::Recon, start)
            .expect("create succeeds");

        assert_eq!(
            run_dir.path.file_name().and_then(|n| n.to_str()),
            Some(run_dir.run_id.as_str())
        );

        let mut chars = run_dir.run_id.chars();
        let first = chars.next().expect("non-empty run_id");
        assert!(first.is_ascii_lowercase() || first.is_ascii_digit());
        assert!(chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
    }

    #[test]
    fn oversize_file_becomes_external() {
        assert!(exceeds_in_repo_limit(MAX_IN_REPO_FILE_BYTES + 1, 0));
        assert!(!exceeds_in_repo_limit(1024, 0));
        assert!(exceeds_in_repo_limit(1024, MAX_IN_REPO_RUN_BYTES));
    }
}
