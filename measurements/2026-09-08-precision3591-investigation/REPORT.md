# Run report

run id: 2026-09-08-precision3591-investigation
run class: investigation
instrument class: investigation
provenance tier: harness-generated
utc start: 2026-09-08T01:45:22.718460212Z
utc end: 2026-09-08T03:45:22.89423519Z
manifest blake3: 4b68c539b5680dd21a6b869c7d08ab22b3a1e23c5d4fcd6cdbed5996eab4efbf

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
| thermal-headroom-at-start | pass | 63.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | not-applicable | nop instances=none samplers=none | current_tracer=nop, events/enable=0, set_event=(empty), tracing_on=0, no live tracing instance, no osnoise/timerlat kthread on the target cpus |

## Series admission

admitted: no
decided from evidence upstream of the measured latency; the contamination verdict below is recorded and does not decide this.

| evidence | observed | disposition |
|----------|----------|-------------|
| precondition-waiver | not given | clean |
| fixture-use | none | clean |
| tool-exit-codes | cyclictest=0 | clean |
| preconditions | 14 pass, 0 fail, 1 not-applicable, 0 unavailable | clean |
| interference-counters | cal max 1 on cpu11, tlb max 1 on cpu11, res max 79 on cpu7, irq max 1719 on cpu7, over 7200s | not-thresholded |
| smi-delta | cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0 | not-thresholded |
| thermal-maximum | 67.1 C | not-thresholded |
| instrument-class | investigation | excluding |

exclusions:
- instrument class investigation: a traced run inflates the latency it measures, so its numbers never feed the regression series (the two-instrument rule, docs/measurement-protocol.md)

## Contamination verdict

verdict: clean
reason: instrument class investigation: a traced run inflates the latency it measures, so its numbers never feed the regression series (the two-instrument rule, docs/measurement-protocol.md)
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 8.9 |
| thread-max spread | 75.3% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 1 | 1 | 30 | 754 |
| 7 | 1 | 1 | 79 | 1719 |
| 8 | 1 | 1 | 14 | 187 |
| 9 | 1 | 1 | 3 | 40 |
| 10 | 1 | 1 | 27 | 240 |
| 11 | 1 | 1 | 33 | 1289 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 3 |
| p95 | 4 |
| p99 | 10 |
| p99.9 | 11 |

sample count: 215999991
overflow count: 0
maximum: 89 us

## Distribution

ASCII histogram, bins 2 to 89 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | #################################### 20326198
     3 us | ######################################## 159575065
     4 us | #################################### 25940447
     5 us | ################################ 3788999
     6 us | ########################### 286099
     7 us | ###################### 33747
     8 us | ############################# 848945
     9 us | ############################### 2540005
    10 us | ############################### 1868184
    11 us | ############################ 635706
    12 us | ######################### 132270
    13 us | ##################### 20688
    14 us | ################# 2782
    15 us | ############# 467
    16 us | ########### 163
    17 us | ########## 91
    18 us | ######## 48
    19 us | ####### 30
    20 us | ####### 24
    21 us | #### 6
    22 us | #### 5
    23 us | ## 2
    25 us | # 1
    27 us | # 1
    28 us | ## 2
    29 us | ## 2
    30 us | # 1
    31 us | ### 3
    33 us | # 1
    48 us | # 1
    49 us | # 1
    50 us | # 1
    52 us | # 1
    71 us | # 1
    84 us | # 1
    88 us | ## 2
    89 us | # 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 114 | ad33864dbcefd73a70e8d2e4bf729822869bc47a99ffb63c1c3311d8144c44ad |
| cyclictest.hist | 2181 | 6e8f50e4e5c0faab5d672945447598ba684ed84dd42e8a5c23399b9cf56ef0ef |
| cyclictest.json | 4479 | 3869d1a6d49511bd8a4c30cd24bfd267631b6c6f8bd0b17e661a398be3ac449c |
