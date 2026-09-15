#!/bin/sh
# STOP-06: 100 percent branch coverage of crates/stop, and nothing else (D-53).
#
# Branch coverage is not available on the toolchain rust-toolchain.toml pins. cargo-llvm-cov's
# own README states the flag "requires nightly", the underlying -Z coverage-options=branch is an
# unstable compiler flag, and cargo-tarpaulin's help reads "Branch coverage: NOT IMPLEMENTED", so
# tarpaulin cannot do it at any toolchain. D-57 resolves this by pinning a dated nightly for this
# command alone; every other job in CI stays on stable.
#
# The threshold is NOT enforced with --fail-under-branches: cargo-llvm-cov 0.9.1 does not even
# offer that flag (only --fail-under-functions/-lines/-file-lines/-regions exist). The gate reads
# the JSON export instead, because that same read is what makes the gate non-vacuous: a coverage
# run that instrumented nothing reports no uncovered branches and would satisfy any percentage
# threshold while proving nothing at all.
set -eu

NIGHTLY=nightly-2026-08-01

# The workflow's coverage job pins the same date as a second, independent copy of this one
# fact (plan 02-04, D-57). Two copies of one fact is a drift risk on its own: catch a mismatch
# here, inside the gate itself, before it ever reaches main, rather than trusting a human to
# keep a shell variable and a YAML line in sync by eye.
WORKFLOW=.github/workflows/ci.yml
WORKFLOW_NIGHTLY=$(grep -oE 'toolchain: nightly-[0-9-]+' "$WORKFLOW" | head -1 | sed 's/toolchain: //')
if [ "$WORKFLOW_NIGHTLY" != "$NIGHTLY" ]; then
    echo "nr-coverage.sh: nightly date mismatch: this script pins $NIGHTLY, $WORKFLOW pins ${WORKFLOW_NIGHTLY:-<none found>}" >&2
    exit 1
fi

OUT=$(mktemp -t nr-coverage.XXXXXX)
trap 'rm -f "$OUT"' EXIT

cargo "+$NIGHTLY" llvm-cov --branch -p nr-stop \
    --ignore-filename-regex '(^|/)crates/stop/tests/' \
    --json --output-path "$OUT"

python3 - "$OUT" <<'PY'
import json, pathlib, sys

report = json.load(open(sys.argv[1]))
data = report["data"][0]
totals = data["totals"]
branches = totals["branches"]

# lib.rs is the crate root: a doc comment plus `pub mod` declarations, no function of its own
# (the same shape already established by crates/histogram/src/lib.rs). A file with no function
# cannot contain a branch, so LLVM's source-based coverage never lists it in the report at all,
# confirmed empirically against this crate before this check was written. Requiring its presence
# here would make the gate permanently red over a file that is correctly empty of anything to
# cover. If lib.rs ever gains real logic of its own, this exclusion needs to be revisited.
#
# proofs.rs is declared `#[cfg(kani)] mod proofs;` in lib.rs (plan 02-03). That cfg is set only
# by `cargo kani` itself, never by a normal cargo build, so this llvm-cov run (a normal build,
# even under the pinned nightly) never compiles the module and it can never appear in the report,
# independent of how well the rest of the crate is tested. Its own exhaustive verification is
# scripts/nr-proofs.sh (STOP-05), a different gate for a different kind of code: Kani proves
# harnesses across their whole input space, which is not something a test-coverage tool can
# measure at all. Confirmed empirically before this exclusion was written: with proofs.rs
# unexcluded, this script named it "not instrumented" even though branches read 6/6. If proofs.rs
# ever gains code reachable outside cfg(kani), this exclusion needs to be revisited.
EXCLUDED_FROM_PRESENCE_CHECK = {"lib.rs", "proofs.rs"}
src = sorted(
    p.name
    for p in pathlib.Path("crates/stop/src").glob("*.rs")
    if p.name not in EXCLUDED_FROM_PRESENCE_CHECK
)
covered_files = sorted({pathlib.Path(f["filename"]).name for f in data["files"]})
missing = [name for name in src if name not in covered_files]

problems = []
if missing:
    problems.append("not instrumented: " + ", ".join(missing))
if branches["count"] == 0:
    problems.append("the report contains zero branches; the gate would pass vacuously")
if branches["covered"] != branches["count"]:
    problems.append(
        "%d of %d branches uncovered" % (branches["count"] - branches["covered"], branches["count"])
    )
    for f in data["files"]:
        fb = f["summary"]["branches"]
        if fb["covered"] != fb["count"]:
            problems.append("  %s: %d of %d" % (f["filename"], fb["covered"], fb["count"]))

print("branches %d/%d  regions %d/%d  lines %d/%d" % (
    branches["covered"], branches["count"],
    totals["regions"]["covered"], totals["regions"]["count"],
    totals["lines"]["covered"], totals["lines"]["count"],
))

if problems:
    for p in problems:
        print("nr-coverage.sh: " + p, file=sys.stderr)
    sys.exit(1)
PY
