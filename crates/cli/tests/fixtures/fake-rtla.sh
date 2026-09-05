#!/bin/sh
# Fake rtla for nrmeasure's end-to-end pipeline test (plan 01-21, task 1).
# Emits the committed 2026-09-05 rtla hwnoise probe capture on stdout, exactly as the real
# invocation would (rtla hwnoise has no --histfile-style output flag; nrmeasure reads its
# stdout directly, the same convention hwlatdetect already uses).
#
# rtla has no --version flag (docs/rig/recon-2026-09-05/FINDINGS.md); this project probes
# `rtla hwnoise --help` for a version string instead, so this script answers that
# specifically rather than a bare --version.

for arg in "$@"; do
  case "$arg" in
    --help)
      echo 'rtla hwnoise: a summary of hardware-related noise (version 7.0.12)'
      exit 0
      ;;
  esac
done

cat "$(dirname "$0")/../../../../docs/rig/recon-2026-09-05/probe-rtla-hwnoise.txt"
exit 0
