#!/bin/sh
# Regenerates docs/proofs/emergency-stop-proof-report.md from a real cargo kani run (D-54).
#
# On demand, not on every CI run. D-54 asks for the report to be refreshed deliberately and
# reviewed like any published claim under D-07, and a file that rewrites itself on every push is
# neither deliberate nor reviewed. The report records the commit it was generated from so a
# reader can see whether it describes the tree in front of them; a dirty tree is recorded, not
# refused.
#
# If the run fails, this script still renders the report (with the failing rows marked and the
# final verdict line intact) and then exits non-zero: a generator that refuses to write a report
# when the news is bad is a generator that only ever publishes good news.
#
# Usage: scripts/nr-proof-report.sh
set -eu

OUT=docs/proofs/emergency-stop-proof-report.md
RAW=$(mktemp -t nr-proof.XXXXXX)
trap 'rm -f "$RAW"' EXIT

KANI_VERSION=$(cargo kani --version)
SHA=$(git rev-parse HEAD)
DIRTY=clean
if [ -n "$(git status --porcelain)" ]; then DIRTY=dirty; fi

STATUS=0
START=$(date +%s)
cargo kani -p nr-stop > "$RAW" 2>&1 || STATUS=$?
ELAPSED=$(( $(date +%s) - START ))

python3 - "$RAW" "$OUT" "$KANI_VERSION" "$SHA" "$DIRTY" "$ELAPSED" <<'PY'
import re
import sys

raw_path, out_path, kani_version, sha, dirty, elapsed = sys.argv[1:7]

with open(raw_path) as f:
    raw_lines = f.read().splitlines()

# The explicit harness -> (family, requirement) mapping D-46 assigns. Not inferred from the
# harness name at render time: a harness added to proofs.rs without a matching row here shows
# up as an obviously incomplete row below, not a guess, and a mapped harness missing from the
# actual run (a rename, for example) shows up as "missing from run" rather than being silently
# dropped.
MAPPING = [
    ("step_is_total", "1", "STOP-01"),
    ("state_raw_round_trips", "1", "STOP-01"),
    ("state_from_raw_is_total_and_fails_closed", "1", "STOP-01"),
    ("cause_raw_round_trips_and_is_total", "1", "STOP-01"),
    ("abort_never_returns_to_running", "2", "STOP-02"),
    ("running_is_never_reachable_from_stopping_or_stopped", "2", "STOP-02"),
    ("stopped_is_terminal", "2", "STOP-02"),
    ("no_permit_when_not_running", "3", "STOP-03"),
    ("permit_supply_closes_on_the_abort_edge", "3", "STOP-03"),
    ("modelled_consumer_never_emits_after_a_stop", "3", "STOP-03"),
    ("consumer_emit_never_overflows", "4", "STOP-04"),
]

# Parse per-harness blocks. Each begins at "Checking harness <path>::<name>..." and its own
# check count and outcome are the next "** X of Y failed" and "VERIFICATION:- ..." lines
# before the next "Checking harness" line or end of output.
check_re = re.compile(r"^Checking harness (?:\w+::)*([A-Za-z_][A-Za-z0-9_]*)\.\.\.$")
summary_re = re.compile(r"^ \*\* (\d+) of (\d+) failed$")

blocks = []
current_name = None
current_lines = []
for line in raw_lines:
    m = check_re.match(line)
    if m:
        if current_name is not None:
            blocks.append((current_name, current_lines))
        current_name = m.group(1)
        current_lines = []
    elif current_name is not None:
        current_lines.append(line)
if current_name is not None:
    blocks.append((current_name, current_lines))

results = {}
total_checks = 0
total_failed = 0
for name, block_lines in blocks:
    outcome = None
    checks_in_block = 0
    failed_in_block = 0
    for line in block_lines:
        m = summary_re.match(line)
        if m:
            failed_in_block = int(m.group(1))
            checks_in_block = int(m.group(2))
        if line.startswith("VERIFICATION:- "):
            outcome = line.strip()
    results[name] = outcome or "VERIFICATION:- UNKNOWN (no verdict line found for this harness)"
    total_checks += checks_in_block
    total_failed += failed_in_block

total_succeeded = total_checks - total_failed

verdict_line = "no final verdict line found in the run output"
for line in reversed(raw_lines):
    if line.startswith("Complete - ") and line.rstrip().endswith("total."):
        verdict_line = line.strip()
        break

mapped_names = {name for name, _, _ in MAPPING}
extra = sorted(n for n in results if n not in mapped_names)

rows = []
for name, family, req in MAPPING:
    rows.append((name, family, req, results.get(name, "missing from run")))
for name in extra:
    rows.append((name, "unclassified", "unclassified", results[name]))

out = []
out.append("# Emergency-stop proof report")
out.append("")
out.append("## What this report is")
out.append("")
out.append(
    "The committed result of a Kani solver run over crates/stop (package nr-stop), and the "
    "evidence behind STOP-04. The crate contains no unsafe code, so most classes of "
    "undefined behaviour are already excluded by the type system before Kani runs. What this "
    "report adds is a totality proof over the enumerated state space, the latch property, "
    "the gate invariant, and panic and overflow freedom, harness by harness below."
)
out.append("")
out.append("## Provenance")
out.append("")
out.append("- Kani version: " + kani_version)
out.append("- Generated from commit: " + sha + " (" + dirty + ")")
out.append("- Wall clock: " + elapsed + "s")
out.append("")
out.append("## Harnesses")
out.append("")
out.append("One row per harness: which D-46 family it belongs to, which requirement it")
out.append("serves, and its outcome, in Kani's own verdict vocabulary.")
out.append("")
out.append("| Harness | Family | Requirement | Outcome |")
out.append("|---|---|---|---|")
for name, family, req, outcome in rows:
    out.append("| " + name + " | " + family + " | " + req + " | " + outcome + " |")
out.append("")
out.append("## Checks")
out.append("")
out.append(
    str(total_checks) + " checks reported across every harness above, "
    + str(total_succeeded) + " succeeded."
)
out.append("")
out.append("## Verdict")
out.append("")
out.append("The final line of the run, verbatim:")
out.append("")
out.append("    " + verdict_line)
out.append("")
out.append("## What this report does not say")
out.append("")
out.append(
    "This report covers the sequential transition function and the gate predicate. It says "
    "nothing about thread interleavings: crate::latch's atomic publication layer is "
    "unverified here and stays unverified until CHAN-06 brings loom into CI in Phase 4. See "
    "docs/proofs/emergency-stop-proof-scope.md for the full statement of what is proved, "
    "what the type system already excludes, and what nothing in this phase covers."
)
out.append("")
out.append("## Regenerating this report")
out.append("")
out.append("    sh scripts/nr-proof-report.sh")
out.append("")
out.append(
    "This command re-runs the solver and rewrites this file. It does not run in CI and is "
    "not a gate; it is refreshed deliberately and reviewed like any other published claim, "
    "per D-07."
)
out.append("")

with open(out_path, "w") as f:
    f.write("\n".join(out))
PY

exit "$STATUS"
