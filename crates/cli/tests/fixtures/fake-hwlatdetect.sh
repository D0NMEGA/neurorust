#!/bin/sh
# Fake hwlatdetect for nrmeasure's end-to-end pipeline test (plan 01-07, task 3).
# Writes the real 2026-08-28 tuned-idle capture to stdout: nrmeasure reads this
# tool's stdout directly (hwlatdetect has no --histfile-style output flag), so
# this is what a real invocation's captured output looks like.

if [ "$1" = "--version" ]; then
  echo "hwlatdetect version 2.9-1ubuntu1"
  exit 0
fi

cat "$(dirname "$0")/../../../../measurements/2026-08-28-precision3591/hwlatdetect-tuned-15m.txt"
exit 0
