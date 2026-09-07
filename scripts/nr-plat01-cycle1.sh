#!/bin/sh
# nr-plat01-cycle1.sh
#
# PLAT-01 investigation, cycle 1 step 4: the tracing-overhead calibration run.
#
# What this measures is not the stall. It is how much the armed tracer inflates the
# baseline maximum on this machine, so that both break thresholds are derived from a
# maximum observed WITH tracing already on. That two-step calibration is the step the RT
# wiki documents and that almost everyone omits, and omitting it produces thresholds that
# either never fire or fire on ordinary traced operation.
#
# Preconditions, in order, before this is run:
#   1. measurement mode on   (sudo -n /usr/local/sbin/nr-measure-mode on)
#   2. tracer armed and VERIFIED  (sudo sh scripts/nr-arm-trace.sh, which exits non-zero
#      if the ring buffer holds nothing from CPUs 6-11)
#
# The run is launched detached and waits for every SSH session to close first. systemd-run
# returns immediately and the NoActiveSshSessions precondition is asserted the moment the
# unit starts, so a directly typed invocation fails while the operator is still connected.
#
# A dropped SSH connection leaves an ESTABLISHED socket the rig does not reap for two
# hours, and this loop counts it exactly as the precondition does. If this appears to hang,
# that is why: check `ss -Htn state established '( sport = :22 )'` for a peer that is not
# you, and clear it with
#   sudo ss -K -tn state established '( sport = :22 and dport = :<its port> )'
set -eu

BREAKTRACE=100000   # us. Deliberately far above anything plausible so it never fires and
                    # the run completes; this cycle wants a completed run, not a snapshot.
DURATION=1800       # s. Thirty minutes.

# Recorded verbatim because it is an experimental input: it belongs in the manifest note
# and in the write-up, not only in this script. Armed and verified by nr-arm-trace.sh.
EVENTS="sched:sched_switch sched:sched_wakeup irq:irq_handler_entry irq:irq_handler_exit"
EVENTS="$EVENTS timer:hrtimer_expire_entry workqueue:workqueue_execute_start"
EVENTS="$EVENTS ipi:ipi_send_cpu ipi:ipi_send_cpumask tlb:tlb_flush"

NOTE="PLAT-01 cycle 1: tracing-overhead calibration, not a figure."
NOTE="$NOTE Second take. The first (2026-09-07-precision3591-investigation, max 75 us) was"
NOTE="$NOTE captured by a harness whose admission gate did not enforce the two-instrument"
NOTE="$NOTE rule, so it recorded excluded_from_series false on a traced run that can never"
NOTE="$NOTE feed the series; fixed in a646407 and re-taken rather than corrected in a"
NOTE="$NOTE footnote. The first capture is retained and published beside this one: its"
NOTE="$NOTE measurement was sound and only its admission field was wrong."
NOTE="$NOTE The tracer was not re-armed between the two takes; the same armed state carried"
NOTE="$NOTE across, confirmed by nr-measure-mode status immediately before this launch."
NOTE="$NOTE Event set armed and verified by scripts/nr-arm-trace.sh before the first run: $EVENTS."
NOTE="$NOTE tracing_cpumask 3fffff over all 22 logical cpus; buffer_size_kb 8195 per cpu"
NOTE="$NOTE (the kernel rounded up from 8192)."
NOTE="$NOTE Buffer verified non-empty on the isolated cores before launch: 2405 of 70460"
NOTE="$NOTE lines came from cpus 6-11 at the report, 2414 nine lines later, the buffer"
NOTE="$NOTE being live; 2405 is the recorded verification count."
NOTE="$NOTE breaktrace $BREAKTRACE us, set far above anything plausible so it never fires"
NOTE="$NOTE and the run completes."
NOTE="$NOTE Purpose is the traced baseline maximum M_traced, against M_untraced of 78 us"
NOTE="$NOTE from 2026-09-01-precision3591-calibration-clean, per the two-step calibration."

if [ ! -x /usr/local/sbin/nr-run-measurement ]; then
    echo "nr-plat01-cycle1.sh: /usr/local/sbin/nr-run-measurement is missing" >&2
    echo "  install it first: sudo ./deploy/sudoers/install.sh" >&2
    exit 2
fi

echo "nr-plat01-cycle1.sh: waiting for every SSH session to close before launching"
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
