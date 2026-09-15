//! The local run directory equivalent for `nr-stop-harness`.
//!
//! A deliberate copy of `crates/cli/src/rundir.rs`'s naming and creation logic: the same
//! `<YYYY-MM-DD>-<rig-slug>-<run-class>[-NN]` convention, the same disambiguation-by-directory-
//! existence, and the same refusal on an existing manifest. `crates/cli`'s own `Cargo.toml`
//! declares only a `[[bin]]` target, so `RunDir` and its helpers cannot be imported from there at
//! any visibility; this is not a missing `pub` to fix. Promoting this logic into `nr-manifest` is
//! the right move when a third caller needs it, per this project's own copy-twice-before-
//! abstracting rule; for two callers, a second small file costs less than widening a shared
//! crate's public API for one.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use nr_manifest::RunClass;
use thiserror::Error;
use time::OffsetDateTime;
use time::macros::format_description;

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

/// A resolved, created run directory. `run_id` equals the directory's own name and satisfies
/// `nr_manifest`'s run_id charset rule (`^[a-z0-9][a-z0-9-]{0,63}$`).
#[derive(Debug)]
pub struct RunDir {
    pub path: PathBuf,
    pub run_id: String,
}

impl RunDir {
    /// Computes the deterministic name (disambiguating against sibling directories already
    /// present under `root`), validates it against `nr_manifest`'s run_id charset rule, creates
    /// the directory, and returns a handle. A bad rig slug therefore fails before any filesystem
    /// write.
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
        // Belt and suspenders: `next_available_name` only ever proposes a name not already
        // present in `existing`, so this should be unreachable outside a race between two
        // concurrent invocations.
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
/// `RunClass::Headline` -> `"headline"`, matching `crates/cli/src/rundir.rs::run_class_slug`
/// exactly. `pub` so `report.rs` can render the same two words without a third copy of this
/// three-line trick.
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
/// Disambiguation is driven by directory existence alone: a directory occupying a name, even
/// one left behind by an interrupted run with no manifest yet, is never silently reused, so an
/// in-flight or crashed run can never be clobbered.
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

/// `nr_manifest`'s run_id charset rule, checked by hand rather than with a regex dependency,
/// matching `nr_manifest::validate`'s own approach.
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
/// [`RunDir::create`], and again by the caller immediately before the final manifest write
/// (`manifest.json` is written last), closing the gap between directory creation and that final
/// write.
pub fn refuse_if_manifest_exists(dir: &Path) -> Result<(), RunDirError> {
    if dir.join("manifest.json").exists() {
        return Err(RunDirError::AlreadyPopulated {
            path: dir.to_path_buf(),
        });
    }
    Ok(())
}
