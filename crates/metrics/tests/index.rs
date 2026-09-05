//! Proves that making `RunSummary::p99_us`/`max_us` `Option<u64>` did not change a single byte
//! of the committed `measurements/INDEX.md`: this crate's own `render_index`, driven from the
//! eight real committed manifests, must reproduce the file exactly.
//!
//! `nrmeasure verify --write-index` against the real tree (`crates/cli/src/cmd/verify.rs`) is
//! the second, independent proof named in this plan; this one runs at the nr-metrics level with
//! no dependency on the cli crate, so a bug shared between the two is unlikely to hide in both
//! places at once. Finding 6 of `01-EXTERNAL-AUDIT.md`.

use std::fs;
use std::path::{Path, PathBuf};

use nr_manifest::{ArtifactKind, RunManifest};
use nr_metrics::index::{RunOutcome, RunSummary, render_index};
use time::macros::format_description;

fn measurements_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../measurements")
}

/// Independently mirrors `crates/cli/src/cmd/verify.rs`'s own `compute_percentiles`: locates
/// the run's cyclictest histogram artifact and computes p99/max from it. Kept separate (no cli
/// dependency here) rather than shared, so this really is an independent re-derivation.
fn compute_percentiles(run_dir: &Path, manifest: &RunManifest) -> Option<(u64, u64)> {
    let artifact = manifest
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == ArtifactKind::CyclictestHist)?;
    let hist_path = run_dir.join(&artifact.path);
    let run = nr_histogram::hist::parse_hist_file(&hist_path, None).ok()?;
    let percentiles = run.percentiles(&[0.99]).ok()?;
    let &(_, p99_us) = percentiles.values.first()?;
    Some((p99_us, percentiles.max_us))
}

fn load_summaries(measurements_root: &Path) -> Vec<RunSummary> {
    let mut names: Vec<String> = fs::read_dir(measurements_root)
        .expect("measurements/ exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();

    names
        .into_iter()
        .map(|name| {
            let run_dir = measurements_root.join(&name);
            let manifest: RunManifest = serde_json::from_str(
                &fs::read_to_string(run_dir.join("manifest.json")).expect("read manifest.json"),
            )
            .expect("manifest.json parses");

            let (p99_us, max_us) = match compute_percentiles(&run_dir, &manifest) {
                Some((p99, max)) => (Some(p99), Some(max)),
                None => (None, None),
            };
            let date_format = format_description!("[year]-[month]-[day]");
            let date = manifest
                .utc_start
                .date()
                .format(&date_format)
                .expect("utc_start formats");

            RunSummary {
                date,
                run_id: manifest.run_id.clone(),
                run_class: manifest.run_class.clone(),
                instrument_class: manifest.instrument_class.clone(),
                provenance_tier: manifest.provenance_tier.clone(),
                verdict: Some(manifest.interference.verdict.clone()),
                p99_us,
                max_us,
                in_series: !manifest.excluded_from_series,
                reason: manifest.exclusion_reason.clone(),
                outcome: RunOutcome::Measured,
            }
        })
        .collect()
}

#[test]
fn committed_index_is_unchanged_by_optional_columns() {
    let root = measurements_root();
    let summaries = load_summaries(&root);
    let rendered = render_index(&summaries);
    let committed = fs::read_to_string(root.join("INDEX.md")).expect("read INDEX.md");
    assert_eq!(
        rendered, committed,
        "making p99_us/max_us Option<u64> must not change a single byte of the committed index"
    );
}
