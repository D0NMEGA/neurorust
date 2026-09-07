# Run report

run id: 2026-09-02-precision3591-calibration-contaminated
run class: calibration-contaminated
instrument class: headline-series
provenance tier: harness-generated
utc start: 2026-09-02T07:23:59.42739464Z
utc end: 2026-09-02T07:38:59.567003593Z
manifest blake3: 79951ad31fb55a44eae6bec8f7394284a9f6a772953724b658ab66402f371d60

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
| governor-is-performance-on-all-cpus | pass | performance on 22 CPUs | performance on all CPUs |
| no-turbo-enabled | pass | 1 | 1 |
| deep-cstates-disabled | pass | C6 not present, C10 not present | C6 disabled, C10 disabled |
| isolcpus-covers-target-cpus | pass | 6-11 | 6-11 |
| kernel-is-realtime | pass | 1 | 1 |
| rt-tuning-service-active | pass | active | active |
| on-ac-power | pass | AC=1 | AC=1 |
| thermal-headroom-at-start | pass | 60.0 C | package temp <= 70 C |
| no-package-manager-activity | pass | none | no apt, dpkg, unattended-upgrade or snapd process |
| tracers-quiescent | pass | nop | nop |

## Series admission

not recorded: this manifest predates the admission record (D-28)

## Contamination verdict

verdict: uncalibrated
reason: excluded_from_series forced true: --allow-precondition-violation waived 3 precondition violation(s) for this calibration-contaminated run: NoActiveSshSessions (observed "1", expected "0"); DisplayManagerInactive (observed "gdm.service=active, sddm.service=inactive, lightdm.service=inactive", expected "inactive"); NoGraphicalSession (observed "1", expected "0")
this verdict is an inference from the shape of this run's own measured latency; it does not by itself remove the run from the series (see Series admission above).

tail metrics (D-24):

not computed: this manifest predates D-24.

counter deltas per isolated cpu:

| cpu | cal ipis | tlb ipis | res ipis | irqs |
|-----|----------|----------|----------|------|
| 6 | 2 | 1 | 3 | 12 |
| 7 | 2 | 1 | 4 | 35 |
| 8 | 2 | 1 | 5 | 67 |
| 9 | 2 | 1 | 6 | 37 |
| 10 | 2 | 1 | 1 | 13 |
| 11 | 2 | 1 | 5 | 47 |

res ipis are /proc/interrupts RES rescheduling interrupts, the closest per-cpu proxy for scheduling interference available here: /proc/stat's ctxt is machine-wide with no per-cpu breakdown (plan 01-05).

## Results

| percentile | latency (us) |
|------------|---------------|
| p50 | 2 |
| p95 | 4 |
| p99 | 9 |
| p99.9 | 10 |

sample count: 26996068
overflow count: 794
maximum: 3856 us

## Distribution

ASCII histogram, bins 1 to 377 us, count axis on a log2 scale, one row per non-empty bin.

