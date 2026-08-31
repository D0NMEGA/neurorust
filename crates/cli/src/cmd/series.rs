//! `nrmeasure series`: append runs to the metrics series and compare against the
//! baseline (BENCH-08, D-11). Implemented in plan 01-14; this plan only declares
//! the argument surface so `main.rs`'s subcommand set is stable across both plans.

use std::path::PathBuf;

use clap::Args as ClapArgs;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Root directory containing run directories to fold into the series.
    #[arg(long, default_value = "./measurements")]
    pub measurements_root: PathBuf,
}

pub fn run(_args: Args) -> anyhow::Result<i32> {
    anyhow::bail!("nrmeasure series is not implemented until plan 01-14")
}
