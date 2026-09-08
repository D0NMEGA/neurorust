# Run report

run id: 2026-09-08-precision3591-headline
run class: headline
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-08T05:17:26.167067763Z
utc end: 2026-09-08T09:17:26.368295436Z
manifest blake3: 82f928acbdd4b84833d77288e8b81dd7eca95f29b2f2a6916f51e42a7ba5e0b1

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
| thermal-headroom-at-start | pass | 60.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | current_tracer=nop events/enable=0 set_event=(empty) tracing_on=0 instances=none samplers=none | current_tracer=nop, events/enable=0, set_event=(empty), tracing_on=0, no live tracing instance, no osnoise/timerlat kthread on the target cpus |

## Series admission

admitted: yes
decided from evidence upstream of the measured latency; the contamination verdict below is recorded and does not decide this.

| evidence | observed | disposition |
|----------|----------|-------------|
| precondition-waiver | not given | clean |
| fixture-use | none | clean |
| tool-exit-codes | cyclictest=0 | clean |
| preconditions | 15 pass, 0 fail, 0 not-applicable, 0 unavailable | clean |
| interference-counters | cal max 2 on cpu6, tlb max 1 on cpu11, res max 87 on cpu6, irq max 4158 on cpu6, over 14400s | not-thresholded |
| smi-delta | cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0 | not-thresholded |
| thermal-maximum | 67.1 C | not-thresholded |
| instrument-class | headline-series | clean |

exclusions: none

## Contamination verdict

verdict: clean
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 10.1 |
| thread-max spread | 74.1% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 2 | 1 | 87 | 4158 |
| 7 | 1 | 1 | 81 | 3951 |
| 8 | 1 | 1 | 13 | 390 |
| 9 | 1 | 1 | 9 | 56 |
| 10 | 1 | 1 | 12 | 566 |
| 11 | 1 | 1 | 16 | 149 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 8 |
| p99.9 | 10 |

sample count: 431999988
overflow count: 0
maximum: 81 us

## Distribution

ASCII histogram, bins 2 to 81 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 272988423
     3 us | ###################################### 116524809
     4 us | ################################### 29135188
     5 us | ############################# 1627043
     6 us | ######################## 95275
     7 us | ############################## 2405323
     8 us | ################################ 5330214
     9 us | ############################### 2964176
    10 us | ############################ 774759
    11 us | ######################## 130124
    12 us | #################### 19640
    13 us | ################ 2992
    14 us | ############# 609
    15 us | ########### 226
    16 us | ########## 125
    17 us | ######### 84
    18 us | ######## 47
    19 us | ####### 27
    20 us | ###### 14
    21 us | ##### 10
    22 us | ### 3
    23 us | # 1
    24 us | # 1
    25 us | #### 5
    26 us | ##### 9
    27 us | ########## 151
    28 us | ############ 269
    29 us | ########## 158
    30 us | ########## 142
    31 us | ######### 63
    32 us | ####### 34
    33 us | ##### 8
    34 us | ### 4
    42 us | # 1
    43 us | ## 2
    44 us | ### 3
    45 us | #### 6
    46 us | #### 5
    47 us | # 1
    53 us | # 1
    56 us | # 1
    57 us | # 1
    59 us | # 1
    61 us | # 1
    62 us | # 1
    65 us | # 1
    67 us | # 1
    68 us | ## 2
    70 us | # 1
    73 us | # 1
    77 us | # 1
    81 us | # 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 61 | 8b3a6f104bfc5f0d3ebeb25de3dd7005487c3692fe24edd2ad22930be87ee508 |
| cyclictest.hist | 2926 | d339f6f1d78802532dcb8526abf11032dce1d0ab9b755072aa5c5c67f15bc933 |
| cyclictest.json | 4759 | 32db0316d5871a2abe0e0da4c9b575f3140767d3ef35afd2ebfb5397fb438d56 |
