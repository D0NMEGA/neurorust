//! `nrmeasure run`: execute a measurement run and write a stamped run directory.
//! The argument surface and orchestration are built out in task 2 of this plan;
//! this placeholder exists only so task 1's `main.rs` (which types
//! `Command::Run(cmd::run::Args)`) compiles on its own.

use clap::Args as ClapArgs;

#[derive(ClapArgs, Debug)]
pub struct Args {}

pub fn run(_args: Args) -> anyhow::Result<i32> {
    anyhow::bail!("nrmeasure run is implemented later in this plan (task 2)")
}
