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

Write-Host "`n2. First cached load should come from loader"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n3. Second cached load should come from cache"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n4. Update backing store without invalidating cache"
Invoke-SandboxJson POST "/demo/users/42" @{ name = "Grace" } | ConvertTo-Json -Depth 8

Write-Host "`n5. Load still returns cached value"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n6. Invalidate user tag"
Invoke-SandboxJson POST "/demo/invalidate/user/42" | ConvertTo-Json -Depth 8

Write-Host "`n7. Reload returns updated backing store value"
Invoke-SandboxJson POST "/demo/load/42" | ConvertTo-Json -Depth 8

Write-Host "`n8. Actuator diagnostics"
Invoke-SandboxJson GET "/actuator/hydracache/caches/main/diagnostics" | ConvertTo-Json -Depth 8
