# Relative eight-case telemetry report

> Exploratory only. This report is not qualification/bootstrap evidence.

- Generated (UTC): 2026-08-02T20:48:45.057959+00:00
- Source commit: 5530a28960aba2e21370d1d2d521c642afbc2c49
- Targets: HydraCache, Redis, Hazelcast Community
- Workload: 8 cases x SET/GET x configured repeats
- Sampling interval: 1 second by default

## Reproduction

The exact command and environment used for this run:

~~~text
branch=detached@5530a28960ab
source_commit=5530a28960aba2e21370d1d2d521c642afbc2c49
command=scripts/perf/run-relative-eight-cases-telemetry.sh /dev/shm/hydracache-exploratory-fixed-full-cpu4-20260802T202216Z
targets=hydracache,redis,hazelcast-community
hazelcast_image=hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90
hazelcast_client_version=5.5.0
measurement_affinity=4
requests_per_case=100000
repeats=3
telemetry_interval_seconds=1
~~~

Re-run from the recorded source commit with the same image digest, client version, affinity, request count, and repeats.

## Host and validation receipt

~~~text
reference evidence tmpfs verified: root=/dev/shm/hydracache-reference-evidence-v1
reference runtime IRQ guard passed: phase=relative-eight-telemetry-pre measurement=4 irq_files=113 dormant-unmapped-nvme=2
host=hydracache-perf-v1
source_commit=5530a28960aba2e21370d1d2d521c642afbc2c49
source_status=
kernel=Linux 6.8.0-136-generic x86_64 GNU/Linux
cpu_model=AMD EPYC 7232P 8-Core Processor
logical_cpus=4
measurement_affinity=4
targets=hydracache,redis,hazelcast-community
runner_receipt_sha256=97a39b307c063872b5c249eda9cf8341d70e0c293932b75bc67ae596cb0b17ae
runner_receipt=/var/lib/hydracache-perf/runner-provisioned.json
telemetry_interval_seconds=1
redis_benchmark=/usr/bin/redis-benchmark
redis_benchmark_version=redis-benchmark 7.0.15
hazelcast_image=hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90
hazelcast_client=5.5.0
container_affinity target=redis pid=30684 requested=4 effective=4
container_affinity target=hazelcast pid=30756 requested=4 effective=4
reference runtime IRQ delta baseline captured: phase=baseline measurement=4 file=/dev/shm/hydracache-exploratory-fixed-full-cpu4-20260802T202216Z/irq-baseline.tsv
irq_guard_mode=preflight-plus-baseline-delta
reference runtime IRQ delta guard passed: phase=post-relative-eight-telemetry measurement=4 monitored=2
~~~

## Telemetry summary

The summary preserves sample counts and reports p50/p95/max. Missing JVM heap fields remain unavailable; they are never inferred from RSS.

