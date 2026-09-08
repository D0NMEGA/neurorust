#!/bin/sh
# nr-plat03-headline.sh
#
# The PLAT-03 headline capture: four hours of cyclictest on the isolated cores under the
# documented protocol, with no login session. This is the run every later phase quotes, so
# any deviation from the protocol invalidates it and any deviation that happens anyway is
# recorded in the verdict rather than omitted.
#
# cyclictest ALONE. No --with-hwnoise and no --with-hwlatdetect: a firmware instrument in
# the same window changes the conditions the scheduling maximum was measured under, and
# plan 01-23 already took the firmware screens as their own runs. That separation is what
# makes the two figures independently interpretable, which is the point of external audit
# finding 1, and it is why nothing here subtracts one from the other.
#
# Preconditions, in order, before this is run:
#   1. tracing DISARMED. Unlike the PLAT-01 investigation runs, this is a headline-series
#      run, so TracersQuiescent must PASS rather than record NotApplicable. Cycle 3 of the
#      investigation leaves nine events armed and tracing_on at 1; they must be cleared.
#   2. measurement mode on   (sudo -n /usr/local/sbin/nr-measure-mode on)
#   3. a dry run, read while still connected, so the other fourteen preconditions are seen
#      to pass before four hours are committed. NoActiveSshSessions will fail in a dry run
#      by construction, because you are the session; that one is expected.
set -eu

DURATION=14400      # s. Four hours.

NOTE="PLAT-03 headline capture. Four hours under docs/measurement-protocol.md with no login"
NOTE="$NOTE session. This is the run the PLAT-03 verdict is computed from."
NOTE="$NOTE cyclictest alone: no firmware instrument shares this window, because the two do not"
NOTE="$NOTE measure commensurable quantities and neither is subtracted from the other."
NOTE="$NOTE PLAT-01 closed not-reproduced on 2026-09-08 and the roughly 3.8 ms stall remains"
NOTE="$NOTE unexplained; this figure carries that as an explicit limitation, over the 6.5 hours"
NOTE="$NOTE of clean running in which it was not observed. See"
NOTE="$NOTE docs/rig/plat01-stall-investigation.md."

if [ ! -x /usr/local/sbin/nr-run-measurement ]; then
    echo "nr-plat03-headline.sh: /usr/local/sbin/nr-run-measurement is missing" >&2
    exit 2
fi

# The inverse of the investigation launchers' guard, and asked the same way: through
# nr-measure-mode, which is one of the four NOPASSWD grants and prints tracing state in its
# status block. This script runs as the operator and tracefs is root-only, so reading it
# here directly would fail and a naive fallback would refuse every launch, which is the bug
# that cost ninety minutes on 2026-09-07.
STATUS=$(sudo -n /usr/local/sbin/nr-measure-mode status 2>/dev/null || true)
if [ -z "$STATUS" ]; then
    echo "nr-plat03-headline.sh: could not read nr-measure-mode status; check the sudoers grant" >&2
    exit 2
fi
TRACING_ON=$(printf '%s\n' "$STATUS" | awk '$1 == "tracing_on" { print $3 }')
ARMED=$(printf '%s\n' "$STATUS" | awk '$1 == "set_event" { print $3 }')
if [ "$TRACING_ON" = "1" ] || [ -n "$ARMED" ]; then
    echo "nr-plat03-headline.sh: tracing is still armed from the PLAT-01 investigation." >&2
    echo "  tracing_on=$TRACING_ON set_event starts with '${ARMED:-(empty)}'" >&2
    echo "  A headline-series run needs TracersQuiescent to PASS. Disarm first:" >&2
    echo "    sudo sh -c 'echo 0 > /sys/kernel/tracing/tracing_on;" >&2
    echo "                echo > /sys/kernel/tracing/set_event;" >&2
    echo "                echo 1024 > /sys/kernel/tracing/buffer_size_kb'" >&2
    exit 1
fi

echo "nr-plat03-headline.sh: tracing quiescent, proceeding"
echo "nr-plat03-headline.sh: waiting for every SSH session to close before launching"
echo "  (log out now; the capture starts 10 s after the last session drops, and runs 4 hours)"
while [ "$(ss -Htn state established '( sport = :22 )' 2>/dev/null | wc -l)" -gt 0 ]; do
    sleep 5
done
sleep 10

exec sudo -n /usr/local/sbin/nr-run-measurement run \
    --rig-slug precision3591 --class headline --instrument headline-series \
    --thermal-profile normal \
    --cpus 6-11 --main-cpus 0,1 \
    --duration "$DURATION" --interval 200 --histogram-max 400 --priority 99 \
    --note "$NOTE"
