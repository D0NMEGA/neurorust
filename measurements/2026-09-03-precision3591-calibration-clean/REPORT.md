# Run report

run id: 2026-09-03-precision3591-calibration-clean
run class: calibration-clean
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-03T19:46:52.121051298Z
utc end: 2026-09-03T20:06:52.443260235Z
manifest blake3: 28b433b85d1fcb561170dcccc486729e0e2cffb3e49ad33d611e9d926cc201d5

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
| thermal-headroom-at-start | pass | 61.0 C | package temp <= 70 C |
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
| tail excursion ratio (max / p99) | 3.8 |
| thread-max spread | 46.7% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 1 | 1 | 2 | 142 |
| 7 | 1 | 1 | 0 | 27 |
| 8 | 1 | 1 | 1 | 24 |
| 9 | 1 | 1 | 1 | 6 |
| 10 | 1 | 1 | 0 | 29 |
| 11 | 1 | 1 | 0 | 8 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 8 |
| p99.9 | 10 |

sample count: 8999991
overflow count: 0
maximum: 30 us

## Distribution

ASCII histogram, bins 2 to 30 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 5826821
     3 us | ###################################### 2438478
     4 us | ################################# 438399
     5 us | ######################### 18569
     6 us | ################### 1824
     7 us | ############################# 75964
     8 us | ############################## 127154
     9 us | ############################ 57092
    10 us | ######################## 13535
    11 us | ################### 1847
    12 us | ############## 214
    13 us | ######### 32
    14 us | ##### 6
    15 us | ##### 6
    16 us | #### 4
    17 us | ### 2
    18 us | ## 1
    20 us | ## 1
    21 us | ##### 6
    22 us | ##### 7
    23 us | ###### 11
    24 us | ###### 9
    25 us | ##### 6
    26 us | ### 2
    30 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.hist | 1575 | 3fcbd5c29932eed313b0a32adcc540c707935e47b0f66b22462319ef1fe0e3e8 |
| cyclictest.json | 3485 | f8f1e3aada4741722075b3c685f6f7fb2e6b712edbcfd32ec7c2912315062a7b |
| hwlatdetect.txt | 390 | ae3e8c0c48723300f769934f46bf4cf56d7a1a927ab985a4c61f91b1a6e8291f |
