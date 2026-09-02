# Measurement index

Every run directory under `measurements/` has a row here, including a contaminated, regressed, or refused run: BENCH-06 requires a losing configuration to be reported, not omitted.

| date | run id | class | instrument class | provenance tier | verdict | p99 us | max us | in series | reason |
|------|--------|-------|-------------------|------------------|---------|--------|--------|-----------|--------|
| 2026-08-28 | 2026-08-28-precision3591 | screen | headline-series | reconstructed | contaminated | 9 | 3806 | no | SSH commands were executed against the machine during the cyclictest run (ps -L, tmux capture-pane, scp) and a GNOME session was active with a user typing into it; /proc/interrupts showed CAL counts of roughly 137,000 on CPUs 6 to 11 |
| 2026-09-01 | 2026-09-01-precision3591-calibration-clean | calibration-clean | headline-series | harness-generated | uncalibrated | 9 | 78 | no | no calibrated contamination thresholds exist yet (D-17) |
| 2026-09-02 | 2026-09-02-precision3591-calibration-contaminated | calibration-contaminated | headline-series | harness-generated | uncalibrated | 9 | 3856 | no | excluded_from_series forced true: --allow-precondition-violation waived 3 precondition violation(s) for this calibration-contaminated run: NoActiveSshSessions (observed "1", expected "0"); DisplayManagerInactive (observed "gdm.service=active, sddm.service=inactive, lightdm.service=inactive", expected "inactive"); NoGraphicalSession (observed "1", expected "0") |