```
     1 us | ###### 11
     2 us | ######################################## 17083143
     3 us | ###################################### 6887629
     4 us | ################################### 2102134
     5 us | ############################# 180306
     6 us | ##################### 5990
     7 us | ########################### 78543
     8 us | ############################## 316367
     9 us | ############################## 241659
    10 us | ########################### 79353
    11 us | ####################### 16078
    12 us | ################### 2360
    13 us | ############## 382
    14 us | ######### 44
    15 us | ######### 34
    16 us | ######## 24
    17 us | ####### 16
    18 us | ####### 14
    19 us | ##### 7
    20 us | ### 2
    21 us | #### 4
    22 us | #### 4
    23 us | ##### 8
    24 us | ##### 7
    25 us | ###### 12
    26 us | ### 3
    27 us | #### 4
    28 us | ##### 6
    29 us | #### 4
    31 us | ## 1
    32 us | ## 1
    33 us | ## 1
    35 us | ## 1
    38 us | ## 1
    39 us | ## 1
    40 us | ## 1
    43 us | ## 1
    44 us | ### 2
    45 us | #### 5
    49 us | ## 1
    50 us | ### 3
    51 us | ### 3
    52 us | ## 1
    53 us | ## 1
    56 us | ## 1
    57 us | ### 2
    58 us | ### 3
    59 us | ## 1
    61 us | #### 4
    62 us | ### 3
    63 us | ##### 6
    64 us | #### 5
    65 us | #### 5
    66 us | ### 2
    67 us | ### 2
    68 us | #### 4
    69 us | ### 3
    70 us | ##### 7
    71 us | ### 2
    72 us | ###### 9
    73 us | ###### 9
    74 us | #### 4
    75 us | ##### 6
    76 us | ##### 7
    77 us | #### 4
    78 us | #### 5
    79 us | ### 2
    81 us | ##### 6
    82 us | ##### 6
    83 us | ##### 7
    84 us | ##### 7
    85 us | ### 3
    86 us | ##### 7
    87 us | ###### 10
    88 us | ##### 7
    89 us | ### 3
    90 us | ### 3
    91 us | #### 4
    92 us | ##### 6
    93 us | #### 5
    94 us | #### 5
    95 us | ### 3
    96 us | ##### 8
    97 us | ### 3
    98 us | #### 4
    99 us | ##### 6
   100 us | #### 4
   101 us | #### 4
   102 us | ### 3
   103 us | ##### 6
   104 us | ##### 7
   105 us | #### 5
   106 us | ##### 6
   107 us | #### 5
   108 us | ##### 6
   109 us | ### 2
   110 us | #### 5
   111 us | ### 3
   112 us | ## 1
   113 us | ##### 6
   114 us | #### 5
   115 us | ##### 7
   116 us | #### 4
   117 us | #### 4
   118 us | ##### 6
   119 us | ##### 8
   120 us | ##### 6
   121 us | #### 5
   122 us | ### 3
   123 us | #### 5
   124 us | #### 4
   125 us | ##### 7
   126 us | ##### 6
   127 us | ###### 10
   128 us | ###### 9
   129 us | #### 4
   130 us | ### 3
   131 us | #### 4
   132 us | ##### 6
   133 us | #### 4
   134 us | #### 5
   135 us | ##### 6
   136 us | ##### 6
   137 us | ###### 9
   138 us | ##### 6
   139 us | ### 2
   140 us | #### 4
   141 us | ###### 11
   142 us | #### 4
   143 us | ##### 6
   144 us | ### 3
   145 us | ### 3
   146 us | ### 3
   147 us | #### 5
   148 us | #### 5
   149 us | ### 2
   150 us | #### 4
   151 us | ### 2
   152 us | ##### 6
   153 us | #### 4
   154 us | ### 2
   155 us | ## 1
   156 us | ##### 6
   157 us | #### 5
   158 us | #### 4
   159 us | #### 5
   160 us | ### 2
   161 us | ##### 7
   162 us | ##### 7
   163 us | ##### 6
   164 us | ##### 7
   165 us | ### 3
   166 us | ##### 7
   167 us | ### 3
   168 us | #### 5
   169 us | #### 5
   170 us | ### 3
   171 us | ### 3
   172 us | #### 5
   173 us | #### 4
   174 us | #### 4
   175 us | #### 5
   176 us | ##### 7
   177 us | ##### 7
   178 us | #### 5
   179 us | ##### 6
   180 us | ###### 9
   181 us | ### 3
   182 us | ### 3
   183 us | #### 5
   184 us | #### 5
   185 us | ###### 13
   186 us | ##### 6
   187 us | ##### 8
   188 us | ## 1
   189 us | ### 3
   190 us | #### 4
   191 us | ##### 7
   192 us | #### 5
   193 us | ###### 9
   194 us | #### 5
   195 us | ###### 9
   196 us | #### 5
   197 us | #### 5
   198 us | ##### 7
   199 us | #### 4
   200 us | #### 4
   201 us | ##### 6
   202 us | ##### 6
   203 us | ## 1
   204 us | ##### 6
   205 us | ## 1
   206 us | ##### 7
   207 us | ###### 10
   208 us | #### 5
   209 us | ##### 6
   210 us | ### 3
   211 us | #### 4
   212 us | ##### 7
   213 us | #### 5
   214 us | ##### 7
   215 us | ### 2
   216 us | ### 2
   217 us | #### 4
   218 us | #### 4
   219 us | ##### 8
   220 us | ##### 8
   221 us | ##### 6
   222 us | #### 4
   223 us | #### 4
   224 us | ##### 6
   225 us | ##### 8
   226 us | ##### 7
   227 us | ###### 9
   228 us | #### 4
   229 us | ### 3
   230 us | #### 5
   231 us | ###### 11
   232 us | ##### 6
   233 us | ##### 7
   234 us | ##### 7
   235 us | #### 5
   236 us | ### 3
   237 us | ### 2
   238 us | ##### 6
   239 us | ##### 7
   240 us | ###### 11
   241 us | #### 5
   242 us | #### 4
   243 us | #### 5
   244 us | ### 3
   245 us | #### 5
   246 us | ##### 6
   247 us | ## 1
   249 us | ##### 7
   250 us | ##### 6
   251 us | #### 4
   252 us | ### 3
   253 us | ### 3
   254 us | #### 4
   255 us | ### 3
   256 us | ### 3
   257 us | #### 4
   258 us | #### 4
   259 us | #### 4
   260 us | ### 2
   261 us | ### 2
   262 us | ### 3
   263 us | ### 2
   267 us | #### 4
   268 us | ### 3
   270 us | #### 4
   271 us | ### 2
   272 us | ### 3
   274 us | ## 1
   275 us | ## 1
   276 us | ## 1
   277 us | ## 1
   278 us | ## 1
   279 us | ### 2
   280 us | ## 1
   281 us | ## 1
   284 us | ## 1
   285 us | ### 2
   286 us | ## 1
   288 us | ## 1
   289 us | ### 2
   290 us | ## 1
   295 us | ## 1
   298 us | ## 1
   304 us | ### 3
   311 us | ## 1
   312 us | ## 1
   319 us | ### 2
   324 us | ### 2
   326 us | ### 2
   327 us | ## 1
   328 us | ### 2
   330 us | ## 1
   333 us | ## 1
   334 us | ## 1
   335 us | ### 3
   336 us | ## 1
   338 us | ## 1
   339 us | ## 1
   340 us | ### 2
   341 us | ### 2
   342 us | ## 1
   345 us | ### 2
   347 us | ### 2
   349 us | ## 1
   350 us | ## 1
   353 us | ## 1
   359 us | ## 1
   360 us | ### 2
   362 us | ## 1
   365 us | ## 1
   366 us | ## 1
   369 us | ## 1
   370 us | ## 1
   371 us | ### 3
   373 us | ### 2
   377 us | ## 1
```

## Overflow convention

Overflow samples are recorded at the histogram bound, which is a lower bound on their true value. Percentiles at or above the overflow fraction are therefore conservative: the reported value is no larger than the truth.

## Artifacts

| path | bytes | blake3 |
|------|-------|--------|
| cyclictest.hist | 20755 | 60f967aaaa69df90de442fb3bab5ad238a4f37ae6a5768505ad3328a220a6e3a |
| cyclictest.json | 17956 | 36dda4d04323dbf85c187198e79f43ba285322baf30d63c49f61e3a97c846d8a |
