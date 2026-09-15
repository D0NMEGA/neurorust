//! `nr-stop-harness`: the STOP-07 abort latency harness. Two subcommands. `characterise`
//! measures the clock's own read overhead and the cross-core offset between two pinned threads
//! (D-35) and is wired up end to end here. `abort-latency` measures abort latency itself
//! (D-31 through D-34); plan 02-06 task 3 gives it its real flags and behaviour.

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};

use nr_stop_harness::characterise::{cross_core_offset_ns, current_clocksource, read_overhead_ns};
use nr_stop_harness::clock::RawClock;

#[derive(Debug, Parser)]
#[command(
    name = "nr-stop-harness",
    about = "STOP-07 abort latency harness for the neurorust reference rig"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Measures the clock's own read overhead and the cross-core offset between two pinned
    /// threads (D-35). Prints raw samples to stdout, one per line.
    Characterise {
        /// Consecutive clock reads to time for the read-overhead sample.
        #[arg(long, default_value_t = 1_000_000)]
        iterations: usize,
        /// First cpu of the cross-core offset pair.
        #[arg(long, default_value_t = 7)]
        cpu_a: usize,
        /// Second cpu of the cross-core offset pair.
        #[arg(long, default_value_t = 8)]
        cpu_b: usize,
        /// Ping-pong rounds for the cross-core offset sample.
        #[arg(long, default_value_t = 1000)]
        rounds: usize,
        /// Root to read the running clocksource from. Override for testing; the real rig reads
        /// this from `/sys`.
        #[arg(long, default_value = "/sys")]
        sys_root: PathBuf,
    },
    /// Measures abort latency for one poll period (D-31 through D-34). Not yet implemented;
    /// plan 02-06 task 3 wires this up.
    AbortLatency,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Characterise {
            iterations,
            cpu_a,
            cpu_b,
            rounds,
            sys_root,
        } => run_characterise(iterations, cpu_a, cpu_b, rounds, &sys_root),
        Command::AbortLatency => {
            anyhow::bail!("abort-latency is not yet implemented; see plan 02-06 task 3")
        }
    }
}

fn run_characterise(
    iterations: usize,
    cpu_a: usize,
    cpu_b: usize,
    rounds: usize,
    sys_root: &Path,
) -> anyhow::Result<()> {
    let clock = RawClock;

    let overhead = read_overhead_ns(&clock, iterations).context("measuring clock read overhead")?;
    for sample in &overhead {
        println!("read_overhead_ns\t{sample}");
    }

    let offsets = cross_core_offset_ns(&clock, cpu_a, cpu_b, rounds)
        .context("measuring cross-core offset")?;
    for sample in &offsets {
        println!("cross_core_offset_ns\t{sample}");
    }

    match current_clocksource(sys_root) {
        Some(source) => println!("current_clocksource\t{source}"),
        None => println!("current_clocksource\tunavailable"),
    }

    Ok(())
}
