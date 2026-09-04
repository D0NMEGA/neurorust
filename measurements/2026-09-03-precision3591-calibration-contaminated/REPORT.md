# Run report

run id: 2026-09-03-precision3591-calibration-contaminated
run class: calibration-contaminated
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-03T20:53:54.713227883Z
utc end: 2026-09-03T21:53:54.846537922Z
manifest blake3: 24de44342ee606d17d1eea6ec3a7a78c9597b5f1228ba7fd01f874db75e4ff45

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
session:      installed display_manager_active=true
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
| no-active-ssh-sessions | fail | 1 | 0 |
| systemd-default-target-is-multi-user | pass | multi-user.target | multi-user.target |
| display-manager-inactive | fail | gdm.service=active, sddm.service=inactive, lightdm.service=inactive | inactive |
| no-graphical-session | fail | 1 | 0 |
| no-active-login-sessions | pass | none | no local console login session |
| governor-is-performance-on-all-cpus | pass | performance on 22 CPUs | performance on all CPUs |
| no-turbo-enabled | pass | 1 | 1 |
| deep-cstates-disabled | pass | C6 not present, C10 not present | C6 disabled, C10 disabled |
| isolcpus-covers-target-cpus | pass | 6-11 | 6-11 |
| kernel-is-realtime | pass | 1 | 1 |
| rt-tuning-service-active | pass | active | active |
| on-ac-power | pass | AC=1 | AC=1 |
| thermal-headroom-at-start | fail | 71.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | nop | nop |

## Contamination verdict

verdict: contaminated
reason: excluded_from_series forced true: --allow-precondition-violation waived 4 precondition violation(s) for this calibration-contaminated run: NoActiveSshSessions (observed "1", expected "0"); DisplayManagerInactive (observed "gdm.service=active, sddm.service=inactive, lightdm.service=inactive", expected "inactive"); NoGraphicalSession (observed "1", expected "0"); ThermalHeadroomAtStart (observed "71.0 C", expected "package temp <= 70 C")
thresholds: provisional (derived from 2 runs, not a calibrated set; see config/contamination-thresholds.json)

tail metrics (D-24):

| metric | value |
|--------|-------|
| tail excursion ratio (max / p99) | 211.2 |
| thread-max spread | 3.7% |
| overflow rate | 0.0167/s |

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | context switches | irqs |
|-----|----------|----------|-------------------|------|
| 6 | 2 | 1 | 17 | 677 |
| 7 | 2 | 1 | 5 | 32 |
| 8 | 2 | 1 | 7 | 127 |
| 9 | 2 | 1 | 8 | 22 |
| 10 | 2 | 1 | 4 | 104 |
| 11 | 2 | 1 | 0 | 24 |

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 9 |
| p99.9 | 10 |

sample count: 107999267
overflow count: 60
maximum: 1901 us

## Distribution

ASCII histogram, bins 2 to 377 us, count axis on a log2 scale, one row per non-empty bin.

