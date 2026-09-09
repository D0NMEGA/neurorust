#!/bin/sh
# deploy/systemd/install.sh
#
# Installs the neurorust-measure.service and neurorust-measure.timer units to
# /etc/systemd/system/, then enables and starts the timer. Run as root, from a checkout of
# this repository (after the source has reached it, e.g. via scripts/nr-push-to-rig.sh):
#
#   sudo ./deploy/systemd/install.sh
#
# Idempotent: re-running after a change to either unit file re-installs and re-validates
# both. Refuses before touching anything if the push token is missing or wrongly
# permissioned, or if the checkout it is installing from is writable by anyone but root:
# installing a timer that will fail its push every week, or one whose root ExecStart lives
# in a tree anyone can edit, is worse than not installing it at all.
set -eu

# A fixed PATH rather than depending on sudo's own secure_path, which lives in a file this
# repository does not control.
PATH=/usr/sbin:/usr/bin:/sbin:/bin
export PATH

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd)
UNIT_DIR=/etc/systemd/system
TOKEN_FILE=/etc/neurorust/push-token
INSTALL_ROOT=/opt/neurorust

if [ "$(id -u)" -ne 0 ]; then
    echo "install.sh: must run as root (sudo ./deploy/systemd/install.sh)" >&2
    exit 1
fi

for unit in neurorust-measure.service neurorust-measure.timer; do
    if [ ! -f "$SCRIPT_DIR/$unit" ]; then
        echo "install.sh: refusing: $SCRIPT_DIR/$unit is missing" >&2
        exit 1
    fi
done

if [ ! -x "$SCRIPT_DIR/neurorust-weekly-run.sh" ]; then
    echo "install.sh: refusing: $SCRIPT_DIR/neurorust-weekly-run.sh is missing or not executable" >&2
    exit 1
fi

# A weekly job that cannot authenticate is worse than no job: it fails silently every
# Sunday with nothing to show for it but a journal entry nobody is watching for. Refuse to
# install one before the credential exists.
if [ ! -f "$TOKEN_FILE" ]; then
    echo "install.sh: refusing: $TOKEN_FILE is missing" >&2
    echo "  create it with a fine-grained token scoped to this repository, contents write only" >&2
    exit 1
fi
token_mode=$(stat -c '%a' "$TOKEN_FILE")
token_owner=$(stat -c '%U' "$TOKEN_FILE")
if [ "$token_owner" != root ]; then
    echo "install.sh: refusing: $TOKEN_FILE is owned by $token_owner, not root" >&2
    exit 1
fi
if [ "$token_mode" != 600 ]; then
    echo "install.sh: refusing: $TOKEN_FILE is mode $token_mode, must be 0600" >&2
    echo "  fix: chmod 600 $TOKEN_FILE" >&2
    exit 1
fi

# The same reasoning deploy/sudoers/install.sh applies to /usr/local/sbin applies here: a
# root unit whose ExecStart lives in a tree anyone but root can write to is a root shell for
# anything running as that anyone.
if [ -d "$INSTALL_ROOT" ]; then
    install_owner=$(stat -c '%U' "$INSTALL_ROOT")
    install_mode=$(stat -c '%A' "$INSTALL_ROOT")
    if [ "$install_owner" != root ]; then
        echo "install.sh: refusing: $INSTALL_ROOT is owned by $install_owner, not root" >&2
        exit 1
    fi
    case "$install_mode" in
        ?????w????|????????w?)
            echo "install.sh: refusing: $INSTALL_ROOT ($install_mode) is writable by group or other" >&2
            exit 1
            ;;
    esac
fi

if [ "$REPO_ROOT" != "$INSTALL_ROOT" ]; then
    echo "install.sh: warning: this checkout is at $REPO_ROOT, not $INSTALL_ROOT" >&2
    echo "  the units hardcode $INSTALL_ROOT; installing from elsewhere will not take effect" >&2
fi

for unit in neurorust-measure.service neurorust-measure.timer; do
    install -o root -g root -m 0644 "$SCRIPT_DIR/$unit" "$UNIT_DIR/$unit"
    echo "install.sh: installed $UNIT_DIR/$unit"
done

systemctl daemon-reload
systemctl enable --now neurorust-measure.timer

echo
echo "install.sh: done. Next fire:"
systemctl list-timers neurorust-measure.timer --no-pager
