# Run report

run id: 2026-09-01-precision3591-calibration-clean
run class: calibration-clean
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-01T19:11:36.017045377Z
utc end: 2026-09-01T20:11:36.154612272Z
manifest blake3: 42a8cf36fc7faeec65b941201728f746890b198f20bf6541e9847f8b544d68c2

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
| governor-is-performance-on-all-cpus | pass | performance on 22 CPUs | performance on all CPUs |
| no-turbo-enabled | pass | 1 | 1 |
| deep-cstates-disabled | pass | C6 not present, C10 not present | C6 disabled, C10 disabled |
| isolcpus-covers-target-cpus | pass | 6-11 | 6-11 |
| kernel-is-realtime | pass | 1 | 1 |
| rt-tuning-service-active | pass | active | active |
| on-ac-power | pass | AC=1 | AC=1 |
| thermal-headroom-at-start | pass | 59.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | nop | nop |

## Series admission

not recorded: this manifest predates the admission record (D-28)

## Contamination verdict

verdict: uncalibrated
reason: no calibrated contamination thresholds exist yet (D-17)
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).

tail metrics (D-24):

not computed: this manifest predates D-24.

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 1 | 1 | 12 | 274 |
| 7 | 1 | 1 | 14 | 469 |
| 8 | 1 | 1 | 12 | 234 |
| 9 | 1 | 1 | 6 | 18 |
| 10 | 1 | 1 | 5 | 62 |
| 11 | 1 | 1 | 6 | 70 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 9 |
| p99.9 | 10 |

sample count: 107999994
overflow count: 0
maximum: 78 us

## Distribution

ASCII histogram, bins 2 to 78 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 72654045
     3 us | ###################################### 23659695
     4 us | ################################### 7999348
     5 us | ############################ 341612
     6 us | ####################### 35760
     7 us | ############################# 624145
     8 us | ############################### 1501978
     9 us | ############################## 898431
    10 us | ########################### 240774
    11 us | ####################### 37866
    12 us | ################### 4883
    13 us | ############### 750
    14 us | ############ 246
    15 us | ########### 116
    16 us | ######### 62
    17 us | ######## 38
    18 us | ####### 21
    19 us | ###### 15
    20 us | ##### 7
    21 us | #### 4
    22 us | #### 4
    23 us | ## 2
    24 us | ####### 18
    25 us | ######## 36
    26 us | ######## 33
    27 us | ######## 38
    28 us | ######## 35
    29 us | ###### 15
    30 us | ##### 7
    31 us | ## 2
    32 us | ## 2
    33 us | ## 1
    36 us | ## 1
    46 us | ## 1
    51 us | ## 1
    65 us | ## 1
    78 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.hist | 2184 | a140df41c6e0250bb7423a5784b2a9e23116694e5443d2a201de2a1af2034ff1 |
| cyclictest.json | 4255 | 6f5058ed09ef5d76d6755fb8b4d0f06e20651b601d48c8cff0434ef5d3dcc10f |
