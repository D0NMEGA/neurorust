#!/bin/sh
# nr-plat01-cycle3.sh
#
# PLAT-01 investigation, cycle 3, the last of the D-20 budget: chase phenomenon A, the
# sustained burst at roughly 10 ms spacing across all isolated cores. Two hours, because
# phenomenon A occupied about 1.2 seconds inside a 10 minute run on 2026-08-28.
#
# Break limit is T_A = 200 us, derived in cycle 1 and recorded in
# docs/rig/plat01-stall-investigation.md: above every maximum this rig has produced on a
# clean run, and half of the 400 us a phenomenon A burst exceeds.
#
# Expect it not to fire. Across 4.5 hours of clean running the maxima are 13, 30, 35, 37,
# 42, 46, 46, 72, 75, 78 and 96 us, so 200 us is more than twice anything observed. That
# is the point rather than a problem: cycle 2 already failed to reproduce phenomenon B,
# stop condition 3 requires both cycles before non-reproduction may be concluded, and
# ROADMAP criterion 1 requires the total exposure to be stated. A cycle that fires nothing
# is exposure, and it is committed like any other.
#
# The threshold was deliberately NOT lowered to chase the real 40 to 96 us tail. That
# residual is what blocks criterion 3, but it belongs to plan 01-13, and moving T_A after
# publishing its derivation would make the derivation describe an instrument nobody used.
#
# Instrument ordering is (a), two separate runs taken sequentially, and this is a
# departure from the plan's stated preference for (b). Plan 01-12 calls (b) the stronger
# experiment: rtla in its own transient unit, running concurrently with the harness so
# both instruments describe one window. On this machine (b) is self-defeating.
# `rtla timerlat top --cpus 6-11` runs sampling threads at SCHED_FIFO on exactly the
# cores cyclictest measures at priority 99, which is the configuration that destroyed
# measurements/2026-09-06-precision3591-screen-02: 75 percent of cyclictest cycles lost,
# every thread stalled at roughly 750000 us, which is osnoise/runtime_us. Worse, an
# investigation run records TracersQuiescent as NotApplicable, so the precondition plan
# 01-25 widened would not refuse it. The choice is recorded in the manifest note, because
# silently assuming (b) while executing (a) is what the external audit caught.
#
# Preconditions, in order:
#   1. measurement mode on   (sudo -n /usr/local/sbin/nr-measure-mode on)
#   2. tracer armed and verified  (sudo sh scripts/nr-arm-trace.sh)
#
# AFTER this run finishes, capture the ring buffer before anything re-arms tracing.
# cyclictest's --breaktrace writes 0 to tracing_on when it fires, so the buffer is frozen
# and can be read at leisure; it is not frozen if no break fired, in which case it holds a
# rolling window and is still evidence. See nr-plat01-capture-trace.sh.
set -eu

BREAKTRACE=200      # us. T_A from cycle 1.
DURATION=7200       # s. Two hours.

EVENTS="sched:sched_switch sched:sched_wakeup irq:irq_handler_entry irq:irq_handler_exit"
EVENTS="$EVENTS timer:hrtimer_expire_entry workqueue:workqueue_execute_start"
EVENTS="$EVENTS ipi:ipi_send_cpu ipi:ipi_send_cpumask tlb:tlb_flush"

NOTE="PLAT-01 cycle 3: phenomenon A, the sustained burst at roughly 10 ms spacing"
NOTE="$NOTE across all isolated cores. Not a figure. Last cycle of the D-20 budget."
NOTE="$NOTE Break limit $BREAKTRACE us, which is T_A derived in cycle 1 from a traced"
NOTE="$NOTE baseline and recorded in docs/rig/plat01-stall-investigation.md."
NOTE="$NOTE Target is the roughly 1.2 second burst observed inside a 10 minute run on"
NOTE="$NOTE 2026-08-28. Cycle 2 ran 90 minutes at T_B 3000 us and phenomenon B did not"
NOTE="$NOTE reproduce, maximum 72 us; this cycle adds two hours of exposure and is expected"
NOTE="$NOTE not to fire, since 200 us is more than twice any maximum this rig has produced"
NOTE="$NOTE on a clean run. A cycle that fires nothing is exposure and is committed anyway."
NOTE="$NOTE Instrument ordering: (a) sequential, two separate runs, NOT (b) concurrent."
NOTE="$NOTE (b) would run rtla sampling threads at SCHED_FIFO on cpus 6-11, the same cores"
NOTE="$NOTE this run measures at priority 99, which is what starved"
NOTE="$NOTE 2026-09-06-precision3591-screen-02, and an investigation run records"
NOTE="$NOTE TracersQuiescent NotApplicable so nothing would refuse it."
NOTE="$NOTE Event set armed by scripts/nr-arm-trace.sh: $EVENTS."
NOTE="$NOTE tracing_cpumask 3fffff over all 22 logical cpus so an IPI sender is visible and"
NOTE="$NOTE not only its arrival; buffer_size_kb 8195 per cpu."
NOTE="$NOTE Whether the break fired, and the buffer line counts if it did, are recorded in"
NOTE="$NOTE the plan summary rather than here: this note is written at launch."

if [ ! -x /usr/local/sbin/nr-run-measurement ]; then
    echo "nr-plat01-cycle3.sh: /usr/local/sbin/nr-run-measurement is missing" >&2
    exit 2
fi

# Refuse to spend ninety minutes against a tracer that is not actually recording. Cycle 1
# established that this check is the difference between an instrument and a hope.
#
# Asked through nr-measure-mode, not by reading tracefs directly. This script runs as the
# operator; only the nr-run-measurement it execs is root. tracefs is root-only, so a direct
# `cat /sys/kernel/tracing/tracing_on` here returns permission denied, and the first
# version's `|| echo 0` fallback turned that into a refusal every single time, whatever
# the tracer was actually doing. It refused a cycle 2 launch on 2026-09-07 with tracing on
# and armed. nr-measure-mode is one of the four NOPASSWD grants and prints tracing_on in
# its status block, so this reads the real value without needing a password or root here.
TRACING_ON=$(sudo -n /usr/local/sbin/nr-measure-mode status 2>/dev/null \
    | awk '$1 == "tracing_on" { print $3 }')
if [ "$TRACING_ON" != "1" ]; then
    echo "nr-plat01-cycle3.sh: tracing_on is '${TRACING_ON:-unreadable}', not 1." >&2
    echo "  Arm the tracer first: sudo sh ~/neurorust/scripts/nr-arm-trace.sh" >&2
    echo "  If it reads 'unreadable', nr-measure-mode status itself failed; check the" >&2
    echo "  sudoers grant with: sudo -n /usr/local/sbin/nr-measure-mode status" >&2
    exit 1
fi

echo "nr-plat01-cycle3.sh: waiting for every SSH session to close before launching"
echo "  (log out now; the capture starts 10 s after the last session drops)"
while [ "$(ss -Htn state established '( sport = :22 )' 2>/dev/null | wc -l)" -gt 0 ]; do
    sleep 5
done
sleep 10

exec sudo -n /usr/local/sbin/nr-run-measurement run \
    --rig-slug precision3591 --class investigation --instrument investigation \
    --cpus 6-11 --main-cpus 0,1 \
    --duration "$DURATION" --interval 200 --histogram-max 400 --priority 99 \
    --breaktrace "$BREAKTRACE" \
    --note "$NOTE"
