# Relative eight-case telemetry report

> Exploratory only. This report is not qualification/bootstrap evidence.

- Generated (UTC): 2026-08-02T19:52:45.092120+00:00
- Source commit: 117e6b69f44aca38cfa8681492c4630062e22249
- Targets: HydraCache, Redis, Hazelcast Community
- Workload: 8 cases x SET/GET x configured repeats
- Sampling interval: 1 second by default

## Reproduction

The exact command and environment used for this run:

~~~text
branch=detached@117e6b69f44a
source_commit=117e6b69f44aca38cfa8681492c4630062e22249
command=scripts/perf/run-relative-eight-cases-telemetry.sh /dev/shm/hydracache-exploratory-full-20260802T192750Z
targets=hydracache,redis,hazelcast-community
hazelcast_image=hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90
hazelcast_client_version=5.5.0
measurement_affinity=2
requests_per_case=100000
repeats=3
telemetry_interval_seconds=1
~~~

Re-run from the recorded source commit with the same image digest, client version, affinity, request count, and repeats.

## Host and validation receipt

~~~text
reference evidence tmpfs verified: root=/dev/shm/hydracache-reference-evidence-v1
reference runtime IRQ guard passed: phase=relative-eight-telemetry-pre measurement=2 irq_files=113 dormant-unmapped-nvme=2
host=hydracache-perf-v1
source_commit=117e6b69f44aca38cfa8681492c4630062e22249
source_status=
kernel=Linux 6.8.0-136-generic x86_64 GNU/Linux
cpu_model=AMD EPYC 7232P 8-Core Processor
logical_cpus=4
measurement_affinity=2
targets=hydracache,redis,hazelcast-community
runner_receipt_sha256=97a39b307c063872b5c249eda9cf8341d70e0c293932b75bc67ae596cb0b17ae
runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json
telemetry_interval_seconds=1
redis_benchmark=/usr/bin/redis-benchmark
redis_benchmark_version=redis-benchmark 7.0.15
hazelcast_image=hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90
hazelcast_client=5.5.0
reference runtime IRQ delta baseline captured: phase=baseline measurement=2 file=/dev/shm/hydracache-exploratory-full-20260802T192750Z/irq-baseline.tsv
irq_guard_mode=preflight-plus-baseline-delta
reference runtime IRQ delta guard passed: phase=post-relative-eight-telemetry measurement=2 monitored=2
~~~

## Telemetry summary

The summary preserves sample counts and reports p50/p95/max. Missing JVM heap fields remain unavailable; they are never inferred from RSS.

