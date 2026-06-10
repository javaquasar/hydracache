param(
    [string] $BaseUrl = "http://127.0.0.1:3000",
    [string] $Token = ""
)

$ErrorActionPreference = "Stop"
$FlowId = "script-flow-$(Get-Date -Format yyyyMMddHHmmss)"

function Invoke-SandboxJson {
    param(
        [ValidateSet("GET", "POST")]
        [string] $Method,
        [string] $Path,
        [object] $Body = $null
    )

    $uri = "$BaseUrl$Path"
    $headers = @{}
    if ($Token -ne "") {
        $headers["Authorization"] = "Bearer $Token"
    }
    if ($null -eq $Body) {
        return Invoke-RestMethod -Method $Method -Uri $uri -Headers $headers
    }

    return Invoke-RestMethod `
        -Method $Method `
        -Uri $uri `
        -Headers $headers `
        -ContentType "application/json" `
        -Body ($Body | ConvertTo-Json -Depth 8)
}

function Invoke-SandboxText {
    param(
        [string] $Path
    )

    $headers = @{}
    if ($Token -ne "") {
        $headers["Authorization"] = "Bearer $Token"
    }

    return Invoke-RestMethod -Method GET -Uri "$BaseUrl$Path" -Headers $headers
}

Write-Host "HydraCache sandbox demo flow against $BaseUrl"
Write-Host "Flow id: $FlowId"

Write-Host "`n1. Sandbox info"
Invoke-SandboxJson GET "/" | ConvertTo-Json -Depth 8

Write-Host "`n2. Readiness"
Invoke-SandboxJson GET "/ready" | ConvertTo-Json -Depth 8

Write-Host "`n3. Config and presets"
Invoke-SandboxJson GET "/demo/config" | ConvertTo-Json -Depth 8
Invoke-SandboxJson GET "/demo/presets" | ConvertTo-Json -Depth 8

Write-Host "`n4. Reset demo state"
Invoke-SandboxJson POST "/demo/reset" | ConvertTo-Json -Depth 8

Write-Host "`n5. First cached load should come from loader"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n6. Second cached load should come from cache"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n7. Raw local cache put/get"
Invoke-SandboxJson POST "/demo/cache/put" @{
    key = "manual:1"
    value = "alpha"
    ttl_ms = 5000
    tags = @("manual")
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/cache/get" @{
    key = "manual:1"
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n8. Query-cache load with explicit options"
Invoke-SandboxJson POST "/demo/query/users/42/load" @{
    ttl_ms = 5000
    tags = @("users")
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n9. Typed-cache namespaced load"
Invoke-SandboxJson POST "/demo/typed/users/7/load" @{
    ttl_ms = 5000
    tags = @("team:kernel")
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n10. Cached non-database function"
Invoke-SandboxJson POST "/demo/functions/double/21" | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/functions/double/21" | ConvertTo-Json -Depth 8

Write-Host "`n11. TTL scenario"
Invoke-SandboxJson POST "/demo/scenarios/ttl" @{
    key = "ttl:short"
    value = "short"
    ttl_ms = 50
    wait_ms = 80
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n12. Single-flight scenario"
Invoke-SandboxJson POST "/demo/scenarios/single-flight" @{
    key = "sf:1"
    loader_value = "shared"
    concurrency = 8
    loader_delay_ms = 50
    tags = @("sf")
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n13. Invalidation/load race scenario"
Invoke-SandboxJson POST "/demo/scenarios/invalidation-race" @{
    key = "race:1"
    loader_value = "stale"
    tag = "race"
    loader_delay_ms = 80
    invalidate_after_ms = 10
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n14. Negative scenarios"
Invoke-SandboxJson POST "/demo/negative/missing-key" @{
    key = "missing:script"
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/missing-user" @{
    id = 999999
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/loader-error" @{
    key = "loader:error"
    error = "simulated loader failure"
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/expired-entry" @{
    key = "expired:script"
    value = "gone"
    ttl_ms = 50
    wait_ms = 80
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/invalidation-miss" @{
    tag = "missing-tag"
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n15. Update backing store without invalidating cache"
Invoke-SandboxJson POST "/demo/users/42" @{
    name = "Grace"
    flow_id = $FlowId
} | ConvertTo-Json -Depth 8

Write-Host "`n16. Load still returns cached value"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n17. Invalidate user tag"
Invoke-SandboxJson POST "/demo/invalidate/user/42" | ConvertTo-Json -Depth 8

Write-Host "`n18. Reload returns updated backing store value"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n19. Filtered event log for this flow"
Invoke-SandboxJson GET "/demo/events?flow_id=$FlowId&limit=50" | ConvertTo-Json -Depth 8

Write-Host "`n20. Event log"
Invoke-SandboxJson GET "/demo/events" | ConvertTo-Json -Depth 8

Write-Host "`n21. Application report"
Invoke-SandboxJson GET "/demo/report" | ConvertTo-Json -Depth 8

Write-Host "`n22. Export bundle"
Invoke-SandboxJson GET "/demo/export" | ConvertTo-Json -Depth 8

Write-Host "`n23. Built-in self-test"
Invoke-SandboxJson POST "/demo/self-test" | ConvertTo-Json -Depth 8

Write-Host "`n24. Scenario runner and timeline"
Invoke-SandboxJson POST "/demo/scenarios/run" @{
    scenario = "golden-path"
    flow_id = "$FlowId-runner"
    reset = $true
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson GET "/demo/flows/$FlowId-runner/timeline" | ConvertTo-Json -Depth 8

Write-Host "`n25. Compare supported local profiles"
Invoke-SandboxJson POST "/demo/profiles/compare" @{
    scenario = "ttl"
    profiles = @("memory", "sqlite-memory", "sqlite-file", "postgres-compose")
} | ConvertTo-Json -Depth 8

Write-Host "`n26. Replay and fault injection"
Invoke-SandboxJson POST "/demo/replay" @{
    scenario = "negative-suite"
    source_flow_id = "$FlowId-runner"
    flow_id = "$FlowId-replay"
    reset = $true
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/faults/run" @{
    scenario = "invalidation-race"
    loader_delay_ms = 80
    invalidate_after_ms = 10
    flow_id = "$FlowId-fault"
} | ConvertTo-Json -Depth 8

Write-Host "`n27. Manual benchmark and security"
Invoke-SandboxJson POST "/demo/benchmarks/manual" @{
    key_prefix = "script-bench"
    requests = 64
    concurrency = 8
    unique_keys = 4
    loader_delay_ms = 5
    flow_id = "$FlowId-bench"
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson GET "/demo/security" | ConvertTo-Json -Depth 8

Write-Host "`n28. Scenario document DSL"
Invoke-SandboxJson POST "/demo/scenarios/document/run" @{
    name = "script-dsl-golden"
    flow_id = "$FlowId-dsl"
    reset = $true
    steps = @(
        @{
            name = "first load"
            action = "load-user"
            id = 42
            ttl_ms = 5000
            tags = @("script-dsl")
            expected_source = "loader"
        },
        @{
            name = "second load"
            action = "load-user"
            id = 42
            ttl_ms = 5000
            tags = @("script-dsl")
            expected_source = "cache"
        }
    )
    assertions = @(
        @{
            name = "all steps pass"
            metric = "failed-steps"
            op = "eq"
            value = 0
        },
        @{
            name = "has cache hit"
            metric = "cache-hits"
            op = "gte"
            value = 1
        }
    )
} | ConvertTo-Json -Depth 12
Invoke-SandboxJson GET "/demo/flows/$FlowId-dsl/timeline" | ConvertTo-Json -Depth 8

Write-Host "`n29. Benchmark compare"
Invoke-SandboxJson POST "/demo/benchmarks/compare" @{
    baseline = @{
        key_prefix = "script-bench-a"
        requests = 64
        concurrency = 8
        unique_keys = 4
        loader_delay_ms = 5
        flow_id = "$FlowId-bench-a"
    }
    candidate = @{
        key_prefix = "script-bench-b"
        requests = 64
        concurrency = 8
        unique_keys = 16
        loader_delay_ms = 5
        flow_id = "$FlowId-bench-b"
    }
} | ConvertTo-Json -Depth 8

Write-Host "`n30. Observability, seed report, and OpenAPI client check"
Invoke-SandboxJson GET "/demo/observability/traces/latest" | ConvertTo-Json -Depth 8
Invoke-SandboxText "/demo/observability/prometheus"
Invoke-SandboxJson GET "/demo/db/seed-report" | ConvertTo-Json -Depth 8
Invoke-SandboxJson GET "/demo/openapi/client-check" | ConvertTo-Json -Depth 8

Write-Host "`n31. Actuator diagnostics"
Invoke-SandboxJson GET "/actuator/hydracache/caches/main/diagnostics" | ConvertTo-Json -Depth 8

Write-Host "`n32. Committed scenario files and suite"
Invoke-SandboxJson GET "/demo/scenarios/files" | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/scenarios/file/run" @{
    path = "golden-path.yaml"
    format = "yaml"
} | ConvertTo-Json -Depth 12
Invoke-SandboxJson POST "/demo/scenarios/suite/file/run" @{
    path = "regression-suite.json"
} | ConvertTo-Json -Depth 12

Write-Host "`n33. Seeded product query cache"
Invoke-SandboxJson POST "/demo/query/products/200/load" @{
    ttl_ms = 5000
    tags = @("products")
    flow_id = "$FlowId-product"
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/query/products/200/load" @{
    ttl_ms = 5000
    tags = @("products")
    flow_id = "$FlowId-product"
} | ConvertTo-Json -Depth 8

Write-Host "`n34. Seeded order summary query cache"
Invoke-SandboxJson POST "/demo/query/orders/5001/summary/load" @{
    ttl_ms = 5000
    tags = @("orders")
    flow_id = "$FlowId-order"
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/query/orders/5001/summary/load" @{
    ttl_ms = 5000
    tags = @("orders")
    flow_id = "$FlowId-order"
} | ConvertTo-Json -Depth 8

Write-Host "`n35. Flow catalog and retained-flow replay"
Invoke-SandboxJson GET "/demo/flows" | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/flows/$FlowId-product/replay" @{
    scenario = "golden-path"
    flow_id = "$FlowId-flow-replay"
    reset = $true
} | ConvertTo-Json -Depth 12

Write-Host "`n36. OpenAPI generated-client smoke"
Invoke-SandboxJson GET "/demo/openapi/client-smoke" | ConvertTo-Json -Depth 8

Write-Host "`n37. Cluster lifecycle demo"
Invoke-SandboxJson POST "/demo/cluster/lifecycle/run" @{
    cluster = "script-cluster"
    key = "script-cluster:tagged"
    second_key = "script-cluster:key"
    retained_key = "script-cluster:retained"
    tag = "script-cluster"
    value = "alpha"
    flow_id = "$FlowId-cluster"
} | ConvertTo-Json -Depth 12
