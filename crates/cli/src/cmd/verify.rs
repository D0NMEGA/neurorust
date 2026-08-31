//! `nrmeasure verify`: validate every run directory under `measurements/` against
//! its manifest. Implemented in plan 01-08; this plan only declares the argument
//! surface so `main.rs`'s subcommand set is stable across both plans.

use std::path::PathBuf;

use clap::Args as ClapArgs;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Root directory containing run directories to verify.
    #[arg(long, default_value = "./measurements")]
    pub measurements_root: PathBuf,
}

pub fn run(_args: Args) -> anyhow::Result<i32> {
    anyhow::bail!("nrmeasure verify is not implemented until plan 01-08")
}
