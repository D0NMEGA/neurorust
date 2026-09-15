#!/bin/sh
# STOP-05: cargo kani over crates/stop, as a blocking gate (D-52).
#
# This script exists rather than a bare `cargo kani -p nr-stop` in the workflow because
# cargo kani exits 0 when it finds no harnesses at all. A rename, a cfg mistake, or a module
# that stopped being declared would then produce a green required check over an empty proof,
# which is worse than having no check: a gate nobody believes gets fixed, and a gate everybody
# believes and that guards nothing does not.
set -eu

# The harnesses crates/stop/src/proofs.rs is expected to contain, from plan 02-03. Adding a
# harness means adding it here too; that is deliberate friction on the one file that decides
# what "the proof passed" means.
EXPECTED='step_is_total
state_raw_round_trips
state_from_raw_is_total_and_fails_closed
cause_raw_round_trips_and_is_total
abort_never_returns_to_running
running_is_never_reachable_from_stopping_or_stopped
stopped_is_terminal
no_permit_when_not_running
permit_supply_closes_on_the_abort_edge
modelled_consumer_never_emits_after_a_stop
consumer_emit_never_overflows'

RAW=$(mktemp -t nr-proofs.XXXXXX)
trap 'rm -f "$RAW"' EXIT

STATUS=0
cargo kani -p nr-stop > "$RAW" 2>&1 || STATUS=$?
cat "$RAW"

if [ "$STATUS" -ne 0 ]; then
    echo "nr-proofs.sh: cargo kani exited $STATUS" >&2
    exit "$STATUS"
fi

if grep -q 'VERIFICATION:- FAILED' "$RAW"; then
    echo "nr-proofs.sh: at least one harness reported FAILED" >&2
    exit 1
fi

MISSING=0
for harness in $EXPECTED; do
    if ! grep -q "$harness" "$RAW"; then
        echo "nr-proofs.sh: expected harness $harness did not appear in the output" >&2
        MISSING=1
    fi
done
[ "$MISSING" -eq 0 ] || exit 1

EXPECTED_COUNT=$(printf '%s\n' "$EXPECTED" | wc -l | tr -d ' ')
SUCCEEDED=$(grep -c 'VERIFICATION:- SUCCESSFUL' "$RAW" || true)
if [ "$SUCCEEDED" -ne "$EXPECTED_COUNT" ]; then
    echo "nr-proofs.sh: expected $EXPECTED_COUNT successful verifications, saw $SUCCEEDED" >&2
    exit 1
fi

echo "nr-proofs.sh: $SUCCEEDED of $EXPECTED_COUNT harnesses verified"