~~~json
{
  "repeat-1--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 374382592.0,
      "p50": 374161408.0,
      "p95": 374368256.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 14.0254,
      "p50": 11.8086,
      "p95": 13.831949999999999,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 393498624.0,
      "p50": 393498624.0,
      "p95": 393498624.0,
      "samples": 11
    }
  },
  "repeat-1--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 373919744.0,
      "p50": 373714944.0,
      "p95": 373914214.4,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 23.824,
      "p50": 16.36495,
      "p95": 21.205134999999995,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 393154560.0,
      "p50": 393011200.0,
      "p95": 393154560.0,
      "samples": 10
    }
  },
  "repeat-1--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 155357184.0,
      "p50": 155195392.0,
      "p95": 155340595.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 176963584.0,
      "p50": 176963584.0,
      "p95": 176963584.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 146219008.0,
      "p50": 146219008.0,
      "p95": 146219008.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 146219008.0,
      "p50": 146219008.0,
      "p95": 146219008.0,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 151691264.0,
      "p50": 140488704.0,
      "p95": 150574899.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 151715840.0,
      "p50": 141979648.0,
      "p95": 150600089.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 142831616.0,
      "p50": 131725312.0,
      "p95": 141739212.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 142831616.0,
      "p50": 131725312.0,
      "p95": 141739212.8,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 20189184.0,
      "p50": 19216384.0,
      "p95": 20189184.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20463616.0,
      "p95": 20471808.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 57.3825,
      "p50": 43.815799999999996,
      "p95": 55.35956999999999,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27230208.0,
      "p50": 27138048.0,
      "p95": 27230208.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 27230208.0,
      "p50": 26257408.0,
      "p95": 27230208.0,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 20176896.0,
      "p50": 20137984.0,
      "p95": 20176896.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20439040.0,
      "p50": 20396032.0,
      "p95": 20438425.6,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 50.799,
      "p50": 44.237350000000006,
      "p95": 49.86129,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27217920.0,
      "p50": 27179008.0,
      "p95": 27217920.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 27217920.0,
      "p50": 27179008.0,
      "p95": 27217920.0,
      "samples": 4
    }
  },
  "repeat-1--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375328768.0,
      "p50": 374779904.0,
      "p95": 375261184.0,
      "samples": 6
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 6
    },
    "container_cpu_percent": {
      "max": 22.5707,
      "p50": 15.612400000000001,
      "p95": 21.275,
      "samples": 6
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 6
    },
    "vmrss_bytes": {
      "max": 393895936.0,
      "p50": 393883648.0,
      "p95": 393895936.0,
      "samples": 6
    }
  },
  "repeat-1--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 374763520.0,
      "p50": 374222848.0,
      "p95": 374738124.8,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 29.3412,
      "p50": 23.8609,
      "p95": 28.30678,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 393723904.0,
      "p50": 393637888.0,
      "p95": 393723084.8,
      "samples": 5
    }
  },
  "repeat-1--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 181182464.0,
      "p50": 180924416.0,
      "p95": 181156659.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 204066816.0,
      "p50": 204066816.0,
      "p95": 204066816.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 171069440.0,
      "p50": 171069440.0,
      "p95": 171069440.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 171069440.0,
      "p50": 171069440.0,
      "p95": 171069440.0,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 179048448.0,
      "p50": 167874560.0,
      "p95": 177931059.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 179064832.0,
      "p50": 176963584.0,
      "p95": 178854707.2,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 168697856.0,
      "p50": 157540352.0,
      "p95": 167582105.6,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 168697856.0,
      "p50": 157540352.0,
      "p95": 167582105.6,
      "samples": 3
    }
  },
  "repeat-1--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 18714624.0,
      "p50": 18714624.0,
      "p95": 18714624.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25608192.0,
      "p50": 25608192.0,
      "p95": 25608192.0,
      "samples": 1
    }
  },
  "repeat-1--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 18464768.0,
      "p50": 18464768.0,
      "p95": 18464768.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25219072.0,
      "p50": 25219072.0,
      "p95": 25219072.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375508992.0,
      "p50": 374706176.0,
      "p95": 375132979.2,
      "samples": 18
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 18
    },
    "container_cpu_percent": {
      "max": 27.8577,
      "p50": 25.52715,
      "p95": 27.749665,
      "samples": 18
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 18
    },
    "vmrss_bytes": {
      "max": 394027008.0,
      "p50": 393785344.0,
      "p95": 394027008.0,
      "samples": 18
    }
  },
  "repeat-1--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 375181312.0,
      "p50": 374992896.0,
      "p95": 375145267.2,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 28.508,
      "p50": 24.7236,
      "p95": 26.092239999999997,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 394039296.0,
      "p50": 393961472.0,
      "p95": 393980313.6,
      "samples": 17
    }
  },
  "repeat-1--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 195514368.0,
      "p50": 195473408.0,
      "p95": 195514368.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 213082112.0,
      "p50": 213082112.0,
      "p95": 213082112.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 187166720.0,
      "p50": 187166720.0,
      "p95": 187166720.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 187166720.0,
      "p50": 187166720.0,
      "p95": 187166720.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 191393792.0,
      "p50": 184414208.0,
      "p95": 190498406.4,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 204103680.0,
      "p50": 204103680.0,
      "p95": 204103680.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 182861824.0,
      "p50": 175976448.0,
      "p95": 181979545.6,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 182861824.0,
      "p50": 175976448.0,
      "p95": 181979545.6,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7241728.0,
      "p50": 7241728.0,
      "p95": 7241728.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 48.4454,
      "p50": 45.1858,
      "p95": 47.8309,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14557184.0,
      "p50": 14557184.0,
      "p95": 14557184.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19410944.0,
      "p50": 19410944.0,
      "p95": 19410944.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 50.6785,
      "p50": 45.8111,
      "p95": 49.77108,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26726400.0,
      "p50": 26726400.0,
      "p95": 26726400.0,
      "samples": 5
    }
  },
  "repeat-1--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375029760.0,
      "p50": 371001344.0,
      "p95": 371434700.8,
      "samples": 80
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 80
    },
    "container_cpu_percent": {
      "max": 13.4925,
      "p50": 2.4867,
      "p95": 6.201129999999997,
      "samples": 80
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 80
    },
    "vmrss_bytes": {
      "max": 394440704.0,
      "p50": 390451200.0,
      "p95": 390717644.8,
      "samples": 80
    }
  },
  "repeat-1--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 371318784.0,
      "p50": 370708480.0,
      "p95": 371061555.2,
      "samples": 69
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 69
    },
    "container_cpu_percent": {
      "max": 10.7519,
      "p50": 2.7195,
      "p95": 6.168659999999999,
      "samples": 69
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 69
    },
    "vmrss_bytes": {
      "max": 390328320.0,
      "p50": 390144000.0,
      "p95": 390320128.0,
      "samples": 69
    }
  },
  "repeat-1--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 95526912.0,
      "p50": 95471616.0,
      "p95": 95519539.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 113717248.0,
      "p50": 113717248.0,
      "p95": 113717248.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 87793664.0,
      "p50": 87793664.0,
      "p95": 87793664.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 87793664.0,
      "p50": 87793664.0,
      "p95": 87793664.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 92045312.0,
      "p50": 81362944.0,
      "p95": 90999603.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 92045312.0,
      "p50": 87240704.0,
      "p95": 91324620.8,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 84537344.0,
      "p50": 73752576.0,
      "p95": 83463372.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 84537344.0,
      "p50": 73752576.0,
      "p95": 83463372.8,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7200768.0,
      "p50": 7200768.0,
      "p95": 7200768.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 8167424.0,
      "p50": 8167424.0,
      "p95": 8167424.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.5492,
      "p50": 43.335449999999994,
      "p95": 43.520849999999996,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 15073280.0,
      "p50": 15073280.0,
      "p95": 15073280.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 14495744.0,
      "p50": 14495744.0,
      "p95": 14495744.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7905280.0,
      "p50": 7845888.0,
      "p95": 7905280.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 8167424.0,
      "p50": 8105984.0,
      "p95": 8167424.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 44.4036,
      "p50": 44.074749999999995,
      "p95": 44.36109,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 15200256.0,
      "p50": 15140864.0,
      "p95": 15200256.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15200256.0,
      "p50": 15140864.0,
      "p95": 15200256.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 372068352.0,
      "p50": 371965952.0,
      "p95": 372027392.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 14.3184,
      "p50": 9.0136,
      "p95": 12.6388,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 391303168.0,
      "p50": 391290880.0,
      "p95": 391303168.0,
      "samples": 11
    }
  },
  "repeat-1--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 371941376.0,
      "p50": 371580928.0,
      "p95": 371834880.0,
      "samples": 9
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 9
    },
    "container_cpu_percent": {
      "max": 26.2292,
      "p50": 9.2536,
      "p95": 24.67656,
      "samples": 9
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 9
    },
    "vmrss_bytes": {
      "max": 390893568.0,
      "p50": 390848512.0,
      "p95": 390893568.0,
      "samples": 9
    }
  },
  "repeat-1--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 120659968.0,
      "p50": 120631296.0,
      "p95": 120657100.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 139087872.0,
      "p50": 139087872.0,
      "p95": 139087872.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 112594944.0,
      "p50": 112594944.0,
      "p95": 112594944.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 112594944.0,
      "p50": 112594944.0,
      "p95": 112594944.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 119390208.0,
      "p50": 107393024.0,
      "p95": 118190489.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 119406592.0,
      "p50": 114171904.0,
      "p95": 118883123.2,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 111308800.0,
      "p50": 99696640.0,
      "p95": 110147584.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 111308800.0,
      "p50": 99696640.0,
      "p95": 110147584.0,
      "samples": 3
    }
  },
  "repeat-1--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7180288.0,
      "p50": 7180288.0,
      "p95": 7180288.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 8167424.0,
      "p50": 8167424.0,
      "p95": 8167424.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 73.5373,
      "p50": 73.5373,
      "p95": 73.5373,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 15073280.0,
      "p50": 15073280.0,
      "p95": 15073280.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14475264.0,
      "p50": 14475264.0,
      "p95": 14475264.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7180288.0,
      "p50": 7180288.0,
      "p95": 7180288.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 8167424.0,
      "p50": 8167424.0,
      "p95": 8167424.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 69.4542,
      "p50": 69.4542,
      "p95": 69.4542,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 15073280.0,
      "p50": 15073280.0,
      "p95": 15073280.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14475264.0,
      "p50": 14475264.0,
      "p95": 14475264.0,
      "samples": 1
    }
  },
  "repeat-1--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375336960.0,
      "p50": 374876160.0,
      "p95": 375239475.2,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 15.5371,
      "p50": 13.793099999999999,
      "p95": 15.190355,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 394035200.0,
      "p50": 394031104.0,
      "p95": 394033766.4,
      "samples": 8
    }
  },
  "repeat-1--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 375226368.0,
      "p50": 374870016.0,
      "p95": 375154688.0,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 19.4661,
      "p50": 15.0744,
      "p95": 18.648955,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 393916416.0,
      "p50": 393871360.0,
      "p95": 393916416.0,
      "samples": 8
    }
  },
  "repeat-1--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 224616448.0,
      "p50": 224344064.0,
      "p95": 224594944.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 248209408.0,
      "p50": 248209408.0,
      "p95": 248209408.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 214331392.0,
      "p50": 214331392.0,
      "p95": 214331392.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 214331392.0,
      "p50": 214331392.0,
      "p95": 214331392.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 221024256.0,
      "p50": 209956864.0,
      "p95": 219918950.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 221401088.0,
      "p50": 213626880.0,
      "p95": 220280422.4,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 211099648.0,
      "p50": 199872512.0,
      "p95": 209976524.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 211099648.0,
      "p50": 199872512.0,
      "p95": 209976524.8,
      "samples": 4
    }
  },
  "repeat-1--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 8806400.0,
      "p50": 8798208.0,
      "p95": 8805785.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.1056,
      "p50": 42.73055,
      "p95": 43.05352,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15564800.0,
      "p50": 15558656.0,
      "p95": 15564800.0,
      "samples": 4
    }
  },
  "repeat-1--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8785920.0,
      "p50": 8617984.0,
      "p95": 8775475.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.9633,
      "p50": 43.36045,
      "p95": 43.874035,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15544320.0,
      "p50": 15509504.0,
      "p95": 15544320.0,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 368500736.0,
      "p50": 366485504.0,
      "p95": 368250060.8,
      "samples": 82
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 82
    },
    "container_cpu_percent": {
      "max": 149.3628,
      "p50": 2.71525,
      "p95": 10.429285,
      "samples": 82
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 82
    },
    "vmrss_bytes": {
      "max": 387633152.0,
      "p50": 385837056.0,
      "p95": 387600384.0,
      "samples": 82
    }
  },
  "repeat-1--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 350269440.0,
      "p50": 339443712.0,
      "p95": 349974528.0,
      "samples": 61
    },
    "cgroup_memory_peak_bytes": {
      "max": 354164736.0,
      "p50": 345378816.0,
      "p95": 354164736.0,
      "samples": 61
    },
    "container_cpu_percent": {
      "max": 158.08,
      "p50": 4.1465,
      "p95": 27.5096,
      "samples": 61
    },
    "vmhwm_bytes": {
      "max": 372604928.0,
      "p50": 364417024.0,
      "p95": 372604928.0,
      "samples": 61
    },
    "vmrss_bytes": {
      "max": 369561600.0,
      "p50": 358629376.0,
      "p95": 369397760.0,
      "samples": 61
    }
  },
  "repeat-1--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 43442176.0,
      "p50": 43382784.0,
      "p95": 43437260.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 61509632.0,
      "p50": 61509632.0,
      "p95": 61509632.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 36098048.0,
      "p50": 36098048.0,
      "p95": 36098048.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 36098048.0,
      "p50": 36098048.0,
      "p95": 36098048.0,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 39260160.0,
      "p50": 28485632.0,
      "p95": 38204620.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 39260160.0,
      "p50": 30787584.0,
      "p95": 38204620.8,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 32976896.0,
      "p50": 22091776.0,
      "p95": 31896166.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 32976896.0,
      "p50": 22091776.0,
      "p95": 31896166.4,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 4734976.0,
      "p50": 4734976.0,
      "p95": 4734976.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 5025792.0,
      "p50": 5025792.0,
      "p95": 5025792.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 44.5199,
      "p50": 43.87295,
      "p95": 44.51216,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 12034048.0,
      "p50": 12034048.0,
      "p95": 12034048.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 12034048.0,
      "p50": 12034048.0,
      "p95": 12034048.0,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 4763648.0,
      "p50": 4743168.0,
      "p95": 4763648.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 5025792.0,
      "p50": 5003264.0,
      "p95": 5025792.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 44.2219,
      "p50": 43.72045,
      "p95": 44.20234,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 12062720.0,
      "p50": 12042240.0,
      "p95": 12062720.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 12062720.0,
      "p50": 12042240.0,
      "p95": 12062720.0,
      "samples": 4
    }
  },
  "repeat-1--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 370475008.0,
      "p50": 370110464.0,
      "p95": 370461491.2,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 12.442,
      "p50": 8.903300000000002,
      "p95": 12.3364,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 389619712.0,
      "p50": 389613568.0,
      "p95": 389617459.2,
      "samples": 12
    }
  },
  "repeat-1--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 369512448.0,
      "p50": 369360896.0,
      "p95": 369488691.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 36.4846,
      "p50": 29.4988,
      "p95": 35.49216,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 388886528.0,
      "p50": 388730880.0,
      "p95": 388857856.0,
      "samples": 5
    }
  },
  "repeat-1--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 68444160.0,
      "p50": 68423680.0,
      "p95": 68442112.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 87158784.0,
      "p50": 87158784.0,
      "p95": 87158784.0,
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
      "max": 67530752.0,
      "p50": 55504896.0,
      "p95": 66328166.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 67538944.0,
      "p50": 61849600.0,
      "p95": 66970009.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 60166144.0,
      "p50": 48295936.0,
      "p95": 58979123.2,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 60166144.0,
      "p50": 48295936.0,
      "p95": 58979123.2,
      "samples": 3
    }
  },
  "repeat-1--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 4734976.0,
      "p50": 4734976.0,
      "p95": 4734976.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 5025792.0,
      "p50": 5025792.0,
      "p95": 5025792.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 58.4974,
      "p50": 58.4974,
      "p95": 58.4974,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 12034048.0,
      "p50": 12034048.0,
      "p95": 12034048.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12034048.0,
      "p50": 12034048.0,
      "p95": 12034048.0,
      "samples": 1
    }
  },
  "repeat-1--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 4734976.0,
      "p50": 4734976.0,
      "p95": 4734976.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 5025792.0,
      "p50": 5025792.0,
      "p95": 5025792.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 63.3108,
      "p50": 63.3108,
      "p95": 63.3108,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 12034048.0,
      "p50": 12034048.0,
      "p95": 12034048.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12034048.0,
      "p50": 12034048.0,
      "p95": 12034048.0,
      "samples": 1
    }
  },
  "repeat-2--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377143296.0,
      "p50": 376655872.0,
      "p95": 377032704.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 381480960.0,
      "p50": 381480960.0,
      "p95": 381480960.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 23.5154,
      "p50": 12.4913,
      "p95": 21.89545,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 395948032.0,
      "p50": 395894784.0,
      "p95": 395948032.0,
      "samples": 11
    }
  },
  "repeat-2--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 376745984.0,
      "p50": 376332288.0,
      "p95": 376626380.8,
      "samples": 9
    },
    "cgroup_memory_peak_bytes": {
      "max": 378601472.0,
      "p50": 378601472.0,
      "p95": 378601472.0,
      "samples": 9
    },
    "container_cpu_percent": {
      "max": 21.6992,
      "p50": 15.4623,
      "p95": 20.02596,
      "samples": 9
    },
    "vmhwm_bytes": {
      "max": 397766656.0,
      "p50": 397766656.0,
      "p95": 397766656.0,
      "samples": 9
    },
    "vmrss_bytes": {
      "max": 395472896.0,
      "p50": 395464704.0,
      "p95": 395469619.2,
      "samples": 9
    }
  },
  "repeat-2--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 355758080.0,
      "p50": 355418112.0,
      "p95": 355707699.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 376950784.0,
      "p50": 376950784.0,
      "p95": 376950784.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 345047040.0,
      "p50": 345047040.0,
      "p95": 345047040.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 345047040.0,
      "p50": 345047040.0,
      "p95": 345047040.0,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 352305152.0,
      "p50": 341196800.0,
      "p95": 351192473.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 352342016.0,
      "p50": 342845440.0,
      "p95": 351270502.4,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 341811200.0,
      "p50": 330741760.0,
      "p95": 340721868.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 341811200.0,
      "p50": 330741760.0,
      "p95": 340721868.8,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 19034112.0,
      "p50": 19034112.0,
      "p95": 19034112.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 61.0503,
      "p50": 43.8124,
      "p95": 58.47203999999999,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 26075136.0,
      "p50": 26075136.0,
      "p95": 26075136.0,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 20303872.0,
      "p50": 20277248.0,
      "p95": 20303872.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20537344.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 44.7334,
      "p50": 43.8156,
      "p95": 44.596315000000004,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27344896.0,
      "p50": 27318272.0,
      "p95": 27344896.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 27344896.0,
      "p50": 27318272.0,
      "p95": 27344896.0,
      "samples": 4
    }
  },
  "repeat-2--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377294848.0,
      "p50": 377061376.0,
      "p95": 377290752.0,
      "samples": 6
    },
    "cgroup_memory_peak_bytes": {
      "max": 381480960.0,
      "p50": 381480960.0,
      "p95": 381480960.0,
      "samples": 6
    },
    "container_cpu_percent": {
      "max": 17.0984,
      "p50": 14.82565,
      "p95": 16.7724,
      "samples": 6
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 6
    },
    "vmrss_bytes": {
      "max": 396124160.0,
      "p50": 396118016.0,
      "p95": 396123136.0,
      "samples": 6
    }
  },
  "repeat-2--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377167872.0,
      "p50": 376856576.0,
      "p95": 377119539.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 381480960.0,
      "p50": 381480960.0,
      "p95": 381480960.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 25.1181,
      "p50": 22.1326,
      "p95": 24.610979999999998,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 396013568.0,
      "p50": 396009472.0,
      "p95": 396013568.0,
      "samples": 5
    }
  },
  "repeat-2--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 381906944.0,
      "p50": 381775872.0,
      "p95": 381893836.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 404877312.0,
      "p50": 404877312.0,
      "p95": 404877312.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 370053120.0,
      "p50": 370053120.0,
      "p95": 370053120.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 370053120.0,
      "p50": 370053120.0,
      "p95": 370053120.0,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 379887616.0,
      "p50": 368713728.0,
      "p95": 378770227.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 379887616.0,
      "p50": 377085952.0,
      "p95": 379607449.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 367648768.0,
      "p50": 356503552.0,
      "p95": 366534246.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 367648768.0,
      "p50": 356503552.0,
      "p95": 366534246.4,
      "samples": 3
    }
  },
  "repeat-2--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 19079168.0,
      "p50": 19079168.0,
      "p95": 19079168.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 80.614,
      "p50": 80.614,
      "p95": 80.614,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25710592.0,
      "p50": 25710592.0,
      "p95": 25710592.0,
      "samples": 1
    }
  },
  "repeat-2--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 18644992.0,
      "p50": 18644992.0,
      "p95": 18644992.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 0.0,
      "p50": 0.0,
      "p95": 0.0,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25288704.0,
      "p50": 25288704.0,
      "p95": 25288704.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377466880.0,
      "p50": 377032704.0,
      "p95": 377293209.6,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381480960.0,
      "p95": 381825024.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 40.0288,
      "p50": 23.8708,
      "p95": 30.301519999999993,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 396218368.0,
      "p50": 396009472.0,
      "p95": 396218368.0,
      "samples": 17
    }
  },
  "repeat-2--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377458688.0,
      "p50": 377159680.0,
      "p95": 377360384.0,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 381480960.0,
      "p50": 381480960.0,
      "p95": 381480960.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 27.7485,
      "p50": 25.3907,
      "p95": 26.955859999999998,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 396251136.0,
      "p50": 396169216.0,
      "p95": 396247859.2,
      "samples": 17
    }
  },
  "repeat-2--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 396259328.0,
      "p50": 396079104.0,
      "p95": 396239667.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 413929472.0,
      "p50": 413929472.0,
      "p95": 413929472.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 386076672.0,
      "p50": 386076672.0,
      "p95": 386076672.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 386076672.0,
      "p50": 386076672.0,
      "p95": 386076672.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 391745536.0,
      "p50": 384684032.0,
      "p95": 390831308.8,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 405164032.0,
      "p50": 405164032.0,
      "p95": 405164032.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 381739008.0,
      "p50": 374968320.0,
      "p95": 380878028.8,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 381739008.0,
      "p50": 374968320.0,
      "p95": 380878028.8,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7266304.0,
      "p50": 7266304.0,
      "p95": 7266304.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 53.8613,
      "p50": 45.1886,
      "p95": 52.1581,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14581760.0,
      "p50": 14581760.0,
      "p95": 14581760.0,
      "samples": 5
    }
  },
  "repeat-2--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19341312.0,
      "p50": 19169280.0,
      "p95": 19306905.6,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 53.1052,
      "p50": 45.8377,
      "p95": 51.70912,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26656768.0,
      "p50": 26484736.0,
      "p95": 26622361.6,
      "samples": 5
    }
  },
  "repeat-2--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 376209408.0,
      "p50": 375267328.0,
      "p95": 375640064.0,
      "samples": 81
    },
    "cgroup_memory_peak_bytes": {
      "max": 378601472.0,
      "p50": 378249216.0,
      "p95": 378601472.0,
      "samples": 81
    },
    "container_cpu_percent": {
      "max": 10.4416,
      "p50": 2.4045,
      "p95": 5.7871,
      "samples": 81
    },
    "vmhwm_bytes": {
      "max": 397766656.0,
      "p50": 396472320.0,
      "p95": 397766656.0,
      "samples": 81
    },
    "vmrss_bytes": {
      "max": 394788864.0,
      "p50": 394711040.0,
      "p95": 394788864.0,
      "samples": 81
    }
  },
  "repeat-2--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377225216.0,
      "p50": 375664640.0,
      "p95": 375984947.2,
      "samples": 69
    },
    "cgroup_memory_peak_bytes": {
      "max": 378249216.0,
      "p50": 378249216.0,
      "p95": 378249216.0,
      "samples": 69
    },
    "container_cpu_percent": {
      "max": 8.0565,
      "p50": 2.4382,
      "p95": 5.8977,
      "samples": 69
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 69
    },
    "vmrss_bytes": {
      "max": 395169792.0,
      "p50": 394903552.0,
      "p95": 395169792.0,
      "samples": 69
    }
  },
  "repeat-2--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 296386560.0,
      "p50": 296192000.0,
      "p95": 296357683.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 314376192.0,
      "p50": 314376192.0,
      "p95": 314376192.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 286830592.0,
      "p50": 286830592.0,
      "p95": 286830592.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 286830592.0,
      "p50": 286830592.0,
      "p95": 286830592.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 292651008.0,
      "p50": 282021888.0,
      "p95": 291596697.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 292671488.0,
      "p50": 288169984.0,
      "p95": 291996262.4,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 283561984.0,
      "p50": 272773120.0,
      "p95": 282488012.8,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 283561984.0,
      "p50": 272773120.0,
      "p95": 282488012.8,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7413760.0,
      "p50": 7413760.0,
      "p95": 7413760.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 69.7504,
      "p50": 43.38805,
      "p95": 65.81115999999999,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 14680064.0,
      "p50": 14680064.0,
      "p95": 14680064.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8130560.0,
      "p50": 8083456.0,
      "p95": 8130560.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 44.508,
      "p50": 44.273849999999996,
      "p95": 44.50302,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15396864.0,
      "p50": 15349760.0,
      "p95": 15396864.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 376086528.0,
      "p50": 375832576.0,
      "p95": 376061952.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 378601472.0,
      "p50": 378601472.0,
      "p95": 378601472.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 10.4053,
      "p50": 9.3782,
      "p95": 10.367650000000001,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 397766656.0,
      "p50": 397766656.0,
      "p95": 397766656.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 395161600.0,
      "p50": 395161600.0,
      "p95": 395161600.0,
      "samples": 11
    }
  },
  "repeat-2--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 375840768.0,
      "p50": 375316480.0,
      "p95": 375785062.4,
      "samples": 9
    },
    "cgroup_memory_peak_bytes": {
      "max": 378601472.0,
      "p50": 378601472.0,
      "p95": 378601472.0,
      "samples": 9
    },
    "container_cpu_percent": {
      "max": 24.322,
      "p50": 8.7931,
      "p95": 23.015279999999997,
      "samples": 9
    },
    "vmhwm_bytes": {
      "max": 397766656.0,
      "p50": 397766656.0,
      "p95": 397766656.0,
      "samples": 9
    },
    "vmrss_bytes": {
      "max": 394866688.0,
      "p50": 394854400.0,
      "p95": 394863411.2,
      "samples": 9
    }
  },
  "repeat-2--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 321548288.0,
      "p50": 321224704.0,
      "p95": 321515929.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 340361216.0,
      "p50": 340361216.0,
      "p95": 340361216.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 311619584.0,
      "p50": 311619584.0,
      "p95": 311619584.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 311619584.0,
      "p50": 311619584.0,
      "p95": 311619584.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 319717376.0,
      "p50": 308379648.0,
      "p95": 318583603.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 319844352.0,
      "p50": 314974208.0,
      "p95": 319357337.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 310259712.0,
      "p50": 298688512.0,
      "p95": 309102592.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 310259712.0,
      "p50": 298688512.0,
      "p95": 309102592.0,
      "samples": 3
    }
  },
  "repeat-2--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7413760.0,
      "p50": 7413760.0,
      "p95": 7413760.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 96.1063,
      "p50": 96.1063,
      "p95": 96.1063,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14680064.0,
      "p50": 14680064.0,
      "p95": 14680064.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7413760.0,
      "p50": 7413760.0,
      "p95": 7413760.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 86.607,
      "p50": 86.607,
      "p95": 86.607,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14680064.0,
      "p50": 14680064.0,
      "p95": 14680064.0,
      "samples": 1
    }
  },
  "repeat-2--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377176064.0,
      "p50": 376899584.0,
      "p95": 377174630.4,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 15.1275,
      "p50": 13.412749999999999,
      "p95": 15.09117,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 396079104.0,
      "p50": 396079104.0,
      "p95": 396079104.0,
      "samples": 8
    }
  },
  "repeat-2--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377106432.0,
      "p50": 376948736.0,
      "p95": 377104998.4,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 17.5116,
      "p50": 15.85615,
      "p95": 17.228275,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 396025856.0,
      "p50": 396017664.0,
      "p95": 396025856.0,
      "samples": 8
    }
  },
  "repeat-2--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 425373696.0,
      "p50": 425332736.0,
      "p95": 425368166.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 449040384.0,
      "p50": 449040384.0,
      "p95": 449040384.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 413192192.0,
      "p50": 413192192.0,
      "p95": 413192192.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 413192192.0,
      "p50": 413192192.0,
      "p95": 413192192.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 421453824.0,
      "p50": 410630144.0,
      "p95": 420391526.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 421994496.0,
      "p50": 414291968.0,
      "p95": 420893491.2,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 409927680.0,
      "p50": 398747648.0,
      "p95": 408810086.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 409927680.0,
      "p50": 398747648.0,
      "p95": 408810086.4,
      "samples": 4
    }
  },
  "repeat-2--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 9355264.0,
      "p50": 9091072.0,
      "p95": 9316556.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.2816,
      "p50": 42.82335,
      "p95": 43.23813,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15855616.0,
      "p50": 15849472.0,
      "p95": 15855616.0,
      "samples": 4
    }
  },
  "repeat-2--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8773632.0,
      "p50": 8605696.0,
      "p95": 8763187.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.8521,
      "p50": 43.3647,
      "p95": 43.7867,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15532032.0,
      "p50": 15497216.0,
      "p95": 15532032.0,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375521280.0,
      "p50": 374740992.0,
      "p95": 375193600.0,
      "samples": 86
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 86
    },
    "container_cpu_percent": {
      "max": 7.0075,
      "p50": 2.2830500000000002,
      "p95": 5.417375,
      "samples": 86
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 86
    },
    "vmrss_bytes": {
      "max": 394436608.0,
      "p50": 394182656.0,
      "p95": 394436608.0,
      "samples": 86
    }
  },
  "repeat-2--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 375177216.0,
      "p50": 374378496.0,
      "p95": 374992076.8,
      "samples": 69
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 69
    },
    "container_cpu_percent": {
      "max": 8.0577,
      "p50": 2.5177,
      "p95": 7.18772,
      "samples": 69
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 69
    },
    "vmrss_bytes": {
      "max": 394366976.0,
      "p50": 393900032.0,
      "p95": 394125312.0,
      "samples": 69
    }
  },
  "repeat-2--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 244137984.0,
      "p50": 243980288.0,
      "p95": 244117094.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 262426624.0,
      "p50": 262426624.0,
      "p95": 262426624.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 235155456.0,
      "p50": 235155456.0,
      "p95": 235155456.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 235155456.0,
      "p50": 235155456.0,
      "p95": 235155456.0,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 240746496.0,
      "p50": 230182912.0,
      "p95": 239685427.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 248365056.0,
      "p50": 248365056.0,
      "p95": 248365056.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 231993344.0,
      "p50": 221403136.0,
      "p95": 230918758.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 231993344.0,
      "p50": 221403136.0,
      "p95": 230918758.4,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5099520.0,
      "p50": 5099520.0,
      "p95": 5099520.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 53.5832,
      "p50": 43.485,
      "p95": 52.087309999999995,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 12365824.0,
      "p50": 12365824.0,
      "p95": 12365824.0,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8192000.0,
      "p50": 8187904.0,
      "p95": 8192000.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 44.0599,
      "p50": 43.7478,
      "p95": 44.013415,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15458304.0,
      "p50": 15454208.0,
      "p95": 15458304.0,
      "samples": 4
    }
  },
  "repeat-2--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 375869440.0,
      "p50": 375328768.0,
      "p95": 375834624.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 377819136.0,
      "p50": 377585664.0,
      "p95": 377819136.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 13.0578,
      "p50": 9.0419,
      "p95": 11.29275,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 394862592.0,
      "p50": 394833920.0,
      "p95": 394858496.0,
      "samples": 11
    }
  },
  "repeat-2--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 375214080.0,
      "p50": 375130112.0,
      "p95": 375206912.0,
      "samples": 6
    },
    "cgroup_memory_peak_bytes": {
      "max": 377585664.0,
      "p50": 377585664.0,
      "p95": 377585664.0,
      "samples": 6
    },
    "container_cpu_percent": {
      "max": 27.0111,
      "p50": 23.38225,
      "p95": 26.8337,
      "samples": 6
    },
    "vmhwm_bytes": {
      "max": 396472320.0,
      "p50": 396472320.0,
      "p95": 396472320.0,
      "samples": 6
    },
    "vmrss_bytes": {
      "max": 394514432.0,
      "p50": 394512384.0,
      "p95": 394514432.0,
      "samples": 6
    }
  },
  "repeat-2--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 269590528.0,
      "p50": 269557760.0,
      "p95": 269587251.2,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 288169984.0,
      "p50": 288169984.0,
      "p95": 288169984.0,
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
      "max": 268324864.0,
      "p50": 256262144.0,
      "p95": 267118592.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 268333056.0,
      "p50": 262819840.0,
      "p95": 267781734.4,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 259129344.0,
      "p50": 247291904.0,
      "p95": 257945600.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 259129344.0,
      "p50": 247291904.0,
      "p95": 257945600.0,
      "samples": 3
    }
  },
  "repeat-2--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5017600.0,
      "p50": 5017600.0,
      "p95": 5017600.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 25.4735,
      "p50": 25.4735,
      "p95": 25.4735,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12283904.0,
      "p50": 12283904.0,
      "p95": 12283904.0,
      "samples": 1
    }
  },
  "repeat-2--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 5038080.0,
      "p50": 5038080.0,
      "p95": 5038080.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20471808.0,
      "p50": 20471808.0,
      "p95": 20471808.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 63.9206,
      "p50": 63.9206,
      "p95": 63.9206,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27045888.0,
      "p50": 27045888.0,
      "p95": 27045888.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12304384.0,
      "p50": 12304384.0,
      "p95": 12304384.0,
      "samples": 1
    }
  },
  "repeat-3--p1024-c50-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378277888.0,
      "p50": 377921536.0,
      "p95": 378208256.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 15.4802,
      "p50": 12.7693,
      "p95": 15.26155,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 397217792.0,
      "p50": 397209600.0,
      "p95": 397217792.0,
      "samples": 11
    }
  },
  "repeat-3--p1024-c50-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377991168.0,
      "p50": 377733120.0,
      "p95": 377978265.6,
      "samples": 10
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 10
    },
    "container_cpu_percent": {
      "max": 19.7145,
      "p50": 14.248249999999999,
      "p95": 18.395729999999997,
      "samples": 10
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 10
    },
    "vmrss_bytes": {
      "max": 396926976.0,
      "p50": 396922880.0,
      "p95": 396926976.0,
      "samples": 10
    }
  },
  "repeat-3--p1024-c50-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 556560384.0,
      "p50": 556484608.0,
      "p95": 556551168.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 577957888.0,
      "p50": 577957888.0,
      "p95": 577957888.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 544100352.0,
      "p50": 544100352.0,
      "p95": 544100352.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 544100352.0,
      "p50": 544100352.0,
      "p95": 544100352.0,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 552783872.0,
      "p50": 541794304.0,
      "p95": 551696384.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 552812544.0,
      "p50": 543467520.0,
      "p95": 551761305.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 540696576.0,
      "p50": 529633280.0,
      "p95": 539608473.6,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 540696576.0,
      "p50": 529633280.0,
      "p95": 539608473.6,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 17752064.0,
      "p50": 17664000.0,
      "p95": 17752064.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.8381,
      "p50": 43.692,
      "p95": 43.818329999999996,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 24793088.0,
      "p50": 24705024.0,
      "p95": 24793088.0,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 20205568.0,
      "p50": 20172800.0,
      "p95": 20205568.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 67.4941,
      "p50": 44.227149999999995,
      "p95": 64.047325,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27246592.0,
      "p50": 27213824.0,
      "p95": 27246592.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 27246592.0,
      "p50": 27213824.0,
      "p95": 27246592.0,
      "samples": 4
    }
  },
  "repeat-3--p1024-c50-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378548224.0,
      "p50": 378052608.0,
      "p95": 378461184.0,
      "samples": 6
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 6
    },
    "container_cpu_percent": {
      "max": 17.0113,
      "p50": 16.25985,
      "p95": 16.875325,
      "samples": 6
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 6
    },
    "vmrss_bytes": {
      "max": 397287424.0,
      "p50": 397279232.0,
      "p95": 397286400.0,
      "samples": 6
    }
  },
  "repeat-3--p1024-c50-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 378155008.0,
      "p50": 377905152.0,
      "p95": 378151731.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 21.162,
      "p50": 21.0656,
      "p95": 21.160999999999998,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 397238272.0,
      "p50": 397225984.0,
      "p95": 397235814.4,
      "samples": 5
    }
  },
  "repeat-3--p1024-c50-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 582377472.0,
      "p50": 582291456.0,
      "p95": 582368870.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 605229056.0,
      "p50": 605229056.0,
      "p95": 605229056.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 568889344.0,
      "p50": 568889344.0,
      "p95": 568889344.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 568889344.0,
      "p50": 568889344.0,
      "p95": 568889344.0,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 580608000.0,
      "p50": 569651200.0,
      "p95": 579512320.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 580608000.0,
      "p50": 577957888.0,
      "p95": 580342988.8,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 566534144.0,
      "p50": 555384832.0,
      "p95": 565419212.8,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 566534144.0,
      "p50": 555384832.0,
      "p95": 565419212.8,
      "samples": 3
    }
  },
  "repeat-3--p1024-c50-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 18747392.0,
      "p50": 18747392.0,
      "p95": 18747392.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 72.8676,
      "p50": 72.8676,
      "p95": 72.8676,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25378816.0,
      "p50": 25378816.0,
      "p95": 25378816.0,
      "samples": 1
    }
  },
  "repeat-3--p1024-c50-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 18120704.0,
      "p50": 18120704.0,
      "p95": 18120704.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 74.8211,
      "p50": 74.8211,
      "p95": 74.8211,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 25161728.0,
      "p50": 25161728.0,
      "p95": 25161728.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c1-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378658816.0,
      "p50": 378335232.0,
      "p95": 378567065.6,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 26.5018,
      "p50": 23.8256,
      "p95": 24.95468,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 397426688.0,
      "p50": 397164544.0,
      "p95": 397426688.0,
      "samples": 17
    }
  },
  "repeat-3--p256-c1-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 378650624.0,
      "p50": 378441728.0,
      "p95": 378608025.6,
      "samples": 17
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 17
    },
    "container_cpu_percent": {
      "max": 24.6191,
      "p50": 23.5572,
      "p95": 24.55894,
      "samples": 17
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 17
    },
    "vmrss_bytes": {
      "max": 397389824.0,
      "p50": 397336576.0,
      "p95": 397363609.6,
      "samples": 17
    }
  },
  "repeat-3--p256-c1-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 597061632.0,
      "p50": 596996096.0,
      "p95": 597058355.2,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 614809600.0,
      "p50": 614809600.0,
      "p95": 614809600.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 584925184.0,
      "p50": 584925184.0,
      "p95": 584925184.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 584925184.0,
      "p50": 584925184.0,
      "p95": 584925184.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 592543744.0,
      "p50": 585732096.0,
      "p95": 591683584.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 605720576.0,
      "p50": 605720576.0,
      "p95": 605720576.0,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 580640768.0,
      "p50": 573833216.0,
      "p95": 579774054.4,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 580640768.0,
      "p50": 573833216.0,
      "p95": 579774054.4,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7245824.0,
      "p50": 7245824.0,
      "p95": 7245824.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 48.7094,
      "p50": 45.2021,
      "p95": 48.039280000000005,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 14561280.0,
      "p50": 14561280.0,
      "p95": 14561280.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c1-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 19279872.0,
      "p50": 18931712.0,
      "p95": 19279872.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 52.9746,
      "p50": 45.8971,
      "p95": 51.62882,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 26595328.0,
      "p50": 26247168.0,
      "p95": 26595328.0,
      "samples": 5
    }
  },
  "repeat-3--p256-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377356288.0,
      "p50": 376641536.0,
      "p95": 377070182.4,
      "samples": 78
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 78
    },
    "container_cpu_percent": {
      "max": 7.5191,
      "p50": 2.4434500000000003,
      "p95": 4.301874999999991,
      "samples": 78
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 78
    },
    "vmrss_bytes": {
      "max": 396210176.0,
      "p50": 396173312.0,
      "p95": 396206080.0,
      "samples": 78
    }
  },
  "repeat-3--p256-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377769984.0,
      "p50": 377036800.0,
      "p95": 377482444.8,
      "samples": 65
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 65
    },
    "container_cpu_percent": {
      "max": 10.9921,
      "p50": 2.736,
      "p95": 7.6798999999999955,
      "samples": 65
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 65
    },
    "vmrss_bytes": {
      "max": 397180928.0,
      "p50": 396394496.0,
      "p95": 396650905.6,
      "samples": 65
    }
  },
  "repeat-3--p256-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 496881664.0,
      "p50": 496816128.0,
      "p95": 496877977.6,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 514973696.0,
      "p50": 514973696.0,
      "p95": 514973696.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 485707776.0,
      "p50": 485707776.0,
      "p95": 485707776.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 485707776.0,
      "p50": 485707776.0,
      "p95": 485707776.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 493412352.0,
      "p50": 482873344.0,
      "p95": 492372787.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 493412352.0,
      "p50": 488902656.0,
      "p95": 492735897.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 482488320.0,
      "p50": 471662592.0,
      "p95": 481410662.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 482488320.0,
      "p50": 471662592.0,
      "p95": 481410662.4,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7421952.0,
      "p50": 7421952.0,
      "p95": 7421952.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 58.7812,
      "p50": 43.5034,
      "p95": 56.50082499999999,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 14688256.0,
      "p50": 14688256.0,
      "p95": 14688256.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8146944.0,
      "p50": 8089600.0,
      "p95": 8146944.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 53.3409,
      "p50": 44.18855,
      "p95": 51.97998,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15413248.0,
      "p50": 15355904.0,
      "p95": 15413248.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377507840.0,
      "p50": 377161728.0,
      "p95": 377458278.4,
      "samples": 12
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 12
    },
    "container_cpu_percent": {
      "max": 13.6789,
      "p50": 8.6613,
      "p95": 11.805819999999999,
      "samples": 12
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 12
    },
    "vmrss_bytes": {
      "max": 396623872.0,
      "p50": 396619776.0,
      "p95": 396623872.0,
      "samples": 12
    }
  },
  "repeat-3--p256-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377131008.0,
      "p50": 376950784.0,
      "p95": 377098240.0,
      "samples": 5
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 5
    },
    "container_cpu_percent": {
      "max": 29.9208,
      "p50": 28.139,
      "p95": 29.71648,
      "samples": 5
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 5
    },
    "vmrss_bytes": {
      "max": 396300288.0,
      "p50": 396296192.0,
      "p95": 396299468.8,
      "samples": 5
    }
  },
  "repeat-3--p256-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 521904128.0,
      "p50": 521863168.0,
      "p95": 521900032.0,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 540708864.0,
      "p50": 540708864.0,
      "p95": 540708864.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 510496768.0,
      "p50": 510496768.0,
      "p95": 510496768.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 510496768.0,
      "p50": 510496768.0,
      "p95": 510496768.0,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 520617984.0,
      "p50": 509001728.0,
      "p95": 519456358.4,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 520888320.0,
      "p50": 515629056.0,
      "p95": 520362393.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 509214720.0,
      "p50": 497598464.0,
      "p95": 508053094.4,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 509214720.0,
      "p50": 497598464.0,
      "p95": 508053094.4,
      "samples": 3
    }
  },
  "repeat-3--p256-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 7421952.0,
      "p50": 7421952.0,
      "p95": 7421952.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 104.6174,
      "p50": 104.6174,
      "p95": 104.6174,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14688256.0,
      "p50": 14688256.0,
      "p95": 14688256.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 7421952.0,
      "p50": 7421952.0,
      "p95": 7421952.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 36.9535,
      "p50": 36.9535,
      "p95": 36.9535,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 14688256.0,
      "p50": 14688256.0,
      "p95": 14688256.0,
      "samples": 1
    }
  },
  "repeat-3--p256-c100-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 378302464.0,
      "p50": 378007552.0,
      "p95": 378273792.0,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 15.9197,
      "p50": 14.61055,
      "p95": 15.63473,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 397242368.0,
      "p50": 397221888.0,
      "p95": 397242368.0,
      "samples": 8
    }
  },
  "repeat-3--p256-c100-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 378499072.0,
      "p50": 377917440.0,
      "p95": 378477568.0,
      "samples": 8
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 8
    },
    "container_cpu_percent": {
      "max": 19.1062,
      "p50": 15.07785,
      "p95": 17.959004999999998,
      "samples": 8
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 8
    },
    "vmrss_bytes": {
      "max": 397193216.0,
      "p50": 397193216.0,
      "p95": 397193216.0,
      "samples": 8
    }
  },
  "repeat-3--p256-c100-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 625758208.0,
      "p50": 625598464.0,
      "p95": 625741619.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 649388032.0,
      "p50": 649388032.0,
      "p95": 649388032.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 612024320.0,
      "p50": 612024320.0,
      "p95": 612024320.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 612024320.0,
      "p50": 612024320.0,
      "p95": 612024320.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c100-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 622309376.0,
      "p50": 611203072.0,
      "p95": 621226803.2,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 622616576.0,
      "p50": 614973440.0,
      "p95": 621494681.6,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 608878592.0,
      "p50": 597647360.0,
      "p95": 607754854.4,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 608878592.0,
      "p50": 597647360.0,
      "p95": 607754854.4,
      "samples": 4
    }
  },
  "repeat-3--p256-c100-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 8986624.0,
      "p50": 8986624.0,
      "p95": 8986624.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.1573,
      "p50": 42.794200000000004,
      "p95": 43.10696,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15745024.0,
      "p50": 15745024.0,
      "p95": 15745024.0,
      "samples": 4
    }
  },
  "repeat-3--p256-c100-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8970240.0,
      "p50": 8738816.0,
      "p95": 8940748.8,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 43.8609,
      "p50": 43.42775,
      "p95": 43.798785,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15532032.0,
      "p50": 15497216.0,
      "p95": 15532032.0,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 376815616.0,
      "p50": 376336384.0,
      "p95": 376631296.0,
      "samples": 83
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 83
    },
    "container_cpu_percent": {
      "max": 7.2588,
      "p50": 2.2654,
      "p95": 3.300259999999999,
      "samples": 83
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 83
    },
    "vmrss_bytes": {
      "max": 395743232.0,
      "p50": 395702272.0,
      "p95": 395742003.2,
      "samples": 83
    }
  },
  "repeat-3--p64-c10-p1--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 377241600.0,
      "p50": 376576000.0,
      "p95": 377126707.2,
      "samples": 70
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 70
    },
    "container_cpu_percent": {
      "max": 12.1371,
      "p50": 2.28815,
      "p95": 5.486389999999998,
      "samples": 70
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 70
    },
    "vmrss_bytes": {
      "max": 396439552.0,
      "p50": 395915264.0,
      "p95": 396169216.0,
      "samples": 70
    }
  },
  "repeat-3--p64-c10-p1--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 444870656.0,
      "p50": 444801024.0,
      "p95": 444868198.4,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 462995456.0,
      "p50": 462995456.0,
      "p95": 462995456.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 433999872.0,
      "p50": 433999872.0,
      "p95": 433999872.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 433999872.0,
      "p50": 433999872.0,
      "p95": 433999872.0,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 441556992.0,
      "p50": 430831616.0,
      "p95": 440460288.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 449257472.0,
      "p50": 449257472.0,
      "p95": 449257472.0,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 430858240.0,
      "p50": 420253696.0,
      "p95": 429783040.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 430858240.0,
      "p50": 420253696.0,
      "p95": 429783040.0,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5107712.0,
      "p50": 5107712.0,
      "p95": 5107712.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 76.0025,
      "p50": 43.37915,
      "p95": 71.12286499999999,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 12374016.0,
      "p50": 12374016.0,
      "p95": 12374016.0,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p1--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 8171520.0,
      "p50": 8171520.0,
      "p95": 8171520.0,
      "samples": 4
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 4
    },
    "container_cpu_percent": {
      "max": 71.9641,
      "p50": 44.19795,
      "p95": 67.81362999999999,
      "samples": 4
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 4
    },
    "vmrss_bytes": {
      "max": 15437824.0,
      "p50": 15437824.0,
      "p95": 15437824.0,
      "samples": 4
    }
  },
  "repeat-3--p64-c10-p10--hazelcast--get": {
    "cgroup_memory_current_bytes": {
      "max": 377126912.0,
      "p50": 376602624.0,
      "p95": 377067520.0,
      "samples": 11
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 11
    },
    "container_cpu_percent": {
      "max": 10.7459,
      "p50": 8.7241,
      "p95": 10.34585,
      "samples": 11
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 11
    },
    "vmrss_bytes": {
      "max": 396054528.0,
      "p50": 396046336.0,
      "p95": 396054528.0,
      "samples": 11
    }
  },
  "repeat-3--p64-c10-p10--hazelcast--set": {
    "cgroup_memory_current_bytes": {
      "max": 376397824.0,
      "p50": 376238080.0,
      "p95": 376387584.0,
      "samples": 6
    },
    "cgroup_memory_peak_bytes": {
      "max": 381825024.0,
      "p50": 381825024.0,
      "p95": 381825024.0,
      "samples": 6
    },
    "container_cpu_percent": {
      "max": 27.5244,
      "p50": 20.34265,
      "p95": 27.464575,
      "samples": 6
    },
    "vmhwm_bytes": {
      "max": 400248832.0,
      "p50": 400248832.0,
      "p95": 400248832.0,
      "samples": 6
    },
    "vmrss_bytes": {
      "max": 395755520.0,
      "p50": 395755520.0,
      "p95": 395755520.0,
      "samples": 6
    }
  },
  "repeat-3--p64-c10-p10--hydra--get": {
    "cgroup_memory_current_bytes": {
      "max": 470024192.0,
      "p50": 469946368.0,
      "p95": 470016409.6,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 488525824.0,
      "p50": 488525824.0,
      "p95": 488525824.0,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 458813440.0,
      "p50": 458813440.0,
      "p95": 458813440.0,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 458813440.0,
      "p50": 458813440.0,
      "p95": 458813440.0,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--hydra--set": {
    "cgroup_memory_current_bytes": {
      "max": 468525056.0,
      "p50": 456904704.0,
      "p95": 467363020.8,
      "samples": 3
    },
    "cgroup_memory_peak_bytes": {
      "max": 468828160.0,
      "p50": 463118336.0,
      "p95": 468257177.6,
      "samples": 3
    },
    "vmhwm_bytes": {
      "max": 457924608.0,
      "p50": 446099456.0,
      "p95": 456742092.8,
      "samples": 3
    },
    "vmrss_bytes": {
      "max": 457924608.0,
      "p50": 446099456.0,
      "p95": 456742092.8,
      "samples": 3
    }
  },
  "repeat-3--p64-c10-p10--redis--get": {
    "cgroup_memory_current_bytes": {
      "max": 5025792.0,
      "p50": 5025792.0,
      "p95": 5025792.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 55.6265,
      "p50": 55.6265,
      "p95": 55.6265,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12292096.0,
      "p50": 12292096.0,
      "p95": 12292096.0,
      "samples": 1
    }
  },
  "repeat-3--p64-c10-p10--redis--set": {
    "cgroup_memory_current_bytes": {
      "max": 5025792.0,
      "p50": 5025792.0,
      "p95": 5025792.0,
      "samples": 1
    },
    "cgroup_memory_peak_bytes": {
      "max": 20566016.0,
      "p50": 20566016.0,
      "p95": 20566016.0,
      "samples": 1
    },
    "container_cpu_percent": {
      "max": 9.656,
      "p50": 9.656,
      "p95": 9.656,
      "samples": 1
    },
    "vmhwm_bytes": {
      "max": 27172864.0,
      "p50": 27172864.0,
      "p95": 27172864.0,
      "samples": 1
    },
    "vmrss_bytes": {
      "max": 12292096.0,
      "p50": 12292096.0,
      "p95": 12292096.0,
      "samples": 1
    }
  }
}
~~~

