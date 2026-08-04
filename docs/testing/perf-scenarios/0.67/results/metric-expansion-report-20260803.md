# HydraCache 0.67 Stage 3 metric-expansion report

> Exploratory evidence only. This output is intentionally separate from qualification/bootstrap evidence.

- Output root: `results\20260803T080554Z-metric-expansion\hydracache-metric-expansion-20260803T080554Z`
- Cases: 78 total; 76 complete; 1 failed; 1 not applicable/other
- Source commit: `ee51e14bba89bfc4030c9d564cf8cfcd4ecca474`
- Sampling interval: one second unless the run metadata says otherwise.

## Case summary

| Experiment | Target | Case | Status | Telemetry | RSS p50/p95/max | Cgroup current p95/max | Container CPU p95/max | Process CPU p95/max | Latency p95 | Errors | RSS slope/min |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 01-long-soak | hydra | baseline | complete | 179 | 113053696/202573005/212365312 | 260336435/269864960 | N/A/N/A | 15.09/16.00 | 1.27 | 0 | 68773001 |
| 01-long-soak | redis | baseline | complete | 183 | 14688256/14704640/14716928 | 22155264/22401024 | 4.02/18.08 | 4.00/5.00 | 1.26 | 0 | 1328662 |
| 01-long-soak | hazelcast | baseline | complete | 177 | 480202752/490852352/493584384 | 571042202/573628416 | 30.77/101.59 | 30.99/101.96 | N/A | 0 | 81874972 |
| 02-ttl | hazelcast | native-expiry | not_applicable | 0 | N/A/N/A/N/A | N/A/N/A | N/A/N/A | N/A/N/A | N/A | 0 | N/A |
| 02-ttl | hydra | ttl-100ms | complete | 16 | 9048064/9269248/9269248 | 61673472/65986560 | N/A/N/A | 3.25/4.00 | 1.08 | 0 | 6281833 |
| 02-ttl | hydra | ttl-1000ms | complete | 16 | 9080832/9297920/9297920 | 61671424/65769472 | N/A/N/A | 3.25/4.00 | 1.11 | 0 | 6348027 |
| 02-ttl | hydra | ttl-10000ms | complete | 16 | 8949760/9183232/9183232 | 61825024/66052096 | N/A/N/A | 3.25/4.00 | 1.09 | 0 | 6085783 |
| 02-ttl | hydra | ttl-60000ms | complete | 16 | 9084928/9322496/9322496 | 61910016/65912832 | N/A/N/A | 3.25/4.00 | 1.06 | 0 | 6347231 |
| 02-ttl | redis | ttl-100ms | complete | 16 | 11071488/11071488/11071488 | 3785728/3801088 | 10.49/33.41 | 2.25/3.00 | 1.08 | 0 | 1750246 |
| 02-ttl | redis | ttl-1000ms | complete | 16 | 10870784/10883072/10883072 | 3792896/3792896 | 4.48/9.56 | 2.25/3.00 | 1.06 | 0 | 1701321 |
| 02-ttl | redis | ttl-10000ms | complete | 16 | 10944512/10944512/10944512 | 3530752/3530752 | 13.01/43.72 | 2.25/3.00 | 1.09 | 0 | 1717414 |
| 02-ttl | redis | ttl-60000ms | complete | 16 | 10878976/10878976/10878976 | 3780608/3780608 | 11.57/37.77 | 3.00/3.00 | 1.10 | 0 | 1734025 |
| 03-payload-key | hydra | payload-64-key-8 | complete | 51 | 12529664/12529664/12529664 | 73916416/74387456 | N/A/N/A | 18.99/22.98 | 1.25 | 0 | 5844584 |
| 03-payload-key | hydra | payload-64-key-32 | complete | 51 | 17866752/17866752/17866752 | 78987264/80162816 | N/A/N/A | 19.99/22.99 | 1.25 | 0 | 12174592 |
| 03-payload-key | hydra | payload-1024-key-8 | complete | 51 | 12603392/12603392/12603392 | 74289152/75014144 | N/A/N/A | 15.98/22.99 | 1.60 | 0 | 5785448 |
| 03-payload-key | hydra | payload-1024-key-32 | complete | 51 | 27467776/27467776/27467776 | 88076288/89997312 | N/A/N/A | 17.99/24.99 | 1.59 | 0 | 23661576 |
| 03-payload-key | hydra | payload-4096-key-8 | complete | 51 | 12713984/12713984/12713984 | 74405888/75423744 | N/A/N/A | 17.49/22.99 | 1.59 | 0 | 5868655 |
| 03-payload-key | hydra | payload-4096-key-32 | complete | 51 | 58171392/58171392/58171392 | 116992000/121376768 | N/A/N/A | 18.99/29.99 | 1.60 | 0 | 60431405 |
| 03-payload-key | redis | payload-64-key-8 | complete | 51 | 10633216/10633216/10633216 | 3364864/3383296 | 19.73/62.60 | 18.49/21.99 | 1.25 | 0 | 29448 |
| 03-payload-key | redis | payload-64-key-32 | complete | 51 | 12390400/12414976/12414976 | 5140480/5185536 | 18.29/21.40 | 18.49/21.99 | 1.26 | 0 | 2144677 |
| 03-payload-key | redis | payload-1024-key-8 | complete | 51 | 10448896/10461184/10461184 | 3395584/3407872 | 15.53/21.30 | 15.49/20.99 | 1.57 | 0 | 24540 |
| 03-payload-key | redis | payload-1024-key-32 | complete | 51 | 24330240/24367104/24367104 | 17250304/17272832 | 15.96/22.64 | 15.98/22.99 | 1.59 | 0 | 16529186 |
| 03-payload-key | redis | payload-4096-key-8 | complete | 51 | 10874880/10887168/10887168 | 3518464/3731456 | 18.22/22.98 | 17.49/23.99 | 1.59 | 0 | 127610 |
| 03-payload-key | redis | payload-4096-key-32 | complete | 51 | 63246336/63258624/63258624 | 56164352/56164352 | 17.54/24.75 | 17.99/23.99 | 1.61 | 0 | 62827261 |
| 03-payload-key | hazelcast | payload-64-key-8 | complete | 50 | 328130560/357786419/357883904 | 340304691/340393984 | 116.15/248.60 | 116.34/248.85 | N/A | 0 | 109569323 |
| 03-payload-key | hazelcast | payload-64-key-32 | complete | 51 | 356478976/358430720/358445056 | 339294208/339492864 | 140.52/251.48 | 139.93/250.86 | N/A | 0 | 120586012 |
| 03-payload-key | hazelcast | payload-1024-key-8 | complete | 50 | 364666880/365456794/365514752 | 346597376/346689536 | 170.98/267.69 | 134.27/213.88 | N/A | 0 | 143700260 |
| 03-payload-key | hazelcast | payload-1024-key-32 | complete | 51 | 367308800/368556032/368582656 | 349468672/349519872 | 170.46/283.41 | 140.90/242.87 | N/A | 0 | 117898521 |
| 03-payload-key | hazelcast | payload-4096-key-8 | complete | 51 | 412704768/413429760/413446144 | 394414080/394809344 | 140.03/213.28 | 140.42/212.88 | N/A | 0 | 176156340 |
| 03-payload-key | hazelcast | payload-4096-key-32 | complete | 51 | 400756736/403120128/403267584 | 383924224/383983616 | 178.46/277.37 | 140.92/219.87 | N/A | 0 | 160766155 |
| 04-clients-pipeline | hydra | clients-1-pipeline-1 | complete | 48 | 18649088/18649088/18649088 | 74248192/82575360 | N/A/N/A | 18.19/41.98 | 0.06 | 0 | 14017137 |
| 04-clients-pipeline | hydra | clients-10-pipeline-1 | complete | 51 | 18718720/18718720/18718720 | 83431424/84103168 | N/A/N/A | 16.99/22.97 | 1.41 | 0 | 13302769 |
| 04-clients-pipeline | hydra | clients-10-pipeline-10 | complete | 50 | 18808832/18808832/18808832 | 81480909/84217856 | N/A/N/A | 10.54/14.99 | 1.24 | 0 | 13614558 |
| 04-clients-pipeline | hydra | clients-50-pipeline-10 | complete | 49 | 19619840/19619840/19619840 | 86248653/88272896 | N/A/N/A | 11.60/18.99 | 5.48 | 0 | 14937416 |
| 04-clients-pipeline | hydra | clients-100-pipeline-10 | complete | 49 | 20660224/20660224/20660224 | 84558643/93573120 | N/A/N/A | 12.79/20.99 | 10.74 | 0 | 16175831 |
| 04-clients-pipeline | redis | clients-1-pipeline-1 | complete | 48 | 14458880/14491648/14512128 | 7290880/7294976 | 31.69/45.79 | 14.64/41.98 | 0.06 | 0 | 5001164 |
| 04-clients-pipeline | redis | clients-10-pipeline-1 | complete | 51 | 14565376/14573568/14573568 | 7372800/7421952 | 18.51/22.08 | 16.49/20.99 | 1.41 | 0 | 4804286 |
| 04-clients-pipeline | redis | clients-10-pipeline-10 | complete | 50 | 14630912/14635008/14635008 | 7401472/7401472 | 2.64/4.05 | 2.55/4.00 | 1.23 | 0 | 4897065 |
| 04-clients-pipeline | redis | clients-50-pipeline-10 | complete | 49 | 14839808/15155200/15155200 | 8003584/8425472 | 4.02/16.91 | 3.00/5.00 | 5.44 | 0 | 5265162 |
| 04-clients-pipeline | redis | clients-100-pipeline-10 | complete | 49 | 15052800/16060416/16060416 | 8790016/9256960 | 3.53/5.86 | 4.00/7.00 | 10.55 | 0 | 5336557 |
| 04-clients-pipeline | hazelcast | clients-1-pipeline-1 | complete | 54 | 332898304/335336448/335970304 | 316577587/317132800 | 158.66/276.95 | 109.09/253.53 | N/A | 0 | 77393611 |
| 04-clients-pipeline | hazelcast | clients-10-pipeline-1 | complete | 50 | 348354560/349423821/349458432 | 330516275/330715136 | 144.84/249.09 | 144.71/249.85 | N/A | 0 | 98038705 |
| 04-clients-pipeline | hazelcast | clients-10-pipeline-10 | complete | 48 | 333819904/363407770/363425792 | 344444928/344612864 | 151.65/269.00 | 150.32/267.87 | N/A | 0 | 138005460 |
| 04-clients-pipeline | hazelcast | clients-50-pipeline-10 | complete | 48 | 334524416/338683904/339238912 | 319707750/320086016 | 141.48/229.46 | 142.36/229.61 | N/A | 0 | 88019774 |
| 04-clients-pipeline | hazelcast | clients-100-pipeline-10 | complete | 48 | 356446208/357335040/358600704 | 338172518/339382272 | 139.24/235.45 | 138.76/235.87 | N/A | 0 | 135767976 |
| 05-workload-mix | hydra | set-100 | complete | 47 | 18718720/18718720/18718720 | 77000704/81440768 | N/A/N/A | 0.00/14.99 | 0.90 | 0 | 14417404 |
| 05-workload-mix | hydra | set-90 | complete | 47 | 18223104/18223104/18223104 | 76906496/83668992 | N/A/N/A | 0.00/14.99 | 1.23 | 0 | 13788280 |
| 05-workload-mix | hydra | set-50 | complete | 48 | 16281600/16281600/16281600 | 75079680/83894272 | N/A/N/A | 1.95/14.99 | 1.24 | 0 | 10988629 |
| 05-workload-mix | hydra | set-10 | complete | 47 | 9609216/9609216/9609216 | 68149248/77291520 | N/A/N/A | 0.00/12.99 | 0.95 | 0 | 2517915 |
| 05-workload-mix | redis | set-100 | complete | 47 | 14417920/14426112/14426112 | 7311360/7311360 | 2.32/9.73 | 0.70/5.00 | 0.90 | 0 | 5035434 |
| 05-workload-mix | redis | set-90 | complete | 47 | 14446592/14458880/14458880 | 7401472/7409664 | 0.05/4.33 | 1.00/4.00 | 1.24 | 0 | 5201741 |
| 05-workload-mix | redis | set-50 | complete | 48 | 14487552/14544896/14544896 | 7454720/7454720 | 0.40/4.10 | 1.00/4.00 | 1.23 | 0 | 4985790 |
| 05-workload-mix | redis | set-10 | complete | 47 | 11399168/11399168/11399168 | 4169728/4169728 | 0.05/3.25 | 1.00/3.00 | 0.92 | 0 | 869583 |
| 05-workload-mix | hazelcast | set-100 | complete | 47 | 296562688/300429312/300503040 | 281005261/281251840 | 116.07/284.33 | 72.36/266.84 | N/A | 0 | 44652155 |
| 05-workload-mix | hazelcast | set-90 | complete | 47 | 298582016/301917389/302002176 | 282957414/283148288 | 98.52/260.18 | 98.64/259.85 | N/A | 0 | 48736856 |
| 05-workload-mix | hazelcast | set-50 | complete | 47 | 302919680/307100058/307175424 | 288178586/288636928 | 194.55/281.51 | 193.90/2815.13 | N/A | 0 | 52782241 |
| 05-workload-mix | hazelcast | set-10 | complete | 47 | 290848768/294762086/294850560 | 275641549/275714048 | 54.64/238.64 | 54.05/237.79 | N/A | 0 | 39922414 |
| 06-key-distribution | hydra | uniform | complete | 50 | 18903040/18903040/18903040 | 86116966/88637440 | N/A/N/A | 10.00/14.99 | 1.24 | 0 | 13745409 |
| 06-key-distribution | hydra | hot | complete | 50 | 12976128/12976128/12976128 | 81564467/82665472 | N/A/N/A | 9.54/13.99 | 1.25 | 0 | 6389253 |
| 06-key-distribution | hydra | zipf | complete | 50 | 17350656/17350656/17350656 | 85458330/87277568 | N/A/N/A | 10.55/14.99 | 1.26 | 0 | 11806155 |
| 06-key-distribution | redis | uniform | complete | 50 | 14745600/14753792/14753792 | 7467008/7467008 | 2.66/4.10 | 2.55/4.00 | 1.23 | 0 | 4907416 |
| 06-key-distribution | redis | hot | complete | 50 | 10596352/10600448/10600448 | 3407872/3407872 | 2.53/3.69 | 2.00/4.00 | 1.24 | 0 | 75120 |
| 06-key-distribution | redis | zipf | complete | 50 | 13705216/13737984/13737984 | 6483968/6483968 | 2.61/4.13 | 2.55/4.00 | 1.24 | 0 | 3796188 |
| 06-key-distribution | hazelcast | uniform | complete | 48 | 348278784/350469734/350507008 | 331538432/331685888 | 161.55/291.22 | 161.42/292.42 | N/A | 0 | 114419433 |
| 06-key-distribution | hazelcast | hot | complete | 48 | 343541760/344504934/345387008 | 325267251/326111232 | 130.42/237.85 | 130.30/237.87 | N/A | 0 | 125464769 |
| 06-key-distribution | hazelcast | zipf | complete | 48 | 331862016/336224870/336564224 | 317296435/317530112 | 129.96/282.35 | 130.57/281.84 | N/A | 0 | 114766023 |
| 07-persistence | hydra | storage | complete | 51 | 18874368/18874368/18874368 | 89563136/90480640 | N/A/N/A | 17.99/21.99 | 1.41 | 0 | 13390923 |
| 07-persistence | redis | ephemeral | complete | 51 | 14553088/14553088/14553088 | 7378944/7405568 | 18.44/30.94 | 15.99/20.99 | 1.42 | 0 | 4779661 |
| 07-persistence | redis | rdb | complete | 51 | 14589952/14589952/14589952 | 7696384/7729152 | 19.05/22.81 | 16.49/20.99 | 1.41 | 0 | 4868011 |
| 07-persistence | redis | aof | complete | 51 | 14917632/14925824/14925824 | 13684736/13684736 | 20.16/51.66 | 16.99/23.99 | 1.41 | 0 | 4922690 |
| 07-persistence | hazelcast | baseline | complete | 52 | 327231488/328993178/329089024 | 309010432/309272576 | 96.47/100.97 | 96.11/100.97 | N/A | 0 | 99856114 |
| 08-allocator | hydra | default | complete | 51 | 18845696/18845696/18845696 | 90441728/91758592 | N/A/N/A | 16.99/21.99 | 1.41 | 0 | 13312684 |
| 08-allocator | hydra | allocator-trim | complete | 51 | 18894848/18894848/18894848 | 90685440/92041216 | N/A/N/A | 16.98/21.99 | 1.42 | 0 | 13312454 |
| 09-memory-pressure | redis | limit-256m | complete | 45 | 14692352/14741504/14741504 | 7249101/7249920 | 3.79/4.11 | 4.00/4.00 | 1.26 | 0 | 5390200 |
| 09-memory-pressure | redis | limit-512m | complete | 45 | 14532608/14544896/14544896 | 7229440/7229440 | 3.84/14.54 | 4.00/4.00 | 1.26 | 0 | 5495564 |
| 09-memory-pressure | hazelcast | limit-256m | failed | 60 | 254631936/254631936/254631936 | 236236800/236236800 | 0.00/0.00 | 0.00/0.00 | N/A | 0 | N/A |
| 09-memory-pressure | hazelcast | limit-512m | complete | 44 | 402245632/404709171/405053440 | 386246656/386715648 | 156.26/224.91 | 155.12/225.88 | N/A | 0 | 209615319 |
| 10-hazelcast-jvm | hazelcast | jvm-probe | complete | 86 | 353280000/353965056/358166528 | 334681088/338657280 | 12.94/288.76 | 12.49/287.84 | N/A | 0 | 68092704 |

