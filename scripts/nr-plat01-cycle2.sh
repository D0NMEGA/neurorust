#!/bin/sh
# nr-plat01-cycle2.sh
#
# PLAT-01 investigation, cycle 2: chase phenomenon B, the isolated multi-millisecond
# event. It is the phenomenon PLAT-01's requirement text names.
#
# Break limit is T_B = 3000 us, derived in cycle 1 and recorded in
# docs/rig/plat01-stall-investigation.md. Below the 3679 to 3806 us per-thread maxima
# observed on 2026-08-28 so any event in that family fires it, and far enough above
# T_A = 200 us that a phenomenon A event does not consume the trace first.
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

BREAKTRACE=3000     # us. T_B from cycle 1.
DURATION=5400       # s. Ninety minutes.

EVENTS="sched:sched_switch sched:sched_wakeup irq:irq_handler_entry irq:irq_handler_exit"
EVENTS="$EVENTS timer:hrtimer_expire_entry workqueue:workqueue_execute_start"
EVENTS="$EVENTS ipi:ipi_send_cpu ipi:ipi_send_cpumask tlb:tlb_flush"

NOTE="PLAT-01 cycle 2: phenomenon B, the isolated multi-millisecond event, not a figure."
NOTE="$NOTE Break limit $BREAKTRACE us, which is T_B derived in cycle 1 from a traced"
NOTE="$NOTE baseline and recorded in docs/rig/plat01-stall-investigation.md."
NOTE="$NOTE Target is the event family that produced the 3679 to 3806 us per-thread maxima"
NOTE="$NOTE on 2026-08-28."
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
    echo "nr-plat01-cycle2.sh: /usr/local/sbin/nr-run-measurement is missing" >&2
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
    echo "nr-plat01-cycle2.sh: tracing_on is '${TRACING_ON:-unreadable}', not 1." >&2
    echo "  Arm the tracer first: sudo sh ~/neurorust/scripts/nr-arm-trace.sh" >&2
    echo "  If it reads 'unreadable', nr-measure-mode status itself failed; check the" >&2
    echo "  sudoers grant with: sudo -n /usr/local/sbin/nr-measure-mode status" >&2
    exit 1
fi

echo "nr-plat01-cycle2.sh: waiting for every SSH session to close before launching"
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