## Artifact index

| Path | Bytes | SHA-256 |
|---|---:|---|
| hardware-validation.txt | 1336 | 3c33b9f99e4b7b9342d7fc13ec6f9742304d07878178c5af0d2a82e5b4e4255f |
| hydra.log | 16 | 6489d6d7a33c5d40e18fc61eeb6c34c341279ee61816394dde5189aa4ad8fae5 |
| hydra.pid | 6 | ddfab98517f33b94577e8ea5ba3e9f6973bc4103e09e8cd88d39a2906280eade |
| irq-baseline.tsv | 32 | ab8a998abf6c5504e41efd9e72d884fd7fa3814e28d94f73abb6e61ae3cc3656 |
| metadata/docker-warnings.txt | 186 | ba431352b1954a86c23115052875b8a5d045c4062a9d512bdf510acc7511e201 |
| metadata/hazelcast.container-id | 65 | a48e3c724759130a8b41d1124344992e2f8c45af3d0d4e7b5e6bcf6f615ba612 |
| metadata/hazelcast.inspect.json | 7675 | bec9f0be4580b89ab43373882c49bf93b1dd30bb065de5c873c0d3eeda10fa54 |
| metadata/redis.container-id | 65 | c430314e6e19af2646075aa8dd9e71b4dee1226e310a98ad06f968e5b4b50c55 |
| metadata/redis.inspect.json | 8670 | 1336d56c1ff91850723fae1d764b66018685949dbb347928251cb1c09b7ddf92 |
| raw/repeat-1--p1024-c50-p1--hazelcast--get.log | 185 | 9145a81c5ed0413596d89a0a1078a30326b551bdc18228e8c40a1eb43dc7ba8d |
| raw/repeat-1--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hazelcast--set.log | 186 | 9f5e8dd590f7eaae92648a0da2946b65319869f82af8248cc57bb625b2ff85f3 |
| raw/repeat-1--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hydra--get.log | 2044 | fed08f124df46c70687be067bdff10f5fc9407993fea65ec066ddba0870f704b |
| raw/repeat-1--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--hydra--set.log | 2046 | 2f99250096ab61c4809d2524504479ce52fc91bb21d369b8381d5907751c5665 |
| raw/repeat-1--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--redis--get.log | 1868 | fade3a71c080459fe0df68b4485d68e93dcfaeba7c463ccc6a600f736f1b0150 |
| raw/repeat-1--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p1--redis--set.log | 1868 | 858988e2710e1f7537e68df2b3f74444a184081ae3785aa3274a1b2f7e592546 |
| raw/repeat-1--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hazelcast--get.log | 187 | d0030c49da0996f758affdc5d4669e170079521ec1fe99d852afe453ae004ecf |
| raw/repeat-1--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hazelcast--set.log | 186 | 9d9b20556bcd9c2b01a1fe82411daaf8ab1e91978e3f20f4bd2790fb95ded0b1 |
| raw/repeat-1--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hydra--get.log | 1393 | d1d23c994c6f6139811ceb9508a4f36fed3091cf8971bfc4b8140f0c4c517794 |
| raw/repeat-1--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--hydra--set.log | 1393 | 16fccf7965a3e675692ebf401d6e6f2e69b7ffe3b0656bf5df126c0330f2501b |
| raw/repeat-1--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--redis--get.log | 369 | 66e31009fba3c8bf675bc845cc11080cb24b8bc479d153f7cb520e7bfd59a286 |
| raw/repeat-1--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p1024-c50-p10--redis--set.log | 367 | 112d64892c9f0ca0122496a3baa45c20b6056a3a28807a885266264c95d3f67a |
| raw/repeat-1--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hazelcast--get.log | 184 | bf665673fc77cc250765a8b58f3e606829e61756ead95eac66994d55bcf8e67e |
| raw/repeat-1--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hazelcast--set.log | 182 | 2b55954728e27702ba18cb19332cc9d51baca88e07097a3d52ea0ca39a3f94d5 |
| raw/repeat-1--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hydra--get.log | 2868 | addcf2532ec475c413b1d2830903399ce16119995a97bf56406ecd28d1b2a84e |
| raw/repeat-1--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--hydra--set.log | 2874 | 2aac04db54f444d02dc6478392c4d57897179cef89796ac7972b3f0de75f3017 |
| raw/repeat-1--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--redis--get.log | 2692 | b66b01933b98bc719d55d8c8364e1abed99771b657bbdc1366252f992cdde1f1 |
| raw/repeat-1--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c1-p1--redis--set.log | 2698 | 8941745682d6a9557bcba3c55d387e765441b2f123250cf733563bf457bb3fe8 |
| raw/repeat-1--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hazelcast--get.log | 185 | 07e3eeeda94c9b901e521736fb57ef5a47a1ca962351fdc29e813469880db7de |
| raw/repeat-1--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hazelcast--set.log | 184 | fc84ac5726615120f9215cfc55dc83208af8ffb480613673889254a2e3d58f26 |
| raw/repeat-1--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hydra--get.log | 2045 | f31c46855a7e531723de62cfaf81951cc6158ccb5fa21312a434e7da5e24bfb8 |
| raw/repeat-1--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--hydra--set.log | 2043 | e6af6281fc603e69796ba973a5668eac4699ec685dd111510925a6df7db600fe |
| raw/repeat-1--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--redis--get.log | 1869 | eb0eaef87aa41d29f01a0295fa9c8a6efc5eafbdcbb87bb73e8c850e61a20920 |
| raw/repeat-1--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p1--redis--set.log | 1869 | 585c25e96e70907ba3f3ef216d9303633e111d1f82627ab9d4a313424ab3bf8c |
| raw/repeat-1--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hazelcast--get.log | 186 | 4f8728e46b4732fe9b12dbc036dffcb85e63acd4ca5087a4980e9ccefd16554a |
| raw/repeat-1--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hazelcast--set.log | 186 | 6bf8e5edac88e3f376abe910b7d69bfd7193b1e4ffc862754840725869196cd9 |
| raw/repeat-1--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hydra--get.log | 1359 | 2d90b0a79972e4698026a86b671d46cd50c815a4f0aa6922a9a58306293a9f86 |
| raw/repeat-1--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--hydra--set.log | 1359 | f57a72d7cc3fd218e4b208c59e50535243efb2e87974a3207b0d74c1a9d21beb |
| raw/repeat-1--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--redis--get.log | 368 | 644644fc4a47c67a0eb67c63195c1ae75af18febfefce258ff6811eae4664216 |
| raw/repeat-1--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c10-p10--redis--set.log | 368 | bae79ff85fd484d023b6b19352ee7a5f6e6646fdc4cbf552ede57d8d51b2d794 |
| raw/repeat-1--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hazelcast--get.log | 186 | 919de0a83441f57fce79fda3c77b6e5b9103768418225027c21a221517b511b3 |
| raw/repeat-1--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hazelcast--set.log | 185 | 9a488868a66a261f0b5ad87623d14d40366272ae3efdaaec1e959654d9cd9e2c |
| raw/repeat-1--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hydra--get.log | 2044 | aa748a64b22272c82f48cc220bcdef53f3d2da211758b6c30ce474e97e0e5a61 |
| raw/repeat-1--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--hydra--set.log | 2044 | 1b9649933ebdc88b1b166001ee126c1a57c7f6c024b9bbed3553884aa152acf6 |
| raw/repeat-1--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--redis--get.log | 1868 | da99910dd58838ac75efb703171afdadf755f3b1894d0617d6d70f80478240a4 |
| raw/repeat-1--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p256-c100-p1--redis--set.log | 1868 | 4071068949705765aeaf0244f50cb478581db973b93d7f5b58680082244d0572 |
| raw/repeat-1--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hazelcast--get.log | 184 | 9490b2ea74c079d802aa73cdcc69d49fd865e1d7112a1cce74eebab407dcf2b6 |
| raw/repeat-1--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hazelcast--set.log | 184 | e030f609d8d6119d5a4b81964b7b35ca112dc0bf0f17f87007826c8fd0c8962b |
| raw/repeat-1--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hydra--get.log | 2042 | be791da4054d07af701a6561662bf78bcd31b8b0a06c60367e35518c736e9977 |
| raw/repeat-1--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--hydra--set.log | 2044 | e84f9f3094552179c3210d8e84e59dc01e6c1e8b95518c6a3246e558b1c0b62f |
| raw/repeat-1--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--redis--get.log | 1868 | 0f0b6c03af5d527cf12094cf22a585ab11f4cb5e0e46160c72f15d86eb00ae56 |
| raw/repeat-1--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p1--redis--set.log | 1868 | 7abedc8e6aee6c8ccc9252b223b094efe1ec8a5f70923737e481f8722a30454d |
| raw/repeat-1--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hazelcast--get.log | 185 | 157835f2b9a585165387e07748d6bbac07c457ead1575c75bb212bda972c8c8f |
| raw/repeat-1--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hazelcast--set.log | 185 | f955431e86acaf36273a7b0f315907aa58913868948530f85de91236db785526 |
| raw/repeat-1--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hydra--get.log | 1360 | 09fa025a9139f1712f3d88b44f70c9ad2ade275c8032971de0e1f162d1553a3c |
| raw/repeat-1--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--hydra--set.log | 1358 | 6132f2cc59f0bbfbd46cfb70392f34048ac12965f5829eca60c6acc20c77658a |
| raw/repeat-1--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--redis--get.log | 367 | 8b99322d9c3d15e501b357a3c9c98b7ae4c65fd73c66361f02177c7c8e815e57 |
| raw/repeat-1--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-1--p64-c10-p10--redis--set.log | 365 | e40ef5e230cc01d8860e3f7f54c50da5a20f43a886d83c125bb435f2d2709648 |
| raw/repeat-1--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hazelcast--get.log | 185 | 7c7174d014204cc6488d101fa3c6c330b6f0d59f81376cd95c472c58d9add5d1 |
| raw/repeat-2--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hazelcast--set.log | 186 | 67aa1465509e9eaf2b7330526c42ef03e43366bf43dbecebcae06a5f222f7630 |
| raw/repeat-2--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hydra--get.log | 2046 | fe08ddf5173fa1a90f116bece6f8eba9e2fa617349384eca6d0a7a598ffb9550 |
| raw/repeat-2--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--hydra--set.log | 2044 | d418ac63dc5656b89328d4dbb18d67aadab81c0c64bcec43a074c34ed6d386b4 |
| raw/repeat-2--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--redis--get.log | 1868 | 534f65fc11d81f22b77520fc9deefc276461b8a47c3b1b3d11428954add6f483 |
| raw/repeat-2--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p1--redis--set.log | 1868 | c83dc35dae4d054d60c696c5027e7172ee8cd48d30ee5292fb41431897126a20 |
| raw/repeat-2--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hazelcast--get.log | 186 | ea2594d9fe04c941eed67bd418ec02fa28e8aaf509237cfe404349ec82d25548 |
| raw/repeat-2--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hazelcast--set.log | 187 | 1221bc379af9760206f91db03340d17004d8752a21fe5b110ac115381d11ca2f |
| raw/repeat-2--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hydra--get.log | 1393 | 4c5fbbb73ada1a0525569413b160a62977a3fe0d5092c29435d4d4d9af1974d9 |
| raw/repeat-2--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--hydra--set.log | 1393 | de48507e2e7681d9fa48fc9f2d9e1d65ae9caf538e0fe0d0430421954e76af44 |
| raw/repeat-2--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--redis--get.log | 367 | 881e74af07f213545432cbfcdf0a93d7568cac7c4617c00c74b04b1a73cf6fa1 |
| raw/repeat-2--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p1024-c50-p10--redis--set.log | 508 | 79b3a6d1bc3286f1bc3c0c6458c020907bf1f2b6625ee8dd76302c42119bc1fb |
| raw/repeat-2--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hazelcast--get.log | 183 | 5c7e97593ccd60bec419e792e034e22ac3d85c2c05bc737208f01c6335baa255 |
| raw/repeat-2--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hazelcast--set.log | 183 | dd25743043e8c742436d3dfe5a11475432a8d65f3f4b88652e24d4940e315350 |
| raw/repeat-2--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hydra--get.log | 2868 | 86ec8e3767e26e930a2ed1b0bba303695212a3be340c6990e8f60f715d8ed3ce |
| raw/repeat-2--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--hydra--set.log | 2868 | 71d93316cb8ca7c9ba2e316f40ebb4543315ecf7f17dd1ab63826c4db95cffab |
| raw/repeat-2--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--redis--get.log | 2698 | 5fe662467b0346f24ce9e4ec65e2790b4d1341bf992c034406c5a574caf2cb09 |
| raw/repeat-2--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c1-p1--redis--set.log | 2692 | f51f9322948c8175e10f24e5b0149c45b1bc45404d81e9707163a1f506bea68a |
| raw/repeat-2--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hazelcast--get.log | 185 | 1b1a483ad4d4878a316a020ed89bfb8e980228ea7d93070f4746c9d8136a74ff |
| raw/repeat-2--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hazelcast--set.log | 185 | ef25687e2fb601e9d35a269e92117bae5db5a47b9729ccd67a37adfdc1b804ac |
| raw/repeat-2--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hydra--get.log | 2043 | 2523376942b00718fd69e5d9101fe16c39e6a97821dcb705bbc43a4c82097230 |
| raw/repeat-2--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--hydra--set.log | 2043 | d474cb71fd87f743a713182bf5b802d298724f40955561028afd03e886ce9ebc |
| raw/repeat-2--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--redis--get.log | 1869 | ce102c721358c134e7e8e1faf0875d4ff6e61331fe95e1a884f0b6f61d899b17 |
| raw/repeat-2--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p1--redis--set.log | 1869 | 317aecf634eace0caf270935b8fd59c4da0cb54951d76aec0a9b93addebd5056 |
| raw/repeat-2--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hazelcast--get.log | 185 | a14b11892ad0723185ace8144d619cd131bd9721990786b4f987b318f8ea538a |
| raw/repeat-2--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hazelcast--set.log | 186 | 4d36b8d4c0d165fda302f49a4dbea71b7cdd35b99f27316df02c40ade510d923 |
| raw/repeat-2--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hydra--get.log | 1359 | b47cfb1b3c1aee0f90bd80b8be45031cb7b097b82cd66ccd2fae57ec9d4feb03 |
| raw/repeat-2--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--hydra--set.log | 1359 | fa35599842f3157de82ba309f495b68a4668dfe4ece95cc5d215b71419e7ac7e |
| raw/repeat-2--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--redis--get.log | 368 | 1f5855b212eb1a095fe21d368c0ae7a35e741067090d173a04661201932bf61b |
| raw/repeat-2--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c10-p10--redis--set.log | 368 | d96ce4cd6fb4f7d29a06b456a9b91fcb14437c231bf69f9397c1045351690127 |
| raw/repeat-2--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hazelcast--get.log | 186 | e9003d5ab0b2be6f76c4653f147e8c55f5a649f06cdd4909980883e6765f682f |
| raw/repeat-2--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hazelcast--set.log | 186 | b22f6cb47a5725cdd9d0244ff4c61da5f37181733f7cccf6b1eebe72798b6473 |
| raw/repeat-2--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hydra--get.log | 2044 | 2f3d53890d306cc50527ad4841e4d23b032f332ce1daef33d0058e332d9f2a75 |
| raw/repeat-2--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--hydra--set.log | 2044 | 456c7a2623e99cf4ab6fad82ca5f3f4d839979d2eb28412c421e04f3dc6ee768 |
| raw/repeat-2--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--redis--get.log | 1868 | 863cfa7feb40e86aedee220034f16b10a45ee7f13384ca2eda12f4f56073aa36 |
| raw/repeat-2--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p256-c100-p1--redis--set.log | 1868 | 69d6b4d202f98ee98176b105d272be67849ddab58ba454316b5d066dd39611c3 |
| raw/repeat-2--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hazelcast--get.log | 183 | 8bdd26b7ec455442dfce8906ef08447a47f851c3e983f0c1d88565c2fd90b4bc |
| raw/repeat-2--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hazelcast--set.log | 184 | 239b66aa7d70d9df29c66c6ed11ca4a39e3629f23588dae28aed659b39dfe590 |
| raw/repeat-2--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hydra--get.log | 2042 | 5a538990393b1f46ea486cd465b246b4bc3775cc4b52435d2938efc3a7954dca |
| raw/repeat-2--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--hydra--set.log | 2042 | 0c983adad0e9b609152694df98f17167fcc5bfd035d5d44efad38f7041f8bc35 |
| raw/repeat-2--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--redis--get.log | 1868 | 295e39b042c3256eb2acfa4bff6d00d6ccf1bed37cf121f6ecf56a412b18f157 |
| raw/repeat-2--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p1--redis--set.log | 1868 | 9419d944fa0304d4535cf16d0ac3c3a137a451bd449674c261716cf4b23e1639 |
| raw/repeat-2--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hazelcast--get.log | 184 | ae03830f4be6f370dec61ad2d76c066dab68a116e171ecf1df9c4d8cdc75d833 |
| raw/repeat-2--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hazelcast--set.log | 186 | b989d89c94ae26ddf141eb84c626eaa407f4102b39d36a86e6eb204ebacb4bd4 |
| raw/repeat-2--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hydra--get.log | 1358 | 2ca3f7a00d8d7360cdcf34040f9e461a2bbef05303572fdf3596fcddf93e802b |
| raw/repeat-2--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--hydra--set.log | 1358 | 847f939a0a8e6a627b0c5d4af5abd7fcfe452fa6e95a655da0e7fa03b525c090 |
| raw/repeat-2--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--redis--get.log | 367 | 206272ed715a56a0491197aad9ec8707a5184f654df4da4f2822ad24b3f393e2 |
| raw/repeat-2--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-2--p64-c10-p10--redis--set.log | 365 | 2cccd2fc46cb08e88c168ca7575b2ab48d8c1b8e9b5ad4e94e80713c08ad668b |
| raw/repeat-2--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hazelcast--get.log | 186 | f8eba758be3e2f0f982b2985c1d62bd721b98029dae2385dafef980ea7cf924c |
| raw/repeat-3--p1024-c50-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hazelcast--set.log | 186 | ff0eb304d16f132474bee82695a2a45b93b9babc7d154a00fab905b4aefb2088 |
| raw/repeat-3--p1024-c50-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hydra--get.log | 2046 | f503e7b776d82669edfa1e465d120719c2ec3f70dd88c2ce13e57c5fef77e6ef |
| raw/repeat-3--p1024-c50-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--hydra--set.log | 2044 | 5b6f990e4a7f51c6c4015f00f5ee062a1a5007ae9631bbf81a931121322bc724 |
| raw/repeat-3--p1024-c50-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--redis--get.log | 1868 | ae5d70ec304c709bf52467d78b9c56a69dc50ef42d5cbcc5ea5fe937da23d247 |
| raw/repeat-3--p1024-c50-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p1--redis--set.log | 1868 | 9a6790df7ce10a2776100cc2c81fd13fb282ccfee1b727f13aea8f82cf2835a4 |
| raw/repeat-3--p1024-c50-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hazelcast--get.log | 187 | 1df1001862d78a628f03160bc9de2d1cc4b1dbfa20283f3ef7c938afa65f28d4 |
| raw/repeat-3--p1024-c50-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hazelcast--set.log | 186 | 8d4e652aa80817fd6bc845c7a37e7cebb2d614b839115f293610e81d541c8532 |
| raw/repeat-3--p1024-c50-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hydra--get.log | 1395 | 8ed932eaf4925b4d98e5b2c837ec4b73790f4019b398bdaf13b36fcc9837928f |
| raw/repeat-3--p1024-c50-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--hydra--set.log | 1393 | 6ce0915fee53b5a4b8b2f869582cfa92a99f64859502c5ccdaead6be066da321 |
| raw/repeat-3--p1024-c50-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--redis--get.log | 367 | 5522fc3acacc4019b10bdfed1ae6a5e4257fdb3313a6a629e8b93fcf194b151c |
| raw/repeat-3--p1024-c50-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p1024-c50-p10--redis--set.log | 367 | 7fcd4d499ef9efd349c464d839f2a3a68699022540e8547eefb9689af7579e61 |
| raw/repeat-3--p1024-c50-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hazelcast--get.log | 183 | 3329a493b392358b1d688ca286b30dd8fa01a9a79b113808d0614308f206aa73 |
| raw/repeat-3--p256-c1-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hazelcast--set.log | 183 | 3945c84494b348283184f690e177f1a8842a42b4aa7aa1dc1f87be1e329b1263 |
| raw/repeat-3--p256-c1-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hydra--get.log | 2874 | c88e4a23f2ecbf40ea22f61a86d61df82cc53a31e8bc296033a39a8facd2af1b |
| raw/repeat-3--p256-c1-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--hydra--set.log | 2868 | 5f2c258df3c985fc65a3658d695e279535777c5df14fbcd692b9148f954143ee |
| raw/repeat-3--p256-c1-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--redis--get.log | 2692 | 75a59dbbf4939aaa1b682ab29b6cfb3e5d847f095c1df8b1c89c4414babd80db |
| raw/repeat-3--p256-c1-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c1-p1--redis--set.log | 2698 | c76c09af44962ec360b4b4cde148193a1b1ada11f97958bdec1a374a7182bda8 |
| raw/repeat-3--p256-c1-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hazelcast--get.log | 185 | d7caacede5016c01bd7b55e3fd5b3efe4738959ab19f2982dea4fe6ae7eab077 |
| raw/repeat-3--p256-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hazelcast--set.log | 185 | 47ae2dbaef724170d1331e076c4ad8accf069ea5fe66b577af1fa0bc208394bb |
| raw/repeat-3--p256-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hydra--get.log | 2043 | 3d83ad8ffbaf843af12138cbd972eba806d9f7bf59718606837424f82c713c5a |
| raw/repeat-3--p256-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--hydra--set.log | 2045 | 282b4d22e9982f3499c646d55f9855028fdfd46d95ae5b3c9b62d9834db0466f |
| raw/repeat-3--p256-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--redis--get.log | 1869 | f35c14d901b377c5fdf92e3375f8b409ad95bd10a6d7355c259759bbb2be626f |
| raw/repeat-3--p256-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p1--redis--set.log | 1869 | 8c6fe19116a81ca0d814c5255dc4b261ae436614a61b6b595370aadbe71a9be3 |
| raw/repeat-3--p256-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hazelcast--get.log | 186 | 3d69bbbb35f2669071d9a9d7a8b00044b39e844b18cef0f1bc580b06fb89a2ff |
| raw/repeat-3--p256-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hazelcast--set.log | 186 | 0c4874f1118072b9af8d15d38cd681e164f9ab60da94e1627c1fb4dce75dbb12 |
| raw/repeat-3--p256-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hydra--get.log | 1359 | 67d733bc26c2c0e002e15c0e0570467f7f60e12b4d9cfa6b59ddd2c187bea63b |
| raw/repeat-3--p256-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--hydra--set.log | 1361 | 5802e6cdacde1740453f72191d03ed2dfe30fd6c2a4fb095594b194495e8d47a |
| raw/repeat-3--p256-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--redis--get.log | 368 | eb2b40e427b5369ee94e0d14c0a0ea9d70c5f32d8b38112fa95cbd27a4571677 |
| raw/repeat-3--p256-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c10-p10--redis--set.log | 368 | 1353c34057175854d12bbd1f7f8be2423fe3424bfa8910b357e2ffc943dbbff8 |
| raw/repeat-3--p256-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hazelcast--get.log | 186 | b20703447d1237c9bfededcad4c7364d433dce372f920bf5cf78e973d003c0fd |
| raw/repeat-3--p256-c100-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hazelcast--set.log | 186 | 81add41bec9243c6f7d86f16b10e5a1c7119850fb83336a3d48a8c672bacbca8 |
| raw/repeat-3--p256-c100-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hydra--get.log | 2044 | c081042e95b1c6c0103c6c5baf122c9d4f3caff629e3482e6b6905cd61134011 |
| raw/repeat-3--p256-c100-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--hydra--set.log | 2044 | 344ba64c083a7e47387c1dfe1d6aa89c33804911ed843a925500783439c37864 |
| raw/repeat-3--p256-c100-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--redis--get.log | 1868 | 8e63f7d604b0ea04468ab9896523e61943578ca19a598d3c63c6b6a13b95e6e8 |
| raw/repeat-3--p256-c100-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p256-c100-p1--redis--set.log | 1868 | b1f8805cc4a71022f1b608febf443344d7fdc99737917de6be3a5c6f09fbeedb |
| raw/repeat-3--p256-c100-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hazelcast--get.log | 184 | f06e46e664a5cc71ce74186d57ec0ef6ca769c56a7faae662a3b5101a44c5472 |
| raw/repeat-3--p64-c10-p1--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hazelcast--set.log | 184 | 24127bd397f88ed14349d02c5cb8c0ffd5c7347210ece257e313c4ef7c9370f3 |
| raw/repeat-3--p64-c10-p1--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hydra--get.log | 2042 | c363b8119e0ac43fca1d780e290cfeece9e5b8b62bca32c6110b1bdebb556b3d |
| raw/repeat-3--p64-c10-p1--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--hydra--set.log | 2044 | f11f3649e3b8a3fc3f146394490f893371709c607ef17428ff437674f94ec3ba |
| raw/repeat-3--p64-c10-p1--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--redis--get.log | 1866 | d56cb87781f4f9594120d0132241d7cc18ab0556b77e01be3bf4ae0387331df4 |
| raw/repeat-3--p64-c10-p1--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p1--redis--set.log | 1868 | 67e5106e0f3be8b5a95950c5e9513efc799c9dd5bd13df8a135283b3da1da9f4 |
| raw/repeat-3--p64-c10-p1--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hazelcast--get.log | 185 | 774981706e5432b9025c9a7a7edc8dd1d69d0eacc9874e6b5525e6acad0c5423 |
| raw/repeat-3--p64-c10-p10--hazelcast--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hazelcast--set.log | 183 | 26e8b13b185afa49b97be0692bda5da4f0c6d6c4c62ed1503e525948881d41af |
| raw/repeat-3--p64-c10-p10--hazelcast--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hydra--get.log | 1360 | ff4ecbcdb8c8a92694b0e8c8036f15bb53180b7f5d51d9a02529fa96a4727526 |
| raw/repeat-3--p64-c10-p10--hydra--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--hydra--set.log | 1358 | f787accd1fabbf3a370526e6d07af5d16be8c2010666c70882b06a986d921a89 |
| raw/repeat-3--p64-c10-p10--hydra--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--redis--get.log | 367 | 9b1d7bc60d60e3c41716aa7938d7625c8d617984ddf9b530f9ce59b1f23f4d3d |
| raw/repeat-3--p64-c10-p10--redis--get.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| raw/repeat-3--p64-c10-p10--redis--set.log | 367 | dcfdaad1464f6ed5210d507c11dcc4d3f8b3d5789b7904c3e16c94810db81749 |
| raw/repeat-3--p64-c10-p10--redis--set.log.telemetry.log | 0 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| reproduction-command.txt | 496 | 4f0a4eeeaf754424f379704a25a3726126bc1f589719c8fba7167026f8897eb6 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.csv | 1379 | 5524d3d01fcf44615e3b5e50c68b013736675e77e1b1e73b5f22447c6fdffd96 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.jsonl | 5008 | 7edcf34a876ad9e2a4ab71d1163300d5ba9e379524b1f6d5c9c2d026a859eb2c |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--get.metadata.json | 8029 | bb7c4080e85d42c5e3f3db14788fc55918dc0b2400e6333bfe315147d5c94660 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.csv | 1277 | 7e3c912ad7b917a4bcdab035dccc55c403f0523b9330353064ce2f5e9cdeccae |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.jsonl | 4551 | 90e6937dbd79a873d1f4c8a11b51bee1fb1d3cacf8c9c25c630ce483cc4f75e5 |
| telemetry/repeat-1--p1024-c50-p1--hazelcast--set.metadata.json | 8029 | bb7c4080e85d42c5e3f3db14788fc55918dc0b2400e6333bfe315147d5c94660 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.csv | 636 | 6effa0ee35298c251804f7aae75353123ff9742e403fe66d30c7a12efa514591 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.jsonl | 1796 | 65a74d5a9e93e7351f2a08cc04601ebc52943fd8761431e610bd279612e608a2 |
| telemetry/repeat-1--p1024-c50-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.csv | 634 | 9226615ba3653bc43d8b7de7af0b404f3f5476955ee1a7ce3260559575d44a1d |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.jsonl | 1794 | 906643974e30c78138aa0510a8e523509f75f1b456107b4ae6cbbefebb066fb3 |
| telemetry/repeat-1--p1024-c50-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p1024-c50-p1--redis--get.csv | 642 | 55197d279f2961a5c617f6d0bccfac56f95ee8d2fe893ae2f2a40106bf5e3cd4 |
| telemetry/repeat-1--p1024-c50-p1--redis--get.jsonl | 1786 | 4c967212da53a5579415366fe2ad786fd5cc2dcf38a04c8a3e23a020cb19547f |
| telemetry/repeat-1--p1024-c50-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p1024-c50-p1--redis--set.csv | 640 | 898ebd5bf9b865ae35638bd069ef7b76053fc662dfc6ee0d1a60df124ceffcdd |
| telemetry/repeat-1--p1024-c50-p1--redis--set.jsonl | 1784 | 747e4d4f8fcafcd89a3ef1ac960432f263d89831c563b16475c00a88c442668e |
| telemetry/repeat-1--p1024-c50-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.csv | 876 | 8bce768486d4ddc09bed63f908e65a55f69c28f580dfc0a1ce0e23edd6533e55 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.jsonl | 2730 | 13e280f07d8af83c2f0122e951f4d52e7917cf30957bc890a96be6f9e58ba038 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--get.metadata.json | 8029 | 2626b6d5955c3fb7c35d701d0c7d2e867a0230e2adce6e7c7be0e1bfa12305ea |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.csv | 773 | 79582310781914e64c9e095cbc8b39731e5867f2f1d374cc817982ca256da08c |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.jsonl | 2272 | b17e10cb8047d063d2109969042e319820bc04e2521b8a45cc6a2e83d7e49562 |
| telemetry/repeat-1--p1024-c50-p10--hazelcast--set.metadata.json | 8029 | 2626b6d5955c3fb7c35d701d0c7d2e867a0230e2adce6e7c7be0e1bfa12305ea |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.csv | 545 | 148a17d24a1f139183da974b3735b4cd22b92387747531ee5f5bd2f9e603f1a9 |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.jsonl | 1346 | 7db1e2af014573e66f01a38988584bbfc7e068cf6b2c0c37f53a7450a5d6c0ae |
| telemetry/repeat-1--p1024-c50-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.csv | 545 | a0bf0907425e00b5aeb471d96070045d765b8f3aeba4a3fc63f8db92ea2f81bf |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.jsonl | 1346 | 25d30d025b392730d20de28de5b5c3b479da357cb37843940cdb1e029c2fada0 |
| telemetry/repeat-1--p1024-c50-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p1024-c50-p10--redis--get.csv | 364 | 96bb400e36dae3653d39c47930d9cdf341f3fe90473a95ef6bc0d98bd84a3aba |
| telemetry/repeat-1--p1024-c50-p10--redis--get.jsonl | 443 | 3154cbddf27f5401c68ce9db59702a6255b2eb0beff5c532e693505dfe88ffea |
| telemetry/repeat-1--p1024-c50-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p1024-c50-p10--redis--set.csv | 364 | c4a1c445f0f435253275ec77201cd91632d35a366514ab3b59b129fa282a8e0b |
| telemetry/repeat-1--p1024-c50-p10--redis--set.jsonl | 443 | 45c41f0035578a86110f8f03bdcbcc841835328d998b04640065d1415e65188c |
| telemetry/repeat-1--p1024-c50-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.csv | 2085 | fa9732eb649e7c07f8ea6bfec8ecc4987b3705c8a6155e59aa170548a887d472 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.jsonl | 8199 | e0e3b2152d6135507e63f2ed06b2805e579d9bca823e39576ceeb0b497c53a7a |
| telemetry/repeat-1--p256-c1-p1--hazelcast--get.metadata.json | 8029 | ea77d935b557e26b725ec00a31dd7b15ddf3845d1bb0d1b40bb5b484c7381553 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.csv | 1981 | 28169e0f3598509dcc6c092cb4d0c7b7b660a7957ca160ec5d144bf7b3e84967 |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.jsonl | 7740 | c7d5e9adb7da9477f1bd292f829f9be8207e3e498b2ae2f58af853bf35573aea |
| telemetry/repeat-1--p256-c1-p1--hazelcast--set.metadata.json | 8029 | 2626b6d5955c3fb7c35d701d0c7d2e867a0230e2adce6e7c7be0e1bfa12305ea |
| telemetry/repeat-1--p256-c1-p1--hydra--get.csv | 725 | ab813833e688ed13e585d2f9ccba7aea96ddb636de1064e0e1c19f988ce6cfdf |
| telemetry/repeat-1--p256-c1-p1--hydra--get.jsonl | 2244 | ff088c71558a5b47492e8a1c36fac807889f8320f9ecb3aee864c697cc3f247a |
| telemetry/repeat-1--p256-c1-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c1-p1--hydra--set.csv | 725 | ecf30c40f209d621d8d19ccfe38e4cdf1b7e47e12e4755440fd8792750edec7d |
| telemetry/repeat-1--p256-c1-p1--hydra--set.jsonl | 2244 | cbb3180c347e651dfcf280ddc260bb0f9ac7ca48a698de35f58769414e00e50e |
| telemetry/repeat-1--p256-c1-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c1-p1--redis--get.csv | 733 | 100ad87104503c55127d9bf111cec34cf64378148f8fcafca0ce2e2e715054e8 |
| telemetry/repeat-1--p256-c1-p1--redis--get.jsonl | 2232 | 9b8e450af9266e362dc70d13504a18f81bd19e3befbf7bf67099c07bd589db02 |
| telemetry/repeat-1--p256-c1-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c1-p1--redis--set.csv | 740 | 2edcfd63fb270b5562d6e0bac4f55ca9b6aeae82e5702fa37cecf0e374b4ad1c |
| telemetry/repeat-1--p256-c1-p1--redis--set.jsonl | 2239 | 159c3c852ed10e9c78436504d8d07f677e388bd5fdc2e86ad2206a2d7645eda3 |
| telemetry/repeat-1--p256-c1-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.csv | 8241 | 494256c2c3129a31cac04a6e8175f127108f475f1d96fa3624ec9ed32b4519f5 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.jsonl | 36365 | d877091af03915e476d463a9fb2ec8631d8093d3d1f27df96f53a1cc41bce006 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--get.metadata.json | 8027 | e3cd21ec8c20f455b4febc5e42a3d0b024a53dbab996a4e91940f7f8bb5d9300 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.csv | 7146 | 2aa6af9586059a4fbe7801fe610125e96d2f577f42277be8eba8edaa5dc35a11 |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.jsonl | 31365 | a6b6ac9dc54c957b779dd34c45a1c043095cad299facd0a9cd1f59cca7c4539a |
| telemetry/repeat-1--p256-c10-p1--hazelcast--set.metadata.json | 8028 | ed4a38055819cd37e602f6f90ee3f17e4adabd2d12ae89da1eccb524a1a067d6 |
| telemetry/repeat-1--p256-c10-p1--hydra--get.csv | 620 | 87dd6eaa7a7da213509e3ce8e7b4b8d497d74008f3522cb9be8dfc7efec9a74b |
| telemetry/repeat-1--p256-c10-p1--hydra--get.jsonl | 1780 | bfdefdcf7d8b1b751654ba105fc7aa38b58c6f550fa510745d83c0cbc33b65db |
| telemetry/repeat-1--p256-c10-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c10-p1--hydra--set.csv | 615 | 6262b8585f6734d22697d80b4e3a21a55bcdec19b07c1329acd1f07dd8556e8c |
| telemetry/repeat-1--p256-c10-p1--hydra--set.jsonl | 1775 | 92c25db74344c88ac4871e2a3ed1fa4b2831ead251a3e8306cbd81ce18db3834 |
| telemetry/repeat-1--p256-c10-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c10-p1--redis--get.csv | 636 | f29cd03a087e48861f0aca0ef884cbad47c02072b65b66a1bfd8a3bd15f39112 |
| telemetry/repeat-1--p256-c10-p1--redis--get.jsonl | 1780 | db9883d6dae224a88b14bfebcaa75f7f4049801af71239750e6d66953ff87fb2 |
| telemetry/repeat-1--p256-c10-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c10-p1--redis--set.csv | 635 | 5dc3aabdc7ac5a6e703127adb759173f5f24d5ff52014936e38e8e2fb8e8925c |
| telemetry/repeat-1--p256-c10-p1--redis--set.jsonl | 1779 | 9498f20c08727c4942a84d8c5cad8228fe07fadfa3a59dc73e0628bed1acad22 |
| telemetry/repeat-1--p256-c10-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.csv | 1375 | 75d6e2f419e2604eb955b22d6c5105a9ad5d28c35ad6b93214ed90349cbefeb5 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.jsonl | 5004 | a4a58656a7b56f5f0b6051c98da69ea279906583744a7568443671b0c18cdfe0 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--get.metadata.json | 8028 | 562ca83d282e98c310eb78d1ac29c136eb8736cbf5335372f0bfc5d94ad86ea3 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.csv | 1174 | d0b56b2d44b1f8e21f58b1e886356ead618be8bf62bacbe927d89c3076e170f8 |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.jsonl | 4093 | ccd786bc889d66625d2bd985172a8051e9a85aef5fdea526149c04db69c827ea |
| telemetry/repeat-1--p256-c10-p10--hazelcast--set.metadata.json | 8029 | 7ba89bddf3ade388a2e4b0286b6a5a59ed3224580c0b39622aa5742906572365 |
| telemetry/repeat-1--p256-c10-p10--hydra--get.csv | 546 | 85ce57b2bf556c179c0b8a49cc742421fc3c7acc0d649d56dde995cc7e4447d1 |
| telemetry/repeat-1--p256-c10-p10--hydra--get.jsonl | 1347 | 15589e46c2630bfd117c96131b0c9166859f7418f66592fd2da6080b6e0153ee |
| telemetry/repeat-1--p256-c10-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c10-p10--hydra--set.csv | 539 | 104e7dc8877aa764ed1a33dffbc1dab2f764a4aff07f863080b5e65d4b7c97bb |
| telemetry/repeat-1--p256-c10-p10--hydra--set.jsonl | 1340 | 3f56d3272bd8b93eebb8c58841379ce463d0e41456da568b01c8a4eef8c1590b |
| telemetry/repeat-1--p256-c10-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c10-p10--redis--get.csv | 365 | 6b68c733f7c60a48421cf0b7088504f27ee939039e2b6beb9e90160ddfca8654 |
| telemetry/repeat-1--p256-c10-p10--redis--get.jsonl | 444 | 2efdc048087d7865dc981f3c5b6d1d27efe645f5019236136f9754cfe5f78813 |
| telemetry/repeat-1--p256-c10-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c10-p10--redis--set.csv | 366 | 8a1f87c2a32b0f0c9499e4bc2e70fff3986294f8e8ea7b294782d787218d5bd0 |
| telemetry/repeat-1--p256-c10-p10--redis--set.jsonl | 445 | fc1e9dded2d83174e1efe00296da37460905127a2af36932b607285c4aaf3fd0 |
| telemetry/repeat-1--p256-c10-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.csv | 1078 | c7879e67637c1326c51669f6141966d370600c42aa4c188c06bf05557c239c68 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.jsonl | 3642 | d374a8d1d8557644417e836859a148d994dc7f7f7e8ebb9590eea2cafe8de0d7 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--get.metadata.json | 8030 | b9232c801bd89de9bf4b8f91c4dc3e8cf6f593eac0cfd58098f09da31468533e |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.csv | 1077 | 04821dea6d50ad29fd72c284f425f1c17486346d847e5fba52f9ddfb0a22a326 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.jsonl | 3641 | 906681e6e4be7c5d8e2efc688d67255cbd6197ce7a36c4b93a5434ae70d0e191 |
| telemetry/repeat-1--p256-c100-p1--hazelcast--set.metadata.json | 8029 | 6849464da16db51a0d84d5358423a3718087ff8e13e1efdd7f9df05e711a5f30 |
| telemetry/repeat-1--p256-c100-p1--hydra--get.csv | 634 | 7209cdb46f2cc06503ed9715de330b2ea96c7749eddb1a4cd853b34595e07650 |
| telemetry/repeat-1--p256-c100-p1--hydra--get.jsonl | 1794 | 9cf73ed2a35857f7fb773767845ceae2953b3c7242d92961f2e8100c38b21d35 |
| telemetry/repeat-1--p256-c100-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c100-p1--hydra--set.csv | 636 | fe3a0ac20565f799434492bb226f277795a3310dccf5dc35ad6d1a02705c1d5d |
| telemetry/repeat-1--p256-c100-p1--hydra--set.jsonl | 1796 | 78a9dc41826487e9ed4a5d5bb6abb758487ad9dda1858e0fff2b906224d34b8f |
| telemetry/repeat-1--p256-c100-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p256-c100-p1--redis--get.csv | 639 | e0f02e288ed9338ede163e12bf27a732b814249ee88daa9710f239522d035ea9 |
| telemetry/repeat-1--p256-c100-p1--redis--get.jsonl | 1783 | ede3703943c32cd0fa21c55ae4fe59dc82680a9978774a604953120eb6163209 |
| telemetry/repeat-1--p256-c100-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p256-c100-p1--redis--set.csv | 638 | c42c102ee6d17ee1c21f9c56763c83dbd227184d0fddc434805b60eef23f8b1d |
| telemetry/repeat-1--p256-c100-p1--redis--set.jsonl | 1782 | 6642996cca21922a0c1cd7491df570d4cc8ec085bcd3f836cab0449a46b5a9af |
| telemetry/repeat-1--p256-c100-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.csv | 8451 | d768b141c74be1465c28426d809697fdcf6cc315387693210bd40e89fa9f1de8 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.jsonl | 37285 | 7cb6f8c10614ec7c90aa57f452c5c8c675f09e8e14f83f238a322776b558705a |
| telemetry/repeat-1--p64-c10-p1--hazelcast--get.metadata.json | 7115 | 1913b0d634da61e57e977ceca79c1354f1a6617c02f5fd4c490a8dda86186277 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.csv | 6357 | a87acf16ea9465289a3016827b0f3c8ef3b2cfc2698a7d03bc0a8b5702e66107 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.jsonl | 27736 | 2f73fc08daad0f7281709e327f0b83805dd3b7b7e3fa335d5c95ba29bd273523 |
| telemetry/repeat-1--p64-c10-p1--hazelcast--set.metadata.json | 6498 | 36ed92be84048a11e8c7c83087658935e8971b9749551b6432de3180a0ae90fa |
| telemetry/repeat-1--p64-c10-p1--hydra--get.csv | 616 | 396270c22f934f9c46b664b994f4543c28eb1174c2b869794c165b021ce5dfb4 |
| telemetry/repeat-1--p64-c10-p1--hydra--get.jsonl | 1776 | c0177f956992c503bacd34309f6de747cf4c5d90d526b24552ea0ff9e8a6c306 |
| telemetry/repeat-1--p64-c10-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p64-c10-p1--hydra--set.csv | 610 | 39b1d2ade818e641c7091515b485f2e443cee6f6afe723a7024c6cd13339534b |
| telemetry/repeat-1--p64-c10-p1--hydra--set.jsonl | 1770 | 79d519801755b1d6b0dd991d19dd43c203c09eb475daa6af005951a696c80032 |
| telemetry/repeat-1--p64-c10-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p64-c10-p1--redis--get.csv | 636 | dc324c8f209ee0d32fbfe68f5a06594b433a478654a69443fa9b05ca0a66218a |
| telemetry/repeat-1--p64-c10-p1--redis--get.jsonl | 1780 | 0507c85622819c802b291bdef2a39cc42c112a0f459eb0f7b2d8e4848c2b620f |
| telemetry/repeat-1--p64-c10-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p64-c10-p1--redis--set.csv | 630 | 585b9968195784500ac25088782d42730a5f5ea3e319bcc7c012f7ff878b2f67 |
| telemetry/repeat-1--p64-c10-p1--redis--set.jsonl | 1774 | ec2b46d2b6902abcbabc3d6b0a10480249b589b2d83dd897dcee335e5a62b398 |
| telemetry/repeat-1--p64-c10-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.csv | 1471 | fd41fce7807d1f86b960ddf124287a05544324a1bb38420c6d018cb3c59d2463 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.jsonl | 5455 | 48c487e0801c59d7a25711c8e5be7a780d568dc61187b8cc3915cdc434aa50ea |
| telemetry/repeat-1--p64-c10-p10--hazelcast--get.metadata.json | 8029 | 0746113d3a21e9318ee364d272c302bffad7632edfe0fe0d09241f75c4a76405 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.csv | 777 | 26fd6ae149caeb6495796f3f0dd7ac26614ecb18144a5328f788c0927453d344 |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.jsonl | 2276 | d5d6bbf754ef0514efc66e16754323361b28d50dfa2223a3c15984d7f8ee23fb |
| telemetry/repeat-1--p64-c10-p10--hazelcast--set.metadata.json | 8029 | 0746113d3a21e9318ee364d272c302bffad7632edfe0fe0d09241f75c4a76405 |
| telemetry/repeat-1--p64-c10-p10--hydra--get.csv | 531 | 010fbe3b0ca3f87ea8077f4be4816487bd9c003b3aed021c3cda020fa2e450f5 |
| telemetry/repeat-1--p64-c10-p10--hydra--get.jsonl | 1332 | 5e6ccc1cdcbdeeb6a2f231becf0b10f74b6778923a59b3b91c05a5086b9b1d68 |
| telemetry/repeat-1--p64-c10-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p64-c10-p10--hydra--set.csv | 530 | 7d4c1f045fde138808006d1f6d402dbe58ee9340e7f85d3391c151f812b3ba58 |
| telemetry/repeat-1--p64-c10-p10--hydra--set.jsonl | 1331 | 34d3a7d96160cfa0007dc22ae486586d382f2b4c34df6cee76598e5e544310f2 |
| telemetry/repeat-1--p64-c10-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-1--p64-c10-p10--redis--get.csv | 366 | fc7260a49454801bfdcb6aa22195d54d9a2df192244a7df788e0cf7f9e13914a |
| telemetry/repeat-1--p64-c10-p10--redis--get.jsonl | 445 | 604e55f4de42c613ac4daef4bf10962ca3dafe24564cc423ab4e4a0749dc28d7 |
| telemetry/repeat-1--p64-c10-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-1--p64-c10-p10--redis--set.csv | 365 | cf47d4108ea102993a1b791bbdc3fc46ddc6c9554ea5657c743fe37950419459 |
| telemetry/repeat-1--p64-c10-p10--redis--set.jsonl | 444 | 34f52d7be08cb55e7895bdacc0e078d711ef4726345c6a11ba8ea9e610f313a0 |
| telemetry/repeat-1--p64-c10-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.csv | 1378 | 98fe3532af4aabc8c0c5a2649c7ca3044a8da0b473a5e367eba25a987ea3a12b |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.jsonl | 5007 | be42ecfaf0c1f303d224d62c8f6ed560e5fd2d803bf6a4ea3ebb565899919f55 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--get.metadata.json | 8029 | 8988d54f3d9b47c02bb83ab37d91179e166778ebf38f777a8782ccff57564408 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.csv | 1179 | 459cd8a0d78cb87097f03d09d2e9480ff559be54d4de759240f1c5a9b457cecd |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.jsonl | 4098 | 108dc85936edd28e73eefa1ce32b8139a57e6145fe718941be3a5a4a636e2588 |
| telemetry/repeat-2--p1024-c50-p1--hazelcast--set.metadata.json | 8029 | 73a73d942479a8f5e43c533fea32d71aa35cf34049efa4a519b6d04c9ea9f1f5 |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.csv | 635 | 93c7c1109e4303fec127c1e3372b0226e50b57e89aaea079c4213b077d96adcc |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.jsonl | 1795 | e0ce72ff61b3f621d41f76420cb829218af06f4a285c9881025411c44555c1d1 |
| telemetry/repeat-2--p1024-c50-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.csv | 634 | ea663098b6cbb85f190b09a8145b51dcfe12ae2285b3dee6e3824f8b520673b5 |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.jsonl | 1794 | e96c75af86caed1327d4b5c970e12efe09a4919c3402baf770d3425190ff8d8a |
| telemetry/repeat-2--p1024-c50-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p1024-c50-p1--redis--get.csv | 648 | 94d659e66db93b718d4acf56a51429371b7f0e28175b647144dafe9c4a3dfc43 |
| telemetry/repeat-2--p1024-c50-p1--redis--get.jsonl | 1792 | d2b14572e44e9464e46ce9171cf8bf2f8c56c1be580de94cf8ccc7edf7715caa |
| telemetry/repeat-2--p1024-c50-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p1024-c50-p1--redis--set.csv | 646 | 01251fc1d8e7507d89d75ee06b14afceae651f4c24ca60e953b623871e7b7474 |
| telemetry/repeat-2--p1024-c50-p1--redis--set.jsonl | 1790 | 7379873e0cf6e9ca3003b4defbe8ad7d6094f1e683757ea443c279ed0358471f |
| telemetry/repeat-2--p1024-c50-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.csv | 875 | a3bc3c97c32cdcf9280764dafcc1bcfc1ef981b429121b8118df2b48f8eb84d9 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.jsonl | 2729 | c4942490ad84d37f61951477525d50ff91ef7b1133817624e75255f236da97d7 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--get.metadata.json | 8029 | e17b66b48d30b02c75baef344de24d8ba4bbbd41b15f4f5b3da2df26ad897d7d |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.csv | 775 | 2d3eb91829434ee8e3870c2b15a7fc1a8837c70019fd1cf7b17688ed40bed6c7 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.jsonl | 2274 | 6978bd96b61ff07ddb79ece43d068e63ccb6b6c0cb5f2b3aa7c74f8ca40d6a23 |
| telemetry/repeat-2--p1024-c50-p10--hazelcast--set.metadata.json | 8029 | 8988d54f3d9b47c02bb83ab37d91179e166778ebf38f777a8782ccff57564408 |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.csv | 546 | ffbbe3ee1f0b53cbbff1b5190e067237309a213b0ceb252daeac702ae14cda49 |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.jsonl | 1347 | 53108b1539ee060af05874016a3d7890af1e4d182d77efafc2ad5317d8b9a17b |
| telemetry/repeat-2--p1024-c50-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.csv | 546 | f3bb91966f23389ecca3027354737a9f000b006212bcb4a3962bdefc48b22e34 |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.jsonl | 1347 | e43137e77ee6f4e656c8715864f3775543af80a8fbe7b85af94c5ac3f6097d31 |
| telemetry/repeat-2--p1024-c50-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p1024-c50-p10--redis--get.csv | 368 | d70986a89f332e4293686bab2bd0a0b878f1665e261ba804c4412ec04433d1b4 |
| telemetry/repeat-2--p1024-c50-p10--redis--get.jsonl | 447 | c2fb55bb6e432ae0853e4e4d51f8ca2ae8660cc9abfe1f0728a69a5f94a56bd3 |
| telemetry/repeat-2--p1024-c50-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p1024-c50-p10--redis--set.csv | 365 | 227bec9baf28011fa92b5cee2cd92b74f43d669b75ca1bbac5d775508a59ca60 |
| telemetry/repeat-2--p1024-c50-p10--redis--set.jsonl | 444 | ffca2bd863638cc92e27aae6747aeb01f06d8b61fbb41d8afd54ee3c892ae8ee |
| telemetry/repeat-2--p1024-c50-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.csv | 1986 | 278e1f43a879c517fea0a470bf397eeeb8d1a104b2fc8aefe8b97d7c573ee101 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.jsonl | 7745 | d49df0365687d42c8e0891e21f57edb7380a110a35d122d4c4d05744ee6f1022 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--get.metadata.json | 8028 | 867a8a40d348dc8c82697c09f82c425f49f199a52442eeb53fbc935dda91a8b8 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.csv | 1983 | 509892598d71e640f58208a273c6c3f24ecb90a347e8247562af154e2f857b82 |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.jsonl | 7742 | d5bfba308d555269194a824f4604c108dd8a9b327b00870fddab695f3b7ae06e |
| telemetry/repeat-2--p256-c1-p1--hazelcast--set.metadata.json | 8029 | e17b66b48d30b02c75baef344de24d8ba4bbbd41b15f4f5b3da2df26ad897d7d |
| telemetry/repeat-2--p256-c1-p1--hydra--get.csv | 726 | ade5a3a254c5da1d41a03743a94083fe991f7a34199155e41b7c660a7fa03d20 |
| telemetry/repeat-2--p256-c1-p1--hydra--get.jsonl | 2245 | f291b5177775c69bfaf3bc6dd9e6aaa1ec7d3b3240da4bc7978d9b5e8055c91b |
| telemetry/repeat-2--p256-c1-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c1-p1--hydra--set.csv | 723 | 8b4256d1a1a4863bb26d8e84fe2c406150ada8108ef8e9dbabb6b0e111289d25 |
| telemetry/repeat-2--p256-c1-p1--hydra--set.jsonl | 2242 | 8e73e2ac60ddf3b7dd68ff533619bc25ace08a5c1a52293d2b9658b304205544 |
| telemetry/repeat-2--p256-c1-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c1-p1--redis--get.csv | 733 | b7555b213d567307b725314fc6b657e1ff9faf0f3c685b24b694a1207527406a |
| telemetry/repeat-2--p256-c1-p1--redis--get.jsonl | 2232 | 95d14b960c326d339d99402d96e85bae64e06c11e0691c0091d4571731c801a6 |
| telemetry/repeat-2--p256-c1-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c1-p1--redis--set.csv | 740 | f381e5d822494f631c30b3ca5391e27a783db1afbc0ff05f9453dd804f5bc56e |
| telemetry/repeat-2--p256-c1-p1--redis--set.jsonl | 2239 | 883ea22f992d12714ae7a917bfe96b0e21bb503e4c37811f8dc0bd44241600d3 |
| telemetry/repeat-2--p256-c1-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.csv | 8335 | a7fcca2601e74435cdbb0edeec446d9623975d26b0b1cb464a8c9f06bebc2d67 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.jsonl | 36814 | 1fe0e84cbfef52fa70c44900c9a886b1bf5280f3321837f62bea1b9d9f2226dd |
| telemetry/repeat-2--p256-c10-p1--hazelcast--get.metadata.json | 8026 | 7c678f3eec095f6436539a2c036262c75a2aecc63b25e6ce5f04888ec19f0b1c |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.csv | 7142 | cc5418752e765c1f4ace4c4b6c0719c588bea3e0a68c7d4e1374b5c5a2366e5e |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.jsonl | 31361 | 9419cf8d58f3a4f6a157812c81fe94ff09ddd69993665bbfbc791b4ad4ca0e95 |
| telemetry/repeat-2--p256-c10-p1--hazelcast--set.metadata.json | 8026 | e4aa7ecc4e42c1673bb7dcaa94681ee399cad728fbbb0f067da54c4d6f9a2ff1 |
| telemetry/repeat-2--p256-c10-p1--hydra--get.csv | 635 | cd9b37152058e4517f15092ae323addf073ca906f1d5475dce5d8bb8d54ab8ad |
| telemetry/repeat-2--p256-c10-p1--hydra--get.jsonl | 1795 | 98763d3b915b9042e4f9d6b53ca2bc0e701bb0c7da5ea0d0eac1a0f814bde158 |
| telemetry/repeat-2--p256-c10-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c10-p1--hydra--set.csv | 634 | 762d8a9befbc6a6f4a39a775611c2742204f07160e4bf16e918a900ff1ed421f |
| telemetry/repeat-2--p256-c10-p1--hydra--set.jsonl | 1794 | 91700490471f148fd0e10e8482ea2068f561c82ab8d2e0a6666289d72b48b7da |
| telemetry/repeat-2--p256-c10-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c10-p1--redis--get.csv | 644 | 68e57cf54e2ab5ebeda290eb18fc0119e4be500625a3dcf1c4792bae2ebaae0e |
| telemetry/repeat-2--p256-c10-p1--redis--get.jsonl | 1788 | deac9017bafb521d7115de896831f6530bda986133c8d6f59c156da70785f7ff |
| telemetry/repeat-2--p256-c10-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c10-p1--redis--set.csv | 640 | b117f807fcba9434cb0e83df4d8d83e45a642484b6d6647444cc68ec9b4b76bc |
| telemetry/repeat-2--p256-c10-p1--redis--set.jsonl | 1784 | ccd0f716d2d9c32356441c49a4ab38b790f9ee54d320342f4ed113254e66a31a |
| telemetry/repeat-2--p256-c10-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.csv | 1370 | 20c9748c0ca19f7e1387ed7e58346a79f8f53c1b3ca020ff4530735960bf3479 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.jsonl | 4999 | dd487914ae9b3d9413a24332dbec8af96d111c1682f451e9ecfe96e4fdd4fd33 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--get.metadata.json | 8029 | 73a73d942479a8f5e43c533fea32d71aa35cf34049efa4a519b6d04c9ea9f1f5 |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.csv | 1172 | 81682e9a9054da6fdca0dbbe1c7be7e0d410eb9165a08631fbc11bde74c6b91d |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.jsonl | 4091 | 066b40a52e6c9dff2142c9a0e837c92826b1d14548b141e28358f9d10bb549bb |
| telemetry/repeat-2--p256-c10-p10--hazelcast--set.metadata.json | 8028 | 4f7b7666f56572bffaddf86d52acb2a542329352bb2ab46b1e549cb187e38eef |
| telemetry/repeat-2--p256-c10-p10--hydra--get.csv | 546 | 33b2788d406b5afdb38b446e4d412efc69a57fb281953dfdfa87d57076eaa1d9 |
| telemetry/repeat-2--p256-c10-p10--hydra--get.jsonl | 1347 | fca27f0392093e2fbda2e93b19335cd2900f527517b11406eb0e6b9cd008e015 |
| telemetry/repeat-2--p256-c10-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.csv | 546 | afce546581fc38b9170d5c079e9cc8c381c13a997b747a389fb30da52455b1a2 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.jsonl | 1347 | 7fd5c12a6b689f7e4b9b82767db2632f5a473366de40f4980c70216705f2aff3 |
| telemetry/repeat-2--p256-c10-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c10-p10--redis--get.csv | 368 | e979b1f9af74495691fe7ba262609fda3a82e9437832708477e49d78872d9038 |
| telemetry/repeat-2--p256-c10-p10--redis--get.jsonl | 447 | 65166caaa89331e879dbc7d2f07c4055c37f0c0ed5a9c6da191ce4e409e2eccc |
| telemetry/repeat-2--p256-c10-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c10-p10--redis--set.csv | 367 | 9a8a5fa2226e55eb3c6286a58cf8183b293e13ef6895b876df6ad5b05ee02309 |
| telemetry/repeat-2--p256-c10-p10--redis--set.jsonl | 446 | c7de0d31f47e53fb54dfede4e6823027464d196adaefe1e1355fd9960ab69a84 |
| telemetry/repeat-2--p256-c10-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.csv | 1076 | 64aa1f2debd8bdeb5cc193d51ebda8569db592a2074c8a4a9f293fbbd45a0e5c |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.jsonl | 3640 | a8b81291f2b16c7ace45cc76b3f88b6179bdcea45db1da1dd898c2cd7e43498a |
| telemetry/repeat-2--p256-c100-p1--hazelcast--get.metadata.json | 8029 | 96af6fca3fb0b749ef7b4d01d8dd816c0e887e9dda94f7ad07610fd98d5f8e01 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.csv | 1076 | 2f2b98972a36c4a84860779009b682efc731be51ca20d7af57b12f862a67dac2 |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.jsonl | 3640 | 29ece607b5e83aa347d0bdf168ff2722a9f3becffb226f46a808378633bdd27f |
| telemetry/repeat-2--p256-c100-p1--hazelcast--set.metadata.json | 8029 | 1478c847713dcb8afbca12c8f113a3f4923e943a62afb3cd07361be8335d405a |
| telemetry/repeat-2--p256-c100-p1--hydra--get.csv | 635 | b87d29e3f77825bce8535e7843562df8d8cbf49f8455a6a8ca9440b485a744dd |
| telemetry/repeat-2--p256-c100-p1--hydra--get.jsonl | 1795 | d5f474eee2d8e0b427bdfbb699e3f325c7c9686d22e8e5bd28046e6c7eb7513d |
| telemetry/repeat-2--p256-c100-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.csv | 635 | 3b857934438f2fd9ef69d0c041e08de8fa7eb489c1a55750f445a805320f0a16 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.jsonl | 1795 | 06d94ed97b40dc5990962e776d2c213acb08e1f587668b6b3b3b9792cc0b88d3 |
| telemetry/repeat-2--p256-c100-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p256-c100-p1--redis--get.csv | 639 | 33b49f552efa0dff5d2f1b5921e3d9a5674ad1b6780cf391426b3e1d3160b2b2 |
| telemetry/repeat-2--p256-c100-p1--redis--get.jsonl | 1783 | 7618a7b4f109112a0692bcb0907e23bed76075f43e875f83d6ee6ddd75321d9f |
| telemetry/repeat-2--p256-c100-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p256-c100-p1--redis--set.csv | 640 | b9f59bb8bf23380dc81d03be11baa90d975ebbdce5599a06535c32e819e8eca9 |
| telemetry/repeat-2--p256-c100-p1--redis--set.jsonl | 1784 | b69e42bbc5c5d8b4bbe20b3125e102b253a8711be27be3b5cec51181aa01432f |
| telemetry/repeat-2--p256-c100-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.csv | 8834 | 4dd30fa9ab8464b70cfaad4b34d69a6ffb93e443af3c604d9321fadf6685b2dd |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.jsonl | 39088 | cf05833ac454c832a2912005ba823a87d47948d0c14c7837fa5d0ce901f496dd |
| telemetry/repeat-2--p64-c10-p1--hazelcast--get.metadata.json | 8029 | aa47d6d066017d105a06da33506fed886686849a7d1b338fde1420797a5995b0 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.csv | 7148 | 8cf6567acd02ce031c723ad9de7a6366a98613823d70c5293c9e0a44867a3b26 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.jsonl | 31367 | 894550a898d227b2c070b895866d83120e363fee570777160415d9c0ef33ad79 |
| telemetry/repeat-2--p64-c10-p1--hazelcast--set.metadata.json | 8030 | b9232c801bd89de9bf4b8f91c4dc3e8cf6f593eac0cfd58098f09da31468533e |
| telemetry/repeat-2--p64-c10-p1--hydra--get.csv | 636 | 04abd3028c3e1e876a8eb3de7263d1d9f54a7ccc3516143acc663b2d88b4292c |
| telemetry/repeat-2--p64-c10-p1--hydra--get.jsonl | 1796 | 51303d6c771a57fb3e3b0afcc454ebff8741d01e79be71f790e18061f5a48a0c |
| telemetry/repeat-2--p64-c10-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p64-c10-p1--hydra--set.csv | 636 | a9264313a5004bc74164f4d110eb32e5bf4b6ab975d13aba62d4605a0abaf534 |
| telemetry/repeat-2--p64-c10-p1--hydra--set.jsonl | 1796 | 3c03c0b87de6dc137aa086d171ccf23d0498c54b522f960b9dbc8993771a37ec |
| telemetry/repeat-2--p64-c10-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p64-c10-p1--redis--get.csv | 642 | aeb79d6199c7b066b0eff253e01b32b83af9c73893372027d590c8b619e6baa7 |
| telemetry/repeat-2--p64-c10-p1--redis--get.jsonl | 1786 | 7c54116ea0313b4d0e29c2759d99da44437d6449484818cf9ce0e59fbb586194 |
| telemetry/repeat-2--p64-c10-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p64-c10-p1--redis--set.csv | 640 | 47172dae2260477df0cb86ba749a93e7f6a1fa9d78dacbbb6aa75ee081da4bb6 |
| telemetry/repeat-2--p64-c10-p1--redis--set.jsonl | 1784 | b4b03d85a54b522fce13ee23999bc27695069264d61e6dd10950259f0b3726e0 |
| telemetry/repeat-2--p64-c10-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.csv | 1372 | b62c280b1e4916d5f1cde3a8867fd83cb69be03ef89b39f6f4a9164f0dbbea62 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.jsonl | 5001 | 63aaa6667bf32f155de8715064bdc1634bb84ab6b94692a9bbc090a1228166cf |
| telemetry/repeat-2--p64-c10-p10--hazelcast--get.metadata.json | 8026 | 115594b5098f3b7971743e81faaa57cb2e19474366868378575629f4100156b7 |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.csv | 872 | 375382fd492ed88331c2f276a7768ff123a3c32eb76e8b651bb63e6b8ce800ef |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.jsonl | 2726 | 089406ccadad60ffa4dea0359f42f0300d954be5bdb87637c3d9a42a4ec087bb |
| telemetry/repeat-2--p64-c10-p10--hazelcast--set.metadata.json | 8026 | 115594b5098f3b7971743e81faaa57cb2e19474366868378575629f4100156b7 |
| telemetry/repeat-2--p64-c10-p10--hydra--get.csv | 546 | d483d84df993cd1a708f07238059c2b039bdb78fa87dfb1c7f6e2a5adbd13b2d |
| telemetry/repeat-2--p64-c10-p10--hydra--get.jsonl | 1347 | 25a2c099b2ce49e9a3322b7beb223d69672d3c94f4af4a94f459dc9563e20432 |
| telemetry/repeat-2--p64-c10-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p64-c10-p10--hydra--set.csv | 545 | dc0a09af0227988c2198b2557a7b7e03a233e3f7c4aff76ddfa5fa90fd3efd17 |
| telemetry/repeat-2--p64-c10-p10--hydra--set.jsonl | 1346 | 210cf759ce1e668655f14307922d685b68ddc25904fbbbea66db39fb672a4033 |
| telemetry/repeat-2--p64-c10-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-2--p64-c10-p10--redis--get.csv | 368 | 3179203ca9be5407ce54e9bd179a3d9baa0729d71ffb39b90f74b574073d9657 |
| telemetry/repeat-2--p64-c10-p10--redis--get.jsonl | 447 | c2248a62ced3f90bd756f5af903cffdb38d6ad7d4e9bf9c88d9383b0b5a82f76 |
| telemetry/repeat-2--p64-c10-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-2--p64-c10-p10--redis--set.csv | 367 | ee7a14fc3ac0d94910ea649bba3f4cef6c7eb174286138add9fd96739a290b42 |
| telemetry/repeat-2--p64-c10-p10--redis--set.jsonl | 446 | 88cc36150d3d86fb1e79a576fb69585fe9c18606f2f2b196b9185925ddcc820a |
| telemetry/repeat-2--p64-c10-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.csv | 1381 | 66db3486bc6ae1983ce04d8653b1ffea4f7c246710ce1c1eecef65fdaa69db28 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.jsonl | 5010 | 4f0c650f4b2ca0a56e8a4b74b4ce3f3c135347e9593f16a45f1967bd5ae82c0d |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--get.metadata.json | 8030 | 383067d06485896ce4113b228777a76060a98f63fb62a1950fefdc778ea45bed |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.csv | 1279 | 00e67b1edc9bb2b2d6f2a8eee59b5d551249b42f238fbb820f5b57193318d738 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.jsonl | 4553 | e04e72447488dabd4dba4ebfb40b46c3ff00511791b319dab170fc76c8b5e644 |
| telemetry/repeat-3--p1024-c50-p1--hazelcast--set.metadata.json | 8030 | 383067d06485896ce4113b228777a76060a98f63fb62a1950fefdc778ea45bed |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.csv | 635 | b4176e63ac4dc057362c90b5f3664028a3a2265beb0616cac7ecad572d4b7a2c |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.jsonl | 1795 | dfdc8a83de4c0ecd81aeb8867f1cce6000056749906850a2cfc43568780ea5bd |
| telemetry/repeat-3--p1024-c50-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.csv | 635 | 474cbf0114751db5cfa00398b703d3e32ec1b208d3ab6cb02c2d3a2763a87c78 |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.jsonl | 1795 | e726756b2291efa8fd6f1aa235572ea6ee7c6b03f6ee741df08a4beb4a2d0ad4 |
| telemetry/repeat-3--p1024-c50-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p1024-c50-p1--redis--get.csv | 647 | 11071c033c724f9c78f8a4fce229864874f1142c79f6cb439d3b96dd1a35bbe5 |
| telemetry/repeat-3--p1024-c50-p1--redis--get.jsonl | 1791 | 001c2816f96745abe7b9747d762a7dcfc0b2778ba0c5ff21f11cb5a27d0f0293 |
| telemetry/repeat-3--p1024-c50-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.csv | 646 | 19c991920624ce956f271fe3c97ca5ed75b34278da8a246e2bd5f95a26709481 |
| telemetry/repeat-3--p1024-c50-p1--redis--set.jsonl | 1790 | 563e512b318773de221ca25a6dfa436e63795f18da82fa3ac55a4fe286d3677d |
| telemetry/repeat-3--p1024-c50-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.csv | 878 | 3cbea228e0406a52345f06543c60ef1c4eb37615308c113395a6d0990d7b458b |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.jsonl | 2732 | 1d1023bfbcd3458872bdc18b61365e358bc7fe2fe73861fa730d22d72256c640 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--get.metadata.json | 8029 | 26dcc206fcf5abb6f0d3d7983a14fb2da32df4158fc1896f9223d42017efef39 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.csv | 773 | 9d6864f9bfdb7c7280746e6e31e57a2d38d26cc012301e57212c576da7d67089 |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.jsonl | 2272 | 174e37612199b28d7b2e4c8123925577fe16bd07927457179c7f4b721c49364a |
| telemetry/repeat-3--p1024-c50-p10--hazelcast--set.metadata.json | 8029 | 26dcc206fcf5abb6f0d3d7983a14fb2da32df4158fc1896f9223d42017efef39 |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.csv | 546 | a322b9130379feb60ba737127c60a27b0be1aa2d450693e6cf43c7b5d2ad665b |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.jsonl | 1347 | 8225b3804ed08e863b718ec7d195692563729ee1332725df472464d1f0e9a875 |
| telemetry/repeat-3--p1024-c50-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.csv | 545 | 16d7d00e5619892619a5eeb029c9db78d8e127f269261b25a0635cc9f71be547 |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.jsonl | 1346 | 45c1dc22212e02ddbb831aa3aa0a6d320a1320172fa9cd7877c97d8e5ad10fa7 |
| telemetry/repeat-3--p1024-c50-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p1024-c50-p10--redis--get.csv | 369 | b248c5d4492dba37de24e766b35a80e07a046f693df310464b66871cd36710ee |
| telemetry/repeat-3--p1024-c50-p10--redis--get.jsonl | 448 | 78007981934a0accb679487e834e677f19807fd47552b232111aeb0725008034 |
| telemetry/repeat-3--p1024-c50-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p1024-c50-p10--redis--set.csv | 368 | 55478a0f173a3507cdc7a48d30771d55a55bf6f89365b66378e70cf46304785b |
| telemetry/repeat-3--p1024-c50-p10--redis--set.jsonl | 447 | c7c482b0731e73d711d22ed1ecf2eb82e5dec306ef324ac529ffe35780589848 |
| telemetry/repeat-3--p1024-c50-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.csv | 1982 | 394e07c99ffb2e61d42a2c2aabed3220adba396acadc881d821e0bc02d67fe9f |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.jsonl | 7741 | 49f91d35e1ca8b34d063264e524cbeb150e32a7f6681a58cfb23cfe9ed86ee12 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--get.metadata.json | 8027 | a720e9ebc78e97926eb7c18de0a6a0ce0b230a91b76e18051e7dc7806c9e30ae |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.csv | 1984 | 63d3fbfa1b1568e717fde0fa1a50a5ac86bb3e9c5fcac30c69ce490536b776bd |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.jsonl | 7743 | 7c75378bbe90a5f0d06a2756bd349d7d8baf2fb44eb2a70b50215e02d1e40d28 |
| telemetry/repeat-3--p256-c1-p1--hazelcast--set.metadata.json | 8029 | d4df0f066b1c9738bd683f37457e25aa9674f7c7367b555638a137609a51405b |
| telemetry/repeat-3--p256-c1-p1--hydra--get.csv | 724 | 71b89b3bfe308ea065e5b1d80de7367c0a65379f3c9175e1a5db2c501b9c52a8 |
| telemetry/repeat-3--p256-c1-p1--hydra--get.jsonl | 2243 | a8dfe01347e71b321f9fd4d5b3960323eb0a2b686d76b540a46e64f806cc29bf |
| telemetry/repeat-3--p256-c1-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c1-p1--hydra--set.csv | 725 | 6eb152cdfd2b42a44a57b5c26371c257b75ca1d4b253087a7b8688b817735bce |
| telemetry/repeat-3--p256-c1-p1--hydra--set.jsonl | 2244 | 0aa176c7265453eb39952e1c7516770100d4316c1255b3573864f74458cbca00 |
| telemetry/repeat-3--p256-c1-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c1-p1--redis--get.csv | 732 | 87ba4eef4ada050dda110fbbd867ee8fe003089254ee0443e3f9042ee4b9928a |
| telemetry/repeat-3--p256-c1-p1--redis--get.jsonl | 2231 | 9d8838bbecbe1558e2bea9638b8c8c927e81bd51987d776c83ce47e9102a5c92 |
| telemetry/repeat-3--p256-c1-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c1-p1--redis--set.csv | 741 | 18860761fd47ac896bd8d6911de5d01eb327d7b5a2101e6d5e7796e98bc2f439 |
| telemetry/repeat-3--p256-c1-p1--redis--set.jsonl | 2240 | 447461a7febcf78db602bee1c7d6b6de0613e46529f6ac8947172841a9f215c1 |
| telemetry/repeat-3--p256-c1-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.csv | 8036 | afbd6fa88c7d66da30e3995ccd4113c7d48fa8e18580b17225951e60e21a2c01 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.jsonl | 35450 | ef9bae3099ad4e2b4bbd238bac94e0e6735b432b6f96d6d4ecec61ac83e3e970 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--get.metadata.json | 8030 | 0875bc76605dfc749c947e3543f2af5551805644dc743b4955025308db4dd38a |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.csv | 6743 | af2b0160a17335826dc7d1178be8105686cb8248df4b852acf1a2b9a9a8f0059 |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.jsonl | 29542 | c4964fe56f992b74cd267d5e0b91f59e3ca33edb597562b686aaaf095aac8c1a |
| telemetry/repeat-3--p256-c10-p1--hazelcast--set.metadata.json | 8030 | 2ccec27cf61c51a190357f20dabbd7d4559449715cec678715b6699daab80f32 |
| telemetry/repeat-3--p256-c10-p1--hydra--get.csv | 635 | 3999f10c060cecac379083e46d7534f0ff5427a3b561f1f587eb4cdf1886df4b |
| telemetry/repeat-3--p256-c10-p1--hydra--get.jsonl | 1795 | e73b43baa3f7e751c72da828cbde5a9460268c78fee36bf7d12b4562615aed3c |
| telemetry/repeat-3--p256-c10-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c10-p1--hydra--set.csv | 635 | 5e8702e675a0a94f8d60e78a820c234ff7c2ef4d88c359bdbeee0fa06bc037da |
| telemetry/repeat-3--p256-c10-p1--hydra--set.jsonl | 1795 | be183ab3857d4047865c3dba3481adcb94638b8273e1e0db180dd408b238458b |
| telemetry/repeat-3--p256-c10-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c10-p1--redis--get.csv | 643 | 395049b7cfd413a45023718bd82aa2b1219bd59502118434c18b2ab3b4ceb540 |
| telemetry/repeat-3--p256-c10-p1--redis--get.jsonl | 1787 | 394e690f101281d823aa13dfa1b27b26efec5e084b50e3c07c0339b148ac3244 |
| telemetry/repeat-3--p256-c10-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c10-p1--redis--set.csv | 643 | 95744cc9174bbe27e8a9d4ba20776bb1396e42815897bb8422e09ba1e21bfa2f |
| telemetry/repeat-3--p256-c10-p1--redis--set.jsonl | 1787 | d30a99b0bd65493a61f785f98609c5de6757f62651bb70c7ed59dbd42a1d4979 |
| telemetry/repeat-3--p256-c10-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.csv | 1465 | 2842e8554572ab927edd488a1d9e97f9f2bfbcf77945435fc61f14d0dbc88ca8 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.jsonl | 5449 | 86610c03be13db1ff00b3f39af1903abed367efd1e2bdb45973f6f509e146ca7 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--get.metadata.json | 8030 | 63511efc0af4f8d63ba4d5d978e5c6c264f2b46e75ece05b2dfdb0faf729c46c |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.csv | 775 | d9b086ae69c57d675681ce72e49e3283589a553649a9018e4e38cbcdc66c5ca9 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.jsonl | 2274 | 8a036a4d826b152c7f03b7ffc1ab8ad07118c2eec8fa00c0d6fc17d2f29c1559 |
| telemetry/repeat-3--p256-c10-p10--hazelcast--set.metadata.json | 8030 | 63511efc0af4f8d63ba4d5d978e5c6c264f2b46e75ece05b2dfdb0faf729c46c |
| telemetry/repeat-3--p256-c10-p10--hydra--get.csv | 546 | e57970184a0eaac0e47fd1432594cf82de3ac07266da86311d8e0913411f07ba |
| telemetry/repeat-3--p256-c10-p10--hydra--get.jsonl | 1347 | 607603847742864fec71c70f11645526f52bffb5f1544ab78fe990edb59cc599 |
| telemetry/repeat-3--p256-c10-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c10-p10--hydra--set.csv | 546 | 1b59ae9608225192bdbfac6a129e012fddab0945b48b6450a443d1d4fbd2b9a8 |
| telemetry/repeat-3--p256-c10-p10--hydra--set.jsonl | 1347 | a5e26a1efe9e3557a9057978cca1bff4d68245029c2a1b74147f0fbbfc16ef31 |
| telemetry/repeat-3--p256-c10-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c10-p10--redis--get.csv | 368 | a254bc20b5cf2d03f5dff6b45a35f6cfa433e87695dbe36f389cbb9cd0b6f11f |
| telemetry/repeat-3--p256-c10-p10--redis--get.jsonl | 447 | 64d9f30582a184e26ec0ac1409ae202ef05901acaf1094e7a5b2bdae8852b07d |
| telemetry/repeat-3--p256-c10-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c10-p10--redis--set.csv | 367 | 509b5bdd3fee33140c587a0c29dcb7d2842e5329e7f3ec49bd3bef7d72d9f62e |
| telemetry/repeat-3--p256-c10-p10--redis--set.jsonl | 446 | 0829dc200fa6c7a9d6cfe21457e5f8139e94fad9b05a44b9dbc84ecd6a747c5d |
| telemetry/repeat-3--p256-c10-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.csv | 1076 | 155085e4491a3326a28a28ecd8179c61620ba0b2ea68403783beaca233bb9c25 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.jsonl | 3640 | f4b8f443900d3c6c2b41ede49f635488fa2e7d6e7e1bed800bc5560d1fdc6ac7 |
| telemetry/repeat-3--p256-c100-p1--hazelcast--get.metadata.json | 8026 | cd3d471f3cf6ae4f6ef19f4ecc946e7695a05ff098eb4e287136943ef39c3dbc |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.csv | 1079 | 53b92a64db54a47861a4a37805bd7c3d75fb8c7a9855a564d8f7a57b1579943e |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.jsonl | 3643 | 0c80d83b20fca44d729a83f715a474d6b6419f25890a5501fc61542f5bad661b |
| telemetry/repeat-3--p256-c100-p1--hazelcast--set.metadata.json | 8027 | a720e9ebc78e97926eb7c18de0a6a0ce0b230a91b76e18051e7dc7806c9e30ae |
| telemetry/repeat-3--p256-c100-p1--hydra--get.csv | 635 | 8bf8f1af6da9e7cfc39ba6172566173a688889897b905989722766b4480ec31b |
| telemetry/repeat-3--p256-c100-p1--hydra--get.jsonl | 1795 | f790bcbc9fa05a1b358ec5694bc56375d3f71257a357e54dd9cc8587f490f7cf |
| telemetry/repeat-3--p256-c100-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c100-p1--hydra--set.csv | 635 | 502c0df9bf215ce70cfdb2b7e5acb40dc7bf680d3bbe5cbf6231feca2d914d7c |
| telemetry/repeat-3--p256-c100-p1--hydra--set.jsonl | 1795 | e5bfc1d649ea3b04fffb778233711dc3df2d055b0f8c98fd23a3ae70c875550d |
| telemetry/repeat-3--p256-c100-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p256-c100-p1--redis--get.csv | 639 | 9bef781e684631a2db7b0c1fc4c8809c596a5e672bd99f20ff18d23edd4db722 |
| telemetry/repeat-3--p256-c100-p1--redis--get.jsonl | 1783 | 3eb25b01752b64e0f508873e9e0dba7a8a3d157f6a0901004ca644fa667ba525 |
| telemetry/repeat-3--p256-c100-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p256-c100-p1--redis--set.csv | 639 | 8485b0b02cf61955ec84e143f3cbb204cec1722e05612ebb15c3a112e0f8f37b |
| telemetry/repeat-3--p256-c100-p1--redis--set.jsonl | 1783 | 0f9b2195f3f005e34d8c147f569ee485463908c6bff98a2a52ddbe8a6ad0aff4 |
| telemetry/repeat-3--p256-c100-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.csv | 8542 | e85e0cb2bd13a43d3f6616c2a80fa12005fded7fe50e021312d87aefa255c0d8 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.jsonl | 37731 | 7dc544b2f8db0e7d36e95d0d272764093d4ea4ef1dd632959e30715f8672e331 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--get.metadata.json | 8030 | 21b0c4b35e0228fbf5486796fdef178bac442a24ed00bbabb27d2cea1c046256 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.csv | 7250 | 275a7776ba93d8f4717c20729194b5c0b8889fde9725667e903559734db2c003 |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.jsonl | 31824 | a013dbb3700c604702bdea5fb0f6eebeb8aa81c5932fce71a5f9e188b157fcec |
| telemetry/repeat-3--p64-c10-p1--hazelcast--set.metadata.json | 8029 | 96af6fca3fb0b749ef7b4d01d8dd816c0e887e9dda94f7ad07610fd98d5f8e01 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.csv | 635 | 70a53628997a2938b64466837c843e58c88167aee4e17ce99f44837b00e7f905 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.jsonl | 1795 | bf642a992f1a1fb0e720c8232d528abc0645d47a3c15b386c716c69e7c0fc054 |
| telemetry/repeat-3--p64-c10-p1--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p64-c10-p1--hydra--set.csv | 635 | 9d5da862a54affcfab1a591df86b6871835428a0aed7bba2579c1136875f9d72 |
| telemetry/repeat-3--p64-c10-p1--hydra--set.jsonl | 1795 | 8d66507dd836e0f9783fe9b5fce2983394769c061e622961b6675773af76ab9c |
| telemetry/repeat-3--p64-c10-p1--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p64-c10-p1--redis--get.csv | 643 | 697de47cb018f51599c7b5907e6440c18643f9bc15736f8ccd93547230b2479a |
| telemetry/repeat-3--p64-c10-p1--redis--get.jsonl | 1787 | 291705246c45a0743b017c9c3f3c2f4d5d7905abb13da3bc18502e7606825727 |
| telemetry/repeat-3--p64-c10-p1--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p64-c10-p1--redis--set.csv | 644 | 6fa870a57ee920bccbb61c4dfe8ffbbdbf5bf54c1847482bb9fe1742de14ebde |
| telemetry/repeat-3--p64-c10-p1--redis--set.jsonl | 1788 | eb587bbe9b64bc5704773672529e4687a6b65b2219c950f35ec9ff698046f11e |
| telemetry/repeat-3--p64-c10-p1--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.csv | 1369 | f67a29a8af72927a7dce20f142ed8ab246cabb7a1bac5d998e24328afd295712 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.jsonl | 4998 | 8e4b558371d035aa26e94ae6043d9fcd804c5a74b1543ca423371a9b1f005e62 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--get.metadata.json | 8030 | d1b420217c7e5f9a131bfc09da71364c05f7310b54d7e274756bba138ecd55ed |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.csv | 877 | 980159400095eb79037cc5be6d0cd4f350bfdc70c150096994ba815c2ef9fcb2 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.jsonl | 2731 | fcd879eacfc3774cda97db1653bb855bdb1856d6b4104a298d4919dea07dd9a1 |
| telemetry/repeat-3--p64-c10-p10--hazelcast--set.metadata.json | 8030 | d1b420217c7e5f9a131bfc09da71364c05f7310b54d7e274756bba138ecd55ed |
| telemetry/repeat-3--p64-c10-p10--hydra--get.csv | 544 | 7e3c994dbc25dc2338d1e5a7910f2e7d6d440d6c6650b771a2660a1b0a1280aa |
| telemetry/repeat-3--p64-c10-p10--hydra--get.jsonl | 1345 | ada729021d53cb821d563218e29d0d2ebc7e3265ec3f4d817cb18e5c00781277 |
| telemetry/repeat-3--p64-c10-p10--hydra--get.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p64-c10-p10--hydra--set.csv | 546 | eba6ec012487b6ec28f783e2e5362ac35e7a5c729f621025cbbe0065d45307a7 |
| telemetry/repeat-3--p64-c10-p10--hydra--set.jsonl | 1347 | 5e8ff7b2fb3de505635746994cc57a02b8d1b649da858b311b7803275efeb1e7 |
| telemetry/repeat-3--p64-c10-p10--hydra--set.metadata.json | 153 | f719d0a3a00d84a9a1c0f0193870f4d620e3f3f33b19948e5f568bad54dbc397 |
| telemetry/repeat-3--p64-c10-p10--redis--get.csv | 368 | 943767523ae33bd839992944989637c161452baa9f4a037ded90df7ffde7683e |
| telemetry/repeat-3--p64-c10-p10--redis--get.jsonl | 447 | e290a4995c3f14454bff6fcb4421778be7dded8c068a4f3444b8cb3b058a1bd9 |
| telemetry/repeat-3--p64-c10-p10--redis--get.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry/repeat-3--p64-c10-p10--redis--set.csv | 366 | 426d647867d5b001934023f6f1379752cdfe704d64793ce6956e62d88472ba1c |
| telemetry/repeat-3--p64-c10-p10--redis--set.jsonl | 445 | e44554cd3cf7779613392650c3cb93458590a9cd291685975d836b80710887e6 |
| telemetry/repeat-3--p64-c10-p10--redis--set.metadata.json | 7377 | e450789ec9b4e6c2126a3afc54aeab38d96e4f128ac3426e97a3146d339ab779 |
| telemetry-summary.json | 94537 | 3d28f9316ab2ce58667f713dcb1f6c79bac64b1b3886ba3be7644587b68bc76c |

Raw benchmark logs, telemetry JSONL/CSV, Docker inspect metadata, image identifiers,
hardware validation, and the artifact manifest are all in this same output directory.
The directory must be copied unchanged into the branch results tree after review.
