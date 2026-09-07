# Run report

run id: 2026-09-06-precision3591-screen-04
run class: screen
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-06T23:47:51.238243548Z
utc end: 2026-09-07T00:03:52.862855597Z
manifest blake3: 25fe5e53a1f9e443e97bf3f63d9d3f01a59e523968763412b42831742cd3e10a

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
| thermal-headroom-at-start | not-applicable | 83.1 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | current_tracer=nop events/enable=0 set_event=(empty) tracing_on=0 | current_tracer=nop, events/enable=0, set_event=(empty), tracing_on=0 |

## Contamination verdict

verdict: clean
reason: contamination verdict Clean was reached against provisional (D-24, not yet calibrated) thresholds; see config/contamination-thresholds.json
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 4.6 |
| thread-max spread | 50.0% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | context switches | irqs |
|-----|----------|----------|-------------------|------|
| 6 | 519 | 1 | 70371 | 253 |
| 7 | 489 | 1 | 60995 | 212 |
| 8 | 489 | 1 | 75395 | 13 |
| 9 | 489 | 1 | 88789 | 3 |
| 10 | 489 | 1 | 64665 | 23 |
| 11 | 489 | 1 | 63821 | 12 |

## Firmware screen

instrument: rtla-hwnoise
tool version: rtla hwnoise: a summary of hardware-related noise (version 7.0.12)
invocation: rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s
requested cpus: 6,7,8,9,10,11
observed cpus: 6,7,8,9,10,11
per-cpu exposure: cpu6=674.25s cpu7=674.25s cpu8=674.25s cpu9=674.25s cpu10=674.25s cpu11=674.25s
maximum: 1 us (the largest Max Single value (one-shot hardware-noise event) across the observed CPUs' final rtla hwnoise rows)
events recorded: 64

- it measures hardware-related noise, the execution gaps left after software noise is accounted for. those are not uniquely identified SMIs either; the exact MSR_SMI_COUNT recorded alongside this screen is the census, and this is not
- its per-CPU figures come from one osnoise sampling thread per CPU in the -c list, so a CPU absent from the observed list was sampled and reported nothing, rather than never being sampled. that is the specific difference from hwlatdetect on this rig
- per-CPU exposure above is what the tool reports; when it reports none, the exposure is unstated rather than a wall-clock duration divided by a CPU count

MSR_SMI_COUNT (0x34) over this run: cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0
the count above is exact and says how many SMIs reached each cpu; it says nothing about how long any of them took

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 3 |
| p95 | 7 |
| p99 | 10 |
| p99.9 | 11 |

sample count: 1799989
overflow count: 0
maximum: 46 us

## Distribution

ASCII histogram, bins 3 to 46 us, count axis on a log2 scale, one row per non-empty bin.

```
     3 us | ######################################## 1056157
     4 us | ##################################### 362797
     5 us | ################################### 185882
     6 us | ################################# 91591
     7 us | ############################## 38240
     8 us | ########################### 10577
     9 us | ############################# 24553
    10 us | ############################# 24733
    11 us | ######################## 4371
    12 us | ################## 469
    13 us | ################ 219
    14 us | ############## 144
    15 us | ############# 87
    16 us | ############ 63
    17 us | ########### 46
    18 us | ######### 25
    19 us | ######## 13
    20 us | ## 1
    21 us | ### 2
    22 us | ##### 4
    23 us | ## 1
    24 us | #### 3
    25 us | ## 1
    26 us | ### 2
    27 us | ## 1
    30 us | ## 1
    31 us | ## 1
    32 us | ### 2
    34 us | ### 2
    46 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 61 | 8b3a6f104bfc5f0d3ebeb25de3dd7005487c3692fe24edd2ad22930be87ee508 |
| rtla-hwnoise.txt | 754071 | 48783734bc9b29a70ea27d347875fb22a93d610ddcc9dd4665d0524bc5df6395 |
| cyclictest.hist | 1817 | e81b392345447da8a9f6a7d3e32b749c7bfee818db76e6ac476e5a8f75e08a87 |
| cyclictest.json | 3986 | 86a7693360c562c4c7ebce0f9ba75e55f39dbb494acbb86fec05af762aa2c38a |
