# The BENCH-08 weekly measurement timer

`neurorust-measure.service` plus `neurorust-measure.timer` take the weekly regression
measurement (BENCH-08) unattended, with nobody logged in. This directory is version
controlled and installed onto the rig; it is not machine-local configuration.

## Installation

From a checkout of this repository at `/opt/neurorust` on the rig, created by cloning
this repository directly from GitHub as root (a separate, root-owned checkout from the
operator's own `~/neurorust`, which is reached via `scripts/nr-push-to-rig.sh` instead):

```sh
sudo ./deploy/systemd/install.sh
```

`install.sh` is idempotent. It refuses before touching anything if `/etc/neurorust/push-token`
is missing or is not mode 0600 owned by root, and if `/opt/neurorust` is writable by anyone
but root, for the same reason `deploy/sudoers/install.sh` checks `/usr/local/sbin`: a root
unit whose `ExecStart` lives in a tree anyone but root can write to is a root shell for
anything running as that anyone.

Create the token before installing, if it does not already exist:

```sh
sudo mkdir -p /etc/neurorust
sudo sh -c 'printf %s "<the fine-grained token>" > /etc/neurorust/push-token'
sudo chmod 600 /etc/neurorust/push-token
```

The token must be scoped to this repository only, contents write only, with an expiry
(T-1-01). Rotate it before it expires; the wrapper script has no way to tell you it is about
to.

## Two checkouts

The rig carries two trees of this repository and they are not the same tree:

| Path | Owner | How it gets there | Used by |
|------|-------|-------------------|---------|
| `/opt/neurorust` | root | `git clone` from GitHub, as root | the timer, via `neurorust-measure.service` |
| `~/neurorust` | the operator | `scripts/nr-push-to-rig.sh` rsync, no `.git` | interactive runs, via `/usr/local/sbin/nr-run-measurement` |

The separation is deliberate. The unattended job should not compete with an interactive
working tree, and a dirty interactive tree is not what a published figure should be built
from. It is also what lets `install.sh` refuse a root unit whose `ExecStart` sits in a tree
anyone but root can write to.

The cost is that the two can drift apart, and a weekly figure and a headline figure built
from different revisions, with nothing saying so, is a provenance gap in the operating
procedure rather than in the code. Check them before trusting a comparison across run
classes:

```sh
sudo git -C /opt/neurorust rev-parse HEAD
cat ~/neurorust/.git-sha
sha256sum /opt/neurorust/target/release/nrmeasure ~/neurorust/target/release/nrmeasure
```

Two things about that check are easy to get wrong. `~/neurorust` has no `.git`, so
`git rev-parse` cannot work there at all; its revision is the `sha=` line of the `.git-sha`
stamp `nr-push-to-rig.sh` writes. And `git` commands against `/opt/neurorust` need root,
because the `safe.directory` exemption is in root's gitconfig, which is the identity the
unit runs as.

The two binaries never match by sha256, even at the same revision. `crates/cli/build.rs`
records where it learned the revision from: `git` when it can read `.git`, `pushed-stamp`
when it reads the `.git-sha` stamp instead. That string is compiled in, so identical source
yields two different binaries by design, and every manifest reports which of the two
measured it. Compare the revisions, not the hashes.

To bring them back into step, run `scripts/nr-push-to-rig.sh` from the dev host and rebuild.
Cargo is not on the PATH there even in a login shell, so the rebuild needs its environment
spelled out:

```sh
cd ~/neurorust && PATH=$HOME/.cargo/bin:$PATH RUSTUP_HOME=$HOME/.rustup \
    CARGO_HOME=$HOME/.cargo cargo build -p nr-cli --release
```

## Triggering a run manually

```sh
sudo systemctl start neurorust-measure.service
```

This runs the same script the timer fires, including the push at the end. It takes roughly
an hour (3600 seconds of cyclictest plus a 900 second firmware screen plus harness overhead).

## Reading the result

```sh
journalctl -u neurorust-measure
```

A clean run's journal shows the harness's own precondition and verification output. A
refused run shows the same diagnostic the harness would print at a terminal, since the
wrapper script captures it and writes it into the journal before recording the coverage gap.

## Checking the next scheduled fire

```sh
systemctl list-timers neurorust-measure.timer
```

## Disabling the timer

Before using the machine for Windows, or for any other reason the weekly run should not
fire:

```sh
sudo systemctl disable --now neurorust-measure.timer
```

A week (or more) with the timer disabled is not silently lost: the next successful run's
`nrmeasure series --append` records every skipped week as a `no-run-recorded` gap in
`metrics/coverage.json`, and the phase context's D-08 decision is explicit that a missed
week is recorded this way and is never backfilled. `Persistent=false` on the timer is what
makes this true: a `Persistent=true` timer would fire a catch-up run at the next boot
instead, in conditions that differ from the missed week's, and would silently fill the hole
that is supposed to stay visible.

## Why `TimeoutStartSec`, not its sibling

`neurorust-measure.service` is `Type=oneshot`, and for a oneshot unit the whole run is the
start phase. `RuntimeMaxSec`, which bounds a unit's total runtime on other service types,
has no effect at all on a oneshot unit; systemd says so itself, printing exactly this line
into the journal if the unit ever carries one:

```
RuntimeMaxSec= has no effect in combination with Type=oneshot. Ignoring.
```

That printed line is the only reason this defect was ever caught, in an earlier draft of
this unit. After any change to `neurorust-measure.service`, confirm the journal does not
carry that line:

```sh
journalctl -u neurorust-measure | grep 'RuntimeMaxSec'
```

An empty result is correct. Any output means `RuntimeMaxSec` crept back in and the unit is
not actually bounded; `TimeoutStartSec` is the directive that bounds a oneshot unit's start
phase, and it is what this unit uses instead.
