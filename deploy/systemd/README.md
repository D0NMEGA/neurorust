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

Create the token before installing, if it does not already exist. Read it from the terminal
rather than putting it in the command line, so it reaches neither the shell history nor the
process list:

```sh
sudo mkdir -p /etc/neurorust
read -rsp 'paste token, then Enter: ' T; echo; printf '%s' "$T" | sudo tee /etc/neurorust/push-token >/dev/null; unset T
sudo chown root:root /etc/neurorust/push-token
sudo chmod 0600 /etc/neurorust/push-token
sudo chmod 0700 /etc/neurorust
```

`printf '%s'`, not `echo`: a trailing newline in the token file breaks the credential helper
with an unhelpful authentication error.

## Credential rotation

What T-1-01 asks for is a fine-grained token whose resource owner is this repository's
owner, whose repository access is this one repository alone, whose only permission is
Contents read and write, and which carries an expiry.

What is installed on the rig as of 2026-09-10 is none of those last three. The operator
chose a token with account-wide read and write permissions and no expiry, and that token was
pasted into a chat transcript on 2026-09-10, so it should be treated as already disclosed.
T-1-01 is therefore **accepted, not mitigated**, and this file says so rather than
describing a control that is not there. The file permissions half of the mitigation does
hold: the token is root-owned at mode 0600 inside a 0700 directory, is read through a git
credential helper that resolves it in a subshell, and so appears in neither the unit's
environment, the process list, nor the journal.

The practical consequences, none of which the harness can detect for you:

- There is no rotation deadline, because there is no expiry. Nothing will ever fail and
  point at this. The only thing standing between a disclosed credential and the account is
  somebody deciding to replace it.
- A token that never expires does not produce the coverage gaps an expiring one would, so
  the failure mode here is silent rather than visible. That is the opposite of how the rest
  of this project is built, where D-08 makes a missed week visible on purpose.
- The blast radius is the whole account, not this repository.

To rotate, revoke the old token in the GitHub UI first, then install the replacement with
the block above and confirm the loop still closes:

```sh
sudo systemctl start neurorust-measure.service    # or wait for the next Sunday
journalctl -u neurorust-measure.service --no-pager | tail -5
```

A push that fails on authentication is recorded as a coverage gap like any other refusal, so
a botched rotation shows up in `metrics/coverage.json` rather than passing unnoticed.

## Where a run gets pushed, and why it is not main

Each run pushes to its own branch, `rig/<date>-weekly-<short sha>`, and the operator merges it.
The rig never pushes to `main`.

It used to. On 2026-09-13 that produced the failure mode the branch exists to prevent: the run
itself was clean (p99 9 us, contamination verdict clean, admitted to the series) and the push
was rejected as non-fast-forward, because dev-host commits had landed on `main` during the
week. `systemd` recorded the unit as FAILED, so a good measurement read as a rig fault, and
the only copy of it stayed on the rig until someone went looking five days later.

Adding a fetch and rebase would have been the smaller change and the wrong one. It would have
an unattended machine rewrite its own measurement commit onto remote work nobody had reviewed,
and it still fails on a real conflict, so it narrows the race rather than removing it. A branch
per run removes it: the rig only ever creates refs nothing else writes to, so no amount of
dev-host activity can reject its push. D-11's division is preserved and sharpened, with the rig
publishing evidence and a human deciding what reaches `main`.

Two states that look similar and are not:

- A branch exists and has not been merged. The run happened and its evidence is safe. Merge it.
- No branch exists for that week. The run did not happen, or was refused. That is the coverage
  gap D-08 means, and it is recorded in `metrics/coverage.json`.

Merging is a normal reviewed merge from the dev host:

```sh
# On the Mac, from the repo root:
git fetch origin
git log --oneline origin/main..origin/rig/<date>-weekly-<sha>   # read what the rig recorded
git merge --ff-only origin/rig/<date>-weekly-<sha>              # or a normal merge if main moved
git push origin main
```

The regression and provenance workflows run on the merge, which is what gates the figure
reaching `main`, so nothing is checked later than it was before.

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

## The stop-harness subcommand (plan 02-07)

`nr-run-measurement` accepts a second subcommand, `stop-harness`, which launches
`nr-stop-harness` (STOP-07 abort latency) instead of `nrmeasure`. It inherits every guard the
`run` subcommand has: the same NOPASSWD grant (no sudoers change was needed; see below), a pinned
binary path checked for existence, executability and a non-group-writable mode with its sha256
and mode printed into the journal, `--measurements-root` pinned and refused if a caller passes it,
launch through the same `systemd-run` transient unit for the same disconnect-immediately reason
`run` uses, and a `TimeoutStartSec` backstop derived from what was actually requested rather than
hard-coded.

`stop-harness` shares `run`'s own unit name (`nr-measurement`), not a second one: both subcommands
pin the same isolated cores (6-11 on the rig), so a concurrent capture of either kind would
contaminate the other exactly as two concurrent `run` invocations would. Only one of `run` or
`stop-harness` can be in flight at a time.

Unlike `run`, `stop-harness` takes no `--thresholds`: `nr-stop-harness` has no such flag, and
D-37 excludes the `emergency_stop.abort_latency` stage from the contamination-threshold-gated
series in this phase, so there is nothing to pin.

Two invocations, one per published period:

```sh
sudo /usr/local/sbin/nr-run-measurement stop-harness characterise \
    --rig-slug precision3591

sudo /usr/local/sbin/nr-run-measurement stop-harness abort-latency \
    --period-ns 33000 --trials 200000 --rig-slug precision3591 \
    --characterisation-tsv /home/d0nmega/neurorust/measurements/<characterise-run-id>/clock-characterisation.tsv

sudo /usr/local/sbin/nr-run-measurement stop-harness abort-latency \
    --period-ns 1000000 --trials 200000 --rig-slug precision3591 \
    --characterisation-tsv /home/d0nmega/neurorust/measurements/<characterise-run-id>/clock-characterisation.tsv
```

`--hot-cpu`, `--abort-cpu`, `--cpu-a`, `--cpu-b` and `--priority` all default to the rig's real
isolated-core placement (7, 8, 7, 8, 80) and do not need to be passed; `--characterisation-tsv` is
optional but should point at the characterise run's own output, so the published abort-latency
report carries the D-35 decomposition figures rather than stating them unavailable.

A refusal looks exactly like a `run` refusal: a diagnostic on stderr and a non-zero exit before
`systemd-run` is ever invoked (an unknown subcommand, a pinned path passed on the command line, a
missing or group-writable binary), or, once the unit starts, an `ATTEMPT.json` in what would have
been the run directory naming the precondition that refused it (the rig boots untuned, so a
`characterise` or `abort-latency` run taken before `rt-tuning.service` has been applied refuses on
the governor check; this is the mechanism working, not a defect to route around).

Output lands under `$MEASUREMENTS_ROOT` (`/home/d0nmega/neurorust/measurements`), one directory
per run, named by the same `<date>-<rig-slug>-<run-class>[-NN]` convention `run` already uses:
`recon` for `characterise`, `headline` for `abort-latency`. Watch and read the result the same way
as `run`'s own section above (`journalctl -fu nr-measurement`).

No sudoers change was needed for this subcommand. `deploy/sudoers/nr-measurement` grants
`/usr/local/sbin/nr-run-measurement` by script path with no argument restriction in the sudoers
line itself; the script is what restricts arguments (the `--measurements-root`/`--thresholds` pin
loop), and that loop already covers whatever `"$@"` holds regardless of which subcommand token
leads it.

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
