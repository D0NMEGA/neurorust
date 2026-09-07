# Run report

run id: 2026-09-05-precision3591-screen
run class: screen
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-05T04:43:53.803610519Z
utc end: 2026-09-05T04:59:54.378838579Z
manifest blake3: 4e484e36ec30eaa5b742d7f568b2cb43939b75a653374069b81c7f408543b330

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
| thermal-headroom-at-start | not-applicable | 86.1 C | package temp <= 70 C |
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
| tail excursion ratio (max / p99) | 3.5 |
| thread-max spread | 34.3% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 19 | 1 | 64627 | 57 |
| 7 | 2 | 1 | 56601 | 28 |
| 8 | 2 | 1 | 79106 | 15 |
| 9 | 2 | 1 | 60328 | 28 |
| 10 | 2 | 1 | 58754 | 19 |
| 11 | 2 | 1 | 64233 | 4 |

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
maximum: 35 us

## Distribution

ASCII histogram, bins 3 to 35 us, count axis on a log2 scale, one row per non-empty bin.

```
     3 us | ######################################## 1188419
     4 us | #################################### 288035
     5 us | ################################## 165603
     6 us | ################################ 77250
     7 us | ############################# 25928
     8 us | ######################### 7373
     9 us | ############################# 26347
    10 us | ############################ 17627
    11 us | ###################### 2278
    12 us | ################# 438
    13 us | ############### 214
    14 us | ############## 132
    15 us | ############# 94
    16 us | ############ 77
    17 us | ########### 52
    18 us | ########## 36
    19 us | ########## 28
    20 us | ######### 20
    21 us | ######## 13
    22 us | ##### 4
    23 us | ##### 5
    24 us | ####### 10
    25 us | ### 2
    26 us | #### 3
    27 us | #### 3
    34 us | ### 2
    35 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.hist | 1670 | ebc48d5142c566df445bbb10c1627ad161030a2c351b745ff8270904a7f0f7b2 |
| cyclictest.json | 4013 | 22fab67d507fb447ae7868d4180c14df1827f75f3342f3e855a88437fc06e6a1 |
| hwlatdetect.txt | 995 | db38b7fc642c3295cca7b5e19501b3b0412a208c3d4ec3dccf73d656e2def34e |
