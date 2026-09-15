//! Library surface for `nr-stop-harness`.
//!
//! The package's own `Cargo.toml` declares only a `[[bin]]` target in its literal text; Cargo
//! infers this library target purely from this file's presence, with the package's default
//! library name (`nr_stop_harness`). The file exists so `crates/stop-harness/tests/` can import
//! `clock`, `sched` and `characterise` directly and exercise their platform-independent halves
//! with `cargo test` on the macOS CI leg, rather than needing a subprocess/CLI harness for every
//! internal function this plan's own behavior list requires testing (Rule 3, blocking: without
//! this file, none of this plan's own mandated `cargo test -p nr-stop-harness` commands compile
//! at all). `main.rs` is this library's only other consumer.

pub mod capture;
pub mod characterise;
pub mod clock;
pub mod report;
pub mod rundir;
pub mod sched;
pub mod trial;
