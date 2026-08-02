# Relative eight-case telemetry report

> Exploratory only. This report is not qualification/bootstrap evidence.

- Generated (UTC): 2026-08-02T18:37:27.790065+00:00
- Source commit: cd7d8b323c6cc362a48f67b86beb79c511416ec6
- Targets: HydraCache, Redis, Hazelcast Community
- Workload: 8 cases x SET/GET x configured repeats
- Sampling interval: 1 second by default

## Reproduction

The exact command and environment used for this run:

~~~text
branch=
source_commit=cd7d8b323c6cc362a48f67b86beb79c511416ec6
command=scripts/perf/run-relative-eight-cases-telemetry.sh /dev/shm/hydracache-exploratory-telemetry-20260803T000000Z
targets=hydracache,redis,hazelcast-community
hazelcast_image=hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90
hazelcast_client_version=5.5.0
measurement_affinity=3
requests_per_case=100000
repeats=3
telemetry_interval_seconds=1
~~~

Re-run from the recorded source commit with the same image digest, client version, affinity, request count, and repeats.

## Host and validation receipt

~~~text
reference evidence tmpfs verified: root=/dev/shm/hydracache-reference-evidence-v1
reference runtime IRQ guard passed: phase=relative-eight-telemetry-pre measurement=1-4 irq_files=113 dormant-unmapped-nvme=8
host=hydracache-perf-v1
source_commit=cd7d8b323c6cc362a48f67b86beb79c511416ec6
source_status=
kernel=Linux 6.8.0-136-generic x86_64 GNU/Linux
cpu_model=AMD EPYC 7232P 8-Core Processor
logical_cpus=4
measurement_affinity=3
targets=hydracache,redis,hazelcast-community
runner_receipt_sha256=97a39b307c063872b5c249eda9cf8341d70e0c293932b75bc67ae596cb0b17ae
runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json
telemetry_interval_seconds=1
redis_benchmark=/usr/bin/redis-benchmark
redis_benchmark_version=redis-benchmark 7.0.15
hazelcast_image=hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90
hazelcast_client=5.5.0
reference runtime IRQ delta baseline captured: phase=baseline measurement=3 file=/dev/shm/hydracache-exploratory-telemetry-20260803T000000Z/irq-baseline.tsv
irq_guard_mode=preflight-plus-baseline-delta
run_status=REJECTED_IRQ_DELTA
~~~

## Telemetry summary

The summary preserves sample counts and reports p50/p95/max. Missing JVM heap fields remain unavailable; they are never inferred from RSS.

