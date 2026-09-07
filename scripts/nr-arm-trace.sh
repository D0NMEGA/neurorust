#!/bin/sh
# nr-arm-trace.sh [--status]
#
# PLAT-01 investigation, step 2: arm the diagnostic event set and prove the ring buffer
# actually records on the isolated cores, before a capture cycle is spent against it.
#
# This exists as a version-controlled script rather than a block of shell typed at the rig
# for two reasons. PLAT-02 and D-26 want the rig's procedures under version control, and
# the armed event set is an experimental input: it goes in the manifest note and in the
# write-up, so it has to be reproducible rather than remembered.
#
# The verification at the end is the point of the script. An earlier version of plan 01-12
# assumed `cyclictest --tracemark` armed tracing. It does not; it marks and stops at a
# threshold. A tracer that is armed but records nothing on CPUs 6-11 cannot answer the
# question this investigation asks, and finding that out after a thirty minute capture is
# the failure this step prevents.
#
# Run under sudo. Reading and writing tracefs needs root, and the rig's NOPASSWD grant
# covers only the four /usr/local/sbin scripts, so this one prompts.
set -u

T=/sys/kernel/tracing

# The set is chosen so that each event distinguishes a specific hypothesis about the two
# phenomena, and no event is armed that does not. Every name here was checked against this
# kernel's own available_events on 2026-09-07; do not add one without checking it there.
#
#   sched_switch              which task displaced the cyclictest thread
#   sched_wakeup              whether the wakeup itself was late
#   irq_handler_entry/exit    an interrupt reached an isolated core, and how long it took
#   hrtimer_expire_entry      the tick, if phenomenon A's 10 ms spacing is a tick
#   workqueue_execute_start   deferred work landing on an isolated core
#   ipi_send_cpu              a single-target IPI to an isolated core
#   ipi_send_cpumask          a broadcast IPI. Phenomenon B stalls all six isolated threads
#                             at once, which is the shape of a send to many CPUs rather
#                             than to one, so the cpumask form is the one that can catch it
#                             and ipi_send_cpu alone would miss it.
#   tlb_flush                 a system-wide TLB shootdown, one of the two named suspects
EVENTS="
sched:sched_switch
sched:sched_wakeup
irq:irq_handler_entry
irq:irq_handler_exit
timer:hrtimer_expire_entry
workqueue:workqueue_execute_start
ipi:ipi_send_cpu
ipi:ipi_send_cpumask
tlb:tlb_flush
"

# Per CPU. A phenomenon A burst is roughly 1.2 s of dense events and the buffer overwrites
# oldest-first, so this has to hold the window of interest rather than the whole run.
BUFFER_KB=8192

# The CPU field ftrace prints is zero-padded to three digits, so CPUs 6 to 11 are [006]
# through [011]. Matching on that is what distinguishes "the tracer ran" from "the tracer
# ran on the cores this investigation is about".
ISOLATED_RE='\[00[6-9]\]|\[01[01]\]'

report() {
    echo "=== set_event ==="
    cat "$T/set_event"
    echo "=== tracing_on ==="
    cat "$T/tracing_on"
    echo "=== tracing_cpumask ==="
    cat "$T/tracing_cpumask"
    echo "=== buffer_size_kb (per cpu) ==="
    cat "$T/buffer_size_kb"
    echo "=== total trace lines ==="
    wc -l < "$T/trace"
    echo "=== trace lines on CPUs 6-11 ==="
    grep -cE "$ISOLATED_RE" "$T/trace"
}

if [ ! -d "$T" ]; then
    echo "nr-arm-trace.sh: $T does not exist; is tracefs mounted?" >&2
    exit 2
fi

if [ ! -w "$T/tracing_on" ]; then
    echo "nr-arm-trace.sh: $T is not writable; run this under sudo" >&2
    exit 2
fi

if [ "${1:-}" = "--status" ]; then
    report
    exit 0
fi

if [ $# -gt 0 ]; then
    echo "nr-arm-trace.sh: unrecognised argument: $1 (only --status)" >&2
    exit 2
fi

echo "nr-arm-trace.sh: arming"

# Order matters. Stop tracing before changing the buffer size, and clear any previous event
# set rather than adding to it, so the armed set is exactly what this script names.
echo 0    > "$T/tracing_on"
echo nop  > "$T/current_tracer"
echo      > "$T/set_event"

# All 22 logical CPUs, so an IPI's sender is visible and not only its arrival on the
# isolated core. Entering measurement mode resets this (plan 01-11), so it is set here,
# after that, and never before.
printf '%x\n' $(( (1 << 22) - 1 )) > "$T/tracing_cpumask"
echo "$BUFFER_KB" > "$T/buffer_size_kb"

for e in $EVENTS; do
    if ! echo "$e" >> "$T/set_event"; then
        echo "nr-arm-trace.sh: kernel rejected event '$e'" >&2
        echo "  check it against: grep '$e' $T/available_events" >&2
        exit 1
    fi
done

echo 1 > "$T/tracing_on"

# Long enough that an idle machine still generates timer and scheduler activity on every
# CPU. A shorter window can report zero on the isolated cores for want of anything to
# record, which would look identical to a tracer that cannot reach them.
echo "nr-arm-trace.sh: accumulating for 20 s"
sleep 20

report

ON_ISOLATED=$(grep -cE "$ISOLATED_RE" "$T/trace")
echo
if [ "$ON_ISOLATED" -eq 0 ]; then
    echo "nr-arm-trace.sh: STOP. The buffer holds nothing from CPUs 6-11." >&2
    echo "  An armed tracer that records nothing on the isolated cores cannot answer" >&2
    echo "  this question. Do not spend a capture cycle. Check tracing_cpumask above" >&2
    echo "  covers 6-11, and that set_event lists the events." >&2
    exit 1
fi
echo "nr-arm-trace.sh: ok, $ON_ISOLATED lines from CPUs 6-11. Record every value above."
