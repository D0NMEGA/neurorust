#!/bin/sh
# The BENCH-08 weekly measurement, run unattended by neurorust-measure.timer with nobody
# logged in. Installed at /opt/neurorust/deploy/systemd/neurorust-weekly-run.sh; see
# deploy/systemd/README.md for installation and how to trigger a run manually.
set -eu

# A fixed PATH for a unit that runs with no login session and no shell profile.
PATH=/usr/sbin:/usr/bin:/sbin:/bin
export PATH

REPO=/opt/neurorust
BIN="$REPO/target/release/nrmeasure"
RIG=precision3591
TOKEN_FILE=/etc/neurorust/push-token
export TOKEN_FILE

cd "$REPO"

# One push path, used by both the refusal branch below and the success branch at the end of
# this script, so neither can drift from the other or authenticate more weakly than the
# other. The credential helper value is SINGLE-quoted on purpose: in double quotes this
# shell would expand the token read before git ever runs, which would put the token itself
# into git's argv where any user on the machine can read it out of ps. Single quotes leave
# that read for the shell git itself spawns to run the helper, which is why TOKEN_FILE is
# exported above, so that spawned shell can still resolve it. printf, not echo, writes the
# helper's two output lines: printf's %s takes the token as a plain positional argument
# rather than interpreting it, which also sidesteps echo's very real, if rare, misbehaviour
# on a value that happens to start with a dash. GIT_ASKPASS=/bin/true stops git falling back
# to an interactive prompt if the helper ever returns nothing, rather than hanging an
# unattended unit forever.
#
# The destination is a per-run branch, never main. An earlier version pushed straight to
# main with no fetch, which assumed nothing else had moved since the previous weekly run.
# On 2026-09-13 that assumption broke for the first time: the run itself was clean (p99 9 us,
# verdict clean, admitted to the series) and the push was rejected as non-fast-forward,
# because commits had landed on main during the week. systemd then recorded the whole unit as
# FAILED, so a good measurement looked like a rig fault, and the only copy of it sat on this
# laptop until someone went looking.
#
# Fetch-and-rebase was the obvious repair and is the wrong one here: it would have an
# unattended machine rewrite its own measurement commit onto remote work nobody reviewed, and
# it still fails on a real conflict, so it narrows the race rather than removing it. A branch
# per run removes it. The rig only ever creates refs nothing else writes, so the push cannot
# be rejected by anything a week of dev-host work does, and D-11's division stands: the rig
# measures and publishes evidence, a human decides what reaches main.
#
# The operator merges these branches. A week whose branch exists but is unmerged is therefore
# a different state from a week with no run at all, and only the second is a coverage gap
# under D-08. See deploy/systemd/README.md.
# The branch carries the commit's own short sha, not just the date, so it cannot collide and
# so no push here ever needs a force. Two runs on one day is not hypothetical: 2026-09-10 had
# both a manual trigger and a scheduled one. A date-only name would have made the second push
# either a rejection or a force, and an unattended unit should never be in a position to need
# either.
push_to_origin() {
    branch="rig/$(date -u +%Y-%m-%d)-weekly-$(git rev-parse --short HEAD)"
    GIT_ASKPASS=/bin/true git \
        -c credential.helper='!f(){ printf "username=%s\n" x-access-token; printf "password=%s\n" "$(cat "$TOKEN_FILE")"; };f' \
        push origin "HEAD:refs/heads/$branch"
    echo "pushed to $branch; merge it to publish this run"
}

# D-09: the weekly watch covers scheduling latency and the firmware floor together, because
# a fwupd BIOS or microcode update could silently move the floor every later figure sits on.
# The firmware instrument is rtla hwnoise, not hwlatdetect: hwlatdetect's tracer runs a
# single non-migrating thread that isolcpus keeps off CPUs 6-11 entirely, so it cannot watch
# the cores this project cares about (see firmware-floor-rt-vs-stock.md, under this
# repository's rig documentation, for the full comparison).
# D-10: roughly one hour of cyclictest for the regression series.
#
# stderr is captured rather than left to flow straight to the journal, because it is also
# this script's only source for naming which precondition refused the run below: the
# harness reports that in its own diagnostic output, not in a side file for a wrapper to
# read.
REFUSAL_LOG=$(mktemp)
trap 'rm -f "$REFUSAL_LOG"' EXIT

if ! "$BIN" run --rig-slug "$RIG" --class weekly --instrument headline-series \
        --cpus 6-11 --main-cpus 0,1 \
        --duration 3600 --interval 200 --histogram-max 400 --priority 99 \
        --with-hwnoise --hwnoise-cpus 6-11 --hwnoise-housekeeping 0-5 \
        --hwnoise-duration 900 --hwnoise-priority f:99 \
        --measurements-root "$REPO/measurements" \
        --note "Weekly regression run, unattended, systemd oneshot with no login session." \
        2>"$REFUSAL_LOG"; then
    # Preserve the diagnostic in the journal (it was captured above, not left to flow there
    # directly), then record it: D-06 plus D-08 say a run refused on a precondition, or one
    # that failed outright, produces the same recorded gap a missed week does. It does not
    # produce a retry.
    cat "$REFUSAL_LOG" >&2
    REFUSAL_REASON=$(cat "$REFUSAL_LOG")
    if [ -z "$REFUSAL_REASON" ]; then
        REFUSAL_REASON="nrmeasure run exited nonzero with no diagnostic output"
    fi
    "$BIN" series --record-refusal "$REFUSAL_REASON"
    git add metrics/coverage.json
    git -c user.name=neurorust-rig -c user.email=rig@localhost \
        commit -m "metrics: record coverage gap for refused weekly run"
    push_to_origin
    exit 0
fi

"$BIN" verify --write-index
"$BIN" verify --strict --check-index
"$BIN" series --append

# D-07: the weekly p50/p95/p99 JSON is a regression-tracking artifact and auto-commits.
# Anything that becomes a published claim reaches main only through a reviewed commit
# instead, so this script's own git add list below never names the documentation tree and
# never names the regression baseline file.
git add measurements metrics/latency-series.json metrics/coverage.json measurements/INDEX.md
git -c user.name=neurorust-rig -c user.email=rig@localhost \
    commit -m "metrics: weekly run $(date -u +%Y-%m-%d)"

push_to_origin
