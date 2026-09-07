//! `nrmeasure attempt`: administrative corrections to an `ATTEMPT.json` the harness itself
//! never got to finish writing.
//!
//! Today this is one action, `fail`, added by plan 01-23 after a killed `rtla` orphaned
//! `measurements/2026-09-06-precision3591-screen`'s `ATTEMPT.json` at `status: in-progress`:
//! systemd's SIGTERM, from the `--hwnoise-duration` timeout-bound defect (fixed in `b29e819`),
//! landed before `cmd::run`'s own exit paths (which always rewrite `ATTEMPT.json` on the way
//! out, successful or not) got the chance to run. `nrmeasure verify --strict` correctly
//! refuses an in-progress record (`crates/manifest/src/attempt.rs`'s `AttemptStatus` doc
//! comment, T-1-60): a run that never finished is not a committable state. Before this
//! command the only way to close one out was to hand-edit the JSON, which sets a bad
//! precedent for a project whose core value is reproducible provenance.
//!
//! This performs the same rewrite `cmd::run`'s own failure path performs, with the reason and
//! the end time supplied explicitly rather than inferred from a live process, and it says so
//! plainly in the written `failure.message`: the record states both what actually happened and
//! that this command is what closed it out, and when.

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Args as ClapArgs;
use nr_manifest::{AttemptFailure, AttemptRecord, AttemptStatus};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cmd::run::scan_preserved_files;

fn parse_rfc3339(s: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|err| err.to_string())
}

fn format_rfc3339(t: OffsetDateTime) -> String {
    t.format(&Rfc3339).unwrap_or_else(|_| t.to_string())
}

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// The run directory whose ATTEMPT.json is stuck at status in-progress.
    pub run_dir: PathBuf,

    /// Why the attempt never reached a terminal status. Recorded verbatim in
    /// `failure.message`, alongside a note this command appends stating that the record was
    /// corrected after the fact rather than written live by the harness.
    #[arg(long)]
    pub reason: String,

    /// When the attempt actually stopped running, if it is known from evidence outside the
    /// record itself (RFC 3339, e.g. a documented timeout bound added to `utc_start`). Falls
    /// back to the moment this command runs, which is honest only when nothing better is
    /// known: prefer passing the real bound whenever one exists, since a killed process
    /// usually stopped long before anyone got around to closing out its record.
    #[arg(long, value_parser = parse_rfc3339)]
    pub utc_end: Option<OffsetDateTime>,
}

pub fn run(args: Args) -> Result<i32> {
    let attempt_path = args.run_dir.join("ATTEMPT.json");
    let manifest_path = args.run_dir.join("manifest.json");

    if manifest_path.is_file() {
        bail!(
            "{} already carries a manifest.json; a directory with a manifest must never also \
             get a synthesized failed ATTEMPT.json (they are mutually exclusive, see \
             crates/manifest/src/attempt.rs)",
            args.run_dir.display()
        );
    }

    let text = std::fs::read_to_string(&attempt_path)
        .with_context(|| format!("failed to read {}", attempt_path.display()))?;
    let mut record: AttemptRecord = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse {}", attempt_path.display()))?;

    if record.status != AttemptStatus::InProgress {
        bail!(
            "{} has status {:?}, not in-progress; refusing to overwrite an attempt record \
             that already reached a terminal status",
            attempt_path.display(),
            record.status
        );
    }

    let corrected_at = OffsetDateTime::now_utc();
    let utc_end = args.utc_end.unwrap_or(corrected_at);

    record.utc_end = Some(utc_end);
    record.status = AttemptStatus::Failed;
    record.preserved = scan_preserved_files(&args.run_dir);
    record.failure = Some(AttemptFailure {
        stage: "orphaned-in-progress".to_string(),
        message: format!(
            "{reason} This ATTEMPT.json was left at status in-progress because the harness's \
             own exit paths (which always rewrite the record as completed or failed) never \
             ran. `nrmeasure attempt fail` corrected it to failed at {corrected} (RFC 3339), \
             after the fact, rather than a hand edit; utc_end above is {utc_end_source}.",
            reason = args.reason,
            corrected = format_rfc3339(corrected_at),
            utc_end_source = if args.utc_end.is_some() {
                "the documented stop time passed to this command, not the correction time"
            } else {
                "the correction time itself, the best available estimate"
            },
        ),
    });
    record.usable_for_numerical_analysis = false;

    let json = serde_json::to_string_pretty(&record)
        .context("failed to serialise the corrected ATTEMPT.json")?;
    std::fs::write(&attempt_path, json)
        .with_context(|| format!("failed to write {}", attempt_path.display()))?;

    println!(
        "{}: marked failed (utc_end {})",
        args.run_dir.display(),
        format_rfc3339(utc_end)
    );

    Ok(0)
}
