# Run report

run id: 2026-09-07-precision3591-investigation
run class: investigation
instrument class: investigation
provenance tier: harness-generated
utc start: 2026-09-07T20:22:20.099589765Z
utc end: 2026-09-07T20:52:20.303466756Z
manifest blake3: 103b6dd83ab9d08bfef7ac4ee8466a6d784198984c5e359673c0e4cbdaaae9dc

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

admitted: yes
decided from evidence upstream of the measured latency; the contamination verdict below is recorded and does not decide this.

| evidence | observed | disposition |
|----------|----------|-------------|
| precondition-waiver | not given | clean |
| fixture-use | none | clean |
| tool-exit-codes | cyclictest=0 | clean |
| preconditions | 14 pass, 0 fail, 1 not-applicable, 0 unavailable | clean |
| interference-counters | cal max 2 on cpu8, tlb max 1 on cpu11, res max 26 on cpu7, irq max 641 on cpu7, over 1800s | not-thresholded |
| smi-delta | cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0 | not-thresholded |
| thermal-maximum | 70.1 C | not-thresholded |

exclusions: none

## Contamination verdict

verdict: clean
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 7.5 |
| thread-max spread | 76.0% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 1 | 1 | 13 | 309 |
| 7 | 1 | 1 | 26 | 641 |
| 8 | 2 | 1 | 8 | 62 |
| 9 | 1 | 1 | 1 | 5 |
| 10 | 1 | 1 | 4 | 60 |
| 11 | 1 | 1 | 17 | 499 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 3 |
| p95 | 5 |
| p99 | 10 |
| p99.9 | 11 |

sample count: 53999990
overflow count: 0
maximum: 75 us

## Distribution

ASCII histogram, bins 2 to 75 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ################################### 3700600
     3 us | ######################################## 39627685
     4 us | #################################### 7786157
     5 us | ################################ 1288865
     6 us | ########################## 95427
     7 us | ##################### 9751
     8 us | ############################ 191240
     9 us | ############################## 606632
    10 us | ############################## 477553
    11 us | ############################ 173400
    12 us | ######################## 36577
    13 us | #################### 5275
    14 us | ############### 640
    15 us | ########### 101
    16 us | ######## 31
    17 us | ###### 13
    18 us | ###### 12
    19 us | ###### 12
    20 us | ##### 8
    21 us | ### 3
    22 us | ### 2
    26 us | ## 1
    66 us | ## 1
    69 us | ## 1
    71 us | ## 1
    74 us | ## 1
    75 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 114 | ad33864dbcefd73a70e8d2e4bf729822869bc47a99ffb63c1c3311d8144c44ad |
| cyclictest.hist | 1683 | c04140306f2dbe77fe1dd48958989a45f140b524f74ea72cd59c73d040d204e7 |
| cyclictest.json | 4114 | f7e89b4d34f09d81e56acd8beccb460c65fbf6fb1ae9ae80e49966deff7921a8 |
