# Run report

run id: 2026-09-05-precision3591-screen-03
run class: screen
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-05T05:52:11.262809488Z
utc end: 2026-09-05T06:08:11.759550855Z
manifest blake3: 2d85809c7fa5b518acc5633684b51c71b54f19af4a2f0b3d8b60c3515604fad5

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
| deep-cstates-disabled | pass | C6 not present, C10 not present | C6 disabled, C10 disabled |
| isolcpus-covers-target-cpus | pass | 6-11 | 6-11 |
| kernel-is-realtime | pass | 1 | 1 |
| rt-tuning-service-active | pass | active | active |
| on-ac-power | pass | AC=1 | AC=1 |
| thermal-headroom-at-start | not-applicable | 89.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | nop | nop |

## Series admission

not recorded: this manifest predates the admission record (D-28)

## Contamination verdict

verdict: clean
reason: contamination verdict Clean was reached against provisional (D-24, not yet calibrated) thresholds; see config/contamination-thresholds.json
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 7.9 |
| thread-max spread | 62.0% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 20 | 1 | 65698 | 57 |
| 7 | 2 | 1 | 61207 | 12 |
| 8 | 2 | 1 | 62290 | 17 |
| 9 | 2 | 1 | 54227 | 3 |
| 10 | 2 | 1 | 76939 | 12 |
| 11 | 2 | 1 | 60588 | 5 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 3 |
| p95 | 6 |
| p99 | 10 |
| p99.9 | 12 |

sample count: 1799994
overflow count: 0
maximum: 79 us

## Distribution

ASCII histogram, bins 2 to 79 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ### 2
     3 us | ######################################## 1140961
     4 us | #################################### 320435
     5 us | ################################### 169951
     6 us | ################################# 84170
     7 us | ############################## 32677
     8 us | ########################## 9440
     9 us | ############################# 21121
    10 us | ############################ 15084
    11 us | ######################## 3985
    12 us | #################### 1137
    13 us | ################# 360
    14 us | ############### 175
    15 us | ############## 113
    16 us | ############## 127
    17 us | ############# 78
    18 us | ############ 55
    19 us | ########## 33
    20 us | ######### 23
    21 us | ####### 11
    22 us | ##### 5
    23 us | ####### 10
    24 us | ###### 6
    25 us | ##### 5
    26 us | #### 3
    27 us | ##### 4
    28 us | ## 1
    29 us | ## 1
    30 us | ##### 4
    31 us | #### 3
    32 us | #### 3
    33 us | ## 1
    34 us | ## 1
    35 us | ## 1
    39 us | ## 1
    40 us | ## 1
    41 us | ## 1
    42 us | ## 1
    43 us | ## 1
    46 us | ## 1
    49 us | ## 1
    79 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.hist | 2405 | 25f0162436f6b004882e0a50c5c48a60c4219d9e9cd05fb6a70c26ec38675ef3 |
| cyclictest.json | 4461 | eed01e17aa6c4b83a19cc63abb3006fd0bba345822ecb5936b5a7e41bdd64bea |
| hwlatdetect.txt | 1147 | 9a1d3557db83e2443ee1b6fde6ce04674696de4c766c5a53744a1a0e2e671ed8 |
