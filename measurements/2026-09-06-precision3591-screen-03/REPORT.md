# Run report

run id: 2026-09-06-precision3591-screen-03
run class: screen
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-06T22:32:50.924019542Z
utc end: 2026-09-06T22:48:52.037124854Z
manifest blake3: 0bd5cc00f16266354f57500415152bf0c1171dcf17726e95f60bdea9806bed94

## Rig and tuning

```
system:       Dell Inc. Precision 3591
bios:         1.23.0 released 04/24/2026
cpu:          Intel(R) Core(TM) Ultra 9 185H
topology:     22 logical / 16 cores / 1 socket
p_cores:      
e_cores:      
microcode:    0x28
kernel:       7.0.0-30-realtime (#30.1-Ubuntu SMP PREEMPT_RT Fri Aug  7 13:54:26 UTC 2026)
os:           Ubuntu 26.04.1 LTS (Resolute Raccoon)
session:      installed display_manager_active=false
memory_gb:    30
cmdline:      BOOT_IMAGE=/boot/vmlinuz-7.0.0-30-realtime root=[redacted] ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5,12-21 intel_idle.max_cstate=1 processor.max_cstate=1 nosoftlockup nowatchdog nmi_watchdog=0 tsc=reliable skew_tick=1 crashkernel=2G-4G:320M,4G-32G:512M,32G-64G:1024M,64G-128G:2048M,128G-:4096M
governor:     cpu0=performance cpu1=performance cpu2=performance cpu3=performance cpu4=performance cpu5=performance cpu6=performance cpu7=performance cpu8=performance cpu9=performance cpu10=performance cpu11=performance cpu12=performance cpu13=performance cpu14=performance cpu15=performance cpu16=performance cpu17=performance cpu18=performance cpu19=performance cpu20=performance cpu21=performance
no_turbo:     1
cstates:      cpu0.POLL=on cpu0.C1E=on
power:        AC=1 battery=Not charging
nic_wired:    enp0s31f6 driver=e1000e state=down
nic_ptp:      hardware-clock-present
nic_wifi:     wlp0s20f3 driver=iwlwifi
```

## Preconditions

| check | status | observed | expected |
|-------|--------|----------|----------|
| no-active-ssh-sessions | pass | 0 | 0 |
| systemd-default-target-is-multi-user | pass | multi-user.target | multi-user.target |
| display-manager-inactive | pass | gdm.service=inactive, sddm.service=inactive, lightdm.service=inactive | inactive |
| no-graphical-session | pass | 0 | 0 |
| no-active-login-sessions | pass | none | no local console login session |
| governor-is-performance-on-all-cpus | pass | performance on 22 CPUs | performance on all CPUs |
| no-turbo-enabled | pass | 1 | 1 |
| deep-cstates-disabled | pass | C6 not present, C10 not present on cpus 6-11 | C6 disabled, C10 disabled |
| isolcpus-covers-target-cpus | pass | 6-11 | 6-11 |
| kernel-is-realtime | pass | 1 | 1 |
| rt-tuning-service-active | pass | active | active |
| on-ac-power | pass | AC=1 | AC=1 |
| thermal-headroom-at-start | pass | 58.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | current_tracer=nop events/enable=0 set_event=(empty) tracing_on=0 | current_tracer=nop, events/enable=0, set_event=(empty), tracing_on=0 |

## Contamination verdict

verdict: clean
reason: contamination verdict Clean was reached against provisional (D-24, not yet calibrated) thresholds; see config/contamination-thresholds.json
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 1.6 |
| thread-max spread | 15.4% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | context switches | irqs |
|-----|----------|----------|-------------------|------|
| 6 | 490 | 1 | 44 | 320 |
| 7 | 487 | 1 | 50 | 319 |
| 8 | 487 | 1 | 23 | 49 |
| 9 | 487 | 1 | 110 | 4 |
| 10 | 487 | 1 | 45 | 13 |
| 11 | 487 | 1 | 85 | 87 |

## Firmware screen

instrument: rtla-hwnoise
tool version: rtla hwnoise: a summary of hardware-related noise (version 7.0.12)
invocation: rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s
requested cpus: 6,7,8,9,10,11
observed cpus: 6,7,8,9,10,11
per-cpu exposure: cpu6=674.25s cpu7=674.25s cpu8=674.25s cpu9=674.25s cpu10=674.25s cpu11=674.25s
maximum: 1 us (the largest Max Single value (one-shot hardware-noise event) across the observed CPUs' final rtla hwnoise rows)
events recorded: 13

- it measures hardware-related noise, the execution gaps left after software noise is accounted for. those are not uniquely identified SMIs either; the exact MSR_SMI_COUNT recorded alongside this screen is the census, and this is not
- its per-CPU figures come from one osnoise sampling thread per CPU in the -c list, so a CPU absent from the observed list was sampled and reported nothing, rather than never being sampled. that is the specific difference from hwlatdetect on this rig
- per-CPU exposure above is what the tool reports; when it reports none, the exposure is unstated rather than a wall-clock duration divided by a CPU count

MSR_SMI_COUNT (0x34) over this run: cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0
the count above is exact and says how many SMIs reached each cpu; it says nothing about how long any of them took

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 8 |
| p99.9 | 9 |

sample count: 1799993
overflow count: 0
maximum: 13 us

## Distribution

ASCII histogram, bins 2 to 13 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 1309074
     3 us | #################################### 359135
     4 us | ################################ 76106
     5 us | ###################### 2329
     6 us | ################### 714
     7 us | ############################ 19001
     8 us | ############################# 24770
     9 us | ######################### 7607
    10 us | #################### 1105
    11 us | ############## 128
    12 us | ######## 18
    13 us | ###### 6
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 61 | 8b3a6f104bfc5f0d3ebeb25de3dd7005487c3692fe24edd2ad22930be87ee508 |
| rtla-hwnoise.txt | 756612 | f92c2b2470d986873ab16f1dda1f8f4cdf383805b253ccef09ca3cf3287f3c82 |
| cyclictest.hist | 935 | 8947dca119b1c905f740d140c47112989e7577293f7a9bc146cff3ac7ee1b100 |
| cyclictest.json | 3044 | ce19d6d76833f74f21efbe0500bc4dce51449b0fa3591fe46d0bb97f5ae0d84d |
