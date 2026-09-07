# Publication layout

This document answers, in one place, every publication question `.planning/phases/
01-trustworthy-measurement/01-CONTEXT.md` left to discretion: where raw captures live,
what is committed and what is a pointer, whether histograms are images or generated, how
BENCH-06's losing configurations appear, and what a checksum does and does not prove. Each
answer states the decision and the reason, because the reason is what lets a later phase
change it deliberately rather than by accident. It describes the container the harness
already builds, never the contents; see `docs/measurement-protocol.md` for the conditions a
run must be taken under before it can appear here at all.

## Directory layout

`measurements/<YYYY-MM-DD>-<rig-slug>-<run-class>[-NN]/`. The date is the UTC date the run
started, `rig-slug` is the operator-chosen machine label (e.g. `precision3591`), and
`run-class` is one of the eight run classes (`recon`, `screen`, `calibration-clean`,
`calibration-contaminated`, `investigation`, `headline`, `weekly`, `soak`). `-NN` is
appended from the second run of the same date, rig and class onward, starting at `-02`.

`measurements/2026-08-28-precision3591/` keeps its pre-convention name. Renaming it would
break the links already published in `PROJECT.md` and `REQUIREMENTS.md`, so it carries a
reconstructed manifest instead of a rename.

## What a run directory contains

`manifest.json` (the source of truth), `REPORT.md` (generated, never hand-edited),
`hist.tsv` (derived from the raw capture, so a reader can plot without a Rust toolchain),
and the raw captures byte-identical to what the tool emitted. Raw captures are never
rewritten to embed provenance headers; the sidecar manifest exists precisely so they stay
byte-identical.

## Raw captures: in the repository or behind a pointer

In the repository by default. The thresholds match `crates/cli/src/rundir.rs` exactly:
25 MiB per file and 100 MiB per run directory. A cyclictest histogram from a 1 hour run
is a few tens of kilobytes and hwlatdetect output is smaller, so those are always
committed. The threshold exists for the larger ftrace and trace-cmd captures a PLAT-01
investigation run can produce. A file over the limit is compressed with `zstd -19` first;
only if it is still over the limit is it recorded as an external pointer with its blake3
still in the manifest, so the pointer remains verifiable even though its bytes live
elsewhere.

## Histograms

Generated on demand from the raw capture. No rendered images are committed. The
in-repository visual form is the ASCII histogram in the generated `REPORT.md`, on a
logarithmic count axis at 1 us resolution, plus `hist.tsv` for anyone who wants to plot it
themselves. An ASCII histogram diffs cleanly in review and a PNG does not, and BENCH-05
asks for histograms rather than means, not for a particular image format.

## Losing configurations

This is how BENCH-06's requirement that losing configurations are reported, not omitted,
is operationalised:

- No run directory is ever deleted, and no run is ever left out of `measurements/INDEX.md`.
- A run kept out of the headline series carries `excluded_from_series: true` and a
  non-empty `exclusion_reason` in its manifest; the manifest fails validation without the
  reason. Both are derived from `series_admission` (D-28), the manifest's own record of
  which upstream evidence sources were consulted and which of them excluded the run; a
  manifest whose two summary fields disagree with that record fails validation too.
- `INDEX.md` has one row per run directory: its date, run id, class, instrument class,
  provenance tier, contamination verdict, p99, maximum, whether it is in the series, and
  the exclusion reason when it is not.
- `INDEX.md` is generated, and CI checks it against the generated form rather than
  regenerating it, so removing a row by hand fails the build.
- The deliberately contaminated arm of the D-17 calibration pair is a published result, not
  a discarded one: it documents what contamination looks like, which is what a third party
  needs in order to avoid it.

## What a checksum proves

blake3 checksums prove integrity, not authenticity. They detect accidental corruption and
prove a capture matches its manifest. They do not prove who produced the capture. There is
no signing story in v1. This is stated plainly here rather than left for a reader to infer
more from the presence of hashes than they support.

## Where captures may live

The provenance gate (`crates/cli/src/cmd/verify.rs`) scans the whole tracked tree by file
shape, not only by directory, so a capture-shaped file moved out of `measurements/` fails
the build; this is deliberate. Two trees are exempt:

- `crates/*/tests/fixtures/` holds parser fixtures. They are inputs to tests, not figures.
- `docs/rig/recon-*/` holds schema probes from a rig recon session, every one prefixed
  `probe-`. They pin down output formats, and no number from them is ever published.

## The metrics series

`metrics/latency-series.json` (append only, auto-committed by the rig),
`metrics/baseline.json` (the reviewed comparison baseline, changed only by a human in a
reviewed commit), and `metrics/coverage.json` (one entry per ISO week: a run reference, or
a gap with a reason). This is D-07's split made concrete: the weekly p50, p95 and p99 JSON
is a regression-tracking artifact and auto-commits, while anything that becomes a
published claim (a histogram, a stated verdict, a README figure) reaches `main` only
through a reviewed commit.

## Provenance tiers

`harness-generated` and `reconstructed`. A `harness-generated` manifest was written by the
harness while the run happened, with the full D-14 contract populated by code that was
present when the run happened. A `reconstructed` manifest was rebuilt afterward from other
evidence (`RIG.txt`, a README) for a capture taken before the harness existed; every field
it could not recover is listed in the manifest's `absent_fields` with a specific reason,
never guessed. A reader tells the two apart at a glance from the manifest's own
`provenance_tier` field, or from the `provenance tier:` line at the top of the generated
`REPORT.md`.