~~~json
{
  "repeat-1--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375451648.0,
      "p50": 375156736.0,
      "p95": 375443456.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.881,
      "p50": 0.7433,
      "p95": 0.8552,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 394084352.0,
      "p50": 394076160.0,
      "p95": 394084352.0,
      "samples": 11
    }
  },
  "repeat-1--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 375046144.0,
      "p50": 374444032.0,
      "p95": 375025868.8,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 1.0655,
      "p50": 0.97195,
      "p95": 1.063925,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 393777152.0,
      "p50": 393199616.0,
      "p95": 393771622.4,
      "samples": 10
    }
  },
  "repeat-1--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 157229056.0,
      "p50": 157032448.0,
      "p95": 157205094.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 178470912.0,
      "p50": 178470912.0,
      "p95": 178470912.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 146317312.0,
      "p50": 146317312.0,
      "p95": 146317312.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 146317312.0,
      "p50": 146317312.0,
      "p95": 146317312.0,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 153772032.0,
      "p50": 142888960.0,
      "p95": 152709734.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 154062848.0,
      "p50": 144095232.0,
      "p95": 152956928.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 143245312.0,
      "p50": 132048896.0,
      "p95": 142143692.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 143245312.0,
      "p50": 132048896.0,
      "p95": 142143692.8,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 18976768.0,
      "p50": 18935808.0,
      "p95": 18972672.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.4182,
      "p50": 5.414,
      "p95": 5.41778,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 25853952.0,
      "p50": 25817088.0,
      "p95": 25850265.6,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5907,
      "p50": 5.4738,
      "p95": 5.57901,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27287552.0,
      "p50": 27250688.0,
      "p95": 27283865.6,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27287552.0,
      "p50": 27250688.0,
      "p95": 27283865.6,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 376053760.0,
      "p50": 375750656.0,
      "p95": 376048025.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.4002,
      "p50": 1.0499,
      "p95": 1.3305999999999998,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 394534912.0,
      "p50": 394534912.0,
      "p95": 394534912.0,
      "samples": 5
    }
  },
  "repeat-1--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 375648256.0,
      "p50": 375357440.0,
      "p95": 375591731.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.8347,
      "p50": 1.5496,
      "p95": 1.78488,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 394235904.0,
      "p50": 394223616.0,
      "p95": 394235084.8,
      "samples": 5
    }
  },
  "repeat-1--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 183345152.0,
      "p50": 182992896.0,
      "p95": 183309926.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 205910016.0,
      "p50": 205910016.0,
      "p95": 205910016.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 171200512.0,
      "p50": 171200512.0,
      "p95": 171200512.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 171200512.0,
      "p50": 171200512.0,
      "p95": 171200512.0,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 181907456.0,
      "p50": 170377216.0,
      "p95": 180754432.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 181923840.0,
      "p50": 178515968.0,
      "p95": 181583052.8,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 169480192.0,
      "p50": 158015488.0,
      "p95": 168333721.6,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 169480192.0,
      "p50": 158015488.0,
      "p95": 168333721.6,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 18976768.0,
      "p50": 18976768.0,
      "p95": 18976768.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 16.533,
      "p50": 16.533,
      "p95": 16.533,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25702400.0,
      "p50": 25702400.0,
      "p95": 25702400.0,
      "samples": 1
    }
  },
  "repeat-1--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 18452480.0,
      "p50": 18452480.0,
      "p95": 18452480.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25300992.0,
      "p50": 25300992.0,
      "p95": 25300992.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 376320000.0,
      "p50": 375963648.0,
      "p95": 376201625.6,
      "samples": 18
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 18
    },
    "container_cpu_percent": {
      "max": 1.6489,
      "p50": 1.5484499999999999,
      "p95": 1.62068,
      "samples": 18
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 18
    },
    "vmrss_bytes": {
      "max": 394833920.0,
      "p50": 394563584.0,
      "p95": 394833920.0,
      "samples": 18
    }
  },
  "repeat-1--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 376180736.0,
      "p50": 375996416.0,
      "p95": 376174182.4,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 1.8329,
      "p50": 1.6463,
      "p95": 1.7569,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 394657792.0,
      "p50": 394579968.0,
      "p95": 394657792.0,
      "samples": 17
    }
  },
  "repeat-1--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 197595136.0,
      "p50": 197537792.0,
      "p95": 197589401.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 215105536.0,
      "p50": 215105536.0,
      "p95": 215105536.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 187281408.0,
      "p50": 187281408.0,
      "p95": 187281408.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 187281408.0,
      "p50": 187281408.0,
      "p95": 187281408.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 193462272.0,
      "p50": 186281984.0,
      "p95": 192553779.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 206262272.0,
      "p50": 206262272.0,
      "p95": 206262272.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 183218176.0,
      "p50": 176193536.0,
      "p95": 182314598.4,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 183218176.0,
      "p50": 176193536.0,
      "p95": 182314598.4,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7536640.0,
      "p50": 7536640.0,
      "p95": 7536640.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.563,
      "p50": 3.1422,
      "p95": 3.47886,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14671872.0,
      "p50": 14671872.0,
      "p95": 14671872.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19472384.0,
      "p50": 19472384.0,
      "p95": 19472384.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.2706,
      "p50": 3.1286,
      "p95": 3.2538,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26705920.0,
      "p50": 26705920.0,
      "p95": 26705920.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 371453952.0,
      "p50": 370991104.0,
      "p95": 371352576.0,
      "samples": 80
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 80
    },
    "container_cpu_percent": {
      "max": 0.8188,
      "p50": 0.16094999999999998,
      "p95": 0.3108199999999998,
      "samples": 80
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 80
    },
    "vmrss_bytes": {
      "max": 390283264.0,
      "p50": 390232064.0,
      "p95": 390283264.0,
      "samples": 80
    }
  },
  "repeat-1--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 370913280.0,
      "p50": 370440192.0,
      "p95": 370828288.0,
      "samples": 48
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 48
    },
    "container_cpu_percent": {
      "max": 0.9101,
      "p50": 0.25025,
      "p95": 0.7922549999999999,
      "samples": 48
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 48
    },
    "vmrss_bytes": {
      "max": 389865472.0,
      "p50": 389517312.0,
      "p95": 389646745.6,
      "samples": 48
    }
  },
  "repeat-1--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 97656832.0,
      "p50": 97640448.0,
      "p95": 97656217.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 115675136.0,
      "p50": 115675136.0,
      "p95": 115675136.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 87990272.0,
      "p50": 87990272.0,
      "p95": 87990272.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 87990272.0,
      "p50": 87990272.0,
      "p95": 87990272.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 94273536.0,
      "p50": 83402752.0,
      "p95": 93179904.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 94273536.0,
      "p50": 89194496.0,
      "p95": 93511680.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 84987904.0,
      "p50": 74078208.0,
      "p95": 83901644.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 84987904.0,
      "p50": 74078208.0,
      "p95": 83901644.8,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7430144.0,
      "p50": 7426048.0,
      "p95": 7429734.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 8085504.0,
      "p50": 8085504.0,
      "p95": 8085504.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 6.8335,
      "p50": 5.5384,
      "p95": 6.70399,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 15073280.0,
      "p50": 15073280.0,
      "p95": 15073280.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 14557184.0,
      "p50": 14557184.0,
      "p95": 14557184.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7942144.0,
      "p50": 7942144.0,
      "p95": 7942144.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 7942144.0,
      "p50": 7942144.0,
      "p95": 7942144.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.6402,
      "p50": 5.5532,
      "p95": 5.6315,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 15249408.0,
      "p50": 15249408.0,
      "p95": 15249408.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15249408.0,
      "p50": 15249408.0,
      "p95": 15249408.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 372477952.0,
      "p50": 372029440.0,
      "p95": 372354048.0,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 0.7465,
      "p50": 0.5740000000000001,
      "p95": 0.72901,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 391155712.0,
      "p50": 391131136.0,
      "p95": 391155712.0,
      "samples": 12
    }
  },
  "repeat-1--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 371748864.0,
      "p50": 371585024.0,
      "p95": 371721830.4,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.7979,
      "p50": 1.7034,
      "p95": 1.79664,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 390615040.0,
      "p50": 390582272.0,
      "p95": 390615040.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 122814464.0,
      "p50": 122748928.0,
      "p95": 122807910.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 141336576.0,
      "p50": 141336576.0,
      "p95": 141336576.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 112791552.0,
      "p50": 112791552.0,
      "p95": 112791552.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 112791552.0,
      "p50": 112791552.0,
      "p95": 112791552.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 121585664.0,
      "p50": 109912064.0,
      "p95": 120418304.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 121606144.0,
      "p50": 116289536.0,
      "p95": 121074483.2,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 111775744.0,
      "p50": 100044800.0,
      "p95": 110602649.6,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 111775744.0,
      "p50": 100044800.0,
      "p95": 110602649.6,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7184384.0,
      "p50": 7184384.0,
      "p95": 7184384.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 8085504.0,
      "p50": 8085504.0,
      "p95": 8085504.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 7.4044,
      "p50": 7.4044,
      "p95": 7.4044,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 15073280.0,
      "p50": 15073280.0,
      "p95": 15073280.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14557184.0,
      "p50": 14557184.0,
      "p95": 14557184.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7184384.0,
      "p50": 7184384.0,
      "p95": 7184384.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 8085504.0,
      "p50": 8085504.0,
      "p95": 8085504.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 15073280.0,
      "p50": 15073280.0,
      "p95": 15073280.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14557184.0,
      "p50": 14557184.0,
      "p95": 14557184.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375803904.0,
      "p50": 375562240.0,
      "p95": 375747993.6,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 0.9691,
      "p50": 0.8249,
      "p95": 0.9652149999999999,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 394616832.0,
      "p50": 394612736.0,
      "p95": 394616832.0,
      "samples": 8
    }
  },
  "repeat-1--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 376233984.0,
      "p50": 375740416.0,
      "p95": 376201011.2,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 1.0363,
      "p50": 0.9527,
      "p95": 1.025415,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 394600448.0,
      "p50": 394596352.0,
      "p95": 394599014.4,
      "samples": 8
    }
  },
  "repeat-1--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 226566144.0,
      "p50": 226461696.0,
      "p95": 226556928.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 249888768.0,
      "p50": 249888768.0,
      "p95": 249888768.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 214425600.0,
      "p50": 214425600.0,
      "p95": 214425600.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 214425600.0,
      "p50": 214425600.0,
      "p95": 214425600.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 223465472.0,
      "p50": 212248576.0,
      "p95": 222344806.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 223469568.0,
      "p50": 215584768.0,
      "p95": 222351974.4,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 211390464.0,
      "p50": 200093696.0,
      "p95": 210260582.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 211390464.0,
      "p50": 200093696.0,
      "p95": 210260582.4,
      "samples": 4
    }
  },
  "repeat-1--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 9203712.0,
      "p50": 9195520.0,
      "p95": 9202892.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.4494,
      "p50": 5.4492,
      "p95": 5.44938,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15798272.0,
      "p50": 15798272.0,
      "p95": 15798272.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8962048.0,
      "p50": 8781824.0,
      "p95": 8944025.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.3125,
      "p50": 5.1989,
      "p95": 5.30114,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15618048.0,
      "p50": 15548416.0,
      "p95": 15611084.8,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 373645312.0,
      "p50": 367702016.0,
      "p95": 368446668.8,
      "samples": 78
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 78
    },
    "container_cpu_percent": {
      "max": 11.7619,
      "p50": 0.18905,
      "p95": 0.6534549999999986,
      "samples": 78
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 78
    },
    "vmrss_bytes": {
      "max": 392495104.0,
      "p50": 386646016.0,
      "p95": 387395174.4,
      "samples": 78
    }
  },
  "repeat-1--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 354754560.0,
      "p50": 336969728.0,
      "p95": 354195456.0,
      "samples": 46
    },
    "cgroup_memory_peak_bytes": {
      "max": 355024896.0,
      "p50": 338874368.0,
      "p95": 354995200.0,
      "samples": 46
    },
    "container_cpu_percent": {
      "max": 14.8317,
      "p50": 0.3125,
      "p95": 2.16615,
      "samples": 46
    },
    "vmhwm_bytes": {
      "max": 373211136.0,
      "p50": 357470208.0,
      "p95": 373146624.0,
      "samples": 46
    },
    "vmrss_bytes": {
      "max": 373211136.0,
      "p50": 356001792.0,
      "p95": 373146624.0,
      "samples": 46
    }
  },
  "repeat-1--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 45305856.0,
      "p50": 45266944.0,
      "p95": 45305241.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 63676416.0,
      "p50": 63676416.0,
      "p95": 63676416.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 36270080.0,
      "p50": 36270080.0,
      "p95": 36270080.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 36270080.0,
      "p50": 36270080.0,
      "p95": 36270080.0,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 41811968.0,
      "p50": 30453760.0,
      "p95": 40671027.199999996,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 41816064.0,
      "p50": 31979520.0,
      "p95": 40678809.599999994,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 33329152.0,
      "p50": 22351872.0,
      "p95": 32239820.799999997,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 33329152.0,
      "p50": 22351872.0,
      "p95": 32239820.799999997,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5500928.0,
      "p50": 4739072.0,
      "p95": 5424742.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 5505024.0,
      "p50": 5505024.0,
      "p95": 5505024.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 6.3459,
      "p50": 5.4663,
      "p95": 6.2579400000000005,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 12115968.0,
      "p50": 12115968.0,
      "p95": 12115968.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12115968.0,
      "p50": 12115968.0,
      "p95": 12115968.0,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 4956160.0,
      "p50": 4706304.0,
      "p95": 4931174.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 4972544.0,
      "p50": 4968448.0,
      "p95": 4972134.4,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5708,
      "p50": 5.5146,
      "p95": 5.56518,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 12095488.0,
      "p50": 12083200.0,
      "p95": 12094259.2,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12095488.0,
      "p50": 12083200.0,
      "p95": 12094259.2,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 370528256.0,
      "p50": 370251776.0,
      "p95": 370422374.4,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 0.7385,
      "p50": 0.52615,
      "p95": 0.6719499999999999,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 389144576.0,
      "p50": 389117952.0,
      "p95": 389144576.0,
      "samples": 12
    }
  },
  "repeat-1--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 369594368.0,
      "p50": 369524736.0,
      "p95": 369593548.8,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 2.2207,
      "p50": 1.8753,
      "p95": 2.1637,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 388599808.0,
      "p50": 388444160.0,
      "p95": 388568678.4,
      "samples": 5
    }
  },
  "repeat-1--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 70537216.0,
      "p50": 70475776.0,
      "p95": 70531072.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 89194496.0,
      "p50": 89194496.0,
      "p95": 89194496.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 61095936.0,
      "p50": 61095936.0,
      "p95": 61095936.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 61095936.0,
      "p50": 61095936.0,
      "p95": 61095936.0,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 70082560.0,
      "p50": 57872384.0,
      "p95": 68861542.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 70082560.0,
      "p50": 63946752.0,
      "p95": 69468979.2,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 60624896.0,
      "p50": 48611328.0,
      "p95": 59423539.2,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 60624896.0,
      "p50": 48611328.0,
      "p95": 59423539.2,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 4739072.0,
      "p50": 4739072.0,
      "p95": 4739072.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 5505024.0,
      "p50": 5505024.0,
      "p95": 5505024.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 6.3371,
      "p50": 6.3371,
      "p95": 6.3371,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 12115968.0,
      "p50": 12115968.0,
      "p95": 12115968.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12115968.0,
      "p50": 12115968.0,
      "p95": 12115968.0,
      "samples": 1
    }
  },
  "repeat-1--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 4984832.0,
      "p50": 4984832.0,
      "p95": 4984832.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 5505024.0,
      "p50": 5505024.0,
      "p95": 5505024.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 18.3868,
      "p50": 18.3868,
      "p95": 18.3868,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 12115968.0,
      "p50": 12115968.0,
      "p95": 12115968.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12115968.0,
      "p50": 12115968.0,
      "p95": 12115968.0,
      "samples": 1
    }
  },
  "repeat-2--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377753600.0,
      "p50": 377503744.0,
      "p95": 377685401.6,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 1.317,
      "p50": 0.78145,
      "p95": 1.2371249999999998,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 396476416.0,
      "p50": 396439552.0,
      "p95": 396474572.8,
      "samples": 10
    }
  },
  "repeat-2--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377327616.0,
      "p50": 377055232.0,
      "p95": 377296281.6,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 1.0321,
      "p50": 0.9365,
      "p95": 1.01167,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 396128256.0,
      "p50": 396124160.0,
      "p95": 396128256.0,
      "samples": 10
    }
  },
  "repeat-2--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 358117376.0,
      "p50": 358033408.0,
      "p95": 358109388.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 379523072.0,
      "p50": 379523072.0,
      "p95": 379523072.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 345333760.0,
      "p50": 345333760.0,
      "p95": 345333760.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 345333760.0,
      "p50": 345333760.0,
      "p95": 345333760.0,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 354455552.0,
      "p50": 343455744.0,
      "p95": 353379737.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 354484224.0,
      "p50": 344805376.0,
      "p95": 353404108.8,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 342200320.0,
      "p50": 331016192.0,
      "p95": 341099929.6,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 342200320.0,
      "p50": 331016192.0,
      "p95": 341099929.6,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 20365312.0,
      "p50": 20365312.0,
      "p95": 20365312.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20611072.0,
      "p95": 20622131.2,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.3854,
      "p50": 5.3634,
      "p95": 5.3831999999999995,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27234304.0,
      "p50": 27234304.0,
      "p95": 27234304.0,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 7.1203,
      "p50": 5.4615,
      "p95": 6.95442,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27275264.0,
      "p50": 27201536.0,
      "p95": 27267891.2,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377999360.0,
      "p50": 377774080.0,
      "p95": 377973145.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.0608,
      "p50": 1.0266,
      "p95": 1.05534,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 396558336.0,
      "p50": 396558336.0,
      "p95": 396558336.0,
      "samples": 5
    }
  },
  "repeat-2--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377761792.0,
      "p50": 377503744.0,
      "p95": 377747046.4,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.3916,
      "p50": 1.2879,
      "p95": 1.3877,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 396525568.0,
      "p50": 396517376.0,
      "p95": 396525568.0,
      "samples": 5
    }
  },
  "repeat-2--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 384069632.0,
      "p50": 383852544.0,
      "p95": 384047923.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 407023616.0,
      "p50": 407023616.0,
      "p95": 407023616.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 370171904.0,
      "p50": 370171904.0,
      "p95": 370171904.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 370171904.0,
      "p50": 370171904.0,
      "p95": 370171904.0,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 382693376.0,
      "p50": 371195904.0,
      "p95": 381543628.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 382697472.0,
      "p50": 379523072.0,
      "p95": 382380032.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 368394240.0,
      "p50": 356962304.0,
      "p95": 367251046.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 368394240.0,
      "p50": 356962304.0,
      "p95": 367251046.4,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 19046400.0,
      "p50": 19046400.0,
      "p95": 19046400.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25796608.0,
      "p50": 25796608.0,
      "p95": 25796608.0,
      "samples": 1
    }
  },
  "repeat-2--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 18681856.0,
      "p50": 18681856.0,
      "p95": 18681856.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 15.7705,
      "p50": 15.7705,
      "p95": 15.7705,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25395200.0,
      "p50": 25395200.0,
      "p95": 25395200.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378355712.0,
      "p50": 377933824.0,
      "p95": 378188595.2,
      "samples": 18
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 18
    },
    "container_cpu_percent": {
      "max": 2.1971,
      "p50": 1.5796000000000001,
      "p95": 1.7200799999999992,
      "samples": 18
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 18
    },
    "vmrss_bytes": {
      "max": 396771328.0,
      "p50": 396533760.0,
      "p95": 396771328.0,
      "samples": 18
    }
  },
  "repeat-2--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 378339328.0,
      "p50": 378142720.0,
      "p95": 378326220.8,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 1.7339,
      "p50": 1.5855,
      "p95": 1.6698199999999999,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 396824576.0,
      "p50": 396738560.0,
      "p95": 396824576.0,
      "samples": 17
    }
  },
  "repeat-2--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 398376960.0,
      "p50": 398278656.0,
      "p95": 398368768.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 415977472.0,
      "p50": 415977472.0,
      "p95": 415977472.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 386224128.0,
      "p50": 386224128.0,
      "p95": 386224128.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 386224128.0,
      "p50": 386224128.0,
      "p95": 386224128.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 394047488.0,
      "p50": 387162112.0,
      "p95": 393202073.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 407089152.0,
      "p50": 407089152.0,
      "p95": 407089152.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 382164992.0,
      "p50": 375169024.0,
      "p95": 381266329.6,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 382164992.0,
      "p50": 375169024.0,
      "p95": 381266329.6,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7544832.0,
      "p50": 7544832.0,
      "p95": 7544832.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.5173,
      "p50": 3.1922,
      "p95": 3.45316,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14696448.0,
      "p50": 14696448.0,
      "p95": 14696448.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19587072.0,
      "p50": 19587072.0,
      "p95": 19587072.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.2241,
      "p50": 3.1812,
      "p95": 3.22078,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26722304.0,
      "p50": 26722304.0,
      "p95": 26722304.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 376610816.0,
      "p50": 376174592.0,
      "p95": 376567603.2,
      "samples": 82
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 82
    },
    "container_cpu_percent": {
      "max": 0.3855,
      "p50": 0.1486,
      "p95": 0.21664000000000003,
      "samples": 82
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 82
    },
    "vmrss_bytes": {
      "max": 395534336.0,
      "p50": 395370496.0,
      "p95": 395534336.0,
      "samples": 82
    }
  },
  "repeat-2--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377462784.0,
      "p50": 376832000.0,
      "p95": 377026560.0,
      "samples": 56
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 56
    },
    "container_cpu_percent": {
      "max": 0.7926,
      "p50": 0.1724,
      "p95": 0.45375,
      "samples": 56
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 56
    },
    "vmrss_bytes": {
      "max": 395948032.0,
      "p50": 395931648.0,
      "p95": 395948032.0,
      "samples": 56
    }
  },
  "repeat-2--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 298360832.0,
      "p50": 298242048.0,
      "p95": 298345472.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 316391424.0,
      "p50": 316391424.0,
      "p95": 316391424.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 286973952.0,
      "p50": 286973952.0,
      "p95": 286973952.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 286973952.0,
      "p50": 286973952.0,
      "p95": 286973952.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 294920192.0,
      "p50": 283920384.0,
      "p95": 293833932.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 294952960.0,
      "p50": 290193408.0,
      "p95": 294239027.2,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 283934720.0,
      "p50": 273037312.0,
      "p95": 282849075.2,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 283934720.0,
      "p50": 273037312.0,
      "p95": 282849075.2,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7692288.0,
      "p50": 7688192.0,
      "p95": 7691878.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5185,
      "p50": 5.4733,
      "p95": 5.51398,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 14782464.0,
      "p50": 14782464.0,
      "p95": 14782464.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8392704.0,
      "p50": 8392704.0,
      "p95": 8392704.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.6767,
      "p50": 5.5615,
      "p95": 5.66518,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15507456.0,
      "p50": 15491072.0,
      "p95": 15505817.6,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377012224.0,
      "p50": 376834048.0,
      "p95": 377007718.4,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 0.5743,
      "p50": 0.50135,
      "p95": 0.563905,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 395857920.0,
      "p50": 395849728.0,
      "p95": 395857920.0,
      "samples": 12
    }
  },
  "repeat-2--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 376762368.0,
      "p50": 376524800.0,
      "p95": 376719769.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.9163,
      "p50": 1.7143,
      "p95": 1.8910799999999999,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 395567104.0,
      "p50": 395554816.0,
      "p95": 395567104.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 323391488.0,
      "p50": 323190784.0,
      "p95": 323371417.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 342118400.0,
      "p50": 342118400.0,
      "p95": 342118400.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 311771136.0,
      "p50": 311771136.0,
      "p95": 311771136.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 311771136.0,
      "p50": 311771136.0,
      "p95": 311771136.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 322002944.0,
      "p50": 310624256.0,
      "p95": 320865075.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 322048000.0,
      "p50": 317014016.0,
      "p95": 321544601.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 310755328.0,
      "p50": 299020288.0,
      "p95": 309581824.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 310755328.0,
      "p50": 299020288.0,
      "p95": 309581824.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7438336.0,
      "p50": 7438336.0,
      "p95": 7438336.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 19.37,
      "p50": 19.37,
      "p95": 19.37,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14782464.0,
      "p50": 14782464.0,
      "p95": 14782464.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7438336.0,
      "p50": 7438336.0,
      "p95": 7438336.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14782464.0,
      "p50": 14782464.0,
      "p95": 14782464.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377929728.0,
      "p50": 377868288.0,
      "p95": 377926860.8,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 1.1228,
      "p50": 0.9297,
      "p95": 1.0792599999999999,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 396623872.0,
      "p50": 396607488.0,
      "p95": 396622438.4,
      "samples": 8
    }
  },
  "repeat-2--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 378548224.0,
      "p50": 377804800.0,
      "p95": 378404864.0,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 1.025,
      "p50": 0.931,
      "p95": 1.0226199999999999,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 396627968.0,
      "p50": 396584960.0,
      "p95": 396627968.0,
      "samples": 8
    }
  },
  "repeat-2--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 427520000.0,
      "p50": 426993664.0,
      "p95": 427441356.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 450945024.0,
      "p50": 450945024.0,
      "p95": 450945024.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 413339648.0,
      "p50": 413339648.0,
      "p95": 413339648.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 413339648.0,
      "p50": 413339648.0,
      "p95": 413339648.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 424103936.0,
      "p50": 412966912.0,
      "p95": 422996172.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 424103936.0,
      "p50": 416450560.0,
      "p95": 423003545.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 410230784.0,
      "p50": 398987264.0,
      "p95": 409106432.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 410230784.0,
      "p50": 398987264.0,
      "p95": 409106432.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 9023488.0,
      "p50": 9023488.0,
      "p95": 9023488.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.505,
      "p50": 5.4972,
      "p95": 5.50422,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15851520.0,
      "p50": 15843328.0,
      "p95": 15850700.8,
      "samples": 3
    }
  },
  "repeat-2--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8994816.0,
      "p50": 8994816.0,
      "p95": 8994816.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5399,
      "p50": 5.5028,
      "p95": 5.53619,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15663104.0,
      "p50": 15593472.0,
      "p95": 15656140.8,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375922688.0,
      "p50": 375201792.0,
      "p95": 375662592.0,
      "samples": 86
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 86
    },
    "container_cpu_percent": {
      "max": 0.3752,
      "p50": 0.14025,
      "p95": 0.215175,
      "samples": 86
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 86
    },
    "vmrss_bytes": {
      "max": 394457088.0,
      "p50": 394424320.0,
      "p95": 394448896.0,
      "samples": 86
    }
  },
  "repeat-2--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 376147968.0,
      "p50": 375406592.0,
      "p95": 375844864.0,
      "samples": 61
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 61
    },
    "container_cpu_percent": {
      "max": 0.8259,
      "p50": 0.1757,
      "p95": 0.3874,
      "samples": 61
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 61
    },
    "vmrss_bytes": {
      "max": 394846208.0,
      "p50": 394518528.0,
      "p95": 394842112.0,
      "samples": 61
    }
  },
  "repeat-2--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 246247424.0,
      "p50": 246097920.0,
      "p95": 246234521.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 264228864.0,
      "p50": 264228864.0,
      "p95": 264228864.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 235237376.0,
      "p50": 235237376.0,
      "p95": 235237376.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 235237376.0,
      "p50": 235237376.0,
      "p95": 235237376.0,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 243036160.0,
      "p50": 232552448.0,
      "p95": 242007654.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 250306560.0,
      "p50": 250306560.0,
      "p95": 250306560.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 232296448.0,
      "p50": 221503488.0,
      "p95": 231212032.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 232296448.0,
      "p50": 221503488.0,
      "p95": 231212032.0,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5140480.0,
      "p50": 5140480.0,
      "p95": 5140480.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.6132,
      "p50": 5.4648,
      "p95": 5.59836,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12468224.0,
      "p50": 12468224.0,
      "p95": 12468224.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8351744.0,
      "p50": 8351744.0,
      "p95": 8351744.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 6.2765,
      "p50": 5.58,
      "p95": 6.20685,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15532032.0,
      "p50": 15532032.0,
      "p95": 15532032.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377794560.0,
      "p50": 376674304.0,
      "p95": 377334988.8,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 0.7796,
      "p50": 0.5043,
      "p95": 0.73879,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 396398592.0,
      "p50": 395415552.0,
      "p95": 395927756.8,
      "samples": 12
    }
  },
  "repeat-2--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 376246272.0,
      "p50": 376000512.0,
      "p95": 376209408.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.8594,
      "p50": 1.7867,
      "p95": 1.84562,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 395038720.0,
      "p50": 395030528.0,
      "p95": 395037900.8,
      "samples": 5
    }
  },
  "repeat-2--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 271147008.0,
      "p50": 271081472.0,
      "p95": 271140454.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 290193408.0,
      "p50": 290193408.0,
      "p95": 290193408.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 260050944.0,
      "p50": 260050944.0,
      "p95": 260050944.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 260050944.0,
      "p50": 260050944.0,
      "p95": 260050944.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 270548992.0,
      "p50": 258633728.0,
      "p95": 269357465.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 270548992.0,
      "p50": 264654848.0,
      "p95": 269959577.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 259465216.0,
      "p50": 247504896.0,
      "p95": 258269184.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 259465216.0,
      "p50": 247504896.0,
      "p95": 258269184.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5062656.0,
      "p50": 5062656.0,
      "p95": 5062656.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 15.2627,
      "p50": 15.2627,
      "p95": 15.2627,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12406784.0,
      "p50": 12406784.0,
      "p95": 12406784.0,
      "samples": 1
    }
  },
  "repeat-2--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 5292032.0,
      "p50": 5292032.0,
      "p95": 5292032.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20217856.0,
      "p50": 20217856.0,
      "p95": 20217856.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12406784.0,
      "p50": 12406784.0,
      "p95": 12406784.0,
      "samples": 1
    }
  },
  "repeat-3--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 379219968.0,
      "p50": 378935296.0,
      "p95": 379186790.4,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 1.1011,
      "p50": 0.79095,
      "p95": 1.058485,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 397881344.0,
      "p50": 397873152.0,
      "p95": 397881344.0,
      "samples": 10
    }
  },
  "repeat-3--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 379056128.0,
      "p50": 378877952.0,
      "p95": 379035852.8,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 1.0011,
      "p50": 0.84175,
      "p95": 0.9885,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 397467648.0,
      "p50": 397463552.0,
      "p95": 397467648.0,
      "samples": 10
    }
  },
  "repeat-3--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 558751744.0,
      "p50": 558608384.0,
      "p95": 558739456.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 579739648.0,
      "p50": 579739648.0,
      "p95": 579739648.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 544296960.0,
      "p50": 544296960.0,
      "p95": 544296960.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 544296960.0,
      "p50": 544296960.0,
      "p95": 544296960.0,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 555425792.0,
      "p50": 544178176.0,
      "p95": 554317414.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 555425792.0,
      "p50": 545642496.0,
      "p95": 554317414.4,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 540999680.0,
      "p50": 529856512.0,
      "p95": 539904819.2,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 540999680.0,
      "p50": 529856512.0,
      "p95": 539904819.2,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 18714624.0,
      "p50": 18714624.0,
      "p95": 18714624.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.4243,
      "p50": 5.4127,
      "p95": 5.42314,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 25346048.0,
      "p50": 25346048.0,
      "p95": 25346048.0,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 20377600.0,
      "p50": 20377600.0,
      "p95": 20377600.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5609,
      "p50": 5.4797,
      "p95": 5.55278,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27369472.0,
      "p50": 27344896.0,
      "p95": 27367014.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 27369472.0,
      "p50": 27344896.0,
      "p95": 27367014.4,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 379310080.0,
      "p50": 379117568.0,
      "p95": 379277312.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.0562,
      "p50": 0.9086,
      "p95": 1.04284,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 398012416.0,
      "p50": 397926400.0,
      "p95": 397995212.8,
      "samples": 5
    }
  },
  "repeat-3--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 379195392.0,
      "p50": 379076608.0,
      "p95": 379180646.4,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.3701,
      "p50": 1.2651,
      "p95": 1.36444,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 397996032.0,
      "p50": 397914112.0,
      "p95": 397979648.0,
      "samples": 5
    }
  },
  "repeat-3--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 584499200.0,
      "p50": 584335360.0,
      "p95": 584482816.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 607473664.0,
      "p50": 607473664.0,
      "p95": 607473664.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 569094144.0,
      "p50": 569094144.0,
      "p95": 569094144.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 569094144.0,
      "p50": 569094144.0,
      "p95": 569094144.0,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 583512064.0,
      "p50": 572112896.0,
      "p95": 582372147.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 583516160.0,
      "p50": 579985408.0,
      "p95": 583163084.8,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 567271424.0,
      "p50": 555843584.0,
      "p95": 566128640.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 567271424.0,
      "p50": 555843584.0,
      "p95": 566128640.0,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 18649088.0,
      "p50": 18649088.0,
      "p95": 18649088.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25620480.0,
      "p50": 25620480.0,
      "p95": 25620480.0,
      "samples": 1
    }
  },
  "repeat-3--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 18378752.0,
      "p50": 18378752.0,
      "p95": 18378752.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25227264.0,
      "p50": 25227264.0,
      "p95": 25227264.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 379334656.0,
      "p50": 379000832.0,
      "p95": 379160576.0,
      "samples": 18
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 18
    },
    "container_cpu_percent": {
      "max": 1.7374,
      "p50": 1.46865,
      "p95": 1.5425799999999996,
      "samples": 18
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 18
    },
    "vmrss_bytes": {
      "max": 397869056.0,
      "p50": 397615104.0,
      "p95": 397869056.0,
      "samples": 18
    }
  },
  "repeat-3--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 379637760.0,
      "p50": 379322368.0,
      "p95": 379604992.0,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 1.6514,
      "p50": 1.5547,
      "p95": 1.62556,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 398016512.0,
      "p50": 397942784.0,
      "p95": 398016512.0,
      "samples": 17
    }
  },
  "repeat-3--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 599171072.0,
      "p50": 599097344.0,
      "p95": 599164518.4,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 616787968.0,
      "p50": 616787968.0,
      "p95": 616787968.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 585084928.0,
      "p50": 585084928.0,
      "p95": 585084928.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 585084928.0,
      "p50": 585084928.0,
      "p95": 585084928.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 594743296.0,
      "p50": 587763712.0,
      "p95": 593885593.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 607830016.0,
      "p50": 607830016.0,
      "p95": 607830016.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 581017600.0,
      "p50": 574038016.0,
      "p95": 580122214.4,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 581017600.0,
      "p50": 574038016.0,
      "p95": 580122214.4,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7544832.0,
      "p50": 7544832.0,
      "p95": 7544832.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.7022,
      "p50": 3.1896,
      "p95": 3.60168,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14680064.0,
      "p50": 14680064.0,
      "p95": 14680064.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19468288.0,
      "p50": 19468288.0,
      "p95": 19468288.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 3.2534,
      "p50": 3.1401,
      "p95": 3.2347200000000003,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26763264.0,
      "p50": 26603520.0,
      "p95": 26731315.2,
      "samples": 5
    }
  },
  "repeat-3--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378089472.0,
      "p50": 377554944.0,
      "p95": 377941606.4,
      "samples": 78
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 78
    },
    "container_cpu_percent": {
      "max": 0.4914,
      "p50": 0.15905,
      "p95": 0.3023749999999999,
      "samples": 78
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 78
    },
    "vmrss_bytes": {
      "max": 396840960.0,
      "p50": 396619776.0,
      "p95": 396840960.0,
      "samples": 78
    }
  },
  "repeat-3--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 378359808.0,
      "p50": 377581568.0,
      "p95": 378171392.0,
      "samples": 69
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 69
    },
    "container_cpu_percent": {
      "max": 0.5448,
      "p50": 0.1404,
      "p95": 0.44053999999999965,
      "samples": 69
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 69
    },
    "vmrss_bytes": {
      "max": 397029376.0,
      "p50": 396664832.0,
      "p95": 397029376.0,
      "samples": 69
    }
  },
  "repeat-3--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 499007488.0,
      "p50": 498944000.0,
      "p95": 499001958.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 517136384.0,
      "p50": 517136384.0,
      "p95": 517136384.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 485855232.0,
      "p50": 485855232.0,
      "p95": 485855232.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 485855232.0,
      "p50": 485855232.0,
      "p95": 485855232.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 495902720.0,
      "p50": 485025792.0,
      "p95": 494809088.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 495902720.0,
      "p50": 490774528.0,
      "p95": 495133491.2,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 482795520.0,
      "p50": 471894016.0,
      "p95": 481709875.2,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 482795520.0,
      "p50": 471894016.0,
      "p95": 481709875.2,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7442432.0,
      "p50": 7442432.0,
      "p95": 7442432.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.9064,
      "p50": 5.5313,
      "p95": 5.8688899999999995,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 14770176.0,
      "p50": 14770176.0,
      "p95": 14770176.0,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8183808.0,
      "p50": 8183808.0,
      "p95": 8183808.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 6.356,
      "p50": 5.6578,
      "p95": 6.28618,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15482880.0,
      "p50": 15482880.0,
      "p95": 15482880.0,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378343424.0,
      "p50": 378060800.0,
      "p95": 378277888.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.9393,
      "p50": 0.5816,
      "p95": 0.8773500000000001,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 397160448.0,
      "p50": 397148160.0,
      "p95": 397154304.0,
      "samples": 11
    }
  },
  "repeat-3--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377954304.0,
      "p50": 377761792.0,
      "p95": 377920716.8,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.782,
      "p50": 1.6599,
      "p95": 1.76796,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 396828672.0,
      "p50": 396828672.0,
      "p95": 396828672.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 524443648.0,
      "p50": 524365824.0,
      "p95": 524435865.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 542687232.0,
      "p50": 542687232.0,
      "p95": 542687232.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 510648320.0,
      "p50": 510648320.0,
      "p95": 510648320.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 510648320.0,
      "p50": 510648320.0,
      "p95": 510648320.0,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 522698752.0,
      "p50": 511012864.0,
      "p95": 521530163.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 523001856.0,
      "p50": 517726208.0,
      "p95": 522474291.2,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 509546496.0,
      "p50": 497848320.0,
      "p95": 508376678.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 509546496.0,
      "p50": 497848320.0,
      "p95": 508376678.4,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7426048.0,
      "p50": 7426048.0,
      "p95": 7426048.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14770176.0,
      "p50": 14770176.0,
      "p95": 14770176.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7426048.0,
      "p50": 7426048.0,
      "p95": 7426048.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 19.111,
      "p50": 19.111,
      "p95": 19.111,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
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
      "max": 379338752.0,
      "p50": 378892288.0,
      "p95": 379264204.8,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 0.9332,
      "p50": 0.8675999999999999,
      "p95": 0.926235,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 397639680.0,
      "p50": 397635584.0,
      "p95": 397638246.4,
      "samples": 8
    }
  },
  "repeat-3--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 378949632.0,
      "p50": 378716160.0,
      "p95": 378909491.2,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 1.1316,
      "p50": 0.9757499999999999,
      "p95": 1.10724,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 397631488.0,
      "p50": 397627392.0,
      "p95": 397631488.0,
      "samples": 8
    }
  },
  "repeat-3--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 628215808.0,
      "p50": 628103168.0,
      "p95": 628209664.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 651489280.0,
      "p50": 651489280.0,
      "p95": 651489280.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 612212736.0,
      "p50": 612212736.0,
      "p95": 612212736.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 612212736.0,
      "p50": 612212736.0,
      "p95": 612212736.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 624873472.0,
      "p50": 613791744.0,
      "p95": 623739289.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 624914432.0,
      "p50": 617447424.0,
      "p95": 623814656.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 609161216.0,
      "p50": 597854208.0,
      "p95": 608030105.6,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 609161216.0,
      "p50": 597854208.0,
      "p95": 608030105.6,
      "samples": 4
    }
  },
  "repeat-3--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 9162752.0,
      "p50": 9162752.0,
      "p95": 9162752.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.4976,
      "p50": 5.4761,
      "p95": 5.49545,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15753216.0,
      "p50": 15753216.0,
      "p95": 15753216.0,
      "samples": 3
    }
  },
  "repeat-3--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8974336.0,
      "p50": 8970240.0,
      "p95": 8973926.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5428,
      "p50": 5.4725,
      "p95": 5.535769999999999,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15630336.0,
      "p50": 15560704.0,
      "p95": 15623372.8,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378982400.0,
      "p50": 377139200.0,
      "p95": 377544294.4,
      "samples": 84
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 84
    },
    "container_cpu_percent": {
      "max": 0.4464,
      "p50": 0.149,
      "p95": 0.21237,
      "samples": 84
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 84
    },
    "vmrss_bytes": {
      "max": 396316672.0,
      "p50": 396234752.0,
      "p95": 396312576.0,
      "samples": 84
    }
  },
  "repeat-3--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 379260928.0,
      "p50": 377507840.0,
      "p95": 377987891.2,
      "samples": 65
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 65
    },
    "container_cpu_percent": {
      "max": 0.7725,
      "p50": 0.1648,
      "p95": 0.4697399999999999,
      "samples": 65
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 65
    },
    "vmrss_bytes": {
      "max": 397918208.0,
      "p50": 396558336.0,
      "p95": 396906496.0,
      "samples": 65
    }
  },
  "repeat-3--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 446836736.0,
      "p50": 446806016.0,
      "p95": 446836736.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 465874944.0,
      "p50": 465874944.0,
      "p95": 465874944.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 434155520.0,
      "p50": 434155520.0,
      "p95": 434155520.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 434155520.0,
      "p50": 434155520.0,
      "p95": 434155520.0,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 443904000.0,
      "p50": 432992256.0,
      "p95": 442810982.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 451133440.0,
      "p50": 451133440.0,
      "p95": 451133440.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 431284224.0,
      "p50": 420452352.0,
      "p95": 430196121.6,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 431284224.0,
      "p50": 420452352.0,
      "p95": 430196121.6,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5283840.0,
      "p50": 5042176.0,
      "p95": 5259673.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 5.5832,
      "p50": 5.5644,
      "p95": 5.58132,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 12374016.0,
      "p50": 12374016.0,
      "p95": 12374016.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8245248.0,
      "p50": 8245248.0,
      "p95": 8245248.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 3
    },
    "container_cpu_percent": {
      "max": 7.4053,
      "p50": 5.6309,
      "p95": 7.22786,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 15532032.0,
      "p50": 15532032.0,
      "p95": 15532032.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377843712.0,
      "p50": 377597952.0,
      "p95": 377815040.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 0.6002,
      "p50": 0.5248,
      "p95": 0.58985,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 396697600.0,
      "p50": 396693504.0,
      "p95": 396697600.0,
      "samples": 11
    }
  },
  "repeat-3--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377622528.0,
      "p50": 377597952.0,
      "p95": 377619251.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 388710400.0,
      "p50": 388710400.0,
      "p95": 388710400.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 1.8106,
      "p50": 1.7512,
      "p95": 1.80582,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 407392256.0,
      "p50": 407392256.0,
      "p95": 407392256.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 396410880.0,
      "p50": 396410880.0,
      "p95": 396410880.0,
      "samples": 5
    }
  },
  "repeat-3--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 472113152.0,
      "p50": 472002560.0,
      "p95": 472102092.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 490504192.0,
      "p50": 490504192.0,
      "p95": 490504192.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 458924032.0,
      "p50": 458924032.0,
      "p95": 458924032.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 458924032.0,
      "p50": 458924032.0,
      "p95": 458924032.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 471359488.0,
      "p50": 459448320.0,
      "p95": 470168371.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 471379968.0,
      "p50": 465874944.0,
      "p95": 470829465.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 458383360.0,
      "p50": 446423040.0,
      "p95": 457187328.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 458383360.0,
      "p50": 446423040.0,
      "p95": 457187328.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5029888.0,
      "p50": 5029888.0,
      "p95": 5029888.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
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
      "max": 5271552.0,
      "p50": 5271552.0,
      "p95": 5271552.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20623360.0,
      "p50": 20623360.0,
      "p95": 20623360.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27283456.0,
      "p50": 27283456.0,
      "p95": 27283456.0,
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
| hardware-validation.txt | 1189 | 001a15e53573d3f04950c0d4ac111638b0dd8181ebf286abddbf45c7f68a64ff |
| hydra.log | 16 | 6489d6d7a33c5d40e18fc61eeb6c34c341279ee61816394dde5189aa4ad8fae5 |
| hydra.pid | 6 | 278c35a864fb03690c8af86a978e916e3510e090ac5b24c1b3ab7606c6910597 |
| irq-baseline.tsv | 32 | e0544b0cc12ba8e46618b897cb240b6be665cb77936201612cd8f6766c9529b7 |
| metadata/docker-warnings.txt | 186 | ba431352b1954a86c23115052875b8a5d045c4062a9d512bdf510acc7511e201 |
| metadata/hazelcast.container-id | 65 | 431db1c27c9f41aa15893f04faeede7add9458c5a1b691fe6296ee9a08116f1b |
| metadata/hazelcast.inspect.json | 7674 | 71d3bb25edb1b68cff0203d55bc6a74d73bc717010d8cbf3d59aabdfc03dab05 |
| metadata/redis.container-id | 65 | b0de1750951a4eda4f9565f027fb4eb6dff42f3656bb3466edf515b5db969618 |
| metadata/redis.inspect.json | 8670 | 4dac9c6f51bc26e905e3108d703b0526c4777cc099fccf91594a6311a1b761a7 |
| raw/repeat-1--p1024-c50-p1--hazelcast--get.log | 186 | afd6e9bf03ad0eeaad6b25fa458bfc912244b4eccb4875e343b6f11b7f950506 |
| raw/repeat-1--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hazelcast--set.log | 186 | 43e3d6743caa5bc04a4f270623e3096e7f94fd1568cc88834fddb2ab0c36dd7c |
| raw/repeat-1--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hydra--get.log | 2044 | ba5b0fb62a616c9fc2189d2c73cd1468bb573a3d7744d3851b182bcc0bc7d284 |
| raw/repeat-1--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hydra--set.log | 2044 | d78ed548cc47cce0b04f0cf0e1229e132f38124c90c8de19e06c89148595ee09 |
| raw/repeat-1--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--redis--get.log | 1457 | e873a898b199132999b5b9bf66838afdcfff6bcb26cbac8839b0b71ba81e7280 |
| raw/repeat-1--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--redis--set.log | 1457 | 90a495c32c75c3b56aa57b01250d1e203a619c66b7b18b4b2333ee470391a6c3 |
| raw/repeat-1--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hazelcast--get.log | 187 | 91d4a323c603f52ba7a5efc925fb9e1ee1fdabca1c42fe63bb5759eb2a45a7a8 |
| raw/repeat-1--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hazelcast--set.log | 186 | d8d79e94230f825effa40a150819d206cc66f27d2c79cc078fec41f7c07c7131 |
| raw/repeat-1--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hydra--get.log | 1393 | d3ae6d208c0945dac364390ced6032d624cb95a40e78b07798f82ecdeacf1440 |
| raw/repeat-1--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hydra--set.log | 1393 | bf6a0ebdf53410d1aa0e8dc2b28fdc7387556318e4129269d8bfbf0ba14fa697 |
| raw/repeat-1--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--redis--get.log | 367 | 6e07fada44b43d252cdd60f0c3634a3c58c5b2f157d627bc88a0f8c2e9e58559 |
| raw/repeat-1--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--redis--set.log | 367 | 5413aec425abae7f2e310840b4dae2a6e25cf9190f0bdaed89753ff27e422538 |
| raw/repeat-1--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hazelcast--get.log | 184 | cb683c1db2c6ac8629a7f994b7c3b569d77d7dce2da879dd24ca800282aa33f6 |
| raw/repeat-1--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hazelcast--set.log | 183 | 66fda9351bd50451fc5ce564dcad7ce2848a20c03021e2b067a7898487b3f3a7 |
| raw/repeat-1--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hydra--get.log | 2868 | ee588cdbe2517ce2ecd99a394eaad80913ab62a5bcbecc867778cdd8ddce59a6 |
| raw/repeat-1--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hydra--set.log | 2868 | 39bee94a3a69eebb37d052971c2ef7873fbc10e6feae65913eea975890746898 |
| raw/repeat-1--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--redis--get.log | 2563 | ff9ca099f2710405ae83cf003e6bb2f9e44095314653b1c872440c005e1ffd23 |
| raw/repeat-1--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--redis--set.log | 2700 | 21403ef94b2d7e033baf3f97fab2f1b0302a443d87ee49134847152511637623 |
| raw/repeat-1--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hazelcast--get.log | 185 | 76bd1641c35ca8be127af081a82728770a46126f9aecf2a2b27359f764a7af2c |
| raw/repeat-1--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hazelcast--set.log | 185 | 4c773e516a3ec187ea40b504ec4c568749e8878ef654a7a09bce5ab2d099d8ad |
| raw/repeat-1--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hydra--get.log | 2043 | bd0e8c26fbfd4b6c11e28581dfee0a58e3b2542daa12449689d9913f0dfbe137 |
| raw/repeat-1--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hydra--set.log | 2045 | 308c89db1621bfb2e51e9fcdc64e46c12c0134e6c88f493456d84e2eddfef2e2 |
| raw/repeat-1--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--redis--get.log | 1458 | 25d092b395eaa08f6a71085e2a6f9f82907599fd692e791ff0bfa43d1e9dadad |
| raw/repeat-1--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--redis--set.log | 1458 | 68272d38c6b95e9ebe79037bb38f60573701d15c8b30dd5caa997b1c5a91bfc1 |
| raw/repeat-1--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hazelcast--get.log | 186 | 1e7b5fab79f96c42d4117e7f5b174e0a59b700218d0ebcf9734ae70b4297633f |
| raw/repeat-1--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hazelcast--set.log | 185 | d4b6479f39fbe4446a027391293357fa5c5f7163258d5f86b481bf821bf81295 |
| raw/repeat-1--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hydra--get.log | 1361 | 7f8552313d81dc03fd3caeba8285134a483c1372c9c1f1e3cb1247beab480cb6 |
| raw/repeat-1--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hydra--set.log | 1359 | d3ef3435f86f2cfcddea272b092df193b2b197986a4ebdde03d69ab1b2df8a4d |
| raw/repeat-1--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--redis--get.log | 368 | 327ccddb39d47259ede695db4ab23069184c8dd2254f2bf8c4ca988d8c1510db |
| raw/repeat-1--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--redis--set.log | 368 | 125ed45faa59ec9269defc1b889df627d134fb127baa9f2f4c2f9586d0e64b19 |
| raw/repeat-1--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hazelcast--get.log | 186 | 9fa0bd8713e68a3ca7c07336fbdc315af76d4b9e0b06a65212400bdae5cb3f8d |
| raw/repeat-1--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hazelcast--set.log | 186 | 2f7054854b1a5e7aa11b498365069c442a6e091ec78db475d1ca36549ac97ff5 |
| raw/repeat-1--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hydra--get.log | 2044 | 7ea24c174f91e59e04d96853c37226f7c5268985524168a2b0a07ce42cd69978 |
| raw/repeat-1--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hydra--set.log | 2044 | 6e181ac4e76846842f6d6a98d9d04aaabc28f90092f2bbbdb0d0438889677502 |
| raw/repeat-1--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--redis--get.log | 1457 | c7098e1cf35c75c50d04ac7795b65fa6fb4d508be3e54f23c5834814126b6d3f |
| raw/repeat-1--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--redis--set.log | 1320 | 99dafbfaebbdd14f755dcb6e66165c540398179cb0d55602adc4739d52ff4741 |
| raw/repeat-1--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hazelcast--get.log | 184 | 079198d0b88f58f61ca4b564c3031a36de33be93545b17f36a12c30d3583a1f9 |
| raw/repeat-1--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hazelcast--set.log | 184 | 0339751b28464bd713955920bfb268375edbb0ccefe2dc9e06c60489ec87a24e |
| raw/repeat-1--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hydra--get.log | 2044 | a519d9e5a4ce994b01330bdc8896d35125caba5e29bc5d2238bc93589b709b3d |
| raw/repeat-1--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hydra--set.log | 2044 | 0a1f820630c8c81a4023c87433cae1b2b0d387e3aa01c834a7857bf5b6058714 |
| raw/repeat-1--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--redis--get.log | 1457 | 68dc2cb16cf1b5b4033b2e2fdd2efc2d9b7f192ded5890c4febcf544f40973aa |
| raw/repeat-1--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--redis--set.log | 1455 | c2c2fb9b738f017870fad8d82f2588d43c4037aed18e7d85ae348b1c5989df6a |
| raw/repeat-1--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hazelcast--get.log | 184 | de993335ff0da170e7f0235ec6f8a74df4aa493b4cf3e6a49c89383c4a158db1 |
| raw/repeat-1--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hazelcast--set.log | 183 | f43dd5d56163488d927ffc4f8710b7064e187e8c2611a7ac285a61d49e7dd769 |
| raw/repeat-1--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hydra--get.log | 1360 | 53f731c9137000dc2700e7dfde0f6c1b986ad98704af64fed27dda04ab12e128 |
| raw/repeat-1--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hydra--set.log | 1360 | 5bb4ccc65bbf26e3bb7e852548302783d442ab0f77411d1db67ca5244b426976 |
| raw/repeat-1--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--redis--get.log | 226 | 1ec062db4f09cabbc7f2c59871c87dad29b4169678ec4bfb659a467683bf0f7d |
| raw/repeat-1--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--redis--set.log | 226 | ec46e8304d4efd1645d78234fd81f3673f5a1e183fc24897073bc9fc99f0add6 |
| raw/repeat-1--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hazelcast--get.log | 184 | aff40db94804c41aedcb8f426527406b387070818c8cd504a7eb88755a9bfc4f |
| raw/repeat-2--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hazelcast--set.log | 186 | ac6a5b6693a86cf862aaeed3d1909aefe9a960631a1807b3ff4d8d3c6851a821 |
| raw/repeat-2--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hydra--get.log | 2044 | 05a198fbfbda8df256b75e1d1a6b3e650e6a2a812c2a9872106544e8d2cb952f |
| raw/repeat-2--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hydra--set.log | 2046 | 7cb805e7bce00050b8f6866879e5e20604cbeb1152c172aa4171138c0e68577f |
| raw/repeat-2--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--redis--get.log | 1457 | 9a61deeb80db5542b97c884d5a49ee05ab42950373050c3ea79d7680cbf20147 |
| raw/repeat-2--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--redis--set.log | 1457 | a453801da54d33f010c18fbd8b76c87e35c3ed252be6fd88f951f2acb654409f |
| raw/repeat-2--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hazelcast--get.log | 187 | d28b5b20a42e19826cb851b311574dc77be52ef38c416fee055b25b602a193d4 |
| raw/repeat-2--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hazelcast--set.log | 186 | a67b355545178f3348a80d523986e9171318b1fcdb7ae54eb2eb6c473eda43df |
| raw/repeat-2--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hydra--get.log | 1393 | 656600322491834eb270bb294d741059f9b57d517963117e722b8dd52c14fa1b |
| raw/repeat-2--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hydra--set.log | 1393 | 482446852883db5875250bd7f5d1f5e1c204eac06382bfb8dcb3e4ba2d305ccc |
| raw/repeat-2--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--redis--get.log | 367 | 882e30af565642acc0d3dfa5ff58976ec0380927c293a488c3bf20a7a1d19b77 |
| raw/repeat-2--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--redis--set.log | 367 | bfe7828a6424702756239c381ac1b1a8987a36ddc875708c1606613c53cb4205 |
| raw/repeat-2--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hazelcast--get.log | 184 | a74883380c933993396005bb2b9734449e8b60fd3166d12c82e683139fcacc58 |
| raw/repeat-2--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hazelcast--set.log | 183 | 7303547e1bbcb3f731a36ba1d57914e1ae225f5a0a8a2d831755b62dff89de30 |
| raw/repeat-2--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hydra--get.log | 2874 | 1fc8f4b801a435f15f978c22ad3c44d7f24f522a7d5c7dbe28de4c6ee467fb93 |
| raw/repeat-2--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hydra--set.log | 2874 | 7c988686860de3774fd833736f3cb5502b2ec1f6b6db3541c9a7a4ad3c4c2d4e |
| raw/repeat-2--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--redis--get.log | 2829 | 403c72f3e6c35b1ef7aa82dccfd1a66f6ecbd267dc240ef20c7f18bb9dc6b460 |
| raw/repeat-2--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--redis--set.log | 2835 | 69063489f566999e64859bc2c2e7e571dc5d6c982656761b998668feba1069f1 |
| raw/repeat-2--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hazelcast--get.log | 185 | d0d1a4900becab485f0da000466d7f4f2621c8c934c40d159e83c25feeb4fce1 |
| raw/repeat-2--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hazelcast--set.log | 185 | 5924386a99a32ba8b68e52dd15f5d52bec752707acb952d583401e0f8d0d1d28 |
| raw/repeat-2--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hydra--get.log | 2045 | e3f8fab4d02799d6a0cd1020dd65bec64cf2d8a31d163ba0115ca25378e27fbb |
| raw/repeat-2--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hydra--set.log | 2045 | 303aadf9561db8ceaf6c4d384b02243db180aa0584b3ff69ad22535424fdbb15 |
| raw/repeat-2--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--redis--get.log | 1456 | 62dcc6cadbca2d9522d96745130f4a59fa19cb2536a299e11a8190a39ace232e |
| raw/repeat-2--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--redis--set.log | 1456 | 3b6f0fac6dcff29bdaad7d98fee596d9678391a731a139d1f82bcda9d4e62bbd |
| raw/repeat-2--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hazelcast--get.log | 185 | a755b522453539700b115b293e3987d93c80b4170b748c67d10e454e24d9ad86 |
| raw/repeat-2--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hazelcast--set.log | 184 | 38624a3fe0ba3d538d887947182f2e6ab0996a603d201f2ee50edc5637fde70e |
| raw/repeat-2--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hydra--get.log | 1359 | d93e29281a4cda92640c8337318da14b69ddfb7548534ab5c7fd3f3f1c4c5f86 |
| raw/repeat-2--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hydra--set.log | 1359 | 1016aff94e81df7893ece9180f5eb2c3e41d04cdd6ed65a30ef0975d12b57718 |
| raw/repeat-2--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--redis--get.log | 368 | e9b84c40183712f58df9005627868831ff576d20796a3d0a420c8fbfb78bcf2f |
| raw/repeat-2--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--redis--set.log | 368 | 9d86bca9aac2750058d66f98878b7b31552e0f030a372887d23a3663472c2546 |
| raw/repeat-2--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hazelcast--get.log | 185 | b90c021a164fdba9a525ba046a2482cb62510ff2bd9c934d6fd8de05d7fba3c5 |
| raw/repeat-2--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hazelcast--set.log | 187 | 5280a5072d0fd11fb7d9ce01cac73bc4fe8316089902c1d7ff92eb07021927e8 |
| raw/repeat-2--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hydra--get.log | 2044 | 9761c371e6a6831c7dc1cabd4a0a730a2159e1585f924a3dc7c9b5ca0cce490b |
| raw/repeat-2--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hydra--set.log | 2044 | f0df52b7f9603ada15b83af9ef86c00a97d67c0793e7b338ce65e38eb3ced97d |
| raw/repeat-2--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--redis--get.log | 1457 | 146653c5ca120f9165d773e9b8296efcd7a3640e0e31375876ed237dfb97a6b2 |
| raw/repeat-2--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--redis--set.log | 1457 | 4860c1841acf86eb85ab264b1b94d757fd0a1ae6243e672fccdd95cffaf779e0 |
| raw/repeat-2--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hazelcast--get.log | 183 | eef770736613302a152ac95acc2713aa0a43112931f6dbae898de54fd61b2ee8 |
| raw/repeat-2--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hazelcast--set.log | 183 | d25fa37b3c070b2253b4ec2dfd0ec93a35f2d56444b1c13efc7a0c342ce3e45b |
| raw/repeat-2--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hydra--get.log | 2044 | 6d9916222a6f57b6b43390bbf96c3ba8e21961988f6166b2d507054f1a86cdba |
| raw/repeat-2--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hydra--set.log | 2044 | fa933fadcb1d2870552da2724c0fb362657fcd03af875ec1f2b7c98fc842f435 |
| raw/repeat-2--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--redis--get.log | 1457 | 7251a0a6eca0bb60442a5a0a03d21dcd521ff36ce265e73cb89e3f0ce09a10cb |
| raw/repeat-2--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--redis--set.log | 1457 | b9f33a8a3a61e96e9cc33ab5c255c83f0452a12f6c04f13a50ea8c67f6ad6475 |
| raw/repeat-2--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hazelcast--get.log | 183 | e0d08b90fc93b256e2e9deccfb6d544fe86a857d56098b14cc38691701df31f1 |
| raw/repeat-2--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hazelcast--set.log | 184 | 4f1bca8893e7380acbfaca4fce7e730e2029a31d48830c5ce0a00a6fe6513ca0 |
| raw/repeat-2--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hydra--get.log | 1358 | def9019909681b8790af06deecac75cc69c4e458b8c4eb026d27200334d6608a |
| raw/repeat-2--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hydra--set.log | 1360 | affd3b7b6490c25e8c52a46c09842911065aac217e750fb02fece6fc87f8ccf8 |
| raw/repeat-2--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--redis--get.log | 226 | 293d54474f0c76435a1bb5ee49630b4e663bf66ea4298e30db07b4b5397ab68f |
| raw/repeat-2--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--redis--set.log | 226 | 7d129aef5cfd091d119af75f940fa13eb704ea2de1ff6fae132a9ec4200b269b |
| raw/repeat-2--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hazelcast--get.log | 186 | 0d177a0aeceaaab1898d4a8cb94ba8ddbd5dfca4c907fa8e958f70715cb0d148 |
| raw/repeat-3--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hazelcast--set.log | 186 | 762140da07cdddffa3a366e5a0a6384b99520f17eca0459a302a90ad6be9e286 |
| raw/repeat-3--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hydra--get.log | 2044 | c4e5a3fd8516d82c0117f2e5e046e65d2fadf355edc13357c59b54b361d3a18d |
| raw/repeat-3--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hydra--set.log | 2044 | 3e43fda61e93c0dad178df23ecdc5bcd813779d16ce499b4de2e3c1389d4926a |
| raw/repeat-3--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--redis--get.log | 1457 | 42ec0ffe56df4a15125aba2b9d0103b3296e7e6d30aa4d522e994704573ba25e |
| raw/repeat-3--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--redis--set.log | 1457 | 5827c50ac0d7512a1fd9c0c6bbf86832f01da3ad671d5fda7b1bd643903ce2dc |
| raw/repeat-3--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hazelcast--get.log | 186 | 4249d00c1740039a6f5139565b1fdb48e47b3f223c6952b31f198e3b738b0bc0 |
| raw/repeat-3--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hazelcast--set.log | 188 | 03a038fea680c7a2c1e4c23bf6d0e510f28ae1bbe742740212db96285c167b79 |
| raw/repeat-3--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hydra--get.log | 1395 | c734f797f213ec4d400c58807210b23794a1ac8fadff5a976d266c19208fdc27 |
| raw/repeat-3--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hydra--set.log | 1393 | a8501f9bbba21e6469368b5c85d89a16a519e74f735761326c42aa2a53f5bd7c |
| raw/repeat-3--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--redis--get.log | 367 | b118448722ad7739703d80e7a606ce90c91e10742404f2c19d174bcb539aadf0 |
| raw/repeat-3--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--redis--set.log | 367 | 3ce9d4dd4fd74fa3329a2cb48c8a1945c2f8f05d7e6624d4af95d4a6cbda156d |
| raw/repeat-3--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hazelcast--get.log | 184 | f50d0cbc398a419c26fe4fce75d59a4add09d1b90c4393080845ddb1dac84518 |
| raw/repeat-3--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hazelcast--set.log | 184 | 8c8be289527ff7b1ca3f654378009dcb6a395de64296b95489bd05ae2fb665bc |
| raw/repeat-3--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hydra--get.log | 2868 | 53b8e2f16acce85919ebfa1b6decb4944587e07bd4182c0ab563f48f90b9fae3 |
| raw/repeat-3--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hydra--set.log | 2874 | 60a63355edcb6491c47842c1e586c8066910b76307a9c8597d671ee3f522b99f |
| raw/repeat-3--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--redis--get.log | 2837 | c58f3d4b404a0deba8146c166b09e6264b6e722d14bcf3cd2579e94a85cd946e |
| raw/repeat-3--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--redis--set.log | 2829 | ff2a88c141e09f036015c961af6237983e14c1fc1724010c68cbda444bc58b0c |
| raw/repeat-3--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hazelcast--get.log | 185 | 8bd8b03280453f53f00c0e33617e1c2dadddd2cad8dc2a8eade357408a48a66c |
| raw/repeat-3--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hazelcast--set.log | 183 | 86055970c9f75f023bfd74baa7f5fbe27d857dbd93261835478abf59453bcd9e |
| raw/repeat-3--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hydra--get.log | 2043 | bcc2565f628290fa7dd640c37b8b12b801aed1bb877c78ff4a7b484600337dac |
| raw/repeat-3--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hydra--set.log | 2045 | fc59c1bf5dc9448bb3ab159eaf27b33110ba271ac095694bc38b9842dd0b6b9d |
| raw/repeat-3--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--redis--get.log | 1456 | 4095c0c6dda02807d374db20be414ea27ea9e59fb9cb974a839ad773f3a92749 |
| raw/repeat-3--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--redis--set.log | 1456 | 5c858806cfbb760a46cece54579df12b6ed74a66e5d5071b1758c4c846b68f07 |
| raw/repeat-3--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hazelcast--get.log | 186 | ba790bed53a089cbea536fda98779f74118bc029cfc3fff0e50e9b90efc53d2a |
| raw/repeat-3--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hazelcast--set.log | 185 | 76ebc058ead8bde78685cf5435fe3591df1d3e6795cef23bd65026d4a2e5e154 |
| raw/repeat-3--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hydra--get.log | 1359 | 56339cf2b435a8e60eae25909d64acaf31d2326ca4c4902016513ce2698cef74 |
| raw/repeat-3--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hydra--set.log | 1359 | 8b5d17f2eb7cb56635163dc15e2ba0fe31fac8bd5897b716b8d0d7c33be3cbb3 |
| raw/repeat-3--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--redis--get.log | 368 | 576893664c12e2a3b1d2273f6e679b9f5787c871dfe6906b5414f01bf1cc51bb |
| raw/repeat-3--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--redis--set.log | 368 | cb0ea6fce12c67ac8a5cc1bb835b4f1cda835c25e1ff57201201586fe7e187d8 |
| raw/repeat-3--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hazelcast--get.log | 186 | 223d03d668a939bec6d74c69d3f090b79a8d680a35879d3689480f38b38507a2 |
| raw/repeat-3--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hazelcast--set.log | 186 | 9eba3bdaccc8486bb113a77c822cb969f3c5cafd2be5d84a67592921a1a7f1fd |
| raw/repeat-3--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hydra--get.log | 2044 | 4d50e59cd87417e0965d0a5f95887082869bce5515c3d59b69819693f38ee2a2 |
| raw/repeat-3--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hydra--set.log | 2044 | deed9c5eeff0792d2448be1468e4ee6276cb4926ad9a19baa684965385612dda |
| raw/repeat-3--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--redis--get.log | 1457 | 0fa99090c8b80a83c53371f64a7faf9e48e8e83954b664171fec435df350a706 |
| raw/repeat-3--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--redis--set.log | 1457 | 578cde8bdcfaead4989a600684249bf997b55107158a7c8472f85dc61f11db56 |
| raw/repeat-3--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hazelcast--get.log | 184 | f35d65c987df88dc147296587df510efb55fd492b219fc7a837a0f3f2dac70dc |
| raw/repeat-3--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hazelcast--set.log | 182 | 2480a8c4d78b71901f909062b7246c9be9bf19371b998e1b0375783d3b8ce2ec |
| raw/repeat-3--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hydra--get.log | 2042 | 8d41bcdaabe56dbfa9acb33a5cea5da9ec90aa6c994defd34b311552014ec078 |
| raw/repeat-3--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hydra--set.log | 2042 | 92de55721a3d9d91a15feae511ce7ab5af16f3ffdff3548657f04c7017e19adf |
| raw/repeat-3--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--redis--get.log | 1457 | d739a1eff2c4da513e18717a2014efe8c2e50b88e84138e65717cb9dab5b69f3 |
| raw/repeat-3--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--redis--set.log | 1455 | 061ffcc93e257fdc61aee3f0752a20d772a0b4e71c76235446cd86375a031209 |
| raw/repeat-3--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hazelcast--get.log | 185 | 5172b3eb650a33af44bb17f0b019d2b9c79e61ec1999dd59d6ed61e1896406f8 |
| raw/repeat-3--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hazelcast--set.log | 184 | 171d2789dcd0c5303cdfd63326ff8a9e4dc490b32de99ebe58b422e140b3aa0c |
| raw/repeat-3--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hydra--get.log | 1358 | 7767ebf5cc1727d14e1c691aed8d6bb251090938f1e3e37c628d242cd0222b1e |
| raw/repeat-3--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hydra--set.log | 1358 | 5d3f2d1ffa0f53c573ad2aa07195eee4fbaa56d4aede4d189a5e756a20350af4 |
| raw/repeat-3--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--redis--get.log | 224 | e2628963254c6720745dabe11d5d0b657f582f72373049fdb511d3d237a8bd2f |
| raw/repeat-3--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--redis--set.log | 226 | 057efd353027c9e2cdf00367c925ff96319f2f90f4a61cb7327f38974764640c |
| raw/repeat-3--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| reproduction-command.txt | 485 | 6686158d3175d39e1efa85819aa74c090be75d6b3050a7f3632332d2878e7338 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.csv | 1401 | a637416ad585bafe0939bd17a3f7601bac396ce6751cb759528f946adf367a69 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.jsonl | 5030 | 0143970d3474dbe26243fbe3b6c68ff975899982ab9369349d900e402059f266 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.metadata.json | 8027 | 2e4ea1a731a166c0bad86454a218118315f11e01407792319ad93d25e7d8098e |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.csv | 1302 | 66dd6272ff108c20dd953c0208409267503e7c05b357eb4521377e1972d7501e |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.jsonl | 4576 | 9dad417f14a2172257fdec0df81d77fb2e19ca85540e797848c97c7ddc875bb3 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.metadata.json | 8027 | 93eb6316dd44bceb8d98e37a528515f94cfb2196c3fdbe8aa72db1069155b6e8 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.csv | 636 | c7a165519262018b9825487fb592d47bb6bf0b0e4778110bd8136efe03d7b361 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.jsonl | 1796 | 923e945c6dbe8754c2e7fdf72995fbcbe4314f3118ebe8ea7cac5676ef2711f5 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.csv | 635 | 1a16be5acd15b5204819c69965208f75700dab6d87404da7a2c239f352ea9a60 |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.jsonl | 1795 | 166f5bdc6c4083c11161345cfb6051254668925bf4517127f7383ea73e324544 |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p1024-c50-p1--redis--get.csv | 557 | 645c6a251409c0f2281bac59527011f4f23f0dd489c539483a5729e98b746ae1 |
| telemetry/repeat-1--p1024-c50-p1--redis--get.jsonl | 1346 | a83e7266ff61fe0f6f7158cdcdf45c55e6150cfe5d36311b13de2fc4275d4286 |
| telemetry/repeat-1--p1024-c50-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p1024-c50-p1--redis--set.csv | 559 | 9a3f4359f01f9825071dc9fe073376d6c4b35e9daff3d4014feae834fca201c6 |
| telemetry/repeat-1--p1024-c50-p1--redis--set.jsonl | 1348 | 7d266cb8b72bbdaf458e77e600c6182b02b06e2d0dbd4caa5788573c9bc8f07c |
| telemetry/repeat-1--p1024-c50-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.csv | 788 | 1c2ef9ebe0942876a37fb62ef713df5f6c9406ecc03ee121898db527e77af9b0 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.jsonl | 2287 | c6b30b3b71abb63307e4ca827bec1f57486abcd216040f3775655cb9f5dc2d5e |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.metadata.json | 8027 | 2e4ea1a731a166c0bad86454a218118315f11e01407792319ad93d25e7d8098e |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.csv | 786 | 9bf5658502b59af1b36add672ff55d3bda798c04e38b579a8ae14a436dca32dd |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.jsonl | 2285 | d77bf2427a0dfe6e6ef391e9ff86755abfe8c4449d15a90918ffae1803b94cb2 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.metadata.json | 8027 | 2e4ea1a731a166c0bad86454a218118315f11e01407792319ad93d25e7d8098e |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.csv | 542 | 129fa602d11f7947161e0b35fde6b8a41c9fdbc316776e41422e0c2e7525d7bc |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.jsonl | 1343 | 1124a9367556252f14e8a44fac75739dde7f2b857325ec52370d4c66229a9021 |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.csv | 546 | 97fefd7df47f98414ecc3727da0c22722babc1c0ba41935295e4eddf80ae1bc3 |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.jsonl | 1347 | cd94c7fc44b5f87a84de8eace3e8604128b57e40b63fc7eb459f154ce3b57ca5 |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p1024-c50-p10--redis--get.csv | 371 | 38a8becd88ca1af9f37c9b480020816eefbda8dd72cb29399ee77e9dce5d38f5 |
| telemetry/repeat-1--p1024-c50-p10--redis--get.jsonl | 450 | 2652ec33af01fc14b3c898977743ea4de1f59ab9512b0ed14c68eee354198a6d |
| telemetry/repeat-1--p1024-c50-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p1024-c50-p10--redis--set.csv | 367 | e6d8544ffa73cbba8eb47be29e523ace78712b158cc4250073a067430e307959 |
| telemetry/repeat-1--p1024-c50-p10--redis--set.jsonl | 446 | e9a58031e3154a0974fc89ed99390ab70b1aa5c1e6066d7c9ffb703a7853fc04 |
| telemetry/repeat-1--p1024-c50-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.csv | 2122 | f961d8903213630eb7a677be9dfe0613bdb1c0e098cf1cb311d06c967d35c7c5 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.jsonl | 8236 | 443b257cff3f98700c405f8e19ca21dae93da92482fd0bde04bcbfb7a16eb337 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.metadata.json | 8027 | eb8f9a8b0ad978b464d20d1d324f084a87f6320498ca0edcae1a4bf252766608 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.csv | 2019 | 4b62e5e7ff8a747a8224b2e56d1b838a871f17abc237702dc84af9ff340b57f3 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.jsonl | 7778 | e469aa3b76e542380be7296d6ea92141a1ee5e85be083528d09326884d04ef08 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.metadata.json | 8027 | 5b2f50b095782cfccdd6cebd714a8861e131a80038577275af7d45278008bb11 |
| telemetry/repeat-1--p256-c1-p1--hydra--get.csv | 725 | 8a00299e6ec26dbb0ded48ca596c3245f323de91a792d44168cbecd818a89134 |
| telemetry/repeat-1--p256-c1-p1--hydra--get.jsonl | 2244 | 8d0228f5654afede17aa1e9ee100ad3eb4fa01b554c5e211033465d7f579f714 |
| telemetry/repeat-1--p256-c1-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c1-p1--hydra--set.csv | 724 | c9c9997a85c525172992220370bab4bfd7893fdd7013989a059a1f560cd34755 |
| telemetry/repeat-1--p256-c1-p1--hydra--set.jsonl | 2243 | 6727affe8ef7c775a0bd7366d081965f1187d77a82f78ecda6a78a11eab4bbfb |
| telemetry/repeat-1--p256-c1-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c1-p1--redis--get.csv | 742 | de16cfea98cad3a8a5f6e062207df0cc2d161f768c3d5c1704a7df788cbfb51a |
| telemetry/repeat-1--p256-c1-p1--redis--get.jsonl | 2241 | cba9c225e8412a918c113547f61061dcad52ab4227636854309feab6df806eeb |
| telemetry/repeat-1--p256-c1-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c1-p1--redis--set.csv | 750 | 6bd85cfc326cb83101c9a66256c415f809e24d8a465b7fdd4c0e46dd14c2a053 |
| telemetry/repeat-1--p256-c1-p1--redis--set.jsonl | 2249 | a9f74705e995b7adead6bb7d6677afdc0ee11e967d999b3ebea07bd97f628ac8 |
| telemetry/repeat-1--p256-c1-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.csv | 8476 | b87d91f3a1668dc91cddae82abbd11c2ecc7ad3c96d9019872974050da94c4ec |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.jsonl | 36600 | 00c964b6339762828ff3962045448fde095c78f0e9a9785cc41ab6fb359c2bf7 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.metadata.json | 8028 | 6d1e213f1222b2ca5ffd7545a1b9cecbd6395520047d9a636963da7b4883a373 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.csv | 5200 | 925a761769f650b49156281276e4e1a609f837f71404515cb667118a2901b592 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.jsonl | 21964 | 036066b6d114dbddd44eba447b48a3808bf91c50bd42841c676277af8b72b49e |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.metadata.json | 8028 | 3afe8622c9eb2ca6e1b12b528cca14cb2a664daf83620d616511355bd3c873fb |
| telemetry/repeat-1--p256-c10-p1--hydra--get.csv | 618 | 7a4937e9d0a52b8a69394c404123ea38ca82ca4329db5e984181e1f229f62067 |
| telemetry/repeat-1--p256-c10-p1--hydra--get.jsonl | 1778 | f2531761a2a8891b90c869f21b10f4ee4a0eb8f6aa76fb84947252e4d20ed2a1 |
| telemetry/repeat-1--p256-c10-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c10-p1--hydra--set.csv | 613 | 122c0d75da4205bf448fc97ea4b04507da11be0df6971185f899258e1a9b9b78 |
| telemetry/repeat-1--p256-c10-p1--hydra--set.jsonl | 1773 | a55778d391fef61bdfde575abe76ba8eaf4e65e8b684fe7be6141715fdfb5271 |
| telemetry/repeat-1--p256-c10-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c10-p1--redis--get.csv | 551 | b04090ba00dba0f8aa394add7ab2c851e1b72e366063e75f3d90f520c9fab0c8 |
| telemetry/repeat-1--p256-c10-p1--redis--get.jsonl | 1340 | 04453f74870acdf03f29216c6131b0305cfb265167f2ce2a057da84d9f1a3299 |
| telemetry/repeat-1--p256-c10-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c10-p1--redis--set.csv | 552 | c4dc44a4058c4b61692420fb02f807beee54cb9013dd4b5fa593ffc6a47c0e84 |
| telemetry/repeat-1--p256-c10-p1--redis--set.jsonl | 1341 | e8bfbf516a88c9a17dbc7c60dc376df92633ddee802b046beb9c0abe203acc7b |
| telemetry/repeat-1--p256-c10-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.csv | 1506 | 54c1f1bfbd44bd07db65606926a0e7bbf81f3b5213097b19e2a4981565061f80 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.jsonl | 5490 | 2ae82b347bf3b23f45409633e307abbf20db599e26f55d7cb1f204b5e1811be7 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.metadata.json | 8029 | ba36146327c9e17e539e063975b54f93c30060b40f2077239adb7b5e1851c96d |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.csv | 787 | 003c0565ac4e70e9ebef49da600c9b5902d20e5a693ef5e046dbe7cf458d7b98 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.jsonl | 2286 | 30d28a52aa178baef35602365e4bbd5d52c6f6d2a8da045ced1ec6ce35491177 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.metadata.json | 8029 | ba36146327c9e17e539e063975b54f93c30060b40f2077239adb7b5e1851c96d |
| telemetry/repeat-1--p256-c10-p10--hydra--get.csv | 545 | cdf77f0ea2372bcb9745d00d532087bc4bc2217503b3f3dd6843d2a771eb9813 |
| telemetry/repeat-1--p256-c10-p10--hydra--get.jsonl | 1346 | c7ed6399aebc6d751fa170184a07b95816bd51003c28b552f6289b6e8ab67205 |
| telemetry/repeat-1--p256-c10-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c10-p10--hydra--set.csv | 541 | 6a29c3c1f576a0d1b8bc3738b5f4afb6b2fd5f25e368e8c3a049ab2e375ffd45 |
| telemetry/repeat-1--p256-c10-p10--hydra--set.jsonl | 1342 | 2de29da8fa101e5c4c1ec12d4c81da8240e3e1adfcc0c7dd927231f265f32499 |
| telemetry/repeat-1--p256-c10-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c10-p10--redis--get.csv | 368 | 20bccfe933ba44df7ec32137734e1982d04273efd562c34846880ee7076acba6 |
| telemetry/repeat-1--p256-c10-p10--redis--get.jsonl | 447 | 094e89ed52375c85ddd802d5dee4be5e7114bad987037fd01c24cd0c009fab7b |
| telemetry/repeat-1--p256-c10-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c10-p10--redis--set.csv | 365 | c06e781838b6855a69d2c54cfe8b23324c883dcdb9618e534c6bd55cd9c7060b |
| telemetry/repeat-1--p256-c10-p10--redis--set.jsonl | 444 | 71c9c0c020d508a3efa18ceaf40c870d53690d6aa28ee2c1e51662e43e11428e |
| telemetry/repeat-1--p256-c10-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.csv | 1094 | ff40089c90134042340f3e635e18934d7b5af51e9cdabd7806d1baafd952c708 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.jsonl | 3658 | 88daa5859c8473088244b780a25ec34b019ca1022eb6abbf77f526581793b747 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.metadata.json | 8027 | 286e4525bb69a92587e711b17ba975ec6a737ad898ae9805102e98fd856f41ed |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.csv | 1096 | 854437d7e98f23ba9e0d2a54956af80c5f99f94224ead21ea429882cfd5fd4c6 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.jsonl | 3660 | dc5d9d997edb1f557e4da00465ac782c1fc90147a22ecf59a6dc3e4f681f29d0 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.metadata.json | 8027 | 286e4525bb69a92587e711b17ba975ec6a737ad898ae9805102e98fd856f41ed |
| telemetry/repeat-1--p256-c100-p1--hydra--get.csv | 636 | c4225d88be6f6422bc95744831b1ef1921962444b30667bfb250456e625bc87c |
| telemetry/repeat-1--p256-c100-p1--hydra--get.jsonl | 1796 | 2754e0eb87fddcca9227e4c16b4d8d6da0fb85f183825340d6c537818416ed9c |
| telemetry/repeat-1--p256-c100-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c100-p1--hydra--set.csv | 634 | 7e3211e1d542c13428de921cdc559b2a2f7470fd23d5e557d5be607fbb20653f |
| telemetry/repeat-1--p256-c100-p1--hydra--set.jsonl | 1794 | 2f3a76009cd39955451024bb0835da306798f5a0c75ae69727aaa7d602d4d928 |
| telemetry/repeat-1--p256-c100-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p256-c100-p1--redis--get.csv | 553 | 30d4a99f9e91dc1d0a74d3261c23891f6bac929afc725760875f2aac950c7ec5 |
| telemetry/repeat-1--p256-c100-p1--redis--get.jsonl | 1342 | ce260576e2296f72c1f5b52965ca95ec7f26f4a6f4dcf582fa73f58955979541 |
| telemetry/repeat-1--p256-c100-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p256-c100-p1--redis--set.csv | 555 | 2c3511b349e52a98759d83cc31e1ed93bf44f09d3ef051f852c974e6644e3288 |
| telemetry/repeat-1--p256-c100-p1--redis--set.jsonl | 1344 | c3f3717e91b900b05a19f8f844b429fa468e5a47f0d018691da0166acc0c2d67 |
| telemetry/repeat-1--p256-c100-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.csv | 8278 | a623ed423382cbf9d7c7b8da3a5349a9ac3298919c4da442dec8365df98a959e |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.jsonl | 35692 | 9399257657ea25b73c379ef22e0218345b4d683236c56e4bd9e106d8edc4b9e1 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.metadata.json | 7114 | 6a5fa9dea1ca27f538185d9a3bb87f84fd1fc137f139744fe192242193e41ec6 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.csv | 4992 | cd4b019dc02d6cb96360d16d0949197cca58bb94e7eb687b7c137ed3322a33b2 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.jsonl | 21046 | 7a66fb09a035ebbdf3128336a990750e69ecbdbc4cc4d464f30b4b5e4ce9c2cf |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.metadata.json | 6497 | e4625e537b78c9acedf08709db38e64d3211f35dfaa5ca3aa56ca89d1a5a258d |
| telemetry/repeat-1--p64-c10-p1--hydra--get.csv | 616 | cfdac088f68e6101e2c50b91815785250bab8e1ffcbf1b3ab399f3816247284d |
| telemetry/repeat-1--p64-c10-p1--hydra--get.jsonl | 1776 | d7d5f60928cc221cd83f9d60ed6f6f9e369711b51f63a328cd94d859db94251b |
| telemetry/repeat-1--p64-c10-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p64-c10-p1--hydra--set.csv | 609 | 27659361cefbd65550321368e1fbb5c365cab5e29b7d97f5124c23ec78668813 |
| telemetry/repeat-1--p64-c10-p1--hydra--set.jsonl | 1769 | afc5967dfc2deb80cbea6f50f7a1dd3c859864a936af594b9fe958b433362941 |
| telemetry/repeat-1--p64-c10-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p64-c10-p1--redis--get.csv | 551 | 69ebbf7f88e588bdff2aaf52fcdb0458e04efb5692513f270074f9ec50b2fce1 |
| telemetry/repeat-1--p64-c10-p1--redis--get.jsonl | 1340 | 5149d54fcefaa1563d9f7cde9c5c9b7da4f251abe0667cfa06edb80a96887bc5 |
| telemetry/repeat-1--p64-c10-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p64-c10-p1--redis--set.csv | 548 | a6629f127e4d1508211b9fb17041565e9c55410635dc2a7df7927da525390933 |
| telemetry/repeat-1--p64-c10-p1--redis--set.jsonl | 1337 | 3add6754855f969fabbe9bc4b8f6f9417f9cf32c0a0923964ccbd7c9f54ee4da |
| telemetry/repeat-1--p64-c10-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.csv | 1504 | 45dac8d859988afafbf90dd705aeb7b9abaf46085578cb90ac7f78666e71c788 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.jsonl | 5488 | ec2cc60e4548b9614a6e69ce4c074b74f42ed4e002c5afc5dd81d9e4afae12e3 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.metadata.json | 8028 | 3afe8622c9eb2ca6e1b12b528cca14cb2a664daf83620d616511355bd3c873fb |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.csv | 786 | 721f5b4dcbea25dc25a32110fbd434f3b61bef2cd3f1b8949741774300295a42 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.jsonl | 2285 | 0218e88134bf794032c3306bbc60f3e1970a237d0968a4e6bd22363eb2817b0a |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.metadata.json | 7724 | e7f142aa73ed54f2057c84a50bcb4e62a1710a70eb13558159aef8084278eece |
| telemetry/repeat-1--p64-c10-p10--hydra--get.csv | 530 | 4d85f7c1f04f497860c4b04c6613cf63e79bb1da6b958e13445a65c26c0737af |
| telemetry/repeat-1--p64-c10-p10--hydra--get.jsonl | 1331 | 6e06e9e698674ac0c8aae7a2a182ad8da1c9bfbff5efc7b8122bae476e54526a |
| telemetry/repeat-1--p64-c10-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p64-c10-p10--hydra--set.csv | 530 | 4f90bf47bd3fc6b687c542990eca98c2b5ad5380c493e86b668114f8c7a96113 |
| telemetry/repeat-1--p64-c10-p10--hydra--set.jsonl | 1331 | 4422d0641aa00aea6e31f4d4dd1b45adf890df1a0491eb7ed7e0740e293da168 |
| telemetry/repeat-1--p64-c10-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-1--p64-c10-p10--redis--get.csv | 368 | ec7398b894bddb47f9491d8b9fadb3c203812922e390ed41f1d42992d4fe358c |
| telemetry/repeat-1--p64-c10-p10--redis--get.jsonl | 447 | 240407d89241f2f81aba93d3d0092f76e0fc3011207e06985f3cf5d0e9ff56ad |
| telemetry/repeat-1--p64-c10-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-1--p64-c10-p10--redis--set.csv | 369 | d1410c16c1addb11f3ed144be0b31362ec921f28e7dd8768a603828dcdad6db2 |
| telemetry/repeat-1--p64-c10-p10--redis--set.jsonl | 448 | 831999f0ea638d32d327810d1c2738d1f4b0a6d8528025fc25583b1f2e6d7011 |
| telemetry/repeat-1--p64-c10-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.csv | 1298 | 54d4bf336ef09ac74a8ddb19c1c3d224dffdb41ef8f7baf256ade653ecd456e1 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.jsonl | 4572 | a4457c7917d0ca337556e553543eda68e41ef2f11a6187411dc5cbd4489de0d7 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.metadata.json | 8029 | 52772e8f54c20d36f6297c145a0175e2e1f450991f510c4b4ae9b99af6fb4ca9 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.csv | 1302 | 8f1ea5b4e4d2e5c8a82e3d61f54e9644a9982cda2728db30dada3481b0b35ccb |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.jsonl | 4576 | 0063cbd9b4fe84bf65ffa6f9c7974353e16ba9ac995f1e14346bd72f35655e0f |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.metadata.json | 8029 | 52772e8f54c20d36f6297c145a0175e2e1f450991f510c4b4ae9b99af6fb4ca9 |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.csv | 634 | 303857b5901ee56e963f7fef0fa5b364e3f7cbe5681342e3f9a8f5eaa96c06a7 |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.jsonl | 1794 | 189d50cae506677a808fe9b8636bc318faf621ad5323a0eacca8a186cb5df0b1 |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.csv | 636 | f9cd13dd8d3b526bc3be2f2c0174d32e20605aa1da2c8e1a595c08e71b11ab30 |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.jsonl | 1796 | 123b33acd969032863d88546c8e81a4bc802fe0f00a58a8c9e38377e720361af |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p1024-c50-p1--redis--get.csv | 557 | 65bc101020bfa8eaba36c74a5200dd90c31b2529c4e1ca7650afb7714c1fac74 |
| telemetry/repeat-2--p1024-c50-p1--redis--get.jsonl | 1346 | f63e904d726357fd940d4a75d61b541fac0a1c31bc92cb6845e4e30a81d40f2e |
| telemetry/repeat-2--p1024-c50-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p1024-c50-p1--redis--set.csv | 561 | 54110bef14b2ead7c973fe4dea2cd8e90a5925f1e438f237b4798677c25e9d60 |
| telemetry/repeat-2--p1024-c50-p1--redis--set.jsonl | 1350 | c2768eb43b6122f0a57f7dad24aa3a7197f9b452d49cbedc4f3ab3f12c9e4176 |
| telemetry/repeat-2--p1024-c50-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.csv | 788 | 46a0d4ac3e1bd4723c78d6134fa48a5e133bb5c601502e75ce3428651f6b5cd9 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.jsonl | 2287 | 596039c8d68cec503f4a5b02792280e9f73cde3f3c23748cced31a833347e181 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.metadata.json | 8029 | a2e0c5aaa3f8a9e595a158f22e9d8ce85145facac82154f8d8f90e0f9a94954c |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.csv | 786 | ac560848b20f4f9645cfd37d0582fa4a758061ed21076beb325a798565d1ce4d |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.jsonl | 2285 | 38d3d9167925219671d1d1a7f742bb7ec7d1b9312893d7d58ead9789750e0503 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.metadata.json | 8029 | a2e0c5aaa3f8a9e595a158f22e9d8ce85145facac82154f8d8f90e0f9a94954c |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.csv | 546 | 238c2b5d89ae8ef61a3ad93ad4efb7d7d8663f25c8ea8753ca584e68537f2420 |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.jsonl | 1347 | e973253d6f2d317086e4f3d9a018086302a3b409046c07b4c6548e992525d0c4 |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.csv | 546 | 0f5a7e9200a50167b1d8ebe8cfd5ac6b94131cc1b7c5675597a0bf04902f89cb |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.jsonl | 1347 | 82bca58cc553e08d3f60538dce81ad32b4e399b67c09d01999347ef4a3958101 |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p1024-c50-p10--redis--get.csv | 368 | 8a69a3f826ad811a7c7e733d50204c9f476f2ffdacb50c4fed7c66ae387ff62a |
| telemetry/repeat-2--p1024-c50-p10--redis--get.jsonl | 447 | 1062bf696fea02fabe7b57fd23efe19427e6d766e4fff0dfa815778f7d3c6c27 |
| telemetry/repeat-2--p1024-c50-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p1024-c50-p10--redis--set.csv | 371 | 47d2dd9ee3c755ed02ef06ae32ed9e34866729176ba64214773734db121686af |
| telemetry/repeat-2--p1024-c50-p10--redis--set.jsonl | 450 | 78808c90720d5ff6d165fa2990690840e1a21c2ad1ae44713866f36e3c1a01c4 |
| telemetry/repeat-2--p1024-c50-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.csv | 2124 | 7b42e54964317fb0c148a1bbccc39f37dec6673c61c81997aecc09e897b4bb70 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.jsonl | 8238 | e0c67d4ddc2e93e5e386649c57256699d37f9095386203749452ffd694447676 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.metadata.json | 8028 | be737881661b2ae4cc571e1d730c3d2d0539ed0b22d1eb78e180c81309bdb596 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.csv | 2018 | 43afb655f54a9d2efa729d84e65606bf20d238375dca024cff2c8d77a4f98272 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.jsonl | 7777 | 88aff1aacd596b3ffa0f2af8f445569b8e5448b0e05312340936f12717f4567c |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.metadata.json | 8029 | 52bf6f05e168f3f09d178f75c60a5fd2b375f7d564175dcc250d9f49fb6435db |
| telemetry/repeat-2--p256-c1-p1--hydra--get.csv | 724 | 78f41d24114f352df10c942d2eefad8083cd3fcbefd2e639aba8fcedc3b9c1e4 |
| telemetry/repeat-2--p256-c1-p1--hydra--get.jsonl | 2243 | 7176e93bf57f27ebc06ba3e43e18ff0703b76a3fd311620363f2738203377905 |
| telemetry/repeat-2--p256-c1-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c1-p1--hydra--set.csv | 726 | efe4e961bb88624f0b5110118e71fda4bc9640c12f7e2a90c8e98112cc85e721 |
| telemetry/repeat-2--p256-c1-p1--hydra--set.jsonl | 2245 | c4bf7ba3d39dbe87ece8e5a21a52ba49346bea4dde10cbc059423cc72ecab287 |
| telemetry/repeat-2--p256-c1-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c1-p1--redis--get.csv | 742 | a3ee27640df131e15417a1036db849f4b037d8830fb519633cab1dc70f026808 |
| telemetry/repeat-2--p256-c1-p1--redis--get.jsonl | 2241 | 7a4b3c9f02c25e98c3738f849d7940f1af38d5a728bc841ee0839a8b2d31d440 |
| telemetry/repeat-2--p256-c1-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c1-p1--redis--set.csv | 751 | dfdb13d809355b0f4ab9044c54a83fe67feabef68f6cf45ae1f99004e8575efc |
| telemetry/repeat-2--p256-c1-p1--redis--set.jsonl | 2250 | d06534ae50bb05efa7b2d1dc582a9815a7d383a15552066efc86b7509f4d0a03 |
| telemetry/repeat-2--p256-c1-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.csv | 8690 | c11a90151522bdfb84f0a4c0f21ced220cb8bc4050f94dc474be4d098a11ffa7 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.jsonl | 37524 | 782a0ab143d720e0b14425ae18491a1a38173061009d4b6ef389e57117c5f693 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.metadata.json | 8028 | 3cb4060d1437382599baa4601911c13e5ab5a76a2ab11691129a65a78d7e3cb5 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.csv | 6017 | ac1de79c74f3c75a2d1342aa9b65d7c541d7554ac18d0f330a5641af3d411501 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.jsonl | 25621 | 55d7ea0eac4bc2275e97a96bf533a49ee754278dc715bf263c21074d2159a772 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.metadata.json | 8028 | 5e5b896e823a0e8d3caeac0100f2a2afb29c812a70725583f29ce957998ac0e6 |
| telemetry/repeat-2--p256-c10-p1--hydra--get.csv | 634 | abbc451c76ec9adafdfc4136601b4547867f69b688ff5dfa612bf3c5f93611ed |
| telemetry/repeat-2--p256-c10-p1--hydra--get.jsonl | 1794 | 964018bb5dc2b992c5b571a47ccc9027a7b3fbc1792d0d7dee443591e5ca5b85 |
| telemetry/repeat-2--p256-c10-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c10-p1--hydra--set.csv | 636 | 81df963e794cbb3a56c678df1bb2ef2b33bd5ab14b00c67d9cf821eb60c4b18f |
| telemetry/repeat-2--p256-c10-p1--hydra--set.jsonl | 1796 | 712617c4681f7c4b86b480b1dfa810bba59e2ea353eb599ede75064597eda1eb |
| telemetry/repeat-2--p256-c10-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c10-p1--redis--get.csv | 556 | df6cf03dafe921501ab5f94f2e731b1afa1f12186a17d3c964a7d870807903ca |
| telemetry/repeat-2--p256-c10-p1--redis--get.jsonl | 1345 | 9e08277bed2801503bab11be528c8273e7446b6f997094cceb403ced1fbf39ce |
| telemetry/repeat-2--p256-c10-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c10-p1--redis--set.csv | 558 | 9546c4fc2ce64e31ff54d101ef6735e0b896eed9fc6d405bbd428d1542782d17 |
| telemetry/repeat-2--p256-c10-p1--redis--set.jsonl | 1347 | 1b585f8c9945647bd74082cacfc2a0d0dae4946a1f3d8e6ddda14ffb63853ff7 |
| telemetry/repeat-2--p256-c10-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.csv | 1504 | 9225db6df38b4f02f1ed3546d24c23db2268808b77f65e2161e8fa1728db811d |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.jsonl | 5488 | 8ca5ef3ccd6c38dab483994a61c00bf742aadeb9edab8f595cf23be45b2d4928 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.metadata.json | 8029 | e298911b0fa8136ba86c114e5a8ac1a8d3560eca731dade823e8d32102a3d3f3 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.csv | 785 | 87a30391ac3d10f38faec1fb370259c8557a2b3a11aeabef52ce3d6f52148ef7 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.jsonl | 2284 | cd9b4fe7ef7276776d0bc23a7b9f78206769af748b9fe35e7288e42d7f35d912 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.metadata.json | 8029 | e298911b0fa8136ba86c114e5a8ac1a8d3560eca731dade823e8d32102a3d3f3 |
| telemetry/repeat-2--p256-c10-p10--hydra--get.csv | 546 | 35441fdf79a355244938250957f6e717028a8f0f6101d402b4e8c10a37e376ef |
| telemetry/repeat-2--p256-c10-p10--hydra--get.jsonl | 1347 | 9325f04127bec32d0e90f55de67b48135d8ce534b52428b2ac60cd88a60e26e7 |
| telemetry/repeat-2--p256-c10-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c10-p10--hydra--set.csv | 544 | 6b7d8df65804ffac109a218fd77e3f38ed697655afb701a0157ea73cacf6cba8 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.jsonl | 1345 | 9ee82643022ef09056358c80833a01914f88137a0d5d828631ece37e72af4262 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c10-p10--redis--get.csv | 369 | 5a8e9d062273b2144d74674a3318c5571fa7c19338cda1346319b559453f9784 |
| telemetry/repeat-2--p256-c10-p10--redis--get.jsonl | 448 | c86882671eecd75c059c04a18d72f5fe694e40cd9317edcaa325163ca9489c3b |
| telemetry/repeat-2--p256-c10-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c10-p10--redis--set.csv | 366 | c8e0d88fcff75b4093a0a34d17ec22a0551650731ae67e599070bc2becb2ad9b |
| telemetry/repeat-2--p256-c10-p10--redis--set.jsonl | 445 | 406758bf6f8a88063b5748e9f370556a04059c4f348d04ca1aa32f6996a3570a |
| telemetry/repeat-2--p256-c10-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.csv | 1093 | a4caea22898c66715a7e6c5005397a5c913af045e212cf94b684a864ed94b17a |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.jsonl | 3657 | a69e9802a32c272077098a4e59e77d8cc6b0db92967bf6e5278ca44c42c6ea7f |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.metadata.json | 8028 | 3e2a88c985ab1cec03b168828eb935b243fca80f8769730e5a855b6b8cb5edc1 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.csv | 1092 | 82b3e983d007af16102ff9e872f317694e4081738d2a4b3da02e076b75f391f6 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.jsonl | 3656 | 265f23999ab2944e2696f11a2711d3d2e02db7f612427ac7b6bb50e6f56b2a2a |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.metadata.json | 8028 | be737881661b2ae4cc571e1d730c3d2d0539ed0b22d1eb78e180c81309bdb596 |
| telemetry/repeat-2--p256-c100-p1--hydra--get.csv | 633 | 4cffc2405815c5a7fb279028bf0c17dbb2935d3e0f83ef99c6b1419debb17cab |
| telemetry/repeat-2--p256-c100-p1--hydra--get.jsonl | 1793 | 5006a53df3bafd7d70423720631f15c4a0e9c692dc847e07ac96245ae57ba664 |
| telemetry/repeat-2--p256-c100-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c100-p1--hydra--set.csv | 635 | 82712717dd87c28c72bdc3435db1c47a440d1de31beca2409a0282b6bf943576 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.jsonl | 1795 | b6c79f3d0abf0a02ca5d346f6a1d0fb163550a365a9b3fd2275263e9be59e646 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p256-c100-p1--redis--get.csv | 552 | 59d09f9fa5f4d8c542e6d5d86970f8d1ff9f82c5f3775b7f3e1a162da3e6b903 |
| telemetry/repeat-2--p256-c100-p1--redis--get.jsonl | 1341 | 827ab06a38be82a24af703f0c979cf8c3a07bb5ae8c2263106f1d4718c689c51 |
| telemetry/repeat-2--p256-c100-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p256-c100-p1--redis--set.csv | 554 | a6045a9cffde4197816e8c7984b58a4fbffd86995335decc4d4950df3f896532 |
| telemetry/repeat-2--p256-c100-p1--redis--set.jsonl | 1343 | 07cac8ea7bedd1b39279ffb62f04d1f2551df19c9b83670ab5b4879cfd87bf6a |
| telemetry/repeat-2--p256-c100-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.csv | 9092 | 7013923bdee95d516fe9e344bf365aa7f05cfccc940a2f6b2495171a54759f59 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.jsonl | 39346 | 2b1c6ba7bb4131087b3cabac24fd3d0c1cf0ec2630429cd6b0fe1dc2162d5555 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.metadata.json | 8029 | 540f1c8a50871cac34ec69421322b414b1a1739b9992665e40f7400c32c444da |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.csv | 6539 | c1e41ac649f08154b4d3bd1056f1b6a9a4f4d54d4a19eb5b76901f69d52b7158 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.jsonl | 27918 | 73686d928798683efc26388150b1b88c5ffcdcd3851298bcabb8e8cad0ea88b5 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.metadata.json | 8029 | ecc7686b1fb1231a1d935cc00247c396048ee8354d1203f48399416669dd92f7 |
| telemetry/repeat-2--p64-c10-p1--hydra--get.csv | 633 | 4a8032aef20ad9ada5ae87aa9016741e008f28fca007eb5ccbb6a93db0b09887 |
| telemetry/repeat-2--p64-c10-p1--hydra--get.jsonl | 1793 | 9b81632dbeb13cee538970663f7da867fcaf34c6483d1e70a4f651758272931c |
| telemetry/repeat-2--p64-c10-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p64-c10-p1--hydra--set.csv | 636 | bd95385817062764ffabb140d1ad8457f54763be1dff305e5afa49cb3a549f6c |
| telemetry/repeat-2--p64-c10-p1--hydra--set.jsonl | 1796 | 563a0e3be0870cb30f1c550c6ad8fb32f2458010ecb31dd807a8e9404efe54a1 |
| telemetry/repeat-2--p64-c10-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p64-c10-p1--redis--get.csv | 557 | 7761e399f643724c2c88f2511c5b19e25ed59177e3b200686996774fcb6e96bf |
| telemetry/repeat-2--p64-c10-p1--redis--get.jsonl | 1346 | bc37e2a89430867ef509c5bdfefeba4f2040a078f9e4173b2b2bad6ddbfdb945 |
| telemetry/repeat-2--p64-c10-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p64-c10-p1--redis--set.csv | 556 | 1eeae20b2e4b9e1258252a6efd7587eeaa9909628240efe35e31fe46fffc9784 |
| telemetry/repeat-2--p64-c10-p1--redis--set.jsonl | 1345 | d596a611ea6e8fe2a8024e68db8368f8dd14cad3f8c712c96ac67867b7c1f45d |
| telemetry/repeat-2--p64-c10-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.csv | 1508 | 012611c0326da3bd1e833ba25146c09a4e42394a1369f4f3dbd5b1d37679c298 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.jsonl | 5492 | b2c0f012e789ba70a4be04f7aa1a545b7ade577cd20e9d11e444cb857219a3b5 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.metadata.json | 8029 | 165f70259880ac56b50ed528e03817e19fc374a3831e08bdf6cf212769f7b5d3 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.csv | 788 | 09d6bd55dfd3f31c912e4aeab458583844882bc9a04872543537213be5c592cc |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.jsonl | 2287 | 7cdd7cd276cbdd3dae1ecb3b6ee9162167c9b169c158d04f4a434aeb8e0398bb |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.metadata.json | 8029 | 165f70259880ac56b50ed528e03817e19fc374a3831e08bdf6cf212769f7b5d3 |
| telemetry/repeat-2--p64-c10-p10--hydra--get.csv | 546 | a0d33d5a0ad800fa31db599c78de2ba0ebd9bc40afbc46f8e23a1523f4d6e460 |
| telemetry/repeat-2--p64-c10-p10--hydra--get.jsonl | 1347 | 62386aa55d2e090942ac5e813c6c920a48e58e0f1693eeb2d603e32e21f61b6b |
| telemetry/repeat-2--p64-c10-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p64-c10-p10--hydra--set.csv | 545 | d92217f10ce56154611bea626304feea563a2ee6d954c303f9f5c0455ab012e6 |
| telemetry/repeat-2--p64-c10-p10--hydra--set.jsonl | 1346 | 64fcddfb165a8b2a2f5d711afa46d60b91bda00b68a549e8c1f43e8cad54dc1a |
| telemetry/repeat-2--p64-c10-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-2--p64-c10-p10--redis--get.csv | 371 | b7868c07669e096c462eddd71b0a4f2dbcb6fe475467e0a9a917954a4a12e781 |
| telemetry/repeat-2--p64-c10-p10--redis--get.jsonl | 450 | 37acd5ab10c2833bc79146a2862dade15775805ba222c7fa4f843e5ca0798b02 |
| telemetry/repeat-2--p64-c10-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-2--p64-c10-p10--redis--set.csv | 367 | c2442dc525932db2d41fa6cdfd46e5879d3ee2281a942afaa40d09d126d7a695 |
| telemetry/repeat-2--p64-c10-p10--redis--set.jsonl | 446 | 88ff6299901abbc48a994008f473418772f8ba7399669dbebe6c3c5e026f186b |
| telemetry/repeat-2--p64-c10-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.csv | 1302 | e8ad5564aa9188b9e70f7e02baa3f580ff2448d4c82d014aacc61ec51bdf0dd6 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.jsonl | 4576 | 7d31afccbc99bed0da143b6e2afe0d677fecb2cba44988546f871d6c8cbb0387 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.metadata.json | 8029 | b3e271611d6464cc5d8b007c7208475c4b46917c3d3c21b99e75bbede9619ab8 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.csv | 1301 | b1723852e1a89aed3ae09a79af959ad2a19b40cf87702b7f13868940d045ff57 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.jsonl | 4575 | b163fee0428be05efd352bbac44147d938bb9b41d704997f16742798c50daae3 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.metadata.json | 8029 | b3e271611d6464cc5d8b007c7208475c4b46917c3d3c21b99e75bbede9619ab8 |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.csv | 634 | 813228692cf2b7e7b1b1e9699ad7f061bd277bdb01bf7e6317339d9f57730354 |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.jsonl | 1794 | d321fc921166ee2bce6c97225a329997904fc1a4a2394892dc07c9565d7fd749 |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.csv | 633 | 3c4db9359c68b12dd08f37d138b4805448863b51c51efdbb10f142ec1034a528 |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.jsonl | 1793 | 16b6d764fe7fc082268d45be5e946fe41bb4f60a01b94995e3d0d286b61e397a |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p1024-c50-p1--redis--get.csv | 561 | 8eaffa3b5f1ae1556fbc0a5a0fb5fddf2acba373b04b407a38528843d4152ee9 |
| telemetry/repeat-3--p1024-c50-p1--redis--get.jsonl | 1350 | e5bcd692c48e73bd568ffb3dbfd823e2ab1cae575cdd80b8276d9dcc5192df9d |
| telemetry/repeat-3--p1024-c50-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.csv | 558 | a0e8181e50ecde2fdbc67898257dc0195cc403279b5302f6548edd3141110797 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.jsonl | 1347 | 58e34b58593472cb0523ee7c8f51d974116264d3dc7d3471e958fc0d551e9891 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.csv | 788 | f5c156b61d8194e75ad3b2369b216c290ec3f3f46c49ebb1255b6939d89b0a97 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.jsonl | 2287 | 32b72ea6d0eeae5b849c113c54c552c69a15c7d535906700acadf6af936ce347 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.metadata.json | 8029 | f9a0e63912e38b82889e966bc097bd56f01adf731a94cda18a8b2374f4982f95 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.csv | 786 | cb575345f3182f0e2d16a1575a070d34819196e8f601b3464d7722c7fe0e6160 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.jsonl | 2285 | 9b69be1f77b590d50accdded9e2bfb69db930c2ede82b5243e22d5bb11ab8f07 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.metadata.json | 8029 | f9a0e63912e38b82889e966bc097bd56f01adf731a94cda18a8b2374f4982f95 |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.csv | 545 | 6ce331f038626cdfdb243288e90e8c76b1b64cc26a719d5563d89e3f42f49957 |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.jsonl | 1346 | 8880fa8741ba51185a7ce84b4ae01f76933bc699461ac768d65dbbcb68393dae |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.csv | 545 | d52e8555b35e2bfc522145c201abeab1f14e1a11c97e8d7560f0124897d9d30d |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.jsonl | 1346 | 92ecb211b55ce2db9c55018f681d4b89858ad926138fe2c3d0e03e263a568f78 |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p1024-c50-p10--redis--get.csv | 368 | 1a65552d3df7cfb51496f37dc6d725836a7956b7fa98bc3568de6c6441601904 |
| telemetry/repeat-3--p1024-c50-p10--redis--get.jsonl | 447 | f2b1db96cd036493040e63566bedf0a5f1fa66b734f7345323e127f213f22d7c |
| telemetry/repeat-3--p1024-c50-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p1024-c50-p10--redis--set.csv | 367 | 641c04732f3e7af1fee5026a19ffe5568e76906c667511fdecf1b014919ac8ef |
| telemetry/repeat-3--p1024-c50-p10--redis--set.jsonl | 446 | 5976afea63a327f2f3cdfd54ee5425fc87098dadaa5379efa6b68912315cf401 |
| telemetry/repeat-3--p1024-c50-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.csv | 2122 | bcc514af5151683fdb8022d96b129b7f530d51aca081c4d7cf3828bfdd18b342 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.jsonl | 8236 | 5674a83843b9637c53410aa098d46eb92cf416f21f7dc5fd263cd086978ac4cc |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.metadata.json | 8029 | f4087c80783a06716f03a44e4cdc6a6125eee635f204533b6167cb4d572c5b9e |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.csv | 2019 | 03a6b5c87c26e7d2fac3dde6637d0dbb19a72396d8fd37e1c6e82b56f8e6fb72 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.jsonl | 7778 | faa40a4352f253dd25e1cbca89b58a57c8a732251647da6d13e8bfdb8f4aa91a |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.metadata.json | 8029 | 2c35ef063bffdfa412ee2d0a93ea6a956b10798c64b7e950be80c018ae4ede77 |
| telemetry/repeat-3--p256-c1-p1--hydra--get.csv | 723 | 9df91803b95d16b8a4f9f5e01b6a7e51c3839592a68ac44e5de4eeab43614493 |
| telemetry/repeat-3--p256-c1-p1--hydra--get.jsonl | 2242 | ca1bfb26694a6b6b67e7efac1784e2e555917d70f5ff10d2c4d5bf06667851fb |
| telemetry/repeat-3--p256-c1-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c1-p1--hydra--set.csv | 726 | 53676360f48d72490e26b137bc0a99061d501d5fb137959f87f0c1d84a839280 |
| telemetry/repeat-3--p256-c1-p1--hydra--set.jsonl | 2245 | b9163f36e1b1d5a9a990e72249cc107982383aa5f07cb9e1f8ec20e6ce6567d7 |
| telemetry/repeat-3--p256-c1-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c1-p1--redis--get.csv | 744 | 60ae4a3d0037787b4365b0febce381ac8d320d5fb727bf13486058ed9a6b0cdc |
| telemetry/repeat-3--p256-c1-p1--redis--get.jsonl | 2243 | 381905cf85c8ea9702af83222706a1e640f32de5b0f5446391873f5789f7157b |
| telemetry/repeat-3--p256-c1-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c1-p1--redis--set.csv | 745 | ea294907249cbe3f7a6a68a45233f7ba50dfe37f7ad143c1ab02ed740575ce07 |
| telemetry/repeat-3--p256-c1-p1--redis--set.jsonl | 2244 | a7f7c93bdfb15a5c4747e01c396a86861fca4b5214107fac8b452a1248e11794 |
| telemetry/repeat-3--p256-c1-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.csv | 8272 | c7bb0abba81a5752911b01fed1c175b711f1836f1ac8f46426e8b1f2cc2fcb41 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.jsonl | 35686 | 430706e2a254b9960bd947feb7924e3c7f948140cd8c9de939af6c28556efed9 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.metadata.json | 8029 | e892c8e978a4976441ffe9f97329043ba2a37cef9a3dc14b8c2ca3b87fd9bc54 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.csv | 7354 | afed90196e12b362ac9f64b6cd62d5d9e1db96113b007fd12ea861e7fe1c76f7 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.jsonl | 31573 | f49512f09ed44e91af6475cf6d742fced5cb7a49f13926bf493b9537cd44bdc2 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.metadata.json | 8029 | 8a122b8c2608ce935be46ab1d1a83c3eb6102e6b73616a4603266e6c10f5cff7 |
| telemetry/repeat-3--p256-c10-p1--hydra--get.csv | 632 | 887044e37c9217bbf8f5de12d801891142aaa77be5e6c4d87dfc06c39b4835b9 |
| telemetry/repeat-3--p256-c10-p1--hydra--get.jsonl | 1792 | 392cc30f1be7218a0fa0e45f60a82ba4af7ed779120fad3fc955aba20452f23b |
| telemetry/repeat-3--p256-c10-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c10-p1--hydra--set.csv | 635 | eb1e840813869e1fbc96ae7604c1b4a8521d694bcb58267fb33483f0699df8f9 |
| telemetry/repeat-3--p256-c10-p1--hydra--set.jsonl | 1795 | 1f8e2a2703c73f6e0c8aebaf6980e8cd3d94aabdbfdcd8bb1561454331f414ca |
| telemetry/repeat-3--p256-c10-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c10-p1--redis--get.csv | 558 | cf57a3dd74682f9fe543a243fb01394d8803752b3caf9e552de71771e3f0cda8 |
| telemetry/repeat-3--p256-c10-p1--redis--get.jsonl | 1347 | 015d4be67d18c158f62021552f151b6148821f98757b89f4128e81677fd374d7 |
| telemetry/repeat-3--p256-c10-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c10-p1--redis--set.csv | 556 | 33e7b69d7f67220ca48279f55aac2a716fc63a9c0399e112ce8eb2cf9796da2e |
| telemetry/repeat-3--p256-c10-p1--redis--set.jsonl | 1345 | d49b6394cf87761cdaef4beeffc5308f119041ca10f53c79b39a345508a33cc1 |
| telemetry/repeat-3--p256-c10-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.csv | 1399 | e5a522eb306a01efa8899ec19c8dff1e6a453232cd973072b1253db22de60086 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.jsonl | 5028 | 8269cb8a371d699e88e678ed3b38188cfc95a0fe986f89941354e79a3aef7989 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.metadata.json | 8029 | 8842b5c83a670fb05716a403812941deec8b8ae59310eef9baccd8b0e87f94e0 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.csv | 787 | eca806f6c47fea4cdd1b51af59b04ed82ecc8544416571eeaa4e0006bdcff21a |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.jsonl | 2286 | ffadb40a8c39e047f4a5e82a3b763643ee7b65b1d4d1617937ce49eff8942542 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.metadata.json | 8029 | 8842b5c83a670fb05716a403812941deec8b8ae59310eef9baccd8b0e87f94e0 |
| telemetry/repeat-3--p256-c10-p10--hydra--get.csv | 546 | 445cda411e65670dd7c3a9a9cd40bbed27967edf99616821028cca11b204f1ff |
| telemetry/repeat-3--p256-c10-p10--hydra--get.jsonl | 1347 | d3effcc0a82fada421a29e603e8b90fa62a4747fb964bc57a37924ad4da5546a |
| telemetry/repeat-3--p256-c10-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c10-p10--hydra--set.csv | 545 | 3b98bb0deb819c94e0331c8b59d6d31781ad8ecb6e3e64c8bf056735e7a29dfb |
| telemetry/repeat-3--p256-c10-p10--hydra--set.jsonl | 1346 | 9d8c36c6781833586726413b6e9438631f49f0c382e2115a52481d1ca6855160 |
| telemetry/repeat-3--p256-c10-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c10-p10--redis--get.csv | 367 | 8da43f399b68e02fff88139b67aa90c6817d310c2b51b312aba6e8dd44d2ae52 |
| telemetry/repeat-3--p256-c10-p10--redis--get.jsonl | 446 | 11ff9ade5bf84cc878ff7ce992a29f0a038530831d5cc63173bafcb51bca9665 |
| telemetry/repeat-3--p256-c10-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c10-p10--redis--set.csv | 370 | 37ab6036f09561dfcf01de28a38c3cb7c7ec2953b4e40282471fb828c35b0949 |
| telemetry/repeat-3--p256-c10-p10--redis--set.jsonl | 449 | 2f0b59881bfc2bae570e28926ffafe59548e37046b98cbb5123d941aa1228924 |
| telemetry/repeat-3--p256-c10-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.csv | 1103 | 4cbaee31606251bb401018734ca698c477d60b67a4ca641c78dcff6435f47758 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.jsonl | 3667 | 5eb1a0654cfc3fe5ca7f92da943d74f8ed061e2b35b9503f740dcb0c5209c6f5 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.metadata.json | 8029 | 8d7b4c9dc3b4580605f025c18d07f7e52558be9127c295ce115efaff714193a3 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.csv | 1099 | 1287de3232be4b68da9f13d43e7b97b6c4c244f61d11226bdbbe0d1ab054aa97 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.jsonl | 3663 | e05338696501c089f2ecc4667e09d63cdffb0949125b1d74d519b42247096aeb |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.metadata.json | 8029 | f4087c80783a06716f03a44e4cdc6a6125eee635f204533b6167cb4d572c5b9e |
| telemetry/repeat-3--p256-c100-p1--hydra--get.csv | 635 | c2525b3c9446c89e4fc99db662b9703784eead5a19789ceea569fa5f4555615d |
| telemetry/repeat-3--p256-c100-p1--hydra--get.jsonl | 1795 | 4b025654b8fcdbef95e0d371ab56598c22e494ea0dd7f3e2ab6a372203d677d7 |
| telemetry/repeat-3--p256-c100-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c100-p1--hydra--set.csv | 635 | a53bd683a16124e9619afe826f5f15d4775d2de0a3461ca6f71a06584e6b33ba |
| telemetry/repeat-3--p256-c100-p1--hydra--set.jsonl | 1795 | ce5228ca0dec0abe7c54668b365caa193cda55fa4f398fe0a9e882ed6097eaad |
| telemetry/repeat-3--p256-c100-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p256-c100-p1--redis--get.csv | 555 | e8b55c26a9e3c1c527c095efebf65cb4293bec1fe24f8eafbf69a93c7dea06f2 |
| telemetry/repeat-3--p256-c100-p1--redis--get.jsonl | 1344 | 0b592208c2e090967443bd1ec701ec1b8a12185641e9eda5adb9fd74143d4fda |
| telemetry/repeat-3--p256-c100-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p256-c100-p1--redis--set.csv | 554 | d6e647801e9f507a110f47bbddcff7d4caefb2fc010f6572aed35b759a8be595 |
| telemetry/repeat-3--p256-c100-p1--redis--set.jsonl | 1343 | 6b3eae5b774e633d39838146c50066b161c2fb0f1dcf0d324c676fef2d2e540f |
| telemetry/repeat-3--p256-c100-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.csv | 8892 | a6e5694fcb250e6c987f14027819714de1431442ba68614b3a6342a46040d727 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.jsonl | 38436 | eab3beee0dc79ea9ca2e64c1dbe8c58125e51a2170d489d693fff493746eb6c7 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.metadata.json | 8028 | 6e982b409f82b8ccaf4811e2a2349812b950465064946421bcba66df95a7629f |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.csv | 6933 | 8d999a861c232c7c207c2aa2a63310b652dcf65e874160206254a109a0b1db81 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.jsonl | 29732 | 4ee018c3ab51c58a377854d744df08f0372fcf0f6a82e6ee088733279201f824 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.metadata.json | 8028 | 3e2a88c985ab1cec03b168828eb935b243fca80f8769730e5a855b6b8cb5edc1 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.csv | 635 | 87601aff154c999f71da5559a65f82c33e7b3dd9d7c27b0a6f25f6d0a9557de6 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.jsonl | 1795 | 387abb3691a603ac5b72739a932559dec791b1ca817d5f1a6f353982190eef30 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p64-c10-p1--hydra--set.csv | 634 | 45f1d4469aff5aa7ce6f118722565657ecc57e318ad4c7b3a44938dccf338fcc |
| telemetry/repeat-3--p64-c10-p1--hydra--set.jsonl | 1794 | 502a3fc61d49133c1bcf27ceb8287b27f3ef3b025af66b969ea9a4ed5e13eddf |
| telemetry/repeat-3--p64-c10-p1--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p64-c10-p1--redis--get.csv | 558 | b9eec1b310dc0d1bfd240da28ddaaaeaa0586b37fcd2689a5771617b7ddce902 |
| telemetry/repeat-3--p64-c10-p1--redis--get.jsonl | 1347 | 923972a078d6a84256f9020dba76f81da559326dff2d619c22ce9c57719fa083 |
| telemetry/repeat-3--p64-c10-p1--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p64-c10-p1--redis--set.csv | 557 | 5319bda7bdb011c9e017c45eda4f305958f02f043fdb8cd09a1ce75eb9b12b6e |
| telemetry/repeat-3--p64-c10-p1--redis--set.jsonl | 1346 | dcedaaed885e8ce3ed90467f6a1b904340f708d704d80d080ae71977a9d04224 |
| telemetry/repeat-3--p64-c10-p1--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.csv | 1399 | e0566a9b33a2deaf683b9b9fb74a3de2f5a1dad2e20b4f52cc8382e6567aee57 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.jsonl | 5028 | f476f1fb1f731b8e0ce54c394abccf2d8142567197ac3301f2929d13dd3becaa |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.metadata.json | 8029 | c240033f1e8a90bf96e4ba374c76bff05b3bc724b12f67265dc962dbd112562b |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.csv | 788 | c3f8519fc70efa40635418ee1284c2d1c41701184b0fcd8b2cb51937fa9942f7 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.jsonl | 2287 | 13a4ae72d2d1d688474a9c312736038bf908dadd3875faa2e86186906b83da9f |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.metadata.json | 8029 | c240033f1e8a90bf96e4ba374c76bff05b3bc724b12f67265dc962dbd112562b |
| telemetry/repeat-3--p64-c10-p10--hydra--get.csv | 544 | e8006662765c31ca7b41737e8f5ff21d34ee6ea3b5a08e85be77e367e1e27b82 |
| telemetry/repeat-3--p64-c10-p10--hydra--get.jsonl | 1345 | 299316909a677f8d5c20391ae5853522ca9e2dbd3b8514d16183f56326c0b2aa |
| telemetry/repeat-3--p64-c10-p10--hydra--get.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p64-c10-p10--hydra--set.csv | 546 | 8408b9df71b83a7af771353e3178ce9fad41fed61e4c069fe46e0c34c8b62657 |
| telemetry/repeat-3--p64-c10-p10--hydra--set.jsonl | 1347 | b01765304dff84e5a64de3540acecf0b239b45ed616abe792ae2390f415c783e |
| telemetry/repeat-3--p64-c10-p10--hydra--set.metadata.json | 153 | 03b2bd3260865d419aa291ecfc8194ed10eefe60e5b9ee98b2fcf8a3a0e65c6d |
| telemetry/repeat-3--p64-c10-p10--redis--get.csv | 367 | a78f00b57fed040bc5b9da85d40b7cf004df2e59ad66be2d91c03acb1ba7e6e7 |
| telemetry/repeat-3--p64-c10-p10--redis--get.jsonl | 446 | 73bfdf75a7ce8c6466d1381283b2f40fb6dc2eda2ef5771f4fecd0f627140567 |
| telemetry/repeat-3--p64-c10-p10--redis--get.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry/repeat-3--p64-c10-p10--redis--set.csv | 367 | 0ae1036da7138819bd68a9174f18cb85a00f8b07eb65d37e8d93edb6f9aa91c9 |
| telemetry/repeat-3--p64-c10-p10--redis--set.jsonl | 446 | 751ff1b728cefc9dc61a4fd6a4b75f64943b91cd4e8786ff4798686762ff9206 |
| telemetry/repeat-3--p64-c10-p10--redis--set.metadata.json | 7377 | 613d1c6aac6eeff5cc5116c558805611ee5de5560c55ab7a1671c149795dcf68 |
| telemetry-summary.json | 94159 | ff54ebc3ade344861075102422ead7fc6a57b4ec5afbd1760f69f247ab59e56e |

Raw benchmark logs, telemetry JSONL/CSV, Docker inspect metadata, image identifiers,
hardware validation, and the artifact manifest are all in this same output directory.
The directory must be copied unchanged into the branch results tree after review.