```
     2 us | ######################################## 71003752
     3 us | ##################################### 22391902
     4 us | #################################### 10812282
     5 us | ############################## 825752
     6 us | ##################### 14971
     7 us | ########################### 233422
     8 us | ############################### 1260588
     9 us | ############################### 1053132
    10 us | ############################ 331363
    11 us | ######################## 60374
    12 us | #################### 9221
    13 us | ################ 1569
    14 us | ############# 300
    15 us | ########## 91
    16 us | ######## 44
    17 us | ######## 39
    18 us | ######## 34
    19 us | ####### 24
    20 us | ###### 13
    21 us | #### 5
    22 us | #### 6
    23 us | #### 4
    24 us | ## 2
    25 us | ## 1
    34 us | ## 1
    39 us | ## 2
    40 us | ## 2
    42 us | ## 1
    43 us | ## 1
    44 us | ## 1
    47 us | ## 2
    49 us | ## 1
    50 us | ## 1
    55 us | ## 1
    59 us | ## 1
    64 us | ## 1
    78 us | ## 1
    79 us | ## 1
    80 us | ## 2
    84 us | ## 1
    85 us | ## 1
    86 us | ## 1
    88 us | ## 1
    90 us | ## 1
    95 us | #### 4
    96 us | ### 3
    99 us | ## 1
   101 us | ## 1
   103 us | ## 1
   104 us | ## 1
   105 us | ## 1
   106 us | ## 1
   107 us | ## 1
   110 us | ## 1
   112 us | ## 1
   114 us | ## 2
   115 us | ## 1
   120 us | ## 1
   121 us | ## 1
   122 us | ## 1
   124 us | ## 1
   126 us | ## 1
   130 us | ## 2
   131 us | ## 1
   132 us | ## 2
   133 us | ## 2
   136 us | ## 1
   140 us | ## 1
   141 us | ## 1
   143 us | ## 2
   144 us | ## 1
   150 us | ## 1
   151 us | ## 1
   152 us | ## 1
   153 us | ## 1
   156 us | ## 1
   164 us | ## 1
   165 us | ## 1
   178 us | ## 2
   179 us | ## 1
   180 us | ## 2
   190 us | ## 1
   191 us | ## 1
   193 us | ## 1
   194 us | #### 4
   195 us | ## 1
   196 us | ## 1
   197 us | ## 2
   200 us | ## 2
   201 us | ## 2
   203 us | ## 1
   204 us | ## 1
   206 us | ## 2
   208 us | ## 2
   209 us | ## 1
   210 us | ## 2
   211 us | ## 1
   212 us | ### 3
   215 us | ## 2
   216 us | ## 1
   217 us | ## 2
   218 us | ## 1
   219 us | ## 2
   221 us | ## 1
   223 us | ## 1
   224 us | #### 4
   225 us | ## 1
   227 us | ## 1
   228 us | ## 1
   229 us | ## 2
   230 us | ## 1
   233 us | ## 1
   236 us | ## 1
   237 us | ## 2
   239 us | ## 2
   240 us | ### 3
   241 us | ### 3
   242 us | ## 1
   243 us | ## 1
   244 us | ## 2
   245 us | ## 2
   246 us | ## 1
   248 us | ### 3
   250 us | ## 2
   251 us | ## 2
   252 us | ## 1
   253 us | ## 2
   254 us | ## 2
   256 us | ## 2
   257 us | ## 2
   258 us | ### 3
   259 us | ### 3
   260 us | ## 1
   262 us | ## 2
   263 us | ## 1
   265 us | ## 2
   268 us | ## 1
   270 us | ## 1
   271 us | ## 1
   272 us | ## 1
   274 us | ## 1
   275 us | ## 1
   276 us | #### 4
   277 us | ## 2
   278 us | ## 1
   281 us | ## 2
   282 us | ## 2
   283 us | ## 2
   285 us | #### 4
   286 us | ## 1
   287 us | ## 2
   288 us | ## 1
   291 us | ## 1
   292 us | ## 1
   293 us | ## 1
   295 us | #### 4
   296 us | ## 2
   297 us | #### 4
   298 us | ### 3
   301 us | ### 3
   302 us | ## 2
   303 us | ### 3
   304 us | #### 4
   305 us | ## 2
   306 us | ## 2
   307 us | ## 1
   308 us | #### 4
   309 us | ### 3
   311 us | ### 3
   312 us | ## 1
   313 us | #### 5
   314 us | #### 4
   316 us | ### 3
   317 us | ## 2
   318 us | ## 1
   319 us | ### 3
   320 us | ## 2
   321 us | ## 1
   322 us | ### 3
   323 us | ## 1
   324 us | ## 2
   326 us | ## 2
   327 us | ## 1
   329 us | ## 2
   330 us | ## 1
   331 us | ## 1
   332 us | ## 1
   333 us | ## 1
   334 us | ## 2
   335 us | ### 3
   338 us | ## 1
   339 us | ## 1
   340 us | ## 1
   343 us | ## 1
   344 us | ## 1
   345 us | ### 3
   347 us | ## 1
   348 us | ## 1
   349 us | ## 2
   350 us | ### 3
   351 us | ## 1
   352 us | ### 3
   353 us | ## 1
   354 us | ## 1
   355 us | ## 1
   356 us | ## 1
   357 us | ## 2
   358 us | ## 2
   359 us | ## 1
   360 us | ## 1
   361 us | ## 1
   363 us | ## 2
   365 us | ## 1
   368 us | ## 1
   373 us | ## 1
   377 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.hist | 11309 | e9283d5a2d15dbcd056763ac1b4db6e3213e38e673e8a45bde9a536eef8c63a9 |
| cyclictest.json | 8706 | 46bc3adcf0d8e3485620d041a773244eb1d6c4320992a5d440b4b0b8615d6bb9 |