~~~json
{
  "repeat-1--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 490545152.0,
      "p50": 490131456.0,
      "p95": 490532864.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.8116,
      "p50": 0.6952,
      "p95": 0.7831,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 402325504.0,
      "p50": 402284544.0,
      "p95": 402311168.0,
      "samples": 11
    }
  },
  "repeat-1--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 490119168.0,
      "p50": 489734144.0,
      "p95": 490004275.2,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 1.0668,
      "p50": 0.74495,
      "p95": 1.0161449999999999,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 401915904.0,
      "p50": 401817600.0,
      "p95": 401913651.2,
      "samples": 12
    }
  },
  "repeat-1--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 197296128.0,
      "p50": 196943872.0,
      "p95": 197253120.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 218165248.0,
      "p50": 218165248.0,
      "p95": 218165248.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 146280448.0,
      "p50": 146280448.0,
      "p95": 146280448.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 146280448.0,
      "p50": 146280448.0,
      "p95": 146280448.0,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 193662976.0,
      "p50": 182587392.0,
      "p95": 192580403.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 193662976.0,
      "p50": 184029184.0,
      "p95": 192580403.2,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 142921728.0,
      "p50": 131776512.0,
      "p95": 141825024.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 142921728.0,
      "p50": 131776512.0,
      "p95": 141825024.0,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 34537472.0,
      "p50": 33550336.0,
      "p95": 34438758.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.3859,
      "p50": 5.3825,
      "p95": 5.38556,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 26812416.0,
      "p50": 25632768.0,
      "p95": 26694451.2,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 35405824.0,
      "p50": 35168256.0,
      "p95": 35382067.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35168256.0,
      "p95": 35404185.6,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.7502,
      "p50": 5.5876,
      "p95": 5.7339400000000005,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27230208.0,
      "p50": 27205632.0,
      "p95": 27227750.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27230208.0,
      "p50": 27205632.0,
      "p95": 27227750.4,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 490954752.0,
      "p50": 490645504.0,
      "p95": 490936320.0,
      "samples": 6
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 6
    },
    "container_cpu_percent": {
      "max": 1.3554,
      "p50": 0.9794499999999999,
      "p95": 1.28005,
      "samples": 6
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 6
    },
    "vmrss_bytes": {
      "max": 402718720.0,
      "p50": 402706432.0,
      "p95": 402718720.0,
      "samples": 6
    }
  },
  "repeat-1--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 490512384.0,
      "p50": 490319872.0,
      "p95": 490474700.8,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.9539,
      "p50": 1.5657,
      "p95": 1.90696,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 402558976.0,
      "p50": 402513920.0,
      "p95": 402558976.0,
      "samples": 5
    }
  },
  "repeat-1--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 223125504.0,
      "p50": 222851072.0,
      "p95": 223098060.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 246071296.0,
      "p50": 246071296.0,
      "p95": 246071296.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 171036672.0,
      "p50": 171036672.0,
      "p95": 171036672.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 171036672.0,
      "p50": 171036672.0,
      "p95": 171036672.0,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 221499392.0,
      "p50": 210202624.0,
      "p95": 220369715.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 221499392.0,
      "p50": 218431488.0,
      "p95": 221192601.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 168976384.0,
      "p50": 157675520.0,
      "p95": 167846297.6,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 168976384.0,
      "p50": 157675520.0,
      "p95": 167846297.6,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 33484800.0,
      "p50": 33484800.0,
      "p95": 33484800.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25710592.0,
      "p50": 25710592.0,
      "p95": 25710592.0,
      "samples": 1
    }
  },
  "repeat-1--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 33615872.0,
      "p50": 33615872.0,
      "p95": 33615872.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 19.7617,
      "p50": 19.7617,
      "p95": 19.7617,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25317376.0,
      "p50": 25317376.0,
      "p95": 25317376.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 491319296.0,
      "p50": 490915840.0,
      "p95": 491228774.4,
      "samples": 18
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 18
    },
    "container_cpu_percent": {
      "max": 1.7357,
      "p50": 1.48555,
      "p95": 1.6007199999999997,
      "samples": 18
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 18
    },
    "vmrss_bytes": {
      "max": 403025920.0,
      "p50": 402767872.0,
      "p95": 403025920.0,
      "samples": 18
    }
  },
  "repeat-1--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 491266048.0,
      "p50": 491098112.0,
      "p95": 491220172.8,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 1.9328,
      "p50": 1.6373,
      "p95": 1.7582399999999998,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 402939904.0,
      "p50": 402878464.0,
      "p95": 402936627.2,
      "samples": 17
    }
  },
  "repeat-1--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 237633536.0,
      "p50": 237395968.0,
      "p95": 237596672.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 255213568.0,
      "p50": 255213568.0,
      "p95": 255213568.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 187117568.0,
      "p50": 187117568.0,
      "p95": 187117568.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 187117568.0,
      "p50": 187117568.0,
      "p95": 187117568.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 232833024.0,
      "p50": 226209792.0,
      "p95": 231968768.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 246210560.0,
      "p50": 246210560.0,
      "p95": 246210560.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 182910976.0,
      "p50": 176001024.0,
      "p95": 182025420.8,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 182910976.0,
      "p50": 176001024.0,
      "p95": 182025420.8,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22384640.0,
      "p50": 22384640.0,
      "p95": 22384640.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.1372,
      "p50": 3.1332,
      "p95": 3.13688,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14692352.0,
      "p50": 14692352.0,
      "p95": 14692352.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 34226176.0,
      "p50": 34226176.0,
      "p95": 34226176.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.5659,
      "p50": 3.118,
      "p95": 3.47648,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26685440.0,
      "p50": 26685440.0,
      "p95": 26685440.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 488042496.0,
      "p50": 487342080.0,
      "p95": 487776256.0,
      "samples": 80
    },
    "cgroup_memory_peak_bytes": {
      "max": 491732992.0,
      "p50": 490049536.0,
      "p95": 491732992.0,
      "samples": 80
    },
    "container_cpu_percent": {
      "max": 0.7433,
      "p50": 0.1634,
      "p95": 0.24581999999999984,
      "samples": 80
    },
    "vmhwm_bytes": {
      "max": 403546112.0,
      "p50": 401137664.0,
      "p95": 403546112.0,
      "samples": 80
    },
    "vmrss_bytes": {
      "max": 399884288.0,
      "p50": 399675392.0,
      "p95": 399745024.0,
      "samples": 80
    }
  },
  "repeat-1--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 487989248.0,
      "p50": 487116800.0,
      "p95": 487540736.0,
      "samples": 51
    },
    "cgroup_memory_peak_bytes": {
      "max": 490049536.0,
      "p50": 490049536.0,
      "p95": 490049536.0,
      "samples": 51
    },
    "container_cpu_percent": {
      "max": 0.6754,
      "p50": 0.2457,
      "p95": 0.6014999999999999,
      "samples": 51
    },
    "vmhwm_bytes": {
      "max": 401137664.0,
      "p50": 401137664.0,
      "p95": 401137664.0,
      "samples": 51
    },
    "vmrss_bytes": {
      "max": 399630336.0,
      "p50": 399417344.0,
      "p95": 399628288.0,
      "samples": 51
    }
  },
  "repeat-1--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 137281536.0,
      "p50": 137254912.0,
      "p95": 137280307.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 155652096.0,
      "p50": 155652096.0,
      "p95": 155652096.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 87801856.0,
      "p50": 87801856.0,
      "p95": 87801856.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 87801856.0,
      "p50": 87801856.0,
      "p95": 87801856.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 133591040.0,
      "p50": 122961920.0,
      "p95": 132547788.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 133627904.0,
      "p50": 129056768.0,
      "p95": 132942233.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 84520960.0,
      "p50": 73742336.0,
      "p95": 83445760.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 84520960.0,
      "p50": 73742336.0,
      "p95": 83445760.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22032384.0,
      "p50": 22032384.0,
      "p95": 22032384.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 22986752.0,
      "p50": 22986752.0,
      "p95": 22986752.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5643,
      "p50": 5.4992,
      "p95": 5.55779,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 15204352.0,
      "p50": 15204352.0,
      "p95": 15204352.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 14553088.0,
      "p50": 14553088.0,
      "p95": 14553088.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 22974464.0,
      "p50": 22724608.0,
      "p95": 22949478.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 22986752.0,
      "p50": 22740992.0,
      "p95": 22962176.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.6636,
      "p50": 5.564,
      "p95": 5.653639999999999,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 15265792.0,
      "p50": 15241216.0,
      "p95": 15263334.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15265792.0,
      "p50": 15241216.0,
      "p95": 15263334.4,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 488812544.0,
      "p50": 488370176.0,
      "p95": 488665088.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 1.4337,
      "p50": 0.5273,
      "p95": 1.1535,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 400683008.0,
      "p50": 400670720.0,
      "p95": 400683008.0,
      "samples": 11
    }
  },
  "repeat-1--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 488558592.0,
      "p50": 488202240.0,
      "p95": 488507801.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 491732992.0,
      "p50": 491732992.0,
      "p95": 491732992.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.9198,
      "p50": 1.723,
      "p95": 1.9159599999999999,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 403546112.0,
      "p50": 403546112.0,
      "p95": 403546112.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 400285696.0,
      "p50": 400224256.0,
      "p95": 400284876.8,
      "samples": 5
    }
  },
  "repeat-1--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 162430976.0,
      "p50": 162287616.0,
      "p95": 162416640.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 181080064.0,
      "p50": 181080064.0,
      "p95": 181080064.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 112607232.0,
      "p50": 112607232.0,
      "p95": 112607232.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 112607232.0,
      "p50": 112607232.0,
      "p95": 112607232.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 161099776.0,
      "p50": 149409792.0,
      "p95": 159930777.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 161116160.0,
      "p50": 156102656.0,
      "p95": 160614809.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 111321088.0,
      "p50": 99713024.0,
      "p95": 110160281.6,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 111321088.0,
      "p50": 99713024.0,
      "p95": 110160281.6,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22024192.0,
      "p50": 22024192.0,
      "p95": 22024192.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 22986752.0,
      "p50": 22986752.0,
      "p95": 22986752.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 6.0985,
      "p50": 6.0985,
      "p95": 6.0985,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 15204352.0,
      "p50": 15204352.0,
      "p95": 15204352.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14553088.0,
      "p50": 14553088.0,
      "p95": 14553088.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 22007808.0,
      "p50": 22007808.0,
      "p95": 22007808.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 22986752.0,
      "p50": 22986752.0,
      "p95": 22986752.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 18.9266,
      "p50": 18.9266,
      "p95": 18.9266,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 15204352.0,
      "p50": 15204352.0,
      "p95": 15204352.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14553088.0,
      "p50": 14553088.0,
      "p95": 14553088.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 490647552.0,
      "p50": 490440704.0,
      "p95": 490583040.0,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 0.9846,
      "p50": 0.94055,
      "p95": 0.983445,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 402731008.0,
      "p50": 402726912.0,
      "p95": 402731008.0,
      "samples": 8
    }
  },
  "repeat-1--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 490827776.0,
      "p50": 490600448.0,
      "p95": 490794803.2,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 1.0323,
      "p50": 0.95385,
      "p95": 1.03209,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 402780160.0,
      "p50": 402776064.0,
      "p95": 402780160.0,
      "samples": 8
    }
  },
  "repeat-1--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 266596352.0,
      "p50": 266483712.0,
      "p95": 266588979.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 290050048.0,
      "p50": 290050048.0,
      "p95": 290050048.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 214233088.0,
      "p50": 214233088.0,
      "p95": 214233088.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 214233088.0,
      "p50": 214233088.0,
      "p95": 214233088.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 263176192.0,
      "p50": 251949056.0,
      "p95": 262081945.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 263225344.0,
      "p50": 255596544.0,
      "p95": 262123724.8,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 211001344.0,
      "p50": 199813120.0,
      "p95": 209881907.2,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 211001344.0,
      "p50": 199813120.0,
      "p95": 209881907.2,
      "samples": 4
    }
  },
  "repeat-1--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 24002560.0,
      "p50": 24002560.0,
      "p95": 24002560.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.2575,
      "p50": 5.2328,
      "p95": 5.2550300000000005,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15757312.0,
      "p50": 15757312.0,
      "p95": 15757312.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 23834624.0,
      "p50": 23834624.0,
      "p95": 23834624.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 19.155,
      "p50": 5.544,
      "p95": 17.7939,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15654912.0,
      "p50": 15585280.0,
      "p95": 15647948.8,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 485085184.0,
      "p50": 482611200.0,
      "p95": 484811571.2,
      "samples": 80
    },
    "cgroup_memory_peak_bytes": {
      "max": 489586688.0,
      "p50": 489586688.0,
      "p95": 489586688.0,
      "samples": 80
    },
    "container_cpu_percent": {
      "max": 10.6089,
      "p50": 0.17795,
      "p95": 0.64079,
      "samples": 80
    },
    "vmhwm_bytes": {
      "max": 401137664.0,
      "p50": 401137664.0,
      "p95": 401137664.0,
      "samples": 80
    },
    "vmrss_bytes": {
      "max": 397164544.0,
      "p50": 395059200.0,
      "p95": 397156352.0,
      "samples": 80
    }
  },
  "repeat-1--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 470040576.0,
      "p50": 455557120.0,
      "p95": 469714944.0,
      "samples": 54
    },
    "cgroup_memory_peak_bytes": {
      "max": 470933504.0,
      "p50": 457885696.0,
      "p95": 470866944.0,
      "samples": 54
    },
    "container_cpu_percent": {
      "max": 11.9895,
      "p50": 0.2494,
      "p95": 1.6028099999999996,
      "samples": 54
    },
    "vmhwm_bytes": {
      "max": 382300160.0,
      "p50": 369389568.0,
      "p95": 382127308.8,
      "samples": 54
    },
    "vmrss_bytes": {
      "max": 382300160.0,
      "p50": 367669248.0,
      "p95": 382076723.2,
      "samples": 54
    }
  },
  "repeat-1--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 84979712.0,
      "p50": 84824064.0,
      "p95": 84960665.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 103104512.0,
      "p50": 103104512.0,
      "p95": 103104512.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 36081664.0,
      "p50": 36081664.0,
      "p95": 36081664.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 36081664.0,
      "p50": 36081664.0,
      "p95": 36081664.0,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 80547840.0,
      "p50": 69847040.0,
      "p95": 79487385.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 80670720.0,
      "p50": 69855232.0,
      "p95": 79594291.2,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 32886784.0,
      "p50": 22028288.0,
      "p95": 31809126.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 32886784.0,
      "p50": 22028288.0,
      "p95": 31809126.4,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 19824640.0,
      "p50": 19824640.0,
      "p95": 19824640.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20090880.0,
      "p50": 20090880.0,
      "p95": 20090880.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 6.6866,
      "p50": 5.4371,
      "p95": 6.56165,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 12107776.0,
      "p50": 12107776.0,
      "p95": 12107776.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12107776.0,
      "p50": 12107776.0,
      "p95": 12107776.0,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19804160.0,
      "p50": 19783680.0,
      "p95": 19802112.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 19820544.0,
      "p50": 19787776.0,
      "p95": 19817267.2,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5557,
      "p50": 5.5343,
      "p95": 5.55356,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 12091392.0,
      "p50": 12079104.0,
      "p95": 12090163.2,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12091392.0,
      "p50": 12079104.0,
      "p95": 12090163.2,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 486952960.0,
      "p50": 486567936.0,
      "p95": 486903398.4,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 489586688.0,
      "p50": 489586688.0,
      "p95": 489586688.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 0.6866,
      "p50": 0.51305,
      "p95": 0.6282449999999999,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 401137664.0,
      "p50": 401137664.0,
      "p95": 401137664.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 398974976.0,
      "p50": 398966784.0,
      "p95": 398974976.0,
      "samples": 12
    }
  },
  "repeat-1--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 486096896.0,
      "p50": 485924864.0,
      "p95": 486068224.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 489586688.0,
      "p50": 489586688.0,
      "p95": 489586688.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 2.2932,
      "p50": 1.8066,
      "p95": 2.28782,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 401137664.0,
      "p50": 401137664.0,
      "p95": 401137664.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 398114816.0,
      "p50": 398045184.0,
      "p95": 398102528.0,
      "samples": 5
    }
  },
  "repeat-1--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 110010368.0,
      "p50": 109899776.0,
      "p95": 109999308.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 128512000.0,
      "p50": 128512000.0,
      "p95": 128512000.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 60915712.0,
      "p50": 60915712.0,
      "p95": 60915712.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 60915712.0,
      "p50": 60915712.0,
      "p95": 60915712.0,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 108703744.0,
      "p50": 97030144.0,
      "p95": 107536384.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 108969984.0,
      "p50": 103395328.0,
      "p95": 108412518.4,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 60153856.0,
      "p50": 48304128.0,
      "p95": 58968883.2,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 60153856.0,
      "p50": 48304128.0,
      "p95": 58968883.2,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 19578880.0,
      "p50": 19578880.0,
      "p95": 19578880.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20090880.0,
      "p50": 20090880.0,
      "p95": 20090880.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 4.8216,
      "p50": 4.8216,
      "p95": 4.8216,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 12111872.0,
      "p50": 12111872.0,
      "p95": 12111872.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12111872.0,
      "p50": 12111872.0,
      "p95": 12111872.0,
      "samples": 1
    }
  },
  "repeat-1--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19816448.0,
      "p50": 19816448.0,
      "p95": 19816448.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20090880.0,
      "p50": 20090880.0,
      "p95": 20090880.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 12107776.0,
      "p50": 12107776.0,
      "p95": 12107776.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12107776.0,
      "p50": 12107776.0,
      "p95": 12107776.0,
      "samples": 1
    }
  },
  "repeat-2--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 492961792.0,
      "p50": 492494848.0,
      "p95": 492899123.2,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 497471488.0,
      "p50": 497471488.0,
      "p95": 497471488.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 1.3673,
      "p50": 0.7961499999999999,
      "p95": 1.2165049999999997,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 408653824.0,
      "p50": 408653824.0,
      "p95": 408653824.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 404697088.0,
      "p50": 404697088.0,
      "p95": 404697088.0,
      "samples": 10
    }
  },
  "repeat-2--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 492392448.0,
      "p50": 492191744.0,
      "p95": 492387532.8,
      "samples": 13
    },
    "cgroup_memory_peak_bytes": {
      "max": 494469120.0,
      "p50": 494469120.0,
      "p95": 494469120.0,
      "samples": 13
    },
    "container_cpu_percent": {
      "max": 0.9604,
      "p50": 0.6259,
      "p95": 0.8966199999999999,
      "samples": 13
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 13
    },
    "vmrss_bytes": {
      "max": 404291584.0,
      "p50": 404291584.0,
      "p95": 404291584.0,
      "samples": 13
    }
  },
  "repeat-2--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 397737984.0,
      "p50": 397637632.0,
      "p95": 397731840.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 419106816.0,
      "p50": 419106816.0,
      "p95": 419106816.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 345149440.0,
      "p50": 345149440.0,
      "p95": 345149440.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 345149440.0,
      "p50": 345149440.0,
      "p95": 345149440.0,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 394149888.0,
      "p50": 383502336.0,
      "p95": 393129984.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 394448896.0,
      "p50": 384843776.0,
      "p95": 393384755.2,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 341893120.0,
      "p50": 330776576.0,
      "p95": 340799488.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 341893120.0,
      "p50": 330776576.0,
      "p95": 340799488.0,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 32575488.0,
      "p50": 32575488.0,
      "p95": 32575488.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.2995,
      "p50": 5.2615,
      "p95": 5.2957,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 24813568.0,
      "p50": 24801280.0,
      "p95": 24812339.2,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 35237888.0,
      "p50": 35237888.0,
      "p95": 35237888.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 7.7831,
      "p50": 5.5444,
      "p95": 7.5592299999999994,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27361280.0,
      "p50": 27328512.0,
      "p95": 27358003.2,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27361280.0,
      "p50": 27328512.0,
      "p95": 27358003.2,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 493252608.0,
      "p50": 493035520.0,
      "p95": 493234585.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 497471488.0,
      "p50": 497471488.0,
      "p95": 497471488.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.0853,
      "p50": 0.9374,
      "p95": 1.0651,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 408653824.0,
      "p50": 408653824.0,
      "p95": 408653824.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 405377024.0,
      "p50": 404807680.0,
      "p95": 405263155.2,
      "samples": 5
    }
  },
  "repeat-2--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 492716032.0,
      "p50": 492466176.0,
      "p95": 492666060.8,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 497471488.0,
      "p50": 497471488.0,
      "p95": 497471488.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.3622,
      "p50": 1.2978,
      "p95": 1.3533000000000002,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 408653824.0,
      "p50": 408653824.0,
      "p95": 408653824.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 404717568.0,
      "p50": 404713472.0,
      "p95": 404716748.8,
      "samples": 5
    }
  },
  "repeat-2--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 423960576.0,
      "p50": 423489536.0,
      "p95": 423913472.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 446832640.0,
      "p50": 446832640.0,
      "p95": 446832640.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 370008064.0,
      "p50": 370008064.0,
      "p95": 370008064.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 370008064.0,
      "p50": 370008064.0,
      "p95": 370008064.0,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 422391808.0,
      "p50": 410746880.0,
      "p95": 421227315.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 422395904.0,
      "p50": 419106816.0,
      "p95": 422066995.2,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 367980544.0,
      "p50": 356663296.0,
      "p95": 366848819.2,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 367980544.0,
      "p50": 356663296.0,
      "p95": 366848819.2,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 33353728.0,
      "p50": 33353728.0,
      "p95": 33353728.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25341952.0,
      "p50": 25341952.0,
      "p95": 25341952.0,
      "samples": 1
    }
  },
  "repeat-2--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 33398784.0,
      "p50": 33398784.0,
      "p95": 33398784.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25251840.0,
      "p50": 25251840.0,
      "p95": 25251840.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 493432832.0,
      "p50": 492984320.0,
      "p95": 493356236.8,
      "samples": 18
    },
    "cgroup_memory_peak_bytes": {
      "max": 497471488.0,
      "p50": 497471488.0,
      "p95": 497471488.0,
      "samples": 18
    },
    "container_cpu_percent": {
      "max": 2.3446,
      "p50": 1.4996,
      "p95": 1.6473449999999987,
      "samples": 18
    },
    "vmhwm_bytes": {
      "max": 408653824.0,
      "p50": 408653824.0,
      "p95": 408653824.0,
      "samples": 18
    },
    "vmrss_bytes": {
      "max": 405028864.0,
      "p50": 404799488.0,
      "p95": 405028864.0,
      "samples": 18
    }
  },
  "repeat-2--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 493404160.0,
      "p50": 493125632.0,
      "p95": 493292748.8,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 497471488.0,
      "p50": 497471488.0,
      "p95": 497471488.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 1.695,
      "p50": 1.5499,
      "p95": 1.68476,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 408653824.0,
      "p50": 408653824.0,
      "p95": 408653824.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 405061632.0,
      "p50": 405000192.0,
      "p95": 405022310.4,
      "samples": 17
    }
  },
  "repeat-2--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 438415360.0,
      "p50": 438165504.0,
      "p95": 438396518.4,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 456220672.0,
      "p50": 456220672.0,
      "p95": 456220672.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 386048000.0,
      "p50": 386048000.0,
      "p95": 386048000.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 386048000.0,
      "p50": 386048000.0,
      "p95": 386048000.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 433987584.0,
      "p50": 427335680.0,
      "p95": 433133977.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 447049728.0,
      "p50": 447049728.0,
      "p95": 447049728.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 381861888.0,
      "p50": 374976512.0,
      "p95": 380982067.2,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 381861888.0,
      "p50": 374976512.0,
      "p95": 380982067.2,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22417408.0,
      "p50": 22159360.0,
      "p95": 22417408.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.437,
      "p50": 3.1695,
      "p95": 3.38812,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14708736.0,
      "p50": 14708736.0,
      "p95": 14708736.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 34406400.0,
      "p50": 34226176.0,
      "p95": 34371993.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.8331,
      "p50": 3.1297,
      "p95": 3.71246,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26701824.0,
      "p50": 26521600.0,
      "p95": 26665779.2,
      "samples": 5
    }
  },
  "repeat-2--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 491753472.0,
      "p50": 491157504.0,
      "p95": 491591270.4,
      "samples": 84
    },
    "cgroup_memory_peak_bytes": {
      "max": 494469120.0,
      "p50": 494469120.0,
      "p95": 494469120.0,
      "samples": 84
    },
    "container_cpu_percent": {
      "max": 0.4068,
      "p50": 0.13865,
      "p95": 0.21546999999999986,
      "samples": 84
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 84
    },
    "vmrss_bytes": {
      "max": 403738624.0,
      "p50": 403591168.0,
      "p95": 403734528.0,
      "samples": 84
    }
  },
  "repeat-2--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 492044288.0,
      "p50": 491671552.0,
      "p95": 492016435.2,
      "samples": 69
    },
    "cgroup_memory_peak_bytes": {
      "max": 494469120.0,
      "p50": 494469120.0,
      "p95": 494469120.0,
      "samples": 69
    },
    "container_cpu_percent": {
      "max": 0.9217,
      "p50": 0.1245,
      "p95": 0.5401399999999997,
      "samples": 69
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 69
    },
    "vmrss_bytes": {
      "max": 404131840.0,
      "p50": 403865600.0,
      "p95": 404131840.0,
      "samples": 69
    }
  },
  "repeat-2--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 338546688.0,
      "p50": 338233344.0,
      "p95": 338515968.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 356401152.0,
      "p50": 356401152.0,
      "p95": 356401152.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 286801920.0,
      "p50": 286801920.0,
      "p95": 286801920.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 286801920.0,
      "p50": 286801920.0,
      "p95": 286801920.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 335290368.0,
      "p50": 324243456.0,
      "p95": 334160486.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 335302656.0,
      "p50": 330035200.0,
      "p95": 334512537.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 283635712.0,
      "p50": 272799744.0,
      "p95": 282556825.6,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 283635712.0,
      "p50": 272799744.0,
      "p95": 282556825.6,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22323200.0,
      "p50": 22323200.0,
      "p95": 22323200.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5138,
      "p50": 5.4785,
      "p95": 5.51027,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 14807040.0,
      "p50": 14807040.0,
      "p95": 14807040.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 23068672.0,
      "p50": 23068672.0,
      "p95": 23068672.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 6.6253,
      "p50": 5.5714,
      "p95": 6.51991,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15527936.0,
      "p50": 15523840.0,
      "p95": 15527526.4,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 492003328.0,
      "p50": 491757568.0,
      "p95": 492001280.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 494469120.0,
      "p50": 494469120.0,
      "p95": 494469120.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.7232,
      "p50": 0.5869,
      "p95": 0.6999,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 404094976.0,
      "p50": 404090880.0,
      "p95": 404094976.0,
      "samples": 11
    }
  },
  "repeat-2--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 491823104.0,
      "p50": 491655168.0,
      "p95": 491794432.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 494469120.0,
      "p50": 494469120.0,
      "p95": 494469120.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.9668,
      "p50": 1.8553,
      "p95": 1.9590400000000001,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 403824640.0,
      "p50": 403816448.0,
      "p95": 403824640.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 363266048.0,
      "p50": 363065344.0,
      "p95": 363245977.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 382111744.0,
      "p50": 382111744.0,
      "p95": 382111744.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 311607296.0,
      "p50": 311607296.0,
      "p95": 311607296.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 311607296.0,
      "p50": 311607296.0,
      "p95": 311607296.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 362188800.0,
      "p50": 350224384.0,
      "p95": 360992358.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 362209280.0,
      "p50": 356990976.0,
      "p95": 361687449.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 310366208.0,
      "p50": 298749952.0,
      "p95": 309204582.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 310366208.0,
      "p50": 298749952.0,
      "p95": 309204582.4,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22286336.0,
      "p50": 22286336.0,
      "p95": 22286336.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 8.0538,
      "p50": 8.0538,
      "p95": 8.0538,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14786560.0,
      "p50": 14786560.0,
      "p95": 14786560.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 22286336.0,
      "p50": 22286336.0,
      "p95": 22286336.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14786560.0,
      "p50": 14786560.0,
      "p95": 14786560.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 493002752.0,
      "p50": 492775424.0,
      "p95": 492969779.2,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 497471488.0,
      "p50": 497471488.0,
      "p95": 497471488.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 1.0021,
      "p50": 0.9196500000000001,
      "p95": 0.993385,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 408653824.0,
      "p50": 408653824.0,
      "p95": 408653824.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 404852736.0,
      "p50": 404848640.0,
      "p95": 404851302.4,
      "samples": 8
    }
  },
  "repeat-2--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 492941312.0,
      "p50": 492668928.0,
      "p95": 492886835.2,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 497471488.0,
      "p50": 497471488.0,
      "p95": 497471488.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 7.738,
      "p50": 0.96075,
      "p95": 5.400559999999997,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 408653824.0,
      "p50": 408653824.0,
      "p95": 408653824.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 404799488.0,
      "p50": 404787200.0,
      "p95": 404796620.8,
      "samples": 8
    }
  },
  "repeat-2--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 467193856.0,
      "p50": 467032064.0,
      "p95": 467175424.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 490823680.0,
      "p50": 490823680.0,
      "p95": 490823680.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 413110272.0,
      "p50": 413110272.0,
      "p95": 413110272.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 413110272.0,
      "p50": 413110272.0,
      "p95": 413110272.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 464154624.0,
      "p50": 452810752.0,
      "p95": 463017984.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 464154624.0,
      "p50": 456409088.0,
      "p95": 463021056.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 409985024.0,
      "p50": 398770176.0,
      "p95": 408862515.2,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 409985024.0,
      "p50": 398770176.0,
      "p95": 408862515.2,
      "samples": 4
    }
  },
  "repeat-2--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 23937024.0,
      "p50": 23932928.0,
      "p95": 23936614.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 19.1825,
      "p50": 5.466,
      "p95": 17.81085,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15675392.0,
      "p50": 15675392.0,
      "p95": 15675392.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 23814144.0,
      "p50": 23814144.0,
      "p95": 23814144.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 15.322,
      "p50": 5.3146,
      "p95": 14.321259999999999,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15642624.0,
      "p50": 15572992.0,
      "p95": 15635660.8,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 492175360.0,
      "p50": 490440704.0,
      "p95": 490891673.6,
      "samples": 78
    },
    "cgroup_memory_peak_bytes": {
      "max": 494415872.0,
      "p50": 494415872.0,
      "p95": 494415872.0,
      "samples": 78
    },
    "container_cpu_percent": {
      "max": 0.7879,
      "p50": 0.15239999999999998,
      "p95": 0.26621999999999924,
      "samples": 78
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 78
    },
    "vmrss_bytes": {
      "max": 402890752.0,
      "p50": 402649088.0,
      "p95": 402890752.0,
      "samples": 78
    }
  },
  "repeat-2--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 491106304.0,
      "p50": 490692608.0,
      "p95": 491028070.4,
      "samples": 43
    },
    "cgroup_memory_peak_bytes": {
      "max": 494227456.0,
      "p50": 494227456.0,
      "p95": 494227456.0,
      "samples": 43
    },
    "container_cpu_percent": {
      "max": 0.8013,
      "p50": 0.3139,
      "p95": 0.5542099999999999,
      "samples": 43
    },
    "vmhwm_bytes": {
      "max": 405688320.0,
      "p50": 405688320.0,
      "p95": 405688320.0,
      "samples": 43
    },
    "vmrss_bytes": {
      "max": 403128320.0,
      "p50": 402931712.0,
      "p95": 403116032.0,
      "samples": 43
    }
  },
  "repeat-2--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 286400512.0,
      "p50": 286128128.0,
      "p95": 286365491.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 304250880.0,
      "p50": 304250880.0,
      "p95": 304250880.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 235102208.0,
      "p50": 235102208.0,
      "p95": 235102208.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 235102208.0,
      "p50": 235102208.0,
      "p95": 235102208.0,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 283099136.0,
      "p50": 272347136.0,
      "p95": 282034380.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 290709504.0,
      "p50": 290709504.0,
      "p95": 290709504.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 232075264.0,
      "p50": 221325312.0,
      "p95": 230995763.2,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 232075264.0,
      "p50": 221325312.0,
      "p95": 230995763.2,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 20201472.0,
      "p50": 20201472.0,
      "p95": 20201472.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.7244,
      "p50": 5.3936,
      "p95": 5.69132,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12451840.0,
      "p50": 12451840.0,
      "p95": 12451840.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 23138304.0,
      "p50": 23138304.0,
      "p95": 23138304.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.6932,
      "p50": 5.4599,
      "p95": 5.66987,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15527936.0,
      "p50": 15527936.0,
      "p95": 15527936.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 491651072.0,
      "p50": 491474944.0,
      "p95": 491634688.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 494415872.0,
      "p50": 494415872.0,
      "p95": 494415872.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.6823,
      "p50": 0.6195,
      "p95": 0.67865,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 403685376.0,
      "p50": 403644416.0,
      "p95": 403685376.0,
      "samples": 11
    }
  },
  "repeat-2--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 490668032.0,
      "p50": 490426368.0,
      "p95": 490640179.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 494415872.0,
      "p50": 494415872.0,
      "p95": 494415872.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.8142,
      "p50": 1.654,
      "p95": 1.7916,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 406425600.0,
      "p50": 406425600.0,
      "p95": 406425600.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 402698240.0,
      "p50": 402694144.0,
      "p95": 402698240.0,
      "samples": 5
    }
  },
  "repeat-2--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 311394304.0,
      "p50": 311271424.0,
      "p95": 311382016.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 330035200.0,
      "p50": 330035200.0,
      "p95": 330035200.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 259932160.0,
      "p50": 259932160.0,
      "p95": 259932160.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 259932160.0,
      "p50": 259932160.0,
      "p95": 259932160.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 310325248.0,
      "p50": 298319872.0,
      "p95": 309124710.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 310329344.0,
      "p50": 304652288.0,
      "p95": 309761638.4,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 259096576.0,
      "p50": 247279616.0,
      "p95": 257914880.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 259096576.0,
      "p50": 247279616.0,
      "p95": 257914880.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 19910656.0,
      "p50": 19910656.0,
      "p95": 19910656.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12410880.0,
      "p50": 12410880.0,
      "p95": 12410880.0,
      "samples": 1
    }
  },
  "repeat-2--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19910656.0,
      "p50": 19910656.0,
      "p95": 19910656.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 26951680.0,
      "p50": 26951680.0,
      "p95": 26951680.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12410880.0,
      "p50": 12410880.0,
      "p95": 12410880.0,
      "samples": 1
    }
  },
  "repeat-3--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 494338048.0,
      "p50": 493916160.0,
      "p95": 494256128.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.8355,
      "p50": 0.7044,
      "p95": 0.8053,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 406024192.0,
      "p50": 406016000.0,
      "p95": 406024192.0,
      "samples": 11
    }
  },
  "repeat-3--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 494034944.0,
      "p50": 493672448.0,
      "p95": 493904076.8,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 1.02,
      "p50": 0.8692,
      "p95": 1.01199,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 405626880.0,
      "p50": 405626880.0,
      "p95": 405626880.0,
      "samples": 10
    }
  },
  "repeat-3--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 598507520.0,
      "p50": 598204416.0,
      "p95": 598470656.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 619687936.0,
      "p50": 619687936.0,
      "p95": 619687936.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 543989760.0,
      "p50": 543989760.0,
      "p95": 543989760.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 543989760.0,
      "p50": 543989760.0,
      "p95": 543989760.0,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 595480576.0,
      "p50": 584185856.0,
      "p95": 594370355.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 595496960.0,
      "p50": 585805824.0,
      "p95": 594424217.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 540717056.0,
      "p50": 529620992.0,
      "p95": 539625881.6,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 540717056.0,
      "p50": 529620992.0,
      "p95": 539625881.6,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 35237888.0,
      "p50": 35229696.0,
      "p95": 35237068.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.3048,
      "p50": 5.2055,
      "p95": 5.29487,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27267072.0,
      "p50": 27267072.0,
      "p95": 27267072.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27267072.0,
      "p50": 27267072.0,
      "p95": 27267072.0,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 35225600.0,
      "p50": 35225600.0,
      "p95": 35225600.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.4395,
      "p50": 5.3129,
      "p95": 5.426839999999999,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27357184.0,
      "p50": 27332608.0,
      "p95": 27354726.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27357184.0,
      "p50": 27332608.0,
      "p95": 27354726.4,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 494145536.0,
      "p50": 494047232.0,
      "p95": 494134886.4,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.2825,
      "p50": 0.9261,
      "p95": 1.2313999999999998,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 406073344.0,
      "p50": 406073344.0,
      "p95": 406073344.0,
      "samples": 5
    }
  },
  "repeat-3--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 494325760.0,
      "p50": 494071808.0,
      "p95": 494307737.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.656,
      "p50": 1.4391,
      "p95": 1.623,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 406065152.0,
      "p50": 406065152.0,
      "p95": 406065152.0,
      "samples": 5
    }
  },
  "repeat-3--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 624443392.0,
      "p50": 624316416.0,
      "p95": 624430694.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 647593984.0,
      "p50": 647593984.0,
      "p95": 647593984.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 568893440.0,
      "p50": 568893440.0,
      "p95": 568893440.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 568893440.0,
      "p50": 568893440.0,
      "p95": 568893440.0,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 623194112.0,
      "p50": 611934208.0,
      "p95": 622068121.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 623230976.0,
      "p50": 619687936.0,
      "p95": 622876672.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 566849536.0,
      "p50": 555507712.0,
      "p95": 565715353.6,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 566849536.0,
      "p50": 555507712.0,
      "p95": 565715353.6,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 33718272.0,
      "p50": 33718272.0,
      "p95": 33718272.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 19.1355,
      "p50": 19.1355,
      "p95": 19.1355,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25837568.0,
      "p50": 25837568.0,
      "p95": 25837568.0,
      "samples": 1
    }
  },
  "repeat-3--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 33529856.0,
      "p50": 33529856.0,
      "p95": 33529856.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25452544.0,
      "p50": 25452544.0,
      "p95": 25452544.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 494137344.0,
      "p50": 493981696.0,
      "p95": 494130380.8,
      "samples": 18
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 18
    },
    "container_cpu_percent": {
      "max": 1.5232,
      "p50": 1.48475,
      "p95": 1.5144449999999998,
      "samples": 18
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 18
    },
    "vmrss_bytes": {
      "max": 406056960.0,
      "p50": 405786624.0,
      "p95": 406056960.0,
      "samples": 18
    }
  },
  "repeat-3--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 494641152.0,
      "p50": 494256128.0,
      "p95": 494500249.6,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 1.8847,
      "p50": 1.6632,
      "p95": 1.7598999999999998,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 406204416.0,
      "p50": 406126592.0,
      "p95": 406201139.2,
      "samples": 17
    }
  },
  "repeat-3--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 639025152.0,
      "p50": 638885888.0,
      "p95": 639017779.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 656793600.0,
      "p50": 656793600.0,
      "p95": 656793600.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 584896512.0,
      "p50": 584896512.0,
      "p95": 584896512.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 584896512.0,
      "p50": 584896512.0,
      "p95": 584896512.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 634572800.0,
      "p50": 627867648.0,
      "p95": 633731481.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 647704576.0,
      "p50": 647704576.0,
      "p95": 647704576.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 580681728.0,
      "p50": 573825024.0,
      "p95": 579806003.2,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 580681728.0,
      "p50": 573825024.0,
      "p95": 579806003.2,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22372352.0,
      "p50": 22372352.0,
      "p95": 22372352.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.1894,
      "p50": 3.1571,
      "p95": 3.1858400000000002,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14663680.0,
      "p50": 14663680.0,
      "p95": 14663680.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 34209792.0,
      "p50": 34209792.0,
      "p95": 34209792.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.2702,
      "p50": 3.1011,
      "p95": 3.2368,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26722304.0,
      "p50": 26722304.0,
      "p95": 26722304.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 493039616.0,
      "p50": 492515328.0,
      "p95": 492857344.0,
      "samples": 83
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 83
    },
    "container_cpu_percent": {
      "max": 0.4168,
      "p50": 0.1358,
      "p95": 0.26261999999999985,
      "samples": 83
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 83
    },
    "vmrss_bytes": {
      "max": 405012480.0,
      "p50": 404783104.0,
      "p95": 405012480.0,
      "samples": 83
    }
  },
  "repeat-3--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 493416448.0,
      "p50": 492437504.0,
      "p95": 493068288.0,
      "samples": 72
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 72
    },
    "container_cpu_percent": {
      "max": 0.5533,
      "p50": 0.14250000000000002,
      "p95": 0.40737000000000023,
      "samples": 72
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 72
    },
    "vmrss_bytes": {
      "max": 405360640.0,
      "p50": 404869120.0,
      "p95": 405229568.0,
      "samples": 72
    }
  },
  "repeat-3--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 538882048.0,
      "p50": 538759168.0,
      "p95": 538867916.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 557096960.0,
      "p50": 557096960.0,
      "p95": 557096960.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 485646336.0,
      "p50": 485646336.0,
      "p95": 485646336.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 485646336.0,
      "p50": 485646336.0,
      "p95": 485646336.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 535789568.0,
      "p50": 524781568.0,
      "p95": 534681190.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 535793664.0,
      "p50": 530731008.0,
      "p95": 535034265.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 482467840.0,
      "p50": 471644160.0,
      "p95": 481390182.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 482467840.0,
      "p50": 471644160.0,
      "p95": 481390182.4,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22540288.0,
      "p50": 22540288.0,
      "p95": 22540288.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.7036,
      "p50": 5.32,
      "p95": 5.66524,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 14790656.0,
      "p50": 14790656.0,
      "p95": 14790656.0,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 23031808.0,
      "p50": 23031808.0,
      "p95": 23031808.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 6.1179,
      "p50": 5.3878,
      "p95": 6.04489,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15491072.0,
      "p50": 15474688.0,
      "p95": 15489433.6,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 493645824.0,
      "p50": 493068288.0,
      "p95": 493483622.4,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 0.6913,
      "p50": 0.49185,
      "p95": 0.6633049999999999,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 405327872.0,
      "p50": 405327872.0,
      "p95": 405327872.0,
      "samples": 12
    }
  },
  "repeat-3--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 492748800.0,
      "p50": 492744704.0,
      "p95": 492748800.0,
      "samples": 6
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 6
    },
    "container_cpu_percent": {
      "max": 1.8488,
      "p50": 1.3226499999999999,
      "p95": 1.82395,
      "samples": 6
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 6
    },
    "vmrss_bytes": {
      "max": 405016576.0,
      "p50": 405010432.0,
      "p95": 405016576.0,
      "samples": 6
    }
  },
  "repeat-3--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 564350976.0,
      "p50": 564142080.0,
      "p95": 564330086.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 582889472.0,
      "p50": 582889472.0,
      "p95": 582889472.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 510443520.0,
      "p50": 510443520.0,
      "p95": 510443520.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 510443520.0,
      "p50": 510443520.0,
      "p95": 510443520.0,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 562507776.0,
      "p50": 550838272.0,
      "p95": 561340825.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 562540544.0,
      "p50": 557690880.0,
      "p95": 562055577.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 509124608.0,
      "p50": 497545216.0,
      "p95": 507966668.8,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 509124608.0,
      "p50": 497545216.0,
      "p95": 507966668.8,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 22257664.0,
      "p50": 22257664.0,
      "p95": 22257664.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14757888.0,
      "p50": 14757888.0,
      "p95": 14757888.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 22269952.0,
      "p50": 22269952.0,
      "p95": 22269952.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14770176.0,
      "p50": 14770176.0,
      "p95": 14770176.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 494174208.0,
      "p50": 493537280.0,
      "p95": 494038016.0,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 1.0249,
      "p50": 0.83595,
      "p95": 0.9851749999999999,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 405819392.0,
      "p50": 405817344.0,
      "p95": 405819392.0,
      "samples": 8
    }
  },
  "repeat-3--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 494235648.0,
      "p50": 493729792.0,
      "p95": 494146764.8,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 0.9689,
      "p50": 0.84475,
      "p95": 0.9445049999999999,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 405811200.0,
      "p50": 405807104.0,
      "p95": 405811200.0,
      "samples": 8
    }
  },
  "repeat-3--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 668069888.0,
      "p50": 667920384.0,
      "p95": 668050227.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 691556352.0,
      "p50": 691556352.0,
      "p95": 691556352.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 612007936.0,
      "p50": 612007936.0,
      "p95": 612007936.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 612007936.0,
      "p50": 612007936.0,
      "p95": 612007936.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 664723456.0,
      "p50": 653680640.0,
      "p95": 663634124.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 664772608.0,
      "p50": 657141760.0,
      "p95": 663675904.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 608784384.0,
      "p50": 597587968.0,
      "p95": 607665561.6,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 608784384.0,
      "p50": 597587968.0,
      "p95": 607665561.6,
      "samples": 4
    }
  },
  "repeat-3--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 24068096.0,
      "p50": 24059904.0,
      "p95": 24067276.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.4951,
      "p50": 5.4378,
      "p95": 5.48937,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15810560.0,
      "p50": 15802368.0,
      "p95": 15809740.8,
      "samples": 3
    }
  },
  "repeat-3--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 23826432.0,
      "p50": 23822336.0,
      "p95": 23826022.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5201,
      "p50": 5.4373,
      "p95": 5.51182,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15634432.0,
      "p50": 15564800.0,
      "p95": 15627468.8,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 492609536.0,
      "p50": 492148736.0,
      "p95": 492518809.6,
      "samples": 82
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 82
    },
    "container_cpu_percent": {
      "max": 0.4543,
      "p50": 0.15150000000000002,
      "p95": 0.22942,
      "samples": 82
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 82
    },
    "vmrss_bytes": {
      "max": 404533248.0,
      "p50": 404451328.0,
      "p95": 404529152.0,
      "samples": 82
    }
  },
  "repeat-3--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 493117440.0,
      "p50": 492601344.0,
      "p95": 493031424.0,
      "samples": 61
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497471488.0,
      "p95": 497512448.0,
      "samples": 61
    },
    "container_cpu_percent": {
      "max": 0.9029,
      "p50": 0.1819,
      "p95": 0.5096,
      "samples": 61
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408653824.0,
      "p95": 408735744.0,
      "samples": 61
    },
    "vmrss_bytes": {
      "max": 405131264.0,
      "p50": 404766720.0,
      "p95": 405131264.0,
      "samples": 61
    }
  },
  "repeat-3--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 486858752.0,
      "p50": 486721536.0,
      "p95": 486846464.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 505188352.0,
      "p50": 505188352.0,
      "p95": 505188352.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 433975296.0,
      "p50": 433975296.0,
      "p95": 433975296.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 433975296.0,
      "p50": 433975296.0,
      "p95": 433975296.0,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 483913728.0,
      "p50": 473024512.0,
      "p95": 482805964.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 490962944.0,
      "p50": 490962944.0,
      "p95": 490962944.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 430919680.0,
      "p50": 420188160.0,
      "p95": 429842636.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 430919680.0,
      "p50": 420188160.0,
      "p95": 429842636.8,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 20144128.0,
      "p50": 20140032.0,
      "p95": 20143718.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.4284,
      "p50": 5.4088,
      "p95": 5.4264399999999995,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12394496.0,
      "p50": 12394496.0,
      "p95": 12394496.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 23244800.0,
      "p50": 23212032.0,
      "p95": 23241523.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5942,
      "p50": 5.5608,
      "p95": 5.59086,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15503360.0,
      "p50": 15503360.0,
      "p95": 15503360.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 493137920.0,
      "p50": 492896256.0,
      "p95": 493131776.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.6248,
      "p50": 0.4922,
      "p95": 0.60005,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 404901888.0,
      "p50": 404897792.0,
      "p95": 404901888.0,
      "samples": 11
    }
  },
  "repeat-3--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 492441600.0,
      "p50": 492376064.0,
      "p95": 492434227.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 497512448.0,
      "p50": 497512448.0,
      "p95": 497512448.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.8145,
      "p50": 1.7172,
      "p95": 1.79772,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 408735744.0,
      "p50": 408735744.0,
      "p95": 408735744.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 404615168.0,
      "p50": 404615168.0,
      "p95": 404615168.0,
      "samples": 5
    }
  },
  "repeat-3--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 511950848.0,
      "p50": 511873024.0,
      "p95": 511943065.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 530620416.0,
      "p50": 530620416.0,
      "p95": 530620416.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 458780672.0,
      "p50": 458780672.0,
      "p95": 458780672.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 458780672.0,
      "p50": 458780672.0,
      "p95": 458780672.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 510836736.0,
      "p50": 498896896.0,
      "p95": 509642752.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 511111168.0,
      "p50": 505364480.0,
      "p95": 510536499.2,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 457945088.0,
      "p50": 446119936.0,
      "p95": 456762572.8,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 457945088.0,
      "p50": 446119936.0,
      "p95": 456762572.8,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 19873792.0,
      "p50": 19873792.0,
      "p95": 19873792.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 7.2956,
      "p50": 7.2956,
      "p95": 7.2956,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12374016.0,
      "p50": 12374016.0,
      "p95": 12374016.0,
      "samples": 1
    }
  },
  "repeat-3--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19873792.0,
      "p50": 19873792.0,
      "p95": 19873792.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 35430400.0,
      "p50": 35430400.0,
      "p95": 35430400.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 10.0653,
      "p50": 10.0653,
      "p95": 10.0653,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27222016.0,
      "p50": 27222016.0,
      "p95": 27222016.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12374016.0,
      "p50": 12374016.0,
      "p95": 12374016.0,
      "samples": 1
    }
  }
}
~~~