## Target-level reading

The table is a screening view, not a causal attribution. Compare like-for-like rows (same payload, key length, clients, pipeline, request count and affinity).

### hazelcast

- Complete cases: 22; largest sampled RSS: 493584384 bytes; highest container CPU p95: 194.55%; highest process CPU p95: 193.90%.
- JVM heap is reported independently; `N/A` means the probe was unavailable, not zero heap.

### hydra

- Complete cases: 26; largest sampled RSS: 212365312 bytes; highest container CPU p95: 0.00%; highest process CPU p95: 19.99%.
- JVM heap is reported independently; `N/A` means the probe was unavailable, not zero heap.

### redis

- Complete cases: 28; largest sampled RSS: 63258624 bytes; highest container CPU p95: 31.69%; highest process CPU p95: 18.49%.
- JVM heap is reported independently; `N/A` means the probe was unavailable, not zero heap.

## Metric definitions and limitations

- `container_cpu_percent` is cgroup CPU normalized by effective affinity; Hydra's host process has no container CPU field and uses `process_cpu_percent`.
- `vmrss_bytes`/`vmhwm_bytes` are process RSS/high-water RSS. Cgroup current/peak/limit are separate accounting domains.
- `jvm_heap_*` comes from `jcmd GC.heap_info` when available; slim images may show unavailable. Never substitute RSS for heap.
- PSI, faults, I/O, context switches and host network counters are host/kernel signals sampled with the target. They are supporting evidence and may include unrelated host activity.
- A failed workload or failed target start remains failed even if telemetry files exist. Missing values are preserved as `N/A`.

## Reproduction

Run from the exact source checkout after installing the pinned Hazelcast client and using pinned image digests:

```bash
export DOCKER_HOST=unix:///run/user/1002/docker.sock
export HAZELCAST_IMAGE='hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90'
export HAZELCAST_CLIENT_PYTHON=/home/hydracache-admin/.venvs/hazelcast/bin/python
export HAZELCAST_CLIENT_VERSION=5.5.0
export MEASUREMENT_AFFINITY=4
bash scripts/perf/run-metric-expansion-stage.sh /dev/shm/hydracache-metric-expansion-<timestamp>
```
