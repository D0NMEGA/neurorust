#!/bin/sh
# nr-collect-from-rig.sh <run-id> [host]
#
# Collect exactly one run directory from the reference rig.
#
# The obvious command, `rsync -az rig:~/neurorust/measurements/ measurements/`, is wrong
# and was used once. It does not add the new run, it replaces the whole tree with the
# rig's copy of it. On 2026-09-07 that reverted eleven published REPORT.md files to their
# pre-correction state, wrote the operator's home directory back into manifests published
# with [redacted], and reset a capture's ATTEMPT.json from failed to in-progress, dropping
# the preserved evidence recorded for it. The strict gate caught the attempt record. It
# could not catch the other two, because neither makes a manifest disagree with its
# capture.
#
# The dev host is the publication side. A run is written once on the rig and collected
# once, so this script names the run it collects and can touch nothing else.
set -eu

if [ $# -lt 1 ]; then
    echo "usage: nr-collect-from-rig.sh <run-id> [host]" >&2
    echo "  run-id is a directory name under measurements/ on the rig," >&2
    echo "  for example 2026-09-07-precision3591-recon" >&2
    exit 2
fi

RUN_ID=$1
HOST=${2:-precision3591-rig}

# A run id is a single path component. Anything else could escape measurements/.
case "$RUN_ID" in
    */*|.|..|"")
        echo "nr-collect-from-rig.sh: '$RUN_ID' is not a single directory name" >&2
        exit 2
        ;;
esac

if [ ! -d measurements ]; then
    echo "nr-collect-from-rig.sh: no measurements/ here; run from the repository root" >&2
    exit 2
fi

# A capture is written once. If it is already here, collecting again would either be a
# no-op or would overwrite evidence that has since been corrected on this side, which is
# the failure this script exists to prevent.
if [ -e "measurements/$RUN_ID" ]; then
    echo "nr-collect-from-rig.sh: measurements/$RUN_ID already exists here" >&2
    echo "  a run is collected once; delete it deliberately if you mean to re-collect" >&2
    exit 1
fi

if ! ssh "$HOST" "test -d ~/neurorust/measurements/$RUN_ID"; then
    echo "nr-collect-from-rig.sh: $HOST has no measurements/$RUN_ID" >&2
    echo "  list what it does have: ssh $HOST ls ~/neurorust/measurements/" >&2
    exit 1
fi

echo "nr-collect-from-rig.sh: collecting $RUN_ID from $HOST"
rsync -az "$HOST:~/neurorust/measurements/$RUN_ID/" "measurements/$RUN_ID/"

echo "nr-collect-from-rig.sh: collected:"
ls -1 "measurements/$RUN_ID"
echo "nr-collect-from-rig.sh: next: nrmeasure verify --write-index, then verify --strict --check-index"