## Artifact index

| Path | Bytes | SHA-256 |
|---|---:|---|
| hardware-validation.txt | 1122 | 17330cfe1a819f1c9c1381d3e9f0308de6c31b460a58058391656270ca6d7ac1 |
| hydra.log | 16 | 6489d6d7a33c5d40e18fc61eeb6c34c341279ee61816394dde5189aa4ad8fae5 |
| hydra.pid | 5 | 1c24d28a82ddea58a48c9aa2a33fba57536bc784243e2969b32ce85a0fc5ce1a |
| irq-baseline.tsv | 33 | b684ede81a35bb4b5d7f5b23cef27bd64c93d54c14cf3f166634fcf966c371b2 |
| metadata/docker-warnings.txt | 186 | ba431352b1954a86c23115052875b8a5d045c4062a9d512bdf510acc7511e201 |
| metadata/hazelcast.container-id | 65 | ef6f916b128bd1386415ad416bb68c719c0afd0cd9341e1003b9f64a29b574bb |
| metadata/hazelcast.inspect.json | 7674 | 87a29474fe427756174613b9ca0bdf2dd9efb25bfd52fa60fa6b69e4258f1c49 |
| metadata/redis.container-id | 65 | 8299873e97471565d70c520ef0f80240e71485c06eaa7976372795baa293aada |
| metadata/redis.inspect.json | 8668 | ac1e68fec0886dae9d5e52d961ca2986a24d8838a76ffb27eac6bf6794008754 |
| raw/repeat-1--p1024-c50-p1--hazelcast--get.log | 184 | 5189a5476863440cc767ec75d9685d703c61fbe24c9eba5b1a1ff7dd229f895d |
| raw/repeat-1--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hazelcast--set.log | 186 | 31f10cb81af43130517d0e49d55552a8a3c91cfddcdd8bd585c522d59a8dd217 |
| raw/repeat-1--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hydra--get.log | 2044 | 84a660e585e76be5dc0e333f9685d3c897ec89806e832d7e8725ef7a7c86789a |
| raw/repeat-1--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hydra--set.log | 2044 | 3e59960cab08e44ec89d377567ea0a263a7e0772e1844a752a22dc712b3b886d |
| raw/repeat-1--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--redis--get.log | 1457 | fb48ea7a068040f329f6fa4aafe772098d000d226dee8b51087e1f5e08141cf8 |
| raw/repeat-1--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--redis--set.log | 1457 | b41b3dee44da8d3e29e776e39bdb2539f7ad5dc002925cdd51207c864e75c421 |
| raw/repeat-1--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hazelcast--get.log | 185 | cfbef312a757f6b814ef9d7f417f7626eb7ec71abd141c690df85219d8a4d37c |
| raw/repeat-1--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hazelcast--set.log | 187 | be1a9aea89d7d61307e21845f5d1f162c28fa1a2d01d1bc75f59e4785bc08032 |
| raw/repeat-1--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hydra--get.log | 1395 | 9d53676f5e0526ff70f55bb29e0f633a5a37bba64517431c16f99f4c5c9b1be6 |
| raw/repeat-1--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hydra--set.log | 1393 | fee9961f7a0faefba8fc6d29c57878cb9ee02dd05498ff46f3ab34839187f7c6 |
| raw/repeat-1--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--redis--get.log | 367 | 144bd2f780195b442197078cd36b63c8e1940f42f4e37cf8cff9fe3f17fb96c7 |
| raw/repeat-1--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--redis--set.log | 367 | e06466d62c9ad267367ca3b96897eb48f6b8114999adb5e79b7e74ef89d5218d |
| raw/repeat-1--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hazelcast--get.log | 183 | c988c712a9a5e99c61f7c1b832d4d5b3725f6223e4908762c959a593cf62776c |
| raw/repeat-1--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hazelcast--set.log | 183 | 51ae291c3d046dd176f09871e6e978e0da7c209281ef1d37dcd2964634050044 |
| raw/repeat-1--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hydra--get.log | 2874 | 173fb7ad0a1c1d342fe1f89d10a0724050be815b7de9b16a2c0384bfb35b4d31 |
| raw/repeat-1--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hydra--set.log | 2874 | 463d414758f4ac5d8f5db607fd0fe0c91ae3ae0cd7c919bb6c39bcc1f24aa978 |
| raw/repeat-1--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--redis--get.log | 2829 | cb7063ce017bc532cb0612d638582a2b3b0abff87f166c3358acdc6ad3eac589 |
| raw/repeat-1--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--redis--set.log | 2837 | 609ad3516508569994c7dbd2edf850c6d39e2b22b812121c991992112c7cbb85 |
| raw/repeat-1--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hazelcast--get.log | 185 | a0a58e5fae9cae5c9270a0c62652bec15106945b524364654ca471d5ace99dab |
| raw/repeat-1--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hazelcast--set.log | 185 | 532abf8abfc4bb988c313a6d44932b752fd6e50c8863e36b28d6c3b9c62800d4 |
| raw/repeat-1--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hydra--get.log | 2043 | c6bd52931728835b408372f9fec7a293d0145e609ec40d6b7b6c386957c7a322 |
| raw/repeat-1--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hydra--set.log | 2043 | 43b0786e86733d47fe46d30c91c5d2042ed57af335a381f5db1ee2091f546f56 |
| raw/repeat-1--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--redis--get.log | 1458 | a0f6fc284dc9c435c0e3448fa95e9427c454ec53a42203d6c57431b17b04a738 |
| raw/repeat-1--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--redis--set.log | 1458 | 3ee27a914d9c7a8afa1c9c81bc95b4b3e70d932bf40162ed6aea13ae3aa4399e |
| raw/repeat-1--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hazelcast--get.log | 186 | c9789c20d44d87d4b2474c3cf7480339ee52066be5ebb3498a29a89304aa397d |
| raw/repeat-1--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hazelcast--set.log | 185 | 1daf5d80ac0a7782f72cf690cf7dca4b938cfee83229fd1805dc3db7178abb06 |
| raw/repeat-1--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hydra--get.log | 1359 | 529aee7175b7d4f1e887738b72ef3d39768d40c97777d609e06c0b12c8dd23db |
| raw/repeat-1--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hydra--set.log | 1359 | 668010dfed732a963fb91e6320da1b0e22358f4903cf1ee6b8a74c921dcf1b8b |
| raw/repeat-1--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--redis--get.log | 368 | cad5089151474daebbadc8d7ff62efc9a4768df4a3fd92fbdf42ae26b204c513 |
| raw/repeat-1--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--redis--set.log | 366 | afb0405aee757c9d72732c9a62f70102163021ce26001d1d650450a7ded737a4 |
| raw/repeat-1--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hazelcast--get.log | 186 | d737aaa68797507aa7a4f5cd917743f7d88ec19d1cd9f703ebccd04af96ba178 |
| raw/repeat-1--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hazelcast--set.log | 184 | a192edee8a6e6046c32fce3913aae558c9dd4c5f400a419598b084fe652bc290 |
| raw/repeat-1--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hydra--get.log | 2044 | f302e433a1af93152e83835a9458a8c9b115efbf5c64f016c422b067c9d1539f |
| raw/repeat-1--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hydra--set.log | 2044 | 892fac764006536afb33fa4b4c677ed59f2ecc003d53e7f64ce9cae04028685d |
| raw/repeat-1--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--redis--get.log | 1320 | 37d37810410c6f02d31b59f80284ef4fed761b57df5e31dfa7ef4492ea805ea6 |
| raw/repeat-1--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--redis--set.log | 1457 | 750bc3b2538d7e9cbfcfd554dfcdb47e16b9a6e81ae4e49a982c9d4cb742ce3c |
| raw/repeat-1--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hazelcast--get.log | 184 | 6a738cb4e9e8bfb6e29fc8c676d43d9aaff965bc5cf6bcc3351df9102a217bff |
| raw/repeat-1--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hazelcast--set.log | 185 | cdc044386565f8b6642feff9f55434fe466f4da361a7ac1248eb9f22f09c096b |
| raw/repeat-1--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hydra--get.log | 2042 | e888159a69ce866b75b3cdf6b91510ecd19106a1f38a9f6b76cbc0b93469259c |
| raw/repeat-1--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hydra--set.log | 2044 | 4abd8ad903bd085b5b835dc46f0d8dfee6a537b61d5ced170f364870a04739ba |
| raw/repeat-1--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--redis--get.log | 1320 | 6090d079e2c9e44e591e24a8e35db96e537a6769a5a1b2245ab853dc008d2f94 |
| raw/repeat-1--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--redis--set.log | 1455 | 8c2d85530850f51e4a233a554d0a9df830e7a96f519c3f0131dd12fb9c6e777c |
| raw/repeat-1--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hazelcast--get.log | 183 | 751371677c11b6d3ec31839e646508dcfd8eb17d6f955b75b9ad91ab64e4d885 |
| raw/repeat-1--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hazelcast--set.log | 184 | 65de4afd9893ded5d4a53587ad5b9d47cad041dc72565cd78359c00170fb8a43 |
| raw/repeat-1--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hydra--get.log | 1360 | 1c0a2c4db32dbe7515150f8c9d1e093a79fc662a18b22db3fe38ca9b78d4edcd |
| raw/repeat-1--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hydra--set.log | 1358 | ffbb7ab5aa53450b11fa73d6c33bd535bfb420b38fa088088cef912d16b40fea |
| raw/repeat-1--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--redis--get.log | 224 | c609c89976d6fa67d46d829c5775f060a798c8d114922e0a517d3c98ab6cb14f |
| raw/repeat-1--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--redis--set.log | 226 | dded9a20b721e363ea3c50094857202e8c7d770233cb77b9882121c92e0eba19 |
| raw/repeat-1--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hazelcast--get.log | 186 | d4b2cffb2d7173d46be2e3018dc7c1e0dfd66ce61836a62569c972b9784d8ad6 |
| raw/repeat-2--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hazelcast--set.log | 186 | 9192f7464891719a8cbec5c9af402183bf3c6b0b7405965d5d7513b6d22619f3 |
| raw/repeat-2--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hydra--get.log | 2044 | 7d9a09ef15f5914276c945b5bcda917943ef60f3e2bd7bd16e3884be0ddee656 |
| raw/repeat-2--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hydra--set.log | 2046 | 052dff97eaf07969f00293d0286465eae11ed1e185c2d3b896e9afe8947a843b |
| raw/repeat-2--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--redis--get.log | 1320 | 58dffafe01e7dd5c03399b0deefe9b7743dbcf165cd07209447471894a366f66 |
| raw/repeat-2--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--redis--set.log | 1457 | e6023a3a4197af513c38850e9f21deb6774ac16c5749f56cafc5386c4a94d196 |
| raw/repeat-2--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hazelcast--get.log | 187 | ea72fdfed1fa95bc4c9bb2dfa9cda0f493baacbac0865fc0190b6b68a7008f3f |
| raw/repeat-2--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hazelcast--set.log | 186 | a6f069390e44d004ba5f5bc6f550815b3592c2933b62e22a1a307ba9e7d499c9 |
| raw/repeat-2--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hydra--get.log | 1393 | f47f6b6ab485c52b319785e5c1dad87c7a0929b3e2d70ddf54d30e3cd9eb541e |
| raw/repeat-2--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hydra--set.log | 1393 | fc81934fc23feda7865cb50ec213785ab4ed5c1ad1aa3bb5c7674a45bf05d277 |
| raw/repeat-2--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--redis--get.log | 367 | bd705f0e9102d0a90d880deac753128e2eec80432f7764f57b717b2843436ab1 |
| raw/repeat-2--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--redis--set.log | 367 | f27420409be834ecab3ef91cfb375f346937c1bb479bebd61f2d086085d2a079 |
| raw/repeat-2--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hazelcast--get.log | 185 | 80bf1a145d2de28c0f713911f81172cb02ab15ffdbad3f137e117327bd4bdf37 |
| raw/repeat-2--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hazelcast--set.log | 183 | 38aeaf05fc10388810073a999bf2b7c61d093736fe54a8cc4ec43a812cf985fb |
| raw/repeat-2--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hydra--get.log | 2874 | 99a2ea29696b5a6740f82a44d48267e25e9328892e8e701426fb881a1891a2b5 |
| raw/repeat-2--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hydra--set.log | 2874 | 36b509ed6abfd05cccd89a88711c2eddaa253fd0366063f7fe1116cd5902b8c5 |
| raw/repeat-2--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--redis--get.log | 2563 | 389555e96331f1a995e722a0ae88815ebf8f09d0c2462f3a702feb36a7d10e8b |
| raw/repeat-2--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--redis--set.log | 2835 | 3b9065f6ed2c46102dbd9ff0e21e47b69dd80f91f03dfac93b13e3b508cba98e |
| raw/repeat-2--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hazelcast--get.log | 185 | 949cd9946ef2a5a4f51f4f9c780bcfc5ce5f403ecea921f559ab1747d71c5730 |
| raw/repeat-2--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hazelcast--set.log | 185 | 96ac21a2d683a5d143bcfb1d7e0be5b5315a345fdd0e850549c367f8f549d1e8 |
| raw/repeat-2--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hydra--get.log | 2045 | 16b220dfc999b824d67c27f0577f78171999d3acb32bca10858b13bd1550d477 |
| raw/repeat-2--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hydra--set.log | 2045 | bd0b5419b9602d5fc92bd9a1d371555696e19a35d4a96705f7d79d786f7fbca4 |
| raw/repeat-2--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--redis--get.log | 1456 | 256819524bfefada211833f54c2da7b94e1d18eec85a9f37708cc3d9de3c483a |
| raw/repeat-2--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--redis--set.log | 1319 | f58f1d5b5fc41b05f94ff3942eadd5f79b32fb20f8c6e7d29e359ff9b6f7eedd |
| raw/repeat-2--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hazelcast--get.log | 186 | 465ca983ca976c68687ca455c745c080e0a403690720b9d052e8e8ae8e785d1a |
| raw/repeat-2--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hazelcast--set.log | 185 | 120723e5925b7946440fd2931d6355ccad440f1f0df9eac8b6f88828c39fff72 |
| raw/repeat-2--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hydra--get.log | 1359 | c933a954c477ae5b6ad25e8189cec7c88280087042536f4bdf9117638b5b7b9f |
| raw/repeat-2--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hydra--set.log | 1359 | f4688c7960adccbc2560ff1ec785973857034c11a49a8a4db687f780cfdae9ef |
| raw/repeat-2--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--redis--get.log | 366 | f1f934b05241afbf3430c4f574a8ebb972f0954c8740b0a13dc4180a5527d2e9 |
| raw/repeat-2--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--redis--set.log | 368 | df8437f01e5ea49e798d4fb23d63f63b6bb10f05766cc4ffc095da187ae89c62 |
| raw/repeat-2--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hazelcast--get.log | 187 | 4685b120d71a67a3d77bb338a831bb36eb0826e42130c0c81bb89844973ce00e |
| raw/repeat-2--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hazelcast--set.log | 186 | 8832f3a023022e4a11379fb9e85dac4d2c8d13596bacdd98dee3305e7279549f |
| raw/repeat-2--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hydra--get.log | 2044 | 547f95d7be8bda57e4386e7128f0922f7083667bc3435626a8a35c0fae4b6010 |
| raw/repeat-2--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hydra--set.log | 2044 | 9d8b3363331404cadeb2ee8964b2620b0e3a2e4fda6edd597e781288dedb0413 |
| raw/repeat-2--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--redis--get.log | 1457 | 7701e04e115e2e0004a6d799271cea25d3b76fbbb99a45a565fc03fa07867191 |
| raw/repeat-2--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--redis--set.log | 1320 | d82f2b3bce4a40dec281b41eda17fe60ff1da5700d480f0da9799efad40ada30 |
| raw/repeat-2--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hazelcast--get.log | 184 | 2c6170d64f5b5259381a3bb16ede0fab9cdfc2d7e1c53968d05d53f8c4674b0d |
| raw/repeat-2--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hazelcast--set.log | 183 | fba48bd13af00bc5c9a008836fd9522c26751eafc3dd7f343216a1d7ab1b26ea |
| raw/repeat-2--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hydra--get.log | 2042 | f32b5b9dcc00e717bfaa4a7220c8be0a64735d0a94ef6d4674d6c3c7fabec279 |
| raw/repeat-2--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hydra--set.log | 2044 | d9344750e8e02180ade913ae2b461f5ad74d18f0ddcaefa3a79816a8293d340c |
| raw/repeat-2--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--redis--get.log | 1322 | 0d11f665d336be667b8ed69609502013c60a37edaf30f075effce9ef909b6150 |
| raw/repeat-2--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--redis--set.log | 1322 | 740ca8fa76f344644de73a97d5db2c2dafb1a17d23d4ab2d5eed9abe1074acfb |
| raw/repeat-2--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hazelcast--get.log | 185 | c6b0da6e2c6f29b8e874c77d3949b2d3a51a794548fe2ad12dfbac74a0a892a6 |
| raw/repeat-2--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hazelcast--set.log | 185 | c267deeea3f046c4ba1ba4112eeca90855328525a27c543a5a20a2d7fcbf0e79 |
| raw/repeat-2--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hydra--get.log | 1358 | 979268833f577b1e31156fcb5a717be60e47bbe423c3a7d0df8690bd281f5718 |
| raw/repeat-2--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hydra--set.log | 1358 | 02cbafc7ca1433d85d447b8555699ae81b3f9c60e4b49b335c14528ec224c175 |
| raw/repeat-2--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--redis--get.log | 226 | 7e7192af4413aab5f98366189086ed228f536dc64d37fa1be838742bc9fad33e |
| raw/repeat-2--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--redis--set.log | 226 | 4c646480244f41515c6a9814c5fce6ae2b0cabd404374610ffcd2291f142bdf8 |
| raw/repeat-2--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hazelcast--get.log | 186 | 94902c6173ad645ab99ce105c52305b6f6f11952febbcefd65c674c184e122ea |
| raw/repeat-3--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hazelcast--set.log | 186 | fa6ad46e5423e9116412dc10541abd43cb624bceffbf386292537a99f08f9436 |
| raw/repeat-3--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hydra--get.log | 2044 | b944af6828eda66270482c45242e4853192a20b6080afb06384a4ce775eadf00 |
| raw/repeat-3--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hydra--set.log | 2044 | 86bf013a47db066f2b47fbd5a9645cb8ab6176b487f1f9f52ef9a1d0729864a1 |
| raw/repeat-3--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--redis--get.log | 1322 | ae4c063fb2751dea5de80835f6710ccfc4e20dcaa2b842981229bb81b16d04fb |
| raw/repeat-3--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--redis--set.log | 1320 | 6074e21b68dd907fb19ff7ce8403fb89de21834fae789fa6452ed2e10e30fa3c |
| raw/repeat-3--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hazelcast--get.log | 187 | 36af57be6c1e38fe0a502e7884d27d8ffbec93f58692d939629401bb94e88ea4 |
| raw/repeat-3--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hazelcast--set.log | 186 | e6dca00a19c939ab73e225052b6eb5512e4d7574ab14e75deddf1b393a78b825 |
| raw/repeat-3--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hydra--get.log | 1395 | e42946c6db5ec9816d4c0bf43c83af268cc0a9610f01cd0fe8114868d95f2016 |
| raw/repeat-3--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hydra--set.log | 1393 | 329e53999025ff50a898caacd0305543d984cb9b3b2eacb69f63b5bec4b09b08 |
| raw/repeat-3--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--redis--get.log | 367 | f3b28ba75eb935e1cc82903ca7b504013c067d75675ead88f7ddcc580d5c7dcc |
| raw/repeat-3--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--redis--set.log | 367 | 6bfcd825140215d87c578c259d906ae259b1cbfd0955fe5185d6f636f325cccf |
| raw/repeat-3--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hazelcast--get.log | 184 | 8bde41a4e2707bcb8c9e33dd1fbd26dfc5ce279338a1d748ef4c2640507e9c85 |
| raw/repeat-3--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hazelcast--set.log | 183 | 652d260db48b85244cf91cbd07c2b10bd2a413130195cb2b2588ebb895358bd9 |
| raw/repeat-3--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hydra--get.log | 2868 | 804d7b9d9d64bb882f50a2c77ec866498b856cff05ab444086754f0b7ce2a784 |
| raw/repeat-3--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hydra--set.log | 2868 | 73711a575d47acac8b3b1cbbb397b25f099e934065925ac4d7e2b56051324a66 |
| raw/repeat-3--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--redis--get.log | 2829 | 02f59d34cd8c56944cdcf3f7e6d2407d7c98c6d2cb22a7a867c27e90bfa4061a |
| raw/repeat-3--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--redis--set.log | 2837 | 69a32923523fa21b88c2a7bfa7ea2b20db65fa3927f3181085b25cd726edf196 |
| raw/repeat-3--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hazelcast--get.log | 185 | 6a347fa6c3bd3608f93c4eba842ebd5cfcc8e8d3439b5fe70ef6f2e9a4ba461e |
| raw/repeat-3--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hazelcast--set.log | 185 | 3249f217e1bc6148f9e5ae32e16b27359e84b3cb5bc9665b56ef74138bbcbf35 |
| raw/repeat-3--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hydra--get.log | 2045 | e902b391510513e830434f69eb30c76e58cb662268d705e3d05fe043547999ca |
| raw/repeat-3--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hydra--set.log | 2045 | 2b35d3f4120117bbae69b0efc86fada983fce3150c711808ec1fe73195f3f5db |
| raw/repeat-3--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--redis--get.log | 1321 | d0bbcdc8ec4f9dcfa6bc9efcb9a94676590d4041cec6654472fb5eec1e3356c5 |
| raw/repeat-3--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--redis--set.log | 1329 | 0068a2c339135b0afaa5fe217633f6d42d23682eb950ebc12f8de87b4445b352 |
| raw/repeat-3--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hazelcast--get.log | 186 | 82ca8894620f6b39cdd64f4b40f3ce00ff448abd781402eaa2bc0b0511928c90 |
| raw/repeat-3--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hazelcast--set.log | 186 | 726cd09c1f2b516740b05746aa3477cdc3151a34c47494ea3bc26e33e70acc2d |
| raw/repeat-3--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hydra--get.log | 1359 | 43b0aae11d7b725efb32fd1c5857f96ddcc7aa0bb3c025b4d5a02c1c10fac85b |
| raw/repeat-3--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hydra--set.log | 1359 | 929647ec631819306088d4b9f34cad8e812e322315484319aab444f7a9647ea3 |
| raw/repeat-3--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--redis--get.log | 225 | 5878e8b2d0e95e0d45e05db1e19f295caac89e6f88c27729c599badce614d6d2 |
| raw/repeat-3--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--redis--set.log | 227 | dd17a9773a569d000aa88e9794b82d47660c054c13350bcb479314e67ecf66da |
| raw/repeat-3--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hazelcast--get.log | 186 | 5d1b15d5ab197960abb50ab829fb9bd7e6a0064f741062c45af59880f47f396b |
| raw/repeat-3--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hazelcast--set.log | 186 | a9ba364d19d634f5087768f95e9cc4d025d9023f7af8eb5d1992a14077ca6fa2 |
| raw/repeat-3--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hydra--get.log | 2044 | 23148f1034c422e21b4c1458ebb2180dfffeef057a66e314b9548188448b05ac |
| raw/repeat-3--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hydra--set.log | 2044 | 41b2790cd98a2fad19e06ef19e08c037cf5c84bd7247b4ed258953157994ecb9 |
| raw/repeat-3--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--redis--get.log | 1457 | ed93e2faf0439b38663d86946e54ceaeb115cbae1ab573b7dcd7353e80d01645 |
| raw/repeat-3--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--redis--set.log | 1457 | ecaf30bc152a49de62c14635a98b926c4286d0be8dc82e0574ccf162d2fdd26f |
| raw/repeat-3--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hazelcast--get.log | 184 | 049fe0707debdaa8752c6ecd7318b9cd425528fb18541266310d2bcd37551f11 |
| raw/repeat-3--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hazelcast--set.log | 184 | 5b2240438d11f5e774a2f7ed9c282a2e85f89514d25b208ad49ec61354f9e2ee |
| raw/repeat-3--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hydra--get.log | 2042 | 1c182fec970da7c3bad1c8e7af88bfd8a3751890a8f50048c829a9d22327d485 |
| raw/repeat-3--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hydra--set.log | 2044 | d5e12f9e3bd2b93f141139a9847b72e0768b273d79650ac0cf9ac4686dadb1cd |
| raw/repeat-3--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--redis--get.log | 1320 | c73830e81e1da57d9c2664cb16220e56404cbaa79c98dd08e34be4ca0001c5ac |
| raw/repeat-3--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--redis--set.log | 1457 | 5e277492f214e41fa0050349137b9e0ae4da8903eeba2a241d8c0b5ffd81ef0e |
| raw/repeat-3--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hazelcast--get.log | 184 | 38e82b82ca187a3966da966589d47908e6a69e0230b1aa38cf2c0d67deb0f209 |
| raw/repeat-3--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hazelcast--set.log | 184 | be8bcf9ad6fc2ae7b90c29e0059a4b5f3dd7045f3d55bfbd18c5a9cff6245b08 |
| raw/repeat-3--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hydra--get.log | 1358 | 92e729dfc93e1a9a1b90facd9f80f49efed2b5056ada14c9d10cc8eb4169a7eb |
| raw/repeat-3--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hydra--set.log | 1358 | 76af5bbe09e73aa015250abf5698bb4712d5a2267a3e6039f0aeabeb086ebe3d |
| raw/repeat-3--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--redis--get.log | 226 | c9ae061a68cc03651d9eedd59ac074a72626e22ae8739b699c47d6ba420d18d6 |
| raw/repeat-3--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--redis--set.log | 226 | 94ed69a1d58baf377b58af40a3c838bbf6c113613f371d93ec31400dc3cc623f |
| raw/repeat-3--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| reproduction-command.txt | 469 | 5bbfdfe805ef86a15b67dde5e2d386a868839de3caf1f357b39bc38eb7a923fe |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.csv | 1393 | 7267c49039fc9d0c8eccf938cf22bf853d97728daaa82487f3848f7c4ee47764 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.jsonl | 5022 | f20b6c12d3c5a882aca82d022fee7101b5bc0546441d8a34ce36405769ab7910 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.metadata.json | 8028 | 37965354f5d508bb13d78e2255737fad87e62811cfc6944989263a058c7bd8b9 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.csv | 1491 | 86b4e734a79b003e3d7734dbe1c87659f9321d9515e922e7287f8e3680d7634d |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.jsonl | 5475 | 07645cda710a50309c1fc60e8cbe5b568e804324ad64687fc520e61dfe145730 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.metadata.json | 8027 | 82b1d6872e579b5f1d766e4aeea85884012b76ad618b41149ff99dbe3542bf0e |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.csv | 629 | b989f6155b385b52ea1103df80811531b608348117fe1ff075e380a724ffe750 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.jsonl | 1789 | 13eeacc22dd51fecd8d4b6c4215185e552eb1afea86bc6ae4f2962239b2dd0b4 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.csv | 631 | 52096095b6627798518688c0db63480414a79e59d825c31756abd43a0594983b |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.jsonl | 1791 | 6d095a63294282d16ccf663782ac78a608f57c4c428ad1dad3ae7c68d6cf4c87 |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p1024-c50-p1--redis--get.csv | 557 | 3b8b1ed478f913e91e1cf13f07953163e0a0f7719f2669543b10382994df4462 |
| telemetry/repeat-1--p1024-c50-p1--redis--get.jsonl | 1346 | 233d5075a85b261569a01fef09b3c4fe42b11ff9fbd61be001000689087f3e41 |
| telemetry/repeat-1--p1024-c50-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p1024-c50-p1--redis--set.csv | 557 | 8669f8bd030383b761b490f290facb3201965794a6aa67278edd27fe526640f7 |
| telemetry/repeat-1--p1024-c50-p1--redis--set.jsonl | 1346 | 878941180608b4eb0dacaa47de6e73ecbf55c303fc87b794345b13cb86c681ae |
| telemetry/repeat-1--p1024-c50-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.csv | 884 | 6009c982c987eba75c3d110084923d23e557bdba680c3649631b1051eff5d74b |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.jsonl | 2738 | 258a0e75e133b6417f5bb1ee491736f41e7237d1cdaef091461fbf4d99b93933 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.metadata.json | 8028 | 022bc3357fb274a8906f987d4a8b542eafdfa439d86504fe5c53a88b1e16f36f |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.csv | 781 | 7e8aca8d504e6e2ce5030c23d14f457ea094bd38766b584da187e21502a7e9a3 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.jsonl | 2280 | 2f7974a1a43ba51006434941e1df4f96a138002b271953d2a73d2748a646768f |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.metadata.json | 8028 | 37965354f5d508bb13d78e2255737fad87e62811cfc6944989263a058c7bd8b9 |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.csv | 543 | 80cc90be0e3c666a644442fe5818b8c5b6a79e6a9cc413d66da7bfb5db56d397 |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.jsonl | 1344 | 9ab03df0825485466deb0d2b094c42a88fa3330980db7eb73fe989943c75bb9b |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.csv | 542 | d64939e0978aa10b9bc442f0ebc10474e28f76f416799204ee3ceb319cda779a |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.jsonl | 1343 | fdf0f96a233ea78132c07c8248f3fc3ed8975e07bcf238593e9eeaed8a22788a |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p1024-c50-p10--redis--get.csv | 367 | d7eeab5914892e59b36c736159824289f34547185d81110eaec01d3a99a86a06 |
| telemetry/repeat-1--p1024-c50-p10--redis--get.jsonl | 446 | ba461b0fbf76a1b28ad024110e706439be435b4b61f282159d1fa6aa784a46d6 |
| telemetry/repeat-1--p1024-c50-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p1024-c50-p10--redis--set.csv | 371 | f3cce9882729d96c9fbf606ee2ca09bfcb29a7add6ab2e637628138487465b37 |
| telemetry/repeat-1--p1024-c50-p10--redis--set.jsonl | 450 | 833c3e459232d029180d33000fddf136b8aaf31da30afd9ddd737bb1fef16c3a |
| telemetry/repeat-1--p1024-c50-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.csv | 2103 | e43a5ce32036947c47720859d397993243685b658edeaec5ea01f84ab48a1a54 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.jsonl | 8217 | 4547441427eece783b36ce8d34b7df2e5a30ce065ff97eeb04b4b244042c0308 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.metadata.json | 8028 | 05f360e4805ac5eb3af0c492c1f7087cb3e8a5d783da0a8739e35a3fe91ccdbe |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.csv | 2002 | 61809542f925820210f32584b6890b25036efa42cc0a40aaacd6264d9bfae67d |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.jsonl | 7761 | 0c0450b40a5566f4c43e03d1b078c690604d1bb9514509852834cbf978ec9ffe |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.metadata.json | 8028 | 022bc3357fb274a8906f987d4a8b542eafdfa439d86504fe5c53a88b1e16f36f |
| telemetry/repeat-1--p256-c1-p1--hydra--get.csv | 717 | 95398236b117ab1ab5a1b9f8d75971febb5d65eebe782a075cf2c4dae068ad96 |
| telemetry/repeat-1--p256-c1-p1--hydra--get.jsonl | 2236 | 99b594c30fb2c16f5d358f139b4b37742dea37325d2c9c8e92ebf135cd867830 |
| telemetry/repeat-1--p256-c1-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c1-p1--hydra--set.csv | 719 | 1a3c88b217e1731c2faa78f0d68bc48d5c5a59a291ae47174c2f677c3523b9ed |
| telemetry/repeat-1--p256-c1-p1--hydra--set.jsonl | 2238 | 3b45a0802981bd7cb630208a9a57bc39d23e33f2ea061c0973c0a689a96a9e98 |
| telemetry/repeat-1--p256-c1-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c1-p1--redis--get.csv | 746 | 65f4f05a018bd7b90fd8ea5989699d074043340518cab63c6a797e04bc3a9c7c |
| telemetry/repeat-1--p256-c1-p1--redis--get.jsonl | 2245 | d15376348e25bc97420c7491166b4f0bde6ae963fb6014c33fb4d141ce28aace |
| telemetry/repeat-1--p256-c1-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c1-p1--redis--set.csv | 744 | 1aa462bc554db8c190ea864b3f46e452625bf8f738ed8e31d1b2753dcec945d3 |
| telemetry/repeat-1--p256-c1-p1--redis--set.jsonl | 2243 | f4c8bd6b4170e1c77043ad027fe92c2cdc639813f378b235eef166a1847c6c7b |
| telemetry/repeat-1--p256-c1-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.csv | 8415 | e3c4fa455d5f362e5236a5aeda6075e445fb60324ba941aafdbfb474ba9ead0c |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.jsonl | 36539 | bd2185569e20879281bd21bfc769ba8f3770539435a0804d328f404c32ab3573 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.metadata.json | 8026 | 31a6816e47096518ff34ba72d6969f23a1b2a3286ecdfe222563d44fc85b16ff |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.csv | 5455 | 410fa146c4e7274e8b80684b5dbed1305b093505ceed92266e7f2a6f803581ef |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.jsonl | 23284 | b8a3523b452cac64e002c842797c0e7537949ce826f4e6052f210608216b39d5 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.metadata.json | 8027 | 05c7f1048d8c1fc38dfca2d02765ad3d5cdfdaa6ce6df2dc4df5e99374962f1d |
| telemetry/repeat-1--p256-c10-p1--hydra--get.csv | 619 | d36629a9ed144a26b079388bb7272cc3284bb3d8589020c936086e943b0d5fa5 |
| telemetry/repeat-1--p256-c10-p1--hydra--get.jsonl | 1779 | 29e445a9e020f21398d41efab53274be14ddfcafe9bfb735bcc6bf6a69dc7ea7 |
| telemetry/repeat-1--p256-c10-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c10-p1--hydra--set.csv | 619 | 16171ae27a8b1f001f0b46dccd75ec5c3d975139c4a400cd80dddb8f53e71873 |
| telemetry/repeat-1--p256-c10-p1--hydra--set.jsonl | 1779 | 01f6cd42e969a33d9dfb17701454e7309cbf2f3eb55ccc899287c8fb9b6ad545 |
| telemetry/repeat-1--p256-c10-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c10-p1--redis--get.csv | 553 | 2f6c37b7abb57da38b0cd52f6b94a562464b300c3b57b92e34beb1a8fc1cdf98 |
| telemetry/repeat-1--p256-c10-p1--redis--get.jsonl | 1342 | 836661ec275db18e489d6578ef75afdf50e26586de6ecee42bc5730b03c6ab91 |
| telemetry/repeat-1--p256-c10-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c10-p1--redis--set.csv | 553 | 79113920771d8e24b8e0aad9cbe8089699183c8f063337a4cf7eae32d40b61a4 |
| telemetry/repeat-1--p256-c10-p1--redis--set.jsonl | 1342 | 5c4cf36d6a860d0133ecfd0f401b875d3c68ebf450250512d1445394f1b597c6 |
| telemetry/repeat-1--p256-c10-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.csv | 1389 | b263aea7fc02be3eae48a215126ab6e1da378c5683167d449132081425dea726 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.jsonl | 5018 | d38fb679d76bda1069b05d7c91808a10d8dbde2e758f61e7c16279e103028100 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.metadata.json | 8027 | 82b1d6872e579b5f1d766e4aeea85884012b76ad618b41149ff99dbe3542bf0e |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.csv | 781 | 3cfe8976168bd7cc28692f9e592a6fd2a6ac4c80d46c7585d412f9e9a5a7ab1a |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.jsonl | 2280 | d73257e4af06fb4e0a19972cb8493ed7ed43809f64256bbca12177d93b89e4ec |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.metadata.json | 8027 | 42dd3df33748bd6e1fe2719441f2c52e320232109a925d0baedd6133b8d11f3a |
| telemetry/repeat-1--p256-c10-p10--hydra--get.csv | 542 | 210912bdc3adf8adc052b7be65510c95954f4debcca0d6b2455e9ab1fcc85a1b |
| telemetry/repeat-1--p256-c10-p10--hydra--get.jsonl | 1343 | 1a31528c21b0a740473db552df94027588789b9f906656c212817b6f642c949c |
| telemetry/repeat-1--p256-c10-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c10-p10--hydra--set.csv | 537 | 4f47a3febc5218e4807295a0a8a6cffe2793a08964d9a2c7e78b5805651ea5b0 |
| telemetry/repeat-1--p256-c10-p10--hydra--set.jsonl | 1338 | fad2c4aefd80b9569ffa166e1439f6910e5446f386676aa69a328be61d2c6e28 |
| telemetry/repeat-1--p256-c10-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c10-p10--redis--get.csv | 369 | 67d85804694ea203e95a8443a04dce8b485de9e2a3a154e5d15243b700f48aa5 |
| telemetry/repeat-1--p256-c10-p10--redis--get.jsonl | 448 | f536a796852db6bfee6e9de7775cb8521748a96ec3e363e8300676d94f1e0bd2 |
| telemetry/repeat-1--p256-c10-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c10-p10--redis--set.csv | 370 | 54d2df41a25162f6a350e8113d17b310b1df4e62d73f195d7ba483c4730c788f |
| telemetry/repeat-1--p256-c10-p10--redis--set.jsonl | 449 | 2a56bfdb7ec96fc854f84463830f6a49e02dff0d3cf1a4582bcb7418d02428f7 |
| telemetry/repeat-1--p256-c10-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.csv | 1087 | acb3e8f8da7117d7b5db7191efc2dbe67ac1979493e214385de085282eef235b |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.jsonl | 3651 | 2e97e7dd88aa12e9cff4914dfbc449a2f87572670f3ba673d96f17e139b2d879 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.metadata.json | 8028 | e31290075c0a57fea540299873b83ed9cb37f3f4049be6c7c2208175eaae99fe |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.csv | 1086 | 10523b63123f588ec723a7db4407f1345156956c5585dd9c54fe242175d4ee7d |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.jsonl | 3650 | 6eb5c87f8edd7c57eb47722c9bf9a07a8f290ff85c93c955a4f06fc77c003da0 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.metadata.json | 8028 | 11aec249e5d829df3f8828caa9b09bb2eafd582b5ff6d9cbc3f5fd5ae2742d23 |
| telemetry/repeat-1--p256-c100-p1--hydra--get.csv | 631 | 5e4eda1344a51e633f6208b0ae8537bd52eccec34f11054f67915d7e51da25bd |
| telemetry/repeat-1--p256-c100-p1--hydra--get.jsonl | 1791 | 363137609d3f57c3a7a3bffe0bc032f446223f033a08fe3f19cda1ccf8ba2687 |
| telemetry/repeat-1--p256-c100-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c100-p1--hydra--set.csv | 631 | bb12204ee2df09e144a20c4bca65f1a87967aa4eb11496e400eca8202e75f69c |
| telemetry/repeat-1--p256-c100-p1--hydra--set.jsonl | 1791 | 709dbc77db6b705c288cae35a24cd6841e29a71fe4a32cf0c14791b6cc4e2647 |
| telemetry/repeat-1--p256-c100-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p256-c100-p1--redis--get.csv | 555 | f4527040d369b0fd02c6ff6d74468f721646b7b53326b2b609896bebec34e52e |
| telemetry/repeat-1--p256-c100-p1--redis--get.jsonl | 1344 | 5c747a2e676702163906759c1c80613c4fc1a45182e0d5e5ebd671e8c68595b8 |
| telemetry/repeat-1--p256-c100-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p256-c100-p1--redis--set.csv | 557 | 04f29864b1521d926deadd600b32d845cc1bc0b20f3853ed7889590d8940fe6b |
| telemetry/repeat-1--p256-c100-p1--redis--set.jsonl | 1346 | d17f6fabe30589e2b7a648676ee7ecdf0fc5ca2b568834aee9f06d9cae21d697 |
| telemetry/repeat-1--p256-c100-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.csv | 8414 | 47b7d6659254ccd4f5b99d0d71a6fe448fd26f32ce1069bdc2d5c67900964931 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.jsonl | 36538 | fe31e48a719cb07bfd3a47fbda082447be23aeb58684d90b95cac7220f1fa04e |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.metadata.json | 7113 | 684499eccfef85d4bd79484c9590a137dcb8ab0ce69e80be0ddce79950410e11 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.csv | 5759 | 2e2b4daefd857efc27699b5c0120e7291d64a2a9f15f856729ed8bf08aded58e |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.jsonl | 24653 | 97d8aa259d2874e282fcd1a8e9235a452b75db46b9d8481cabb6b1eb13c1397a |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.metadata.json | 6496 | eda3fa00a01013dc4bca978e857f4ed297a05e07ddde356dd34beb3aab4be394 |
| telemetry/repeat-1--p64-c10-p1--hydra--get.csv | 615 | a92fec6a0e5b60374f1052c3f5254a4743335da21e85638d6b928ef674531786 |
| telemetry/repeat-1--p64-c10-p1--hydra--get.jsonl | 1775 | b02e57ee3a2a7d00dff0a4f3f5ad60ea2a2f9934f25ff0925c7ff8e40b442c9a |
| telemetry/repeat-1--p64-c10-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p64-c10-p1--hydra--set.csv | 606 | 3ca94390a2f06466895ba155d28ffec51af51d72511cff197f4b3df31fa56870 |
| telemetry/repeat-1--p64-c10-p1--hydra--set.jsonl | 1766 | d11cf1050c67200d98975a2dbf164ea51b439f67b6ebfc82c14e99fcab9ca1aa |
| telemetry/repeat-1--p64-c10-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p64-c10-p1--redis--get.csv | 554 | 608d19c3e5d4a88a430fc44e8ac166a1cfadb111cc1a5c3926f3a4393db367d2 |
| telemetry/repeat-1--p64-c10-p1--redis--get.jsonl | 1343 | 68a38e0a8021f053f735047b47fe3af0a6a4c90241f5663289a5e21b722c0b59 |
| telemetry/repeat-1--p64-c10-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p64-c10-p1--redis--set.csv | 551 | ef5157052837f187f61d594cbe044d6ffe809e12a31eb8972283e77b2825741c |
| telemetry/repeat-1--p64-c10-p1--redis--set.jsonl | 1340 | 979f7567f2d38792b13966fe5e2ce15708a91a4919e0c9e8380b8080f305baf7 |
| telemetry/repeat-1--p64-c10-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.csv | 1489 | b6c5bac554261adb19740049ce063a06b1ecd017dfc2c916508ab621f994f1c7 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.jsonl | 5473 | 618fb0bb37f72fa9bb8f7b1267575f40ebd53e696018a96c14fbe5f63ffad842 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.metadata.json | 8027 | 8a4eb582810281ca76e32c110113359e499b7feab9cb2a9deeafea5103e6ec86 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.csv | 782 | 5aa38f3583bb77fc231a004085c7093ce43ffd50f16c502354f2b3049b1fe3cc |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.jsonl | 2281 | 3251c71851652259e8806cf1d91160906b502966f5632a47e6a30174a897a476 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.metadata.json | 8027 | 8a4eb582810281ca76e32c110113359e499b7feab9cb2a9deeafea5103e6ec86 |
| telemetry/repeat-1--p64-c10-p10--hydra--get.csv | 534 | 545a79d1afd2e01b2ca20d3604c0716fdd8ec905f06e06915b3d9adb29ff13c0 |
| telemetry/repeat-1--p64-c10-p10--hydra--get.jsonl | 1335 | f58c2927f3bf40a6d6ef6375c56140c3fd254cc1e2b2be35af61874aa26bbf46 |
| telemetry/repeat-1--p64-c10-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p64-c10-p10--hydra--set.csv | 531 | 2af7ab7278d40dad12185cbbdbdf73c2f7fc07c3d5ac94678f7fc9b80c487830 |
| telemetry/repeat-1--p64-c10-p10--hydra--set.jsonl | 1332 | 3dcf1238194ae65d5c121d5331e7434aced5650d3d74347d25903ababa73996c |
| telemetry/repeat-1--p64-c10-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-1--p64-c10-p10--redis--get.csv | 369 | c213d705f90896988693c891e980c3bf876ebe10be29ae9a6f415bdd7cf33da4 |
| telemetry/repeat-1--p64-c10-p10--redis--get.jsonl | 448 | 2d896a8217c209c9c2c5f8a74c0b0c9299b48abc66cc774305a391d1afca73c2 |
| telemetry/repeat-1--p64-c10-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-1--p64-c10-p10--redis--set.csv | 366 | cc94f42eba975eeb0dbb9552cb1f5557c0cdd34f57c4f4db4b81aa8e5daa6e29 |
| telemetry/repeat-1--p64-c10-p10--redis--set.jsonl | 445 | 6220d105dd8bd2d9238bfa4898b1b2437aef8475a874ca220ca4d1eaced53cd0 |
| telemetry/repeat-1--p64-c10-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.csv | 1288 | 181697e6d757e2866401349e0d43b6eea48be1d7936b3a59ab3b15f2c8043a59 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.jsonl | 4562 | 0053566f34167bd6a20bb4559e732f170cefd4ffc422d3a13e2720b1137b3fe3 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.metadata.json | 8026 | 405f702fd21cc5c3609dc34e12110ba3338ca096750171e0fc3c962632587245 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.csv | 1598 | d9ed824f23648c17825afc224e72bd8d410edf8bf7776d68c35a7062077f16c4 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.jsonl | 5937 | 7104669dbd87a37df4cf0cb28d80fd2d588d193a93cae251db0c8ccd1720038c |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.metadata.json | 8026 | 405f702fd21cc5c3609dc34e12110ba3338ca096750171e0fc3c962632587245 |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.csv | 630 | c0741c289fb36f289e5e0d98389a3e2cd9a839f0f460f98d87b303bb74f67c42 |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.jsonl | 1790 | a429952951f7c429e37a71afb80e652c88aedfd4ec5af1c022c2131e5f42aa8b |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.csv | 632 | a40b7456277173c18875d90a09b28d8d4b656a562292e4fe71b9524de52d4c70 |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.jsonl | 1792 | 23b0e3ef5b7d96c6ee978f73c8f6a98c39d2b86ee3dd5c768ba19515b1b35470 |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p1024-c50-p1--redis--get.csv | 558 | 26f77ce6631e73ce289e1b549bbd67eaf778abe7026197b4d4dc52528dcbde71 |
| telemetry/repeat-2--p1024-c50-p1--redis--get.jsonl | 1347 | 0f16f061fb4606ca83aaa19d09d659fd8f41662d1444ef18ba0f70717aef399b |
| telemetry/repeat-2--p1024-c50-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p1024-c50-p1--redis--set.csv | 558 | 6b18fd736623b61a06da953ee93d8cd6e9bfb8ac2c52573850f4e769e0c751a1 |
| telemetry/repeat-2--p1024-c50-p1--redis--set.jsonl | 1347 | 51638d47b7e206b52306eef3b02a742d8d58f9724f841228feef6f504333170b |
| telemetry/repeat-2--p1024-c50-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.csv | 783 | 5050efcd2666837d8b0de7a8c01f6bc5721246c9216f75c67dd2c4f518421523 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.jsonl | 2282 | 420d516d636341a5659646f0b979c2ad6b4ce65d89aec5596779dee00fb72c34 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.metadata.json | 8026 | a61016b9d5b308ca492adc7f7aa37a36d2c0903fd0668d813e15af8342eb144e |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.csv | 781 | ed194bd2416f002c0107162816bd4cea1a917893bc30fedd0bfc64bbab30bed9 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.jsonl | 2280 | 77cde74a77f0b8cd19aa2e5e32bbdc960146066c2e3d1094de6d1049fc145400 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.metadata.json | 8026 | a61016b9d5b308ca492adc7f7aa37a36d2c0903fd0668d813e15af8342eb144e |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.csv | 543 | e8d04129bded3ba06b65c84bb6579de2f5763d05347c4c431f5d11784c1896a8 |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.jsonl | 1344 | 9ad805020e86315391427bbb926bd68e4fcbd75c7f08f608cc298cefdf49ac3c |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.csv | 543 | d844801a5e40ee6c98abf0a4f9ede6b91ea675c08c473ff065b1cfc4e53ade10 |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.jsonl | 1344 | 02be057924ca10626a9a96dff1a4190f5c3f726d8ac7449c0fc58554d3639d31 |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p1024-c50-p10--redis--get.csv | 367 | d9de6b3fb7a641a122427a253aeeaad84f61537aacac950b2d305503e0842230 |
| telemetry/repeat-2--p1024-c50-p10--redis--get.jsonl | 446 | 23bbff5b5c5c2e2e2a9eef04a4e7316fc562ddffa18188bda09fecd0e8555877 |
| telemetry/repeat-2--p1024-c50-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p1024-c50-p10--redis--set.csv | 367 | f69338060fdcd767b31cb72e38f5551d6d143f4ffd94a68d567c9308f43e5c43 |
| telemetry/repeat-2--p1024-c50-p10--redis--set.jsonl | 446 | 0ec5bdb59f8487f681278c80790b5cb4cf0cf9b90a5fd7c95d9d91a515a2097d |
| telemetry/repeat-2--p1024-c50-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.csv | 2098 | 92a21d4b5b63cbb5ed3b004a79e5179dd4ccb3a0deb04701dd53ee01b4d29646 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.jsonl | 8212 | 8b2be116af5061b58e0a499caaba48c44900282a70f06d41d9558a625dd4ec2e |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.metadata.json | 8026 | cb2ca7e727c1ed48a911dd1470da3ba4842abf51ceda3ee7c0aaa7f8bbe69e1c |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.csv | 2002 | 8ef6953674a9b1d272f6b9ebd16ed57ec0a56d5100b1ca97b858cc8139aa37c7 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.jsonl | 7761 | cfd4251e419f097a43cf5f02c6d25e84e69a47ebed3747c1f4d2e8295894321d |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.metadata.json | 8026 | 73102ad980256535bd3ca760501048731b82989bce5ca0c0ca57815524ce6b0e |
| telemetry/repeat-2--p256-c1-p1--hydra--get.csv | 721 | c01ac35d9576d62fbe0d3e4cdbc1ea72ff751eb0513c26d4d8698e89d7a9c46a |
| telemetry/repeat-2--p256-c1-p1--hydra--get.jsonl | 2240 | cd75940126e0766e01f0781fc574109d2440fea5034560fbb42fb086c7a5e4f6 |
| telemetry/repeat-2--p256-c1-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c1-p1--hydra--set.csv | 720 | 068953ee6ad173e8d8b291a7c8bf430a3fe44f7dc1949be4eea4eab077dcfc77 |
| telemetry/repeat-2--p256-c1-p1--hydra--set.jsonl | 2239 | 74bf256a91f27e063235701e5f0e7887b083703be1ad5727ae9dfc648001c6fc |
| telemetry/repeat-2--p256-c1-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c1-p1--redis--get.csv | 744 | 824db43c511dd4f84ad9f04f7cf5128014bc509b9452ea134b4221bacaa0a875 |
| telemetry/repeat-2--p256-c1-p1--redis--get.jsonl | 2243 | 884ee45f4f9da177660de08eb649638459d5b8f696cde404c1132fe1d8f4b32a |
| telemetry/repeat-2--p256-c1-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c1-p1--redis--set.csv | 745 | 9dbec202d11c1a5a6e507c5c15fb48ba58539b576d2bb2a054973902c1fa320c |
| telemetry/repeat-2--p256-c1-p1--redis--set.jsonl | 2244 | 08c302a3ad6243699bba2d720206f3772b428970a568783c59a31750e17e8ed8 |
| telemetry/repeat-2--p256-c1-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.csv | 8806 | 5037a0af33e2a2b7e97128588c7c26a203f1306c588ab6297b58951b4853bf40 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.jsonl | 38350 | 3617c8f6085ea89ddd4420796b1fc16356d7e87e6d25ae096f4343bc43cd19e4 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.metadata.json | 8028 | 5a80bf311ccf827eb6cab04d9740ec04a2e48f66194e07c3f049e4f65bd69dda |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.csv | 7280 | 18637986c0c22dd25cd1ceff574617aba5672298cf2027805ee96eadb665acb2 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.jsonl | 31499 | b0deedd332eb1c9025f9819c52720178026455674b5cefa0e78dfb0c4e415855 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.metadata.json | 8028 | eba532d1efa03d29ee6ce52be64af3d41bd35049bbb7a3c224d3a9369cc3824b |
| telemetry/repeat-2--p256-c10-p1--hydra--get.csv | 630 | 7f656a71e4b15a0a6dbaa38cbff0e1a49bf9281292adf4e2e4e5e81fcd131cdd |
| telemetry/repeat-2--p256-c10-p1--hydra--get.jsonl | 1790 | 72dddb6ba45f1e2d9e87f84c8628884e831cbb4a26920f43bdb825b847dc4f77 |
| telemetry/repeat-2--p256-c10-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c10-p1--hydra--set.csv | 631 | 325a2e37f0832bd2d1cb31d19abfad2b46957058d61ba4b713bef20032a005cc |
| telemetry/repeat-2--p256-c10-p1--hydra--set.jsonl | 1791 | aaa56fb27d6bff2ea689c1ada77100ea614c2058346a0be28b86cb259dd9f82a |
| telemetry/repeat-2--p256-c10-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c10-p1--redis--get.csv | 557 | 22e7595df6a9bab91fca6c2899acb42d5da8a73601902fcbe5e6b2121fe933a4 |
| telemetry/repeat-2--p256-c10-p1--redis--get.jsonl | 1346 | 709e6aeb7138b6c5bfe637c5eed3c9a73b34d286bf5d28dd2cc297de443d5eb9 |
| telemetry/repeat-2--p256-c10-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c10-p1--redis--set.csv | 558 | f7fd9e95222aac0ddba01242d014f511114623b4810ba6487fe4220286cf1819 |
| telemetry/repeat-2--p256-c10-p1--redis--set.jsonl | 1347 | a2a01438cc542fc402f9ad79dd50b4b3557fa28cfc0a31aa6aaecdbd9cf871b5 |
| telemetry/repeat-2--p256-c10-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.csv | 1392 | 90598ba865bc311bff0fb16110f59a4419122a7624c66ff790f06316149c85b2 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.jsonl | 5021 | ac4cf2c90818db2f2cf5a37099afbddc5d12765fe55d2907e787b2a1165e1fe5 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.metadata.json | 8026 | 61ab382b64ec9c626745fe0608a8d626228e1e2768a305f20d1d6944704005a0 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.csv | 782 | 43d9db7f8a1f793c304c44acb2e6b8678c6640ed7cd98eecf7f7b1c4d5067d66 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.jsonl | 2281 | f129b19e458d6aa6af7db934d20250c7c1a78ba5e593cc9352c694f040d11436 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.metadata.json | 8026 | 61ab382b64ec9c626745fe0608a8d626228e1e2768a305f20d1d6944704005a0 |
| telemetry/repeat-2--p256-c10-p10--hydra--get.csv | 543 | dee01c7232f39be25cc3c0e5caac1bc94c9bd913f44ca89c91adb27bf72b4856 |
| telemetry/repeat-2--p256-c10-p10--hydra--get.jsonl | 1344 | e1477ff5ee405d5bb31b1e9df1b086668184180d0fc211c082364553b6b85660 |
| telemetry/repeat-2--p256-c10-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.csv | 543 | 7fe8b1051b89326f15318319f78d8b682a192957d084527db07bbfcaf936d583 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.jsonl | 1344 | 1518dca1c522632d282f588a2e27a92f6d33744666db0483477afd7a67aa57c1 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c10-p10--redis--get.csv | 370 | 080ea505589208b6d66ee8c605e5f80b84d8776d7f4c1f38f034ad4860cf4969 |
| telemetry/repeat-2--p256-c10-p10--redis--get.jsonl | 449 | 8e44bdde428c810e41903ce9ba88ba0b6d8c735eec2ae6f65f1d07d716fd02b4 |
| telemetry/repeat-2--p256-c10-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c10-p10--redis--set.csv | 366 | 0006099262d46930679cf114fd5ff3bccef777c324eb5bd15ec2efb06ed5adef |
| telemetry/repeat-2--p256-c10-p10--redis--set.jsonl | 445 | a907d3bd941a673770a77e47d1dc5c5460a1270964958c24dc2210af5eb32a81 |
| telemetry/repeat-2--p256-c10-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.csv | 1085 | e60007ee7b6f7713a090f2bc57d406fa40637d31819b06bef5c8c51c2cca8d13 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.jsonl | 3649 | 30de576ce4747ea086501ef8e5f4837d761be1b4cb95168c979071f491b4f9e3 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.metadata.json | 8025 | cba68ff53725fdeddadd99176b4a0f1fea2f035dc855731c15cc93a84d121917 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.csv | 1088 | 8dd18e29326259a02a6106ffb6663b287f1c291b0eaecd20b0d2ff04f17b4a1c |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.jsonl | 3652 | 0a4f01d51c5614b578320e2433f2ddcc0c332a1aeb5044b55ba22682fc1ccd0d |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.metadata.json | 8102 | 12e9bdd8a2f4722b3fa1c55b4cc03b0f6db0a559869667180aa2d475b3591c02 |
| telemetry/repeat-2--p256-c100-p1--hydra--get.csv | 630 | 3495c7c55ffdcb7e223ce6ca2a6e0c515513fe4dd65d9b489762b3b30167ef5a |
| telemetry/repeat-2--p256-c100-p1--hydra--get.jsonl | 1790 | b62233a2e53e92b18e0ec79ef44b2ca3aeee7538d81f50087116280c76680d70 |
| telemetry/repeat-2--p256-c100-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.csv | 631 | f688b89f3d2f4a7bcb3b35f2b23c0f5a4112a5598f96f6d8d481a2d99842d930 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.jsonl | 1791 | 426635d61bdf2a94f1c9cdc4f636036eea944fd24d4a9e7dc8a9fc3beeea46e8 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p256-c100-p1--redis--get.csv | 558 | 4fdb99faab07beb6de21ae83ce2d34a94c3b61eb0ef1863685d5a201b195882c |
| telemetry/repeat-2--p256-c100-p1--redis--get.jsonl | 1347 | 5b704404189512eaa5e145ac77573c5673bf366c8bef024353ed484d5d8b6086 |
| telemetry/repeat-2--p256-c100-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p256-c100-p1--redis--set.csv | 558 | 6a2fd91474f477dbd70550e466b46c88774f44ea5d853ef7eb0795b6cb7da6fd |
| telemetry/repeat-2--p256-c100-p1--redis--set.jsonl | 1347 | 6252bff9af0ddaf0bde36bc106d8b54feb37af62487b6165ff9e3a79ece3d7cc |
| telemetry/repeat-2--p256-c100-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.csv | 8206 | 3095f2028e27e3098a19072a27f0ab6072896f16bb454c3a6bc92a049901e232 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.jsonl | 35620 | 987301233caee2e664b563a3e55f261efe28891356b490aad3340b1c2b2ee64e |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.metadata.json | 8028 | ff7b66dc9ebe3ecfeb55738d6b88e5f3e1ee2c13f841cab7d2affc1dc7cec1d7 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.csv | 4643 | 13d2213faa265b00fc1d9bd7517f9c75be1a0bbc2ca8d053ec3e302cefeca2f2 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.jsonl | 19632 | 19ae7efe20b58f63097556b5a3f745f2c5561de2fd3793d71af4850360722df0 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.metadata.json | 8028 | e31290075c0a57fea540299873b83ed9cb37f3f4049be6c7c2208175eaae99fe |
| telemetry/repeat-2--p64-c10-p1--hydra--get.csv | 628 | 06d0d0a6812fdb7a77ff47f00cd070e3b6d0933d19c48dec36dff4531da5150f |
| telemetry/repeat-2--p64-c10-p1--hydra--get.jsonl | 1788 | 7f7a4dafac312147d4751b6da06bfd6915eac8ee090f9f0ad905c67521533286 |
| telemetry/repeat-2--p64-c10-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p64-c10-p1--hydra--set.csv | 632 | 2cd4e832fe57625f1921cb358dcf10070aeddbe3beb58c555bb6b0d84e095604 |
| telemetry/repeat-2--p64-c10-p1--hydra--set.jsonl | 1792 | 3cf69ea02ba0c58991a5aa9771433e028f6e3319ac33148bb8a8cfa3391dfa57 |
| telemetry/repeat-2--p64-c10-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p64-c10-p1--redis--get.csv | 556 | d14058e229d3e1cca7ed09edb1f6761cc0ef4351cc773f535799d2c8cb2af62a |
| telemetry/repeat-2--p64-c10-p1--redis--get.jsonl | 1345 | 406be22d9f52dc35ab2b12ccfe4dab893885f94e36ecc6c1d12574b91ad11e28 |
| telemetry/repeat-2--p64-c10-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p64-c10-p1--redis--set.csv | 557 | 0c1c934501366628a955b239ab84d7d9141bc1088d6b6b86cae737cee84ce1d9 |
| telemetry/repeat-2--p64-c10-p1--redis--set.jsonl | 1346 | b5b3061d0b301117803b9f33900db59fad86b8d977b931a5deacd34e2f77c0ce |
| telemetry/repeat-2--p64-c10-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.csv | 1389 | 9fd4825c3b67510b82cf4c687b78a60ea684b9b3d79d63a2b5a2ea621d9fc50b |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.jsonl | 5018 | 9db8a91f6f3101c805b8f9df4840ed634f848477d2f97d693ad3ff41731f31fe |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.metadata.json | 8028 | eba532d1efa03d29ee6ce52be64af3d41bd35049bbb7a3c224d3a9369cc3824b |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.csv | 783 | ef24feea1b01f1e96ca80f48c37518ecbb307d1eb2629c275a574651ca2be75f |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.jsonl | 2282 | ea23539527af59806056e838a3f8decf8280a9c6b0145ced3c2436c2b07adffa |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.metadata.json | 8028 | eba532d1efa03d29ee6ce52be64af3d41bd35049bbb7a3c224d3a9369cc3824b |
| telemetry/repeat-2--p64-c10-p10--hydra--get.csv | 542 | bd07d7706f8421136bad1dca1d660de2a824686b35d911f5c2dae4c11e0e93f0 |
| telemetry/repeat-2--p64-c10-p10--hydra--get.jsonl | 1343 | de03752f96d36a92e37be73fe9f2dd9b14b71a9e3180aa783e62bfb03324f10e |
| telemetry/repeat-2--p64-c10-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p64-c10-p10--hydra--set.csv | 543 | 45d2e08a9c91dabfbdb0c55d7ad26ff12e5c4053c749fd18c61e5f23f767697d |
| telemetry/repeat-2--p64-c10-p10--hydra--set.jsonl | 1344 | 9410f02164812314ef8c287e53ae97ab81e8433b3cac3c176720b211832a96dd |
| telemetry/repeat-2--p64-c10-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-2--p64-c10-p10--redis--get.csv | 367 | af726d5d35e21d0db2077a740e7486864b645260e1d8a4553e0b2c1a9e23ab7d |
| telemetry/repeat-2--p64-c10-p10--redis--get.jsonl | 446 | 049def9f88273a5ed090c3000aa237c51a6fb73964f6277976b865cb3e638e63 |
| telemetry/repeat-2--p64-c10-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-2--p64-c10-p10--redis--set.csv | 367 | 8662178375da0cab1f4d852faf36c56807bcfbdbf81da41b8749101d85f00b3b |
| telemetry/repeat-2--p64-c10-p10--redis--set.jsonl | 446 | 898120830a6299e495b8b8da1a103c1c6b9600779ecabd9362ba878fc4e8fa99 |
| telemetry/repeat-2--p64-c10-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.csv | 1394 | 370c935cb0c9231865d1977a2e5ecd95e48171194c27ef9b60b3b1b619d9dcae |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.jsonl | 5023 | 5aa1fe437682526ca754d28624d4919a906368944c20b4195a7385f224c538af |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.metadata.json | 8028 | 03f5e48ca853691e024dabdf87d4ad06a85084699fa3bdf2032add1fb0ee0333 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.csv | 1285 | 4a17ed510fd12fb09148d8d0cd235a416562129666232ca727ef125afcfb60e7 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.jsonl | 4559 | 8fcf62e91cc972a524aec7a3e864e4b541bf33aaa6aca03fda81a41de4416b8b |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.metadata.json | 8028 | 6d4fb715636012f2eef8cca8b2a8b4d5d29797858c8af21958494fd4aaa96438 |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.csv | 631 | 48d4fd0c2299f95a1f1f1a06c3b041260ea14a5c25e118af29775da63021e7ff |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.jsonl | 1791 | 22391783fbe80082bc3fdd850c02425b4334b1ee8259030b6116e7307313c1de |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.csv | 632 | 2ea3adefa88483368fe12d03f4f4dd88285f36a37b5dd3ad2ca463de27651857 |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.jsonl | 1792 | 6b376478fd23bab579e288f70236853920c9e895e653f45442ee71f78a8a72f7 |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p1024-c50-p1--redis--get.csv | 557 | a5df6885e6aaae9e90ce26accc55886eae88edaf3734b0964b5eb4374516d013 |
| telemetry/repeat-3--p1024-c50-p1--redis--get.jsonl | 1346 | 5ef530ed5f62397426cfbb5bd664a711330318fc697ce8be95dc13780507a552 |
| telemetry/repeat-3--p1024-c50-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.csv | 557 | 48ebcd0758960fc3441d79f018c266b20eeffdf297ed615bf4c897982d09b378 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.jsonl | 1346 | 089991975330738acc5a2eefad4b74b2364f38b0bc73bb82324f7e406b44cf45 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.csv | 781 | eaa70a132e483d380ea6c9e509277c4769977d1ded7c2415cc47a550b0aaa1aa |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.jsonl | 2280 | 78b7da763d9c63925af9dd0a827a2c1e8649a1186792e664b18ceff9664423bb |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.metadata.json | 8028 | 03f5e48ca853691e024dabdf87d4ad06a85084699fa3bdf2032add1fb0ee0333 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.csv | 780 | 3c0e2a07d249b4a55a46718d5ab073b168c41b8ac0298d5524e15466e8f70699 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.jsonl | 2279 | a22eed00c4b9a8c5d266dc776f9da60e37ddc7d9d69b466839720975917ddd01 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.metadata.json | 8028 | 03f5e48ca853691e024dabdf87d4ad06a85084699fa3bdf2032add1fb0ee0333 |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.csv | 542 | 4fc3670006a6dc271aecab5af25d5fdd2cbb3f83520178d63a9cb018b1700c61 |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.jsonl | 1343 | c408937d0a2fb4dc59e1c0d41fa65ad38ab08f24c7d353ff0707e017cfffea6c |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.csv | 542 | fea45492929a9ccf56e2d3ac1a53abc57fb4dc9815d88df5d54da2d2e34e6fd3 |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.jsonl | 1343 | dcceb340b1ea51722531299d490172237a15eb26a9c9b640b9e9118ec5a6b429 |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p1024-c50-p10--redis--get.csv | 371 | 712dee232fbd6c6d64372a0e9d10a1caca3e80dcd96a067f39f55b6ca97b38b0 |
| telemetry/repeat-3--p1024-c50-p10--redis--get.jsonl | 450 | 6c0abe9f279dea23f28907874d0789424f4b832ed847bdebaebe0af9569a9793 |
| telemetry/repeat-3--p1024-c50-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p1024-c50-p10--redis--set.csv | 367 | 8fe45f97e2f5de8f8812d74a889cb8cefc461cc1582b6a3a46b24c8a04eec182 |
| telemetry/repeat-3--p1024-c50-p10--redis--set.jsonl | 446 | 3ac6746a53f49cd339241e409353fb6b9f17baf2af389d147a7ac6414f70d225 |
| telemetry/repeat-3--p1024-c50-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.csv | 2104 | 08b1111adff8d38ee4faa141b927ec560a6ace9ff64936aeb310db100bed5a34 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.jsonl | 8218 | f2b096fa242169cbaed118aa6aebfe466fbfe9fdf16339ef7396783d82809f46 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.metadata.json | 8028 | 487ac447fce3d776fd03a6e997cad3d50937d35b8fc47e940699ef09517e7311 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.csv | 1999 | e25fdad7300750060de7ab74c6b56b2fb21bd0349cfc25ee1d26fdf210cb043b |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.jsonl | 7758 | 5a9ea579ad2eef1941afa0bf15b126abc98b987c1f79247d05de04336965f5db |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.metadata.json | 8028 | 8e43b23e284aa36eb6c24346371627410cfcc769cd8c098d856c08f0dfc78564 |
| telemetry/repeat-3--p256-c1-p1--hydra--get.csv | 720 | 7779f437acdf407b771d22c5aaf9f06bffd8093235eff94a63f8a924aa23c46c |
| telemetry/repeat-3--p256-c1-p1--hydra--get.jsonl | 2239 | 48f67b30dcfb26cb94cf67c96a6e8d08b233eef2c059f64df9ec2476a7c2d070 |
| telemetry/repeat-3--p256-c1-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c1-p1--hydra--set.csv | 720 | d3c199d0644b5b127ef463283d99617ccfc0c05118834ed90ed4e7cd550ffdbe |
| telemetry/repeat-3--p256-c1-p1--hydra--set.jsonl | 2239 | 849e9d133a8fd363a3e25210e1e8624add35c0d50bb967318afd2b438bfeca1c |
| telemetry/repeat-3--p256-c1-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c1-p1--redis--get.csv | 745 | 6194edc704620c670819431be416aa2e797b278316ba9d29ad78bc2ba3873627 |
| telemetry/repeat-3--p256-c1-p1--redis--get.jsonl | 2244 | 63318d616992927cdadee3931b8bf58dd0a81e90ffdd689f6c6ed4c2580b6af8 |
| telemetry/repeat-3--p256-c1-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c1-p1--redis--set.csv | 744 | e687b971ce93d5abfd1a2e2a822cefa60eb1a7c6feed5b2d9b9980a0f22b0cde |
| telemetry/repeat-3--p256-c1-p1--redis--set.jsonl | 2243 | 6d4dd856c7f8f8dfa4300abd816372c04b157c7d70705ceb691af42a81570871 |
| telemetry/repeat-3--p256-c1-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.csv | 8713 | da25f35de306fb72b9776058772c6d477ba81bc859d4aa0427b90a25f640b061 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.jsonl | 37902 | 98d1072fdfa9b31c5b887ad3ad1b59c94d33d94870d63f76cbc77997075c56f3 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.metadata.json | 8027 | e5929fa09c3ddc3d67761acec05d5ec73625d578e67c1ae22e22d9952f39bf6e |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.csv | 7599 | aa31104abd8b0777d7f5ef630db2b3674c72fe35f66b63ee3db1bf0d178e338b |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.jsonl | 32883 | bc106029a7a67d876da49392866252f726fe67a5b34a9dcf1bf73b4e16049049 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.metadata.json | 8026 | 8fde40a6c872f2ceb5bcdc23dd65ffcf63f4176f83fe364f4bc2248ce14974da |
| telemetry/repeat-3--p256-c10-p1--hydra--get.csv | 630 | 3c70288e0e86ffb69f39d59137c2ef8569bd5ae43381f0e0aed46f9c7cc37c0c |
| telemetry/repeat-3--p256-c10-p1--hydra--get.jsonl | 1790 | 1e3011481c1ec6f3d7f074d350e9a490940709238293b4796b6dbda4640e2097 |
| telemetry/repeat-3--p256-c10-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c10-p1--hydra--set.csv | 630 | 0d5cbe6261b13bab4c860aa4418eeca9e53df53246fe81c27543146a1427cb0a |
| telemetry/repeat-3--p256-c10-p1--hydra--set.jsonl | 1790 | 596db48bd7a1d9ba7c8f2943254412f62769485f2e26e584cf51537e6926471b |
| telemetry/repeat-3--p256-c10-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c10-p1--redis--get.csv | 554 | d13f4c0825a09e336b65219e083fa873d1924513c150d29178998c3d3494aba9 |
| telemetry/repeat-3--p256-c10-p1--redis--get.jsonl | 1343 | 208239e8279666a023fb594a5d0a9b441f7f4efbe0379f416d0195d878d04513 |
| telemetry/repeat-3--p256-c10-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c10-p1--redis--set.csv | 557 | 7074f9a5d9d261397188fb8d5e4f9ed34e0dd091782ad81dd111702e2a4db8f8 |
| telemetry/repeat-3--p256-c10-p1--redis--set.jsonl | 1346 | 6656dc7bf23137f2dd3641d104a7172ff5e62c95136acc604a3404d63adff220 |
| telemetry/repeat-3--p256-c10-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.csv | 1495 | 941270ec9c39a7e3c14501069b92f26b428a3c4b3ddd29ea7b63634968320e37 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.jsonl | 5479 | 11d112214bac31eb3a88c0389319bf7670b1bb93462d2c323080945f702a6522 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.metadata.json | 8028 | 007f4f2fafb8818a43c3cc564ac65b80288068461484dbef0d33ecfaa2c1f2a6 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.csv | 882 | 54e19428d99c6c5d5c655d73bea555654ed806d44f60e1b94fc2b025c82199c5 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.jsonl | 2736 | 315f71ab1aa9596ad906f579dd67ae523e66bb88b783a098d3d15109af783e5b |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.metadata.json | 8028 | 007f4f2fafb8818a43c3cc564ac65b80288068461484dbef0d33ecfaa2c1f2a6 |
| telemetry/repeat-3--p256-c10-p10--hydra--get.csv | 543 | 1a292728ecd3ecc9263756067d3d339a9bc5f417885f03704a74d1f54018c7b1 |
| telemetry/repeat-3--p256-c10-p10--hydra--get.jsonl | 1344 | 0409055f5400eb4bd7f4818b0c35a227186c0c692e0f9d1161d3d18b3c4c4ec5 |
| telemetry/repeat-3--p256-c10-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c10-p10--hydra--set.csv | 543 | 1ff306d864b02ee90e4254da7aa695471b56575f30590116956dc1c0a4160dba |
| telemetry/repeat-3--p256-c10-p10--hydra--set.jsonl | 1344 | 646cfa834b03c286a5cdb8699bcea8eefd69ca77ed6f5fc787eb1f0379c653bd |
| telemetry/repeat-3--p256-c10-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c10-p10--redis--get.csv | 367 | ebb904e6e527327b0858a02dba49d756f8cf9d4477e58d7f49ea55c4fd588717 |
| telemetry/repeat-3--p256-c10-p10--redis--get.jsonl | 446 | 94c9402e6a79be2211c9112bf5dd7f4f764121b53b4da568c9ab7f468dd624a0 |
| telemetry/repeat-3--p256-c10-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c10-p10--redis--set.csv | 367 | 0832079e109d460ed5235a69194b611d6e3405827b9d91ddbd653d71d38a9246 |
| telemetry/repeat-3--p256-c10-p10--redis--set.jsonl | 446 | aab320dcf2077e9ad24a72cfad3bc8325661711f4e5d023e3199ead7843fe719 |
| telemetry/repeat-3--p256-c10-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.csv | 1089 | f45d5ef3d8ab486b7aa5fa1dd18004cf398fe3473a3b72e1b97f7923c064faad |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.jsonl | 3653 | eab01b1816108091ae10fe34d8274a0fd6b827aa9314215dc6170f319bfcbb3b |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.metadata.json | 8028 | e52216a820ff01119255b902ccf07d224df3531d352d7fc3a4487627b2e8ab0d |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.csv | 1087 | 43e167272fcf40c8bfec76dae48f3d4e62cb60acc89c18e06e768daffe174f3f |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.jsonl | 3651 | 75b06f8d93dde5bc51337a49c48fefe04ada6fb7290ce123f70f01bee77459f4 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.metadata.json | 8028 | e52216a820ff01119255b902ccf07d224df3531d352d7fc3a4487627b2e8ab0d |
| telemetry/repeat-3--p256-c100-p1--hydra--get.csv | 630 | 78021f47bb8e30e6faad243a27af1483fc119a39eb0448fe43d0b3eb8c3620e8 |
| telemetry/repeat-3--p256-c100-p1--hydra--get.jsonl | 1790 | f35e3a3e95d9eb9d51c49fb87bc083f51acfb3e895c8924c6fbf48a1b5d930d6 |
| telemetry/repeat-3--p256-c100-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c100-p1--hydra--set.csv | 632 | 11b532c8f65f4d04265dc1711b834f3caa121f1a58feb9ba4a3531be68cf9ee8 |
| telemetry/repeat-3--p256-c100-p1--hydra--set.jsonl | 1792 | eaaf7be00ce9b00efcc61d86716c60811e1a7eb8461d92799276f3239c7fffe1 |
| telemetry/repeat-3--p256-c100-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p256-c100-p1--redis--get.csv | 555 | 6238f0d47f87764bab6ef5b95bddb08569485f2a012cf60f712a481ccb879372 |
| telemetry/repeat-3--p256-c100-p1--redis--get.jsonl | 1344 | 901fb8b6f5c00b6eeb2bfc9ff41580d2dbce05ad5d4c87a4daf8177e3b72d77d |
| telemetry/repeat-3--p256-c100-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p256-c100-p1--redis--set.csv | 555 | 98b23876dae51d3aac4a35167b1f09052e9bdb7bb643e56f3809b56ba9c1ee9a |
| telemetry/repeat-3--p256-c100-p1--redis--set.jsonl | 1344 | 7782ca29362815c2e4f70c9f351c78a97d4cd48f0bd01bdb463387d2b46857ad |
| telemetry/repeat-3--p256-c100-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.csv | 8594 | d86b1fe9f2c5114e6e1c51e7840e891f8ad7f9039f1af07d437255e5c6829a56 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.jsonl | 37428 | a7080ceb7c68c5ad9113d1d8eb493c57db620a482da1982a946882fd3427aa2a |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.metadata.json | 8026 | 06ffc9af4ab00fd16b3a7531968cf61858c9aa28edb77ed69411d4a1c32a48cb |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.csv | 6470 | bee3c2c409d20c793258bb9551e250e1994f026496cc7010f5159ff8b7cb8d32 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.jsonl | 27849 | eec3783a737d7a6f24c3d3d9c0f1ee5f25a4ef2e22d5a1a4c714e3305bcc1102 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.metadata.json | 8025 | fcfa456684cd6190a7602715ba7a1f70a496de1723b5768852ee41154c795159 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.csv | 630 | 0478e5a6e0efd8d26e7e11c2ea053065d24a9cf2764fa87a7087a6d48c985425 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.jsonl | 1790 | d53878dafb167ec2c31c06467ee8e3ffa7646f7508af68ba0a29ac3b2036fa53 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p64-c10-p1--hydra--set.csv | 630 | eee68a1c0410d8d23dfbc240ec8f6b409c601ce476b6d4b06937f1e080b7d57a |
| telemetry/repeat-3--p64-c10-p1--hydra--set.jsonl | 1790 | b5c304454a58c67b0137331ea657640805d15bc973cff2ad8e08daf5f3b20440 |
| telemetry/repeat-3--p64-c10-p1--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p64-c10-p1--redis--get.csv | 558 | 05f28cc368a61c14a984229eb79ae7427f788e2db67e567c48c70fd0d7fd0a32 |
| telemetry/repeat-3--p64-c10-p1--redis--get.jsonl | 1347 | 75df58eadfefcccdfe47a1050ee95f80c0d8c4ab3ddb9c06c59eaf8572270923 |
| telemetry/repeat-3--p64-c10-p1--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p64-c10-p1--redis--set.csv | 558 | 72843ef0f1376f3845b123e39bea156f99f585b679acecb7cde1ad096125069f |
| telemetry/repeat-3--p64-c10-p1--redis--set.jsonl | 1347 | 23773bab92e7baaa1776bfd4dd9bb643e2586eb1aa16e4db12f360e825f7ff71 |
| telemetry/repeat-3--p64-c10-p1--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.csv | 1390 | d78449332094afbf559e28db6a4f907f45acb073917ffc27ad0290a22ed12df7 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.jsonl | 5019 | 06238a1ee2a0e6c9b36ea22fc41a9d815b28ec5db66ddcd8e75e1f81703f4129 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.metadata.json | 8027 | 3e5015f0d9f99291f614cbfd044f4bd7b29495d09ab6f8bfa9d464d2a0995aac |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.csv | 781 | e61d5b9a758abe67a19c1b9c8f33dcef1e9a7c4a2fd01f39a50fa89364e5491f |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.jsonl | 2280 | 17fb884eed2a3f611c1c147941cb353ef184a1273252511a0aed179d47529890 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.metadata.json | 8027 | 3e5015f0d9f99291f614cbfd044f4bd7b29495d09ab6f8bfa9d464d2a0995aac |
| telemetry/repeat-3--p64-c10-p10--hydra--get.csv | 540 | 8a92075e2d71a0d337a9f14dc9b3078a789da96acfc4ca1acbda425df55e5730 |
| telemetry/repeat-3--p64-c10-p10--hydra--get.jsonl | 1341 | 2eda4c6328232b1fce50cc48a258462c80f333e0c9d4aa184afe098518021ac0 |
| telemetry/repeat-3--p64-c10-p10--hydra--get.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p64-c10-p10--hydra--set.csv | 543 | 2380f8c682c21d7d4b191c690297e153743c921aba229835524d0bc6cd0d712c |
| telemetry/repeat-3--p64-c10-p10--hydra--set.jsonl | 1344 | c32a41bd86dff1e0d491b3cfb1da1149267a5e6e6628cfa2760f72f0da0c7d1a |
| telemetry/repeat-3--p64-c10-p10--hydra--set.metadata.json | 152 | 717452aacaf65823c39b404f606b2c15407ec5452ede0fc2219d4b6b18f1caa6 |
| telemetry/repeat-3--p64-c10-p10--redis--get.csv | 370 | 932dcaa95c960c86d915464cc5510128a284620558d47397f3d1b133b5a4148f |
| telemetry/repeat-3--p64-c10-p10--redis--get.jsonl | 449 | 5a33beb77d4e86fe9cc76d1d6e37e87eb58e3fb07d904091511e126b0aa7fbc4 |
| telemetry/repeat-3--p64-c10-p10--redis--get.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry/repeat-3--p64-c10-p10--redis--set.csv | 371 | 3fdf757b2bbc6e9459f3a3ad09d4561a6a606e273e74b1550bdbb04cd2f4b74a |
| telemetry/repeat-3--p64-c10-p10--redis--set.jsonl | 450 | ac32f3f8fd0bbfc8e558add3e33575a31c44fbc365645ffee42fee0961017027 |
| telemetry/repeat-3--p64-c10-p10--redis--set.metadata.json | 7374 | 712200603fa69d9e237a12878751dcdcb8b401bb115d931029e6e7da40eea462 |
| telemetry-summary.json | 94407 | 87b7bcc2446b168801b0ba51da8afcc3038bb8eedcf500e55fa27e77d33fe388 |

Raw benchmark logs, telemetry JSONL/CSV, Docker inspect metadata, image identifiers,
hardware validation, and the artifact manifest are all in this same output directory.
The directory must be copied unchanged into the branch results tree after review.
