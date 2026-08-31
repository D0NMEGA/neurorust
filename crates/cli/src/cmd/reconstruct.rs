//! `nrmeasure reconstruct`: build a manifest for a capture taken before the harness
//! existed (D-16). Implemented in plan 01-08; this plan only declares the argument
//! surface so `main.rs`'s subcommand set is stable across both plans.

use std::path::PathBuf;

use clap::Args as ClapArgs;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// The existing capture directory to build a reconstructed manifest for.
    #[arg(long)]
    pub run_dir: PathBuf,
}

pub fn run(_args: Args) -> anyhow::Result<i32> {
    anyhow::bail!("nrmeasure reconstruct is not implemented until plan 01-08")
}
