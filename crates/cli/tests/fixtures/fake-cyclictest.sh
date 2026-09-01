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

if [ -n "$histfile" ]; then
  cp "$REAL_HIST" "$histfile"
fi

if [ -n "$jsonfile" ]; then
  # Minimal but schema-valid --json output, derived from the real schema
  # captured in crates/histogram/tests/fixtures/probe-cyclictest-60s.json. The
  # per-thread max values here match $REAL_HIST's own footer exactly
  # (# Max Latencies: 03785 03679 03787 03693 03806 03726), so
  # nr_histogram::json::reconcile accepts the pairing.
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
    "0": {"histogram": {"1": 1}, "cycles": 3000000, "min": 1, "max": 3785, "avg": 2.0, "cpu": 6, "node": 0},
    "1": {"histogram": {"1": 1}, "cycles": 3000000, "min": 1, "max": 3679, "avg": 2.0, "cpu": 7, "node": 0},
    "2": {"histogram": {"1": 1}, "cycles": 3000000, "min": 1, "max": 3787, "avg": 2.0, "cpu": 8, "node": 0},
    "3": {"histogram": {"1": 1}, "cycles": 3000000, "min": 1, "max": 3693, "avg": 2.0, "cpu": 9, "node": 0},
    "4": {"histogram": {"1": 1}, "cycles": 3000000, "min": 1, "max": 3806, "avg": 2.0, "cpu": 10, "node": 0},
    "5": {"histogram": {"1": 1}, "cycles": 3000000, "min": 1, "max": 3726, "avg": 2.0, "cpu": 11, "node": 0}
  }
}
JSON
fi

exit "${FAKE_CYCLICTEST_EXIT:-0}"
