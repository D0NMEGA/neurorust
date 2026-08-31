# Measurement index

Every run directory under `measurements/` has a row here, including a contaminated, regressed, or refused run: BENCH-06 requires a losing configuration to be reported, not omitted.

| date | run id | class | instrument class | provenance tier | verdict | p99 us | max us | in series | reason |
|------|--------|-------|-------------------|------------------|---------|--------|--------|-----------|--------|
| 2026-08-28 | 2026-08-28-precision3591 | screen | headline-series | reconstructed | contaminated | 9 | 3806 | no | SSH commands were executed against the machine during the cyclictest run (ps -L, tmux capture-pane, scp) and a GNOME session was active with a user typing into it; /proc/interrupts showed CAL counts of roughly 137,000 on CPUs 6 to 11 |
