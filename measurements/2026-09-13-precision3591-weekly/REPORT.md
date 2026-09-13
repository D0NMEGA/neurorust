# Run report

run id: 2026-09-13-precision3591-weekly
run class: weekly
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-13T08:00:06.80148987Z
utc end: 2026-09-13T09:15:07.882602363Z
manifest blake3: 691e4747d8ef274f543e5ba482674170952bce8da8210e365f6fda6cb6e98159

## Rig and tuning

```
system:       Dell Inc. Precision 3591
bios:         1.23.0 released 04/24/2026
cpu:          Intel(R) Core(TM) Ultra 9 185H
topology:     22 logical / 16 cores / 1 socket
p_cores:      
e_cores:      
microcode:    0x28
kernel:       7.0.0-31-realtime (#31.1-Ubuntu SMP PREEMPT_RT Tue Aug 11 15:50:26 UTC 2026)
os:           Ubuntu 26.04.1 LTS (Resolute Raccoon)
session:      installed display_manager_active=false
memory_gb:    30
cmdline:      BOOT_IMAGE=/boot/vmlinuz-7.0.0-31-realtime root=[redacted] ro quiet splash isolcpus=6-11 nohz_full=6-11 rcu_nocbs=6-11 irqaffinity=0-5,12-21 intel_idle.max_cstate=1 processor.max_cstate=1 nosoftlockup nowatchdog nmi_watchdog=0 tsc=reliable skew_tick=1 crashkernel=2G-4G:320M,4G-32G:512M,32G-64G:1024M,64G-128G:2048M,128G-:4096M
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
| tool-exit-codes | cyclictest=0 rtla=0 | clean |
| preconditions | 15 pass, 0 fail, 0 not-applicable, 0 unavailable | clean |
| interference-counters | cal max 494 on cpu11, tlb max 1 on cpu11, res max 118 on cpu10, irq max 617 on cpu6, over 3600s | not-thresholded |
| smi-delta | cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0 | not-thresholded |
| thermal-maximum | 64.0 C | not-thresholded |
| instrument-class | headline-series | clean |

exclusions: none

## Contamination verdict

verdict: clean
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 9.0 |
| thread-max spread | 76.5% |
| overflow rate | 0.0000/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 487 | 1 | 77 | 617 |
| 7 | 487 | 1 | 49 | 23 |
| 8 | 487 | 1 | 84 | 100 |
| 9 | 487 | 1 | 45 | 90 |
| 10 | 487 | 1 | 118 | 92 |
| 11 | 494 | 1 | 24 | 321 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Firmware screen

instrument: rtla-hwnoise
tool version: rtla hwnoise: a summary of hardware-related noise (version 7.0.14)
invocation: rtla hwnoise -c 6-11 -H 0-5 -P f:99 -d 900s
requested cpus: 6,7,8,9,10,11
observed cpus: 6,7,8,9,10,11
per-cpu exposure: cpu6=674.25s cpu7=674.25s cpu8=674.25s cpu9=674.25s cpu10=674.25s cpu11=674.25s
maximum: 1 us (the largest Max Single value (one-shot hardware-noise event) across the observed CPUs' final rtla hwnoise rows)
events recorded: 5

- it measures hardware-related noise, the execution gaps left after software noise is accounted for. those are not uniquely identified SMIs either; the exact MSR_SMI_COUNT recorded alongside this screen is the census, and this is not
- its per-CPU figures come from one osnoise sampling thread per CPU in the -c list. a CPU that was sampled and observed nothing still emits its own row carrying its exposure (cpu 7 in measurements/2026-09-06-precision3591-screen-03/rtla-hwnoise.txt is the committed example: full exposure, zero events, an explicit row). a CPU absent from the observed list therefore produced no row at all, and coverage cannot be confirmed for it; verify --strict reports that as a problem
- per-CPU exposure above is what the tool reports; when it reports none, the exposure is unstated rather than a wall-clock duration divided by a CPU count

MSR_SMI_COUNT (0x34) over this run: cpu6=0 cpu7=0 cpu8=0 cpu9=0 cpu10=0 cpu11=0
the count above is exact and says how many SMIs reached each cpu; it says nothing about how long any of them took

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 9 |
| p99.9 | 10 |

sample count: 107999991
overflow count: 0
maximum: 81 us

## Distribution

ASCII histogram, bins 2 to 81 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 70213830
     3 us | ###################################### 24958635
     4 us | ################################### 8936124
     5 us | ############################# 580341
     6 us | ###################### 23097
     7 us | ############################# 514046
     8 us | ############################### 1448592
     9 us | ############################### 997245
    10 us | ############################ 279995
    11 us | ######################## 41867
    12 us | ################### 5000
    13 us | ############### 706
    14 us | ########### 150
    15 us | ######### 60
    16 us | ######## 33
    17 us | ####### 22
    18 us | ##### 10
    19 us | ###### 13
    20 us | ### 3
    22 us | ## 2
    23 us | ####### 25
    24 us | ######## 38
    25 us | ######## 39
    26 us | ######## 38
    27 us | ######## 32
    28 us | ###### 12
    29 us | ##### 10
    30 us | ### 3
    31 us | ## 1
    36 us | ## 1
    37 us | ## 1
    38 us | ## 1
    49 us | ## 1
    52 us | ## 1
    53 us | ## 1
    59 us | ## 1
    60 us | ## 2
    61 us | ## 2
    62 us | ## 1
    63 us | ## 2
    67 us | ## 1
    72 us | ## 2
    75 us | ## 1
    80 us | ## 1
    81 us | ### 3
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.stderr.txt | 61 | 8b3a6f104bfc5f0d3ebeb25de3dd7005487c3692fe24edd2ad22930be87ee508 |
| rtla-hwnoise.txt | 756612 | 436b302020fd73921db3fc95ffb38941cd602d5d09b3e61cabee82daa08b0181 |
| cyclictest.hist | 2575 | 3cca9523f54a59a197a94bb9c8ad1a29259ce4b6b4502db8bb540827b0f80246 |
| cyclictest.json | 4321 | 55af9bf78fcef9fb30e8953f44e8ec58a1600132a2302243c2805e49f2fe247c |
