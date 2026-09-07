# Run report

run id: 2026-09-07-precision3591-screen
run class: screen
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-07T01:17:11.530076323Z
utc end: 2026-09-07T01:33:13.1929631Z
manifest blake3: b011ec2df7c9f4df358f7a4f45825cc56b5a8e42625baf4a78b30c42a54280e1

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
| thermal-headroom-at-start | not-applicable | 79.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | current_tracer=nop events/enable=0 set_event=(empty) tracing_on=0 | current_tracer=nop, events/enable=0, set_event=(empty), tracing_on=0 |

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
| tail excursion ratio (max / p99) | 9.6 |
| thread-max spread | 84.4% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 488 | 1 | 97 | 233 |
| 7 | 487 | 1 | 34 | 237 |
| 8 | 487 | 1 | 142 | 21 |
| 9 | 487 | 1 | 40 | 6 |
| 10 | 487 | 1 | 158 | 15 |
| 11 | 487 | 1 | 43 | 115 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Firmware screen

instrument: rtla-hwnoise
tool version: rtla hwnoise: a summary of hardware-related noise (version 7.0.12)
invocation: rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s
requested cpus: 6,7,8,9,10,11
observed cpus: 6,7,8,9,10,11
per-cpu exposure: cpu6=674.25s cpu7=674.25s cpu8=674.25s cpu9=674.25s cpu10=674.25s cpu11=674.25s
maximum: 1 us (the largest Max Single value (one-shot hardware-noise event) across the observed CPUs' final rtla hwnoise rows)
events recorded: 44

- it measures hardware-related noise, the execution gaps left after software noise is accounted for. those are not uniquely identified SMIs either; the exact MSR_SMI_COUNT recorded alongside this screen is the census, and this is not
- its per-CPU figures come from one osnoise sampling thread per CPU in the -c list. a CPU that was sampled and observed nothing still emits its own row carrying its exposure (cpu 7 in measurements/2026-09-06-precision3591-screen-03/rtla-hwnoise.txt is the committed example: full exposure, zero events, an explicit row). a CPU absent from the observed list therefore produced no row at all, and coverage cannot be confirmed for it; verify --strict reports that as a problem
- per-CPU exposure above is what the tool reports; when it reports none, the exposure is unstated rather than a wall-clock duration divided by a CPU count

MSR_SMI_COUNT (0x34) over this run: cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0
the count above is exact and says how many SMIs reached each cpu; it says nothing about how long any of them took

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 5 |
| p99 | 10 |
| p99.9 | 11 |

sample count: 1799992
overflow count: 0
maximum: 96 us

## Distribution

ASCII histogram, bins 2 to 96 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 1315515
     3 us | ################################### 218828
     4 us | ################################# 109066
     5 us | ################################ 86200
     6 us | ############################ 19693
     7 us | ####################### 3209
     8 us | ######################## 3960
     9 us | ############################ 19515
    10 us | ############################ 18673
    11 us | ######################## 4742
    12 us | ################## 499
    13 us | ########### 44
    14 us | ######### 25
    15 us | ####### 10
    16 us | ####### 10
    17 us | ### 2
    96 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 61 | 8b3a6f104bfc5f0d3ebeb25de3dd7005487c3692fe24edd2ad22930be87ee508 |
| rtla-hwnoise.txt | 754071 | a6cff6950483738bf4a9febd7c62e8e948c950cd926c0eed9f6f39720afecfa7 |
| cyclictest.hist | 1180 | fa61e82e99265910dca1c5e595f55e8a3f502a5f8337aa750676871a79305032 |
| cyclictest.json | 3465 | 0f5a2dbeb02b0342d777d7b9128e24ecf5d8a51fc3619feb2e97a8ffb551e681 |
