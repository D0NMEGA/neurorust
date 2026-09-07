# Run report

run id: 2026-09-06-precision3591-screen-02
run class: screen
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-06T21:25:02.326588752Z
utc end: 2026-09-06T21:41:03.579725025Z
manifest blake3: 4d797413d9ff36663c55afd5bbc49a72c1baa52eca778d85f69ae6f5f1dad89b

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
| thermal-headroom-at-start | pass | 66.1 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | current_tracer=nop events/enable=0 set_event=(empty) tracing_on=0 | current_tracer=nop, events/enable=0, set_event=(empty), tracing_on=0 |

## Contamination verdict

verdict: contaminated
reason: contamination verdict Contaminated was reached against provisional (D-24, not yet calibrated) thresholds; see config/contamination-thresholds.json
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 83335.7 |
| thread-max spread | 0.0% |
| overflow rate | 5.9942/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | context switches | irqs |
|-----|----------|----------|-------------------|------|
| 6 | 734 | 1 | 31 | 119 |
| 7 | 731 | 1 | 130 | 50 |
| 8 | 730 | 1 | 51 | 55 |
| 9 | 730 | 1 | 132 | 3 |
| 10 | 730 | 1 | 140 | 13 |
| 11 | 730 | 1 | 37 | 51 |

## Firmware screen

instrument: rtla-hwnoise
tool version: rtla hwnoise: a summary of hardware-related noise (version 7.0.12)
invocation: rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s
requested cpus: 6,7,8,9,10,11
observed cpus: 6,7,8,9,10,11
per-cpu exposure: cpu6=2250.00s cpu7=2250.00s cpu8=2250.00s cpu9=2250.00s cpu10=2250.00s cpu11=2250.75s
maximum: 7 us (the largest Max Single value (one-shot hardware-noise event) across the observed CPUs' final rtla hwnoise rows)
events recorded: 39

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
| p99 | 9 |
| p99.9 | 11 |

sample count: 450314
overflow count: 360
maximum: 750021 us

## Distribution

ASCII histogram, bins 2 to 16 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 294271
     3 us | ##################################### 98593
     4 us | ################################## 39630
     5 us | ######################### 2352
     6 us | ############### 97
     7 us | ######################### 2404
     8 us | ############################ 6833
     9 us | ########################### 4330
    10 us | ####################### 1216
    11 us | ################# 193
    12 us | ########### 30
    13 us | ##### 4
    16 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 61 | 8b3a6f104bfc5f0d3ebeb25de3dd7005487c3692fe24edd2ad22930be87ee508 |
| rtla-hwnoise.txt | 757218 | 81972591651c8d871927843b39ab0ee3729c42fb102ac7868d1968bf54203966 |
| cyclictest.hist | 3144 | 7194cf6d3c012648152e83e2a8dbfe864d3076e35e12e6eb9b2bd402f4205e17 |
| cyclictest.json | 3039 | 7b28bbf162b2af5744fb812623b20ac03135136961011155671e641d883c596b |
