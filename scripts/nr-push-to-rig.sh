#!/bin/sh
# nr-push-to-rig.sh [--allow-dirty] [host]
#
# The one supported way this repository's source reaches the reference rig.
#
# The rig has no git and ~/neurorust there is an rsync mirror with no .git, so
# crates/cli/build.rs cannot read a revision when it compiles there. This script stamps the
# revision it is pushing into .git-sha, which build.rs reads and records as
# git_sha_source: pushed-stamp: an asserted revision, labelled as asserted.
#
# It never sends measurements/, metrics/, target/, .git/, .planning/ or CLAUDE.md. The rig holds
# captures the dev host has not seen yet, and rsync must not be able to touch them.
set -eu

HOST=precision3591-rig
ALLOW_DIRTY=0

for arg in "$@"; do
    case "$arg" in
        --allow-dirty)
            ALLOW_DIRTY=1
            ;;
        -*)
            echo "nr-push-to-rig.sh: unrecognised option: $arg" >&2
            exit 1
            ;;
        *)
            HOST="$arg"
            ;;
    esac
done

# Without a revision there is nothing to stamp, and pushing anyway would silently
# leave a stale .git-sha (or none at all) describing bytes that no longer match.
if ! SHA=$(git rev-parse HEAD); then
    echo "nr-push-to-rig.sh: git rev-parse HEAD failed; nothing to stamp, refusing to push" >&2
    exit 1
fi

DIRTY_STATUS=$(git status --porcelain)
if [ -n "$DIRTY_STATUS" ]; then
    if [ "$ALLOW_DIRTY" -eq 1 ]; then
        DIRTY=true
    else
        echo "nr-push-to-rig.sh: working tree is dirty; refusing to push without --allow-dirty" >&2
        echo "nr-push-to-rig.sh: an unannounced dirty push is how a sha stops describing the bytes" >&2
        exit 1
    fi
else
    DIRTY=false
fi

STAMPED_UTC=$(date -u +%FT%TZ)

cat > .git-sha <<EOF
sha=$SHA
dirty=$DIRTY
stamped_utc=$STAMPED_UTC
EOF

echo "nr-push-to-rig.sh: pushing $SHA (dirty=$DIRTY) to $HOST"
echo "nr-push-to-rig.sh: paths: Cargo.toml Cargo.lock crates schemas config scripts deploy docs .git-sha"

# The explicit include list this repository's source reaches the rig through. Fails
# closed: only these paths are ever sent, so measurements/, metrics/, target/, .git/,
# .planning/ and CLAUDE.md cannot be touched by this script no matter what else the
# repository root later grows. This script only ever adds or updates the files it
# names; it passes rsync no removal flag, so nothing the rig already holds is deleted.
rsync -az \
    Cargo.toml Cargo.lock crates schemas config scripts deploy docs \
    "$HOST:~/neurorust/"

# .git-sha travels last, in its own transfer, so a push that dies partway through the
# paths above leaves the previous, complete stamp in place rather than a partial one.
rsync -az .git-sha "$HOST:~/neurorust/.git-sha"

LOCAL_DIGEST=$(shasum -a 256 .git-sha | awk '{print $1}')
REMOTE_DIGEST=$(ssh "$HOST" 'sha256sum ~/neurorust/.git-sha' | awk '{print $1}')
if [ "$LOCAL_DIGEST" != "$REMOTE_DIGEST" ]; then
    echo "nr-push-to-rig.sh: .git-sha digest mismatch after transfer, refusing to report success" >&2
    echo "nr-push-to-rig.sh: local $LOCAL_DIGEST remote $REMOTE_DIGEST" >&2
    exit 1
fi

echo "nr-push-to-rig.sh: verified .git-sha arrived byte-identical (sha256 $LOCAL_DIGEST)"
echo "nr-push-to-rig.sh: done"
