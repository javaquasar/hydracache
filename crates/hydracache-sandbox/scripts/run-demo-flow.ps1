param(
    [string] $BaseUrl = "http://127.0.0.1:3000"
)

$ErrorActionPreference = "Stop"

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

Write-Host "`n1. Sandbox info"
Invoke-SandboxJson GET "/" | ConvertTo-Json -Depth 8

Write-Host "`n2. Readiness"
Invoke-SandboxJson GET "/ready" | ConvertTo-Json -Depth 8

Write-Host "`n3. Reset demo state"
Invoke-SandboxJson POST "/demo/reset" | ConvertTo-Json -Depth 8

Write-Host "`n4. First cached load should come from loader"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n5. Second cached load should come from cache"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n6. Raw local cache put/get"
Invoke-SandboxJson POST "/demo/cache/put" @{
    key = "manual:1"
    value = "alpha"
    ttl_ms = 5000
    tags = @("manual")
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/cache/get" @{
    key = "manual:1"
} | ConvertTo-Json -Depth 8

Write-Host "`n7. Query-cache load with explicit options"
Invoke-SandboxJson POST "/demo/query/users/42/load" @{
    ttl_ms = 5000
    tags = @("users")
} | ConvertTo-Json -Depth 8

Write-Host "`n8. Typed-cache namespaced load"
Invoke-SandboxJson POST "/demo/typed/users/7/load" @{
    ttl_ms = 5000
    tags = @("team:kernel")
} | ConvertTo-Json -Depth 8

Write-Host "`n9. Cached non-database function"
Invoke-SandboxJson POST "/demo/functions/double/21" | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/functions/double/21" | ConvertTo-Json -Depth 8

Write-Host "`n10. TTL scenario"
Invoke-SandboxJson POST "/demo/scenarios/ttl" @{
    key = "ttl:short"
    value = "short"
    ttl_ms = 50
    wait_ms = 80
} | ConvertTo-Json -Depth 8

Write-Host "`n11. Single-flight scenario"
Invoke-SandboxJson POST "/demo/scenarios/single-flight" @{
    key = "sf:1"
    loader_value = "shared"
    concurrency = 8
    loader_delay_ms = 50
    tags = @("sf")
} | ConvertTo-Json -Depth 8

Write-Host "`n12. Invalidation/load race scenario"
Invoke-SandboxJson POST "/demo/scenarios/invalidation-race" @{
    key = "race:1"
    loader_value = "stale"
    tag = "race"
    loader_delay_ms = 80
    invalidate_after_ms = 10
} | ConvertTo-Json -Depth 8

Write-Host "`n13. Negative scenarios"
Invoke-SandboxJson POST "/demo/negative/missing-key" @{
    key = "missing:script"
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/missing-user" @{
    id = 999999
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/loader-error" @{
    key = "loader:error"
    error = "simulated loader failure"
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/expired-entry" @{
    key = "expired:script"
    value = "gone"
    ttl_ms = 50
    wait_ms = 80
} | ConvertTo-Json -Depth 8
Invoke-SandboxJson POST "/demo/negative/invalidation-miss" @{
    tag = "missing-tag"
} | ConvertTo-Json -Depth 8

Write-Host "`n14. Update backing store without invalidating cache"
Invoke-SandboxJson POST "/demo/users/42" @{ name = "Grace" } | ConvertTo-Json -Depth 8

Write-Host "`n15. Load still returns cached value"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n16. Invalidate user tag"
Invoke-SandboxJson POST "/demo/invalidate/user/42" | ConvertTo-Json -Depth 8

Write-Host "`n17. Reload returns updated backing store value"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n18. Event log"
Invoke-SandboxJson GET "/demo/events" | ConvertTo-Json -Depth 8

Write-Host "`n19. Application report"
Invoke-SandboxJson GET "/demo/report" | ConvertTo-Json -Depth 8

Write-Host "`n20. Actuator diagnostics"
Invoke-SandboxJson GET "/actuator/hydracache/caches/main/diagnostics" | ConvertTo-Json -Depth 8
