//! `nr-stop-harness`: the STOP-07 abort latency harness. Two subcommands. `characterise`
//! measures the clock's own read overhead and the cross-core offset between two pinned threads
//! (D-35) and is wired up end to end here. `abort-latency` measures abort latency itself
//! (D-31 through D-34); plan 02-06 task 3 gives it its real flags and behaviour.

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};

use nr_stop_harness::characterise::{cross_core_offset_ns, current_clocksource, read_overhead_ns};
use nr_stop_harness::clock::RawClock;
use nr_stop_harness::trial::{TrialConfig, run_trials};

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
    /// Measures abort latency for one poll period (D-31 through D-34) and prints one TSV row
    /// per trial to stdout. Run once per period so each gets its own raw capture; plan 02-08
    /// runs it against the two periods this project measures.
    AbortLatency {
        /// The node iteration period this run measures, in nanoseconds. No default: the two
        /// periods this project publishes are 33_000 ns, one frame period at the 30 kHz target
        /// rate, and 1_000_000 ns, a 1 kHz node, and this subcommand runs once per period
        /// rather than looping over both internally, so the caller always states which one a
        /// given run and its raw capture belong to.
        #[arg(long)]
        period_ns: u64,
        /// Number of abort trials. A TrialRow renders to roughly 55 bytes of TSV; the in-repo
        /// capture-file limit is 25 MiB and the whole run-directory limit is 100 MiB, so a
        /// single capture file holds roughly 470,000 rows before the run directory logic would
        /// push it behind an external pointer. 200,000 keeps two periods comfortably inside one
        /// run directory with room for the manifest and the report; raise it with that trade in
        /// mind.
        #[arg(long, default_value_t = 200_000)]
        trials: usize,
        /// Core the hot (polling) thread pins to. 7 is inside the rig's isolated set 6-11.
        #[arg(long, default_value_t = 7)]
        hot_cpu: usize,
        /// Core the abort thread pins to. 8 is inside the rig's isolated set 6-11.
        #[arg(long, default_value_t = 8)]
        abort_cpu: usize,
        /// SCHED_FIFO priority for both threads. 80, not 99: the two threads sit on separate
        /// isolated cores and never contend, so nothing is bought by taking the top priority,
        /// and 99 stays free for anything that must preempt.
        #[arg(long, default_value_t = 80)]
        priority: u8,
        /// Seed for the abort-phase sequence, recorded so a run is reproducible.
        #[arg(long, default_value_t = 1)]
        seed: u64,
    },
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
        Command::AbortLatency {
            period_ns,
            trials,
            hot_cpu,
            abort_cpu,
            priority,
            seed,
        } => run_abort_latency(period_ns, trials, hot_cpu, abort_cpu, priority, seed),
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

#[allow(clippy::too_many_arguments)]
fn run_abort_latency(
    period_ns: u64,
    trials: usize,
    hot_cpu: usize,
    abort_cpu: usize,
    priority: u8,
    seed: u64,
) -> anyhow::Result<()> {
    let clock = RawClock;
    let config = TrialConfig {
        period_ns,
        trials,
        hot_cpu,
        abort_cpu,
        priority,
        seed,
        // A real invocation of this binary, never a test: a scheduling-sensitive figure taken
        // on a default-policy thread is not defensible under this project's own rig discipline
        // (D-36), so a failure to pin or to obtain SCHED_FIFO refuses the whole run rather than
        // silently publishing a number the harness cannot stand behind.
        require_realtime_scheduling: true,
    };

    let outcome = run_trials(&clock, &config).context("running abort-latency trials")?;

    println!("trial\tphase_ns\tabort_raw_ns\tobserved_raw_ns\tlatency_ns");
    for row in &outcome.rows {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            row.trial, row.phase_ns, row.abort_raw_ns, row.observed_raw_ns, row.latency_ns
        );
    }

    Ok(())
}
