#!/bin/sh
# Fake cyclictest for nrmeasure's end-to-end pipeline test (plan 01-07, task 3).
# Emits the committed 2026-08-28 real capture so the orchestration, manifest
# emission and report rendering are all covered without a rig. nrmeasure never
# invokes this through a shell; it is exec'd directly with an explicit argv.
#
# FAKE_CYCLICTEST_EXIT overrides the exit code, so the non-zero tool-exit path
# (BENCH-06: recorded, not hidden) is testable without a real failing tool.

# FAKE_CYCLICTEST_MARKER, if set, is touched on every invocation EXCEPT --version.
# It lets a test prove that the measurement never started, which is how the
# fail-fast ordering of run.rs step 1 is asserted rather than assumed.
#
# FAKE_CYCLICTEST_ARGV_FILE, if set, receives the exact argv this script was
# invoked with (excluding --version calls), one argument per line. A test uses
# this to assert the manifest records the executed argv byte for byte, rather
# than trusting a hand-reconstructed expectation of what the harness passed.
#
# FAKE_CYCLICTEST_SENTINEL, if set, is touched the moment this script starts its
# real (non --version) invocation, before FAKE_CYCLICTEST_SLEEP_SECONDS. A test
# polls for this file to prove the harness's ATTEMPT.json already exists while
# the tool is still running, rather than asserting on timing alone.
#
# FAKE_CYCLICTEST_SLEEP_SECONDS, if set, sleeps that many seconds before writing
# any output, so a test has a window in which to observe the harness's
# in-progress state.
#
# FAKE_CYCLICTEST_STDERR, if set, is written verbatim to stderr, so a test can
# assert the harness preserves a tool's stderr to <tool>.stderr.txt.
#
# FAKE_CYCLICTEST_BAD_HIST, if set, writes a syntactically invalid .hist file
# instead of the real capture, so a test can exercise the parse-failure path
# (finding 7 of 01-EXTERNAL-AUDIT.md) without a real broken tool.

REAL_HIST="$(dirname "$0")/../../../histogram/tests/fixtures/cyclictest-rt-isolated-idle-10m.hist"

histfile=""
jsonfile=""
for arg in "$@"; do
  case "$arg" in
    --version)
      echo "rt-tests version 2.9-1ubuntu1 (cyclictest V 2.80)"
      exit 0
      ;;
    --histfile=*)
      histfile="${arg#--histfile=}"
      ;;
    --json=*)
      jsonfile="${arg#--json=}"
      ;;
  esac
done

if [ -n "$FAKE_CYCLICTEST_MARKER" ]; then
  : > "$FAKE_CYCLICTEST_MARKER"
fi

if [ -n "$FAKE_CYCLICTEST_ARGV_FILE" ]; then
  for arg in "$@"; do
    printf '%s\n' "$arg" >> "$FAKE_CYCLICTEST_ARGV_FILE"
  done
fi

if [ -n "$FAKE_CYCLICTEST_SENTINEL" ]; then
  : > "$FAKE_CYCLICTEST_SENTINEL"
fi

if [ -n "$FAKE_CYCLICTEST_SLEEP_SECONDS" ]; then
  sleep "$FAKE_CYCLICTEST_SLEEP_SECONDS"
fi

if [ -n "$FAKE_CYCLICTEST_STDERR" ]; then
  printf '%s\n' "$FAKE_CYCLICTEST_STDERR" >&2
fi

if [ -n "$histfile" ]; then
  if [ -n "$FAKE_CYCLICTEST_BAD_HIST" ]; then
    printf 'this is not a valid cyclictest histogram\n' > "$histfile"
  else
    cp "$REAL_HIST" "$histfile"
  fi
fi

if [ -n "$jsonfile" ]; then
  # Minimal but schema-valid --json output, derived from the real schema
  # captured in crates/histogram/tests/fixtures/probe-cyclictest-60s.json. The
  # per-thread max values here match $REAL_HIST's own footer exactly
  # (# Max Latencies: 03785 03679 03787 03693 03806 03726), so
  # nr_histogram::json::reconcile accepts the pairing. `cycles` is likewise the
  # real per-thread sample total from $REAL_HIST (binned counts plus that
  # thread's own overflow count from "# Histogram Overflows:"), not a rounded
  # 3000000: 01-19 task 3 made reconcile() check sample count as well as
  # maxima, and a flat, wrong cycles value now fails the pairing it used to
  # pass silently (finding 6 of 01-EXTERNAL-AUDIT.md).
  cat > "$jsonfile" <<'JSON'
{
  "file_version": 1,
  "cmdline:": "cyclictest --mainaffinity=0,1 --affinity=6-11 --threads=6 --mlockall --policy=fifo --priority=99 --interval=200 --distance=0 --histogram=400 --histfile=cyclictest-rt-isolated-idle-10m.hist --json=cyclictest.json --duration=600",
  "rt_test_version:": "2.80",
  "start_time": "Fri, 28 Aug 2026 00:00:00 -0500",
  "end_time": "Fri, 28 Aug 2026 00:10:00 -0500",
  "return_code": 0,
  "sysinfo": {
    "sysname": "Linux",
    "nodename": "[redacted]",
    "release": "7.0.0-30-realtime",
    "version": "#30-Ubuntu SMP PREEMPT_RT Fri Jul 31 18:22:54 UTC 2026",
    "machine": "x86_64",
    "realtime": 1
  },
  "num_threads": 6,
  "resolution_in_ns": 0,
  "thread": {
    "0": {"histogram": {"1": 1}, "cycles": 2999329, "min": 1, "max": 3785, "avg": 2.0, "cpu": 6, "node": 0},
    "1": {"histogram": {"1": 1}, "cycles": 2999297, "min": 1, "max": 3679, "avg": 2.0, "cpu": 7, "node": 0},
    "2": {"histogram": {"1": 1}, "cycles": 2999302, "min": 1, "max": 3787, "avg": 2.0, "cpu": 8, "node": 0},
    "3": {"histogram": {"1": 1}, "cycles": 2999303, "min": 1, "max": 3693, "avg": 2.0, "cpu": 9, "node": 0},
    "4": {"histogram": {"1": 1}, "cycles": 2999313, "min": 1, "max": 3806, "avg": 2.0, "cpu": 10, "node": 0},
    "5": {"histogram": {"1": 1}, "cycles": 2999300, "min": 1, "max": 3726, "avg": 2.0, "cpu": 11, "node": 0}
  }
}
JSON
fi

exit "${FAKE_CYCLICTEST_EXIT:-0}"
