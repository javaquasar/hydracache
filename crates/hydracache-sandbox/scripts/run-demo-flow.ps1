param(
    [string] $BaseUrl = "http://127.0.0.1:3000"
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
    if ($null -eq $Body) {
        return Invoke-RestMethod -Method $Method -Uri $uri
    }

    return Invoke-RestMethod `
        -Method $Method `
        -Uri $uri `
        -ContentType "application/json" `
        -Body ($Body | ConvertTo-Json -Depth 8)
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

Write-Host "`n24. Actuator diagnostics"
Invoke-SandboxJson GET "/actuator/hydracache/caches/main/diagnostics" | ConvertTo-Json -Depth 8
