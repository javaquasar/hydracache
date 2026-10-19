param([switch]$SelfTest,[switch]$PreSeal)
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Check($Meta,$Inventory) {
    if ($Meta.kind -cne 'local-gnu-preparation-attribution' -or $Meta.functional_gate_passed -cne $false -or $Meta.server -cne $false -or $Meta.product -cne $false -or $Meta.repeats -ne 3) { throw 'Scope mismatch' }
    if ($Meta.preparation_completed_each -cne $true -or $Meta.historical_refusal_reproduced -cne $false) { throw 'Observed outcome mismatch' }
    if ($Meta.source_commit -cnotmatch '^[0-9a-f]{40}$') { throw 'Source identity missing' }
    if ($Meta.source_contract_commit -cne '2b984ff2a3e74ec3855af39af065bd996790a2a9' -or (Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'source-contract-identity.log')).Trim() -cne $Meta.source_contract_commit) { throw 'Static suite revision missing' }
    $taskManifest = (Get-FileHash -LiteralPath (Join-Path $taskRoot 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($Meta.qualification_manifest_sha256 -cne $taskManifest -or $taskManifest -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') { throw 'Qualification input drift' }
    $taskActual = @(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object Name -ne 'inventory.json' | ForEach-Object { [IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/') } | Sort-Object)
    $taskListed = @($Inventory.path | Sort-Object)
    if ($taskListed.Count -ne @($taskListed | Select-Object -Unique).Count -or (Compare-Object -CaseSensitive $taskActual $taskListed)) { throw 'Inventory differs' }
    foreach ($taskEntry in $Inventory) {
        if ($taskEntry.path.Contains('..') -or [IO.Path]::IsPathRooted($taskEntry.path)) { throw 'Invalid relative path' }
        if ((Get-FileHash -LiteralPath (Join-Path $PSScriptRoot $taskEntry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskEntry.sha256) { throw 'Raw byte mismatch' }
    }
    foreach ($taskCheck in @(
        @('source-focused.log','test result: ok. 3 passed; 0 failed;'),
        @('source-full-confirmed.log','test result: ok. 295 passed; 0 failed;'),
        @('source-adapter-focused.log','test result: ok. 3 passed; 0 failed;'),
        @('source-adapter-regressions.log','test result: ok. 3 passed; 0 failed;'),
        @('windows-unit.log','test result: ok. 54 passed; 0 failed;'),
        @('windows-doc.log','test result: ok. 18 passed; 0 failed;'),
        @('format-final.log','format_exit=0'),
        @('docs-final.log','doc-check: OK'),
        @('registry-final.log','gated-test-check: OK'),
        @('performance-final.log','performance-contract-check 0.74: OK (local, non-promotable)')
    )) {
        if (-not (Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $taskCheck[0])).Contains($taskCheck[1])) { throw "Missing check: $($taskCheck[0])" }
    }
    $taskRecipe = (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'run.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
    foreach ($taskPair in @(@('gnu194-confirmed','1.94.0'),@('gnu188-confirmed','1.88.0'))) {
        $taskDir = Join-Path $PSScriptRoot $taskPair[0]
        $taskBuild = Get-Content -LiteralPath (Join-Path $taskDir 'build.json') -Raw | ConvertFrom-Json
        $taskReceipt = Get-Content -LiteralPath (Join-Path $taskDir 'receipt.json') -Raw | ConvertFrom-Json
        foreach ($taskObject in @($taskBuild,$taskReceipt)) {
            if ($taskObject.source_commit -cne $Meta.source_commit -or $taskObject.compiler -cne $taskPair[1] -or $taskObject.runner_sha256 -cne $taskRecipe -or $taskObject.server -cne $false -or $taskObject.product -cne $false) { throw 'Cohort identity mismatch' }
        }
        if ($taskBuild.uid -cne '1000' -or $taskBuild.unit_tests -ne 2 -or $taskBuild.image -cne $taskReceipt.image -or $taskReceipt.image -cnotmatch '^[0-9a-f]{64}  /') { throw 'Build mismatch' }
        if (-not (Get-Content -Raw -LiteralPath (Join-Path $taskDir 'causal-units.log')).Contains('test result: ok. 2 passed; 0 failed;')) { throw 'Missing causal units' }
        if (-not (Get-Content -Raw -LiteralPath (Join-Path $taskDir 'clippy.log')).Contains('Finished `dev` profile')) { throw 'Strict clippy incomplete' }
        if ($taskReceipt.repeats -ne 3 -or $taskReceipt.functional_gate_passed -cne $false -or $taskReceipt.original_send -cne $false -or $taskReceipt.completion_allowed -cne $false -or $taskReceipt.end_to_end_original -cne $false -or $taskReceipt.diagnostic_only -cne $true) { throw 'Not a diagnostic receipt' }
        if ($taskReceipt.clock_service_before -cne 'active' -or $taskReceipt.clock_service_after -cne 'active' -or (Get-Content -Raw -LiteralPath (Join-Path $taskDir 'clock-restored.log')).Trim() -cne 'active') { throw 'Clock service not restored' }
        if ((Get-Content -Raw -LiteralPath (Join-Path $taskDir 'bundle-before.log')) -cne (Get-Content -Raw -LiteralPath (Join-Path $taskDir 'bundle-after.log'))) { throw 'Bundle drift' }
        foreach ($taskRepeat in 1..3) {
            $taskRaw = Get-Content -Raw -LiteralPath (Join-Path $taskDir "preparation-$taskRepeat.log")
            if (-not $taskRaw.Contains('test result: ok. 1 passed; 0 failed;') -or -not $taskRaw.Contains('HC074-PREPARATION-CLOSED admission=false renewal=false send=false server=false product=false')) { throw 'Incomplete/zero-test diagnostic' }
            if (-not $taskRaw.Contains('HC074-PREPARATION-OUTCOME phase=enrollment healthy=true kernel=simulated original_constructor=false send=false')) { throw 'Preparation did not complete' }
            $taskRows = @([regex]::Matches($taskRaw,'HC074-PREPARATION (\{[^\r\n]+\})') | ForEach-Object { $_.Groups[1].Value | ConvertFrom-Json })
            if ($taskRows.Count -ne 3 -or $taskRows[0].phase -cne 'consent' -or $taskRows[1].phase -cne 'outputs' -or $taskRows[2].phase -cne 'enrollment') { throw 'Missing preparation phases' }
            foreach ($taskRow in $taskRows) {
                if ($taskRow.clock.observations -lt 1 -or -not $taskRow.clock.first.issued_boot -or -not $taskRow.clock.last.now_boot -or $taskRow.auth_calls -ne $taskRow.signature_calls) { throw 'Incomplete observed facts' }
                if ($null -ne $taskRow.clock.first_refusal -or $taskRow.clock.first.issued_ns -ne $taskRows[0].clock.first.issued_ns -or $taskRow.clock.last.issued_ns -ne $taskRow.clock.first.issued_ns -or $taskRow.clock.last.age_ns -ge 10000000000 -or $taskRow.clock.last.age_ns -lt 0 -or $taskRow.clock.last.issued_boot -cne $taskRow.clock.last.now_boot) { throw 'Original challenge observation differs' }
            }
        }
    }
}
$taskMeta = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'metadata.json') | ConvertFrom-Json
$taskInventory = if ($PreSeal) {
    if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'inventory.json')) { throw 'Already sealed' }
    @(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | ForEach-Object {
        [pscustomobject]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    })
} else { @(Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') | ConvertFrom-Json) }
Check $taskMeta $taskInventory
if ($SelfTest) {
    foreach ($taskKind in @('gate','server','product','repeats','source','manifest','hash','missing')) {
        $taskCopy = $taskMeta | ConvertTo-Json -Depth 10 | ConvertFrom-Json
        $taskEntries = @($taskInventory | ConvertTo-Json -Depth 10 | ConvertFrom-Json)
        switch ($taskKind) {
            gate { $taskCopy.functional_gate_passed = $true }
            server { $taskCopy.server = $true }
            product { $taskCopy.product = $true }
            repeats { $taskCopy.repeats = 0 }
            source { $taskCopy.source_commit = '0' * 40 }
            manifest { $taskCopy.qualification_manifest_sha256 = '0' * 64 }
            hash { $taskEntries[0].sha256 = '0' * 64 }
            missing { $taskEntries = @($taskEntries | Select-Object -Skip 1) }
        }
        $taskRejected = $false
        try { Check $taskCopy $taskEntries } catch { $taskRejected = $true }
        if (-not $taskRejected) { throw "Mutation accepted: $taskKind" }
    }
}
'GNU preparation raw packet verified; functional gate remains separate; mutation_controls=8'
