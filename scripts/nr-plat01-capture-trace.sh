#!/bin/sh
# nr-plat01-capture-trace.sh <label>
#
# Snapshot the ftrace ring buffer after an investigation capture, and report what is in
# it, WITHOUT committing anything. Run under sudo, immediately after the run finishes and
# before anything re-arms tracing.
#
# cyclictest's --breaktrace writes 0 to tracing_on when it fires, so after a break the
# buffer is frozen and holds what the kernel was doing up to the event. If no break fired,
# tracing is still on and the buffer holds a rolling window; that is still evidence about
# what the machine does ordinarily, and this script says which case it is rather than
# leaving the reader to infer it.
#
# The snapshot is written to /tmp and deliberately NOT placed in the run directory. The
# manifest is sealed when the run ends (D-12 makes it immutable evidence), so a file added
# afterwards cannot be listed in it with a blake3, and `verify --strict` reports any
# capture-shaped file in a run directory that is not listed. Where these bytes end up, in
# the repository or as StorageLocation::External, is decided once the size is known: at
# the event rate cycle 1 showed, a full 8195 KB per cpu ring across 22 cpus is on the
# order of 180 MB, which does not belong in git by default.
set -eu

if [ $# -ne 1 ]; then
    echo "usage: nr-plat01-capture-trace.sh <label>   e.g. cycle2" >&2
    exit 2
fi
LABEL=$1
T=/sys/kernel/tracing
OUT=/tmp/ftrace-$LABEL.txt

if [ ! -r "$T/trace" ]; then
    echo "nr-plat01-capture-trace.sh: cannot read $T/trace; run under sudo" >&2
    exit 2
fi

TRACING_ON=$(cat "$T/tracing_on")
if [ "$TRACING_ON" = "0" ]; then
    echo "tracing_on is 0: the buffer is FROZEN, which is what --breaktrace does when it fires."
else
    echo "tracing_on is 1: NO break fired. The buffer is live and holds a rolling window."
fi

cp "$T/trace" "$OUT"
chmod 0644 "$OUT"

echo "wrote $OUT"
echo "bytes: $(wc -c < "$OUT")"
awk '
    { total++ }
    /\[00[6-9]\]|\[01[01]\]/ { isolated++ }
    END { print "total lines: " total + 0; print "lines on cpus 6-11: " isolated + 0 }
' "$OUT"

# The events most likely to name phenomenon B, counted so the shape of the buffer is
# visible before anyone reads 180 MB of it. A broadcast IPI or a TLB shootdown landing on
# the isolated cores is the hypothesis this event set was chosen to test.
echo "--- event counts in the whole buffer ---"
for e in ipi_send_cpumask ipi_send_cpu tlb_flush workqueue_execute_start \
         irq_handler_entry hrtimer_expire_entry sched_switch sched_wakeup; do
    printf '  %-24s %s\n' "$e" "$(grep -c "$e" "$OUT" || true)"
done

echo "--- the last 40 lines before the buffer ends ---"
tail -40 "$OUT"
