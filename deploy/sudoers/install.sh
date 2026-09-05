#!/bin/sh
# deploy/sudoers/install.sh
#
# Installs the four neurorust rig scripts to /usr/local/sbin/ and the
# accompanying NOPASSWD sudoers rule to /etc/sudoers.d/nr-measurement. Run as
# root, from a checkout of this repository, after `git pull`:
#
#   sudo ./deploy/sudoers/install.sh
#
# Idempotent: re-running after a change to any of the four scripts, or to
# deploy/sudoers/nr-measurement, re-installs and re-validates everything.
set -eu

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd)
SCRIPTS_DIR="$REPO_ROOT/scripts"
SUDOERS_SRC="$SCRIPT_DIR/nr-measurement"
SUDOERS_DST=/etc/sudoers.d/nr-measurement
TARGET_DIR=/usr/local/sbin
SCRIPTS="nr-recon nr-probe nr-measure-mode nr-run-measurement"

if [ "$(id -u)" -ne 0 ]; then
    echo "install.sh: must run as root (sudo ./deploy/sudoers/install.sh)" >&2
    exit 1
fi

# Refuse before touching anything if a script is missing. A partial install
# would leave the sudoers rule naming a script that is not there.
for s in $SCRIPTS; do
    if [ ! -f "$SCRIPTS_DIR/$s" ]; then
        echo "install.sh: refusing: $SCRIPTS_DIR/$s is missing" >&2
        exit 1
    fi
done

if [ ! -f "$SUDOERS_SRC" ]; then
    echo "install.sh: refusing: $SUDOERS_SRC is missing" >&2
    exit 1
fi

# A NOPASSWD-executable script the invoking user can edit is a root shell.
# /usr/local/sbin must be root-owned and not writable by group or other
# before anything is installed into it.
target_owner=$(stat -c '%U' "$TARGET_DIR")
target_mode=$(stat -c '%A' "$TARGET_DIR")
if [ "$target_owner" != root ]; then
    echo "install.sh: refusing: $TARGET_DIR is owned by $target_owner, not root" >&2
    exit 1
fi
case "$target_mode" in
    ?????w????|????????w?)
        echo "install.sh: refusing: $TARGET_DIR ($target_mode) is writable by group or other" >&2
        exit 1
        ;;
esac

for s in $SCRIPTS; do
    install -o root -g root -m 0755 "$SCRIPTS_DIR/$s" "$TARGET_DIR/$s"
    echo "install.sh: installed $TARGET_DIR/$s"
done

# Validate before activating. A syntactically broken sudoers file locks the
# machine out of sudo entirely, so it is never written to /etc/sudoers.d
# without first being checked as a standalone temporary copy.
TMP_SUDOERS=$(mktemp)
trap 'rm -f "$TMP_SUDOERS"' EXIT
cp "$SUDOERS_SRC" "$TMP_SUDOERS"
visudo -c -f "$TMP_SUDOERS"
install -o root -g root -m 0440 "$TMP_SUDOERS" "$SUDOERS_DST"
echo "install.sh: installed $SUDOERS_DST"

# The grants were hand-installed across two files: nr-recon (2026-09-02) carried
# nr-recon, nr-probe and nr-measure-mode, and nr-run-measurement (2026-09-04)
# carried the fourth. Remove both so exactly one file describes the grant. This
# runs only after the replacement above is validated and installed, so there is
# no window in which the operator holds no grant.
for old in nr-recon nr-run-measurement; do
    if [ -f "/etc/sudoers.d/$old" ]; then
        rm -f "/etc/sudoers.d/$old"
        echo "install.sh: removed the superseded /etc/sudoers.d/$old"
    fi
done

echo
echo "install.sh: done. Resulting grant:"
if [ -n "${SUDO_USER:-}" ]; then
    sudo -l -U "$SUDO_USER"
else
    sudo -l
fi
