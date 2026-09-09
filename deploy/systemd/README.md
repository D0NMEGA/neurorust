# The BENCH-08 weekly measurement timer

`neurorust-measure.service` plus `neurorust-measure.timer` take the weekly regression
measurement (BENCH-08) unattended, with nobody logged in. This directory is version
controlled and installed onto the rig; it is not machine-local configuration.

## Installation

From a checkout of this repository at `/opt/neurorust` on the rig (reached via
`scripts/nr-push-to-rig.sh`, since the rig has no git of its own):

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
