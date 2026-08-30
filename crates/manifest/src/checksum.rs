//! blake3 checksums over raw captures (D-12).
//!
//! blake3 checksums prove integrity, not authenticity. They detect accidental
//! corruption and prove a capture matches its manifest. They do not prove who
//! produced the capture. There is no signing story in v1 (T-1-07).

use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use thiserror::Error;

use crate::fields::{ArtifactRecord, StorageLocation};
use crate::validate::ValidationError;

/// Read in 64 KiB chunks so a large ftrace capture does not need to be fully
/// resident.
const CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum ChecksumError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

/// Streaming blake3 digest of `path`, returned as lowercase hex.
pub fn blake3_file(path: &Path) -> Result<String, ChecksumError> {
    let mut file = File::open(path).map_err(|source| ChecksumError::Io {
        path: path.display().to_string(),
        source,
    })?;

    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; CHUNK_SIZE];
    loop {
        let read = file.read(&mut buf).map_err(|source| ChecksumError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Verifies one artifact record against the filesystem under `run_dir`: the recorded
/// path is safe to join (ASVS V12, T-1-16), the file exists for an in-repo artifact,
/// and its blake3 matches the recorded digest.
pub fn verify_artifact(run_dir: &Path, record: &ArtifactRecord) -> Result<(), ValidationError> {
    let relative =
        safe_relative_path(&record.path).ok_or_else(|| ValidationError::UnsafeArtifactPath {
            path: record.path.clone(),
        })?;

    match &record.stored {
        StorageLocation::External { url } => {
            if url.trim().is_empty() {
                return Err(ValidationError::ArtifactMissing {
                    path: record.path.clone(),
                });
            }
            Ok(())
        }
        StorageLocation::InRepo => {
            let full_path = run_dir.join(&relative);
            if !full_path.is_file() {
                return Err(ValidationError::ArtifactMissing {
                    path: record.path.clone(),
                });
            }

            let computed =
                blake3_file(&full_path).map_err(|_| ValidationError::ArtifactMissing {
                    path: record.path.clone(),
                })?;
            if computed != record.blake3 {
                return Err(ValidationError::ChecksumMismatch {
                    path: record.path.clone(),
                    recorded: record.blake3.clone(),
                    computed,
                });
            }
            Ok(())
        }
    }
}

/// Rejects an absolute path or one containing a `..` component before it is ever
/// joined to a run directory.
fn safe_relative_path(path: &str) -> Option<PathBuf> {
    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return None;
    }
    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return None;
    }
    Some(candidate.to_path_buf())
}
