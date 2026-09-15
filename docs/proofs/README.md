# docs/proofs

## What this is

The directory for machine-checked and tool-derived claims about this codebase: results a tool
computed and a human reviewed, rather than a measurement taken on the reference rig. It is a
sibling of `docs/rig/`, which holds measurement claims taken under the Phase 1 protocol.

## What is in it now

- `emergency-stop-proof-report.md`: the committed Kani solver run behind STOP-01 through
  STOP-04, naming the Kani version, the source commit, every harness, and its outcome.
- `emergency-stop-proof-scope.md`: what that proof establishes, what the compiler establishes,
  and what nothing in Phase 2 establishes, including the thread-interleaving gap.
- `phase-6-output-gate-contract.md`: the written contract naming what Phase 6's real decoder and
  sink must not do to keep the STOP-03 guarantee the proof establishes at FSM level.

## What lands here later

Phase 4's loom results for CHAN-06, once loom exhaustively explores the atomic interleavings
`emergency-stop-proof-scope.md` names as unverified. Any future Creusot output for CREU-01, the
v2 functional proof, if that work happens.

## Why proof results live here rather than under measurements/

`measurements/` is governed by the D-13 manifest gate, whose schema describes rig captures: an
environment snapshot, a set of tool invocations, and a per-file blake3 checksum against a
physical run directory. A solver run is none of those things, and forcing one into that schema
would distort both: the manifest schema would grow fields a proof report cannot fill, and a
reader of `measurements/` would have to learn to skip entries that are not measurements (D-54).
