# Run report

run id: 2026-09-05-precision3591-screen-02
run class: screen
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-05T05:05:25.993688134Z
utc end: 2026-09-05T05:16:26.544834275Z
manifest blake3: edc21c5327a74d0aae5e73f5ff7766d7fe6ec13ce0216fc95cd47b78ab6e85aa

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
| thermal-headroom-at-start | not-applicable | 86.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | nop | nop |

## Series admission

not recorded: this manifest predates the admission record (D-28)

## Contamination verdict

verdict: clean
reason: hwlatdetect exited with code 1
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 3.7 |
| thread-max spread | 43.2% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 23 | 1 | 60386 | 48 |
| 7 | 5 | 1 | 60786 | 38 |
| 8 | 3 | 1 | 61533 | 14 |
| 9 | 3 | 1 | 56586 | 29 |
| 10 | 3 | 1 | 61283 | 11 |
| 11 | 2 | 1 | 71663 | 5 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 3 |
| p95 | 6 |
| p99 | 10 |
| p99.9 | 11 |

sample count: 1799994
overflow count: 0
maximum: 37 us

## Distribution

ASCII histogram, bins 2 to 37 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ############# 107
     3 us | ######################################## 1225403
     4 us | #################################### 262263
     5 us | ################################## 163382
     6 us | ################################ 73214
     7 us | ############################# 22996
     8 us | ########################## 8511
     9 us | ############################# 25703
    10 us | ########################### 14363
    11 us | ###################### 2348
    12 us | ################## 558
    13 us | ################ 311
    14 us | ################ 268
    15 us | ############### 197
    16 us | ############## 121
    17 us | ############# 92
    18 us | ########### 54
    19 us | ########## 29
    20 us | ######### 20
    21 us | ####### 10
    22 us | ####### 9
    23 us | ######## 14
    24 us | ###### 7
    25 us | ##### 5
    26 us | ### 2
    27 us | ### 2
    29 us | ### 2
    32 us | ## 1
    36 us | ## 1
    37 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.hist | 1817 | 634c16caec6ee904c1d9a74580f10960c0a5127c72a9062a5ca2010417f9cf95 |
| cyclictest.json | 4122 | 4be11b110da7e845b73c8f6f229a0447b37cb1701c59d5123724bfa7d9439a04 |
| hwlatdetect.txt | 634 | 9cc18d936cfb98e218a9acd74b409f47ae3dcff6261e35ea23016c92af3ab1ef |
