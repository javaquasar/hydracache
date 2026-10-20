$ErrorActionPreference='Stop'
$taskSource='48854c1e67b8acf8eaf7a36fd2381ad2b52ac9ed'
$taskRecipe=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
function ReadJson([string]$Name){ Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Name) | ConvertFrom-Json }
function Receipt($Value,[string]$Mode){
    if($Value.source -cne $taskSource -or $Value.mode -cne $Mode -or $Value.recipe -cne $taskRecipe -or $Value.passed -isnot [bool] -or $Value.passed -cne $true){throw 'Receipt binding'}
    foreach($taskFlag in @('original_join','original_send','actual_role_build','production_enrollment','server','supervisor','product','new_live_or_fault_matrix','clock_changed')){
        if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $false){throw "Scope upgrade: $taskFlag"}
    }
}
function Image($Value,[string]$Mode,[int]$Tests){
    if($Value.source -cne $taskSource -or $Value.expected_tests -ne $Tests -or $Value.sha256 -cnotmatch '^[0-9a-f]{64}$' -or -not $Value.image -or $Value.actual_role_build -isnot [bool] -or $Value.actual_role_build -cne $false){throw 'Test image binding'}
    if($Mode -ceq 'windows'){
        if($Value.image -notmatch '^[A-Za-z]:\\.+\.exe$'){throw 'Windows test image path'}
    }elseif($Value.image -cnotmatch '^/home/artur/\.cache/hydracache-074-(194|188)-check/debug/deps/[^/]+$' -or -not $Value.image.StartsWith("/home/artur/.cache/hydracache-074-$Mode-check/debug/deps/",[StringComparison]::Ordinal)){throw 'Linux test image path'}
}
function TestLog([string]$Name,[int]$Tests){
    if(-not (Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Name)).Contains("test result: ok. $Tests passed; 0 failed; 0 ignored;")){throw "Test result: $Name"}
}
$taskInventory=ReadJson 'inventory.json'
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
$taskPaths=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskEntry in $taskInventory.files){
    if(-not $taskEntry.path -or $taskEntry.path -match '[:\\\x00-\x1f]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskPaths.Add($taskEntry.path)){throw 'Invalid member'}
    $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskEntry.path)
    if($taskFile.PSIsContainer -or $taskFile.Attributes -band [IO.FileAttributes]::ReparsePoint -or $taskFile.Length -ne $taskEntry.bytes -or (Get-FileHash -LiteralPath $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskEntry.sha256){throw "Changed member: $($taskEntry.path)"}
}
[string[]]$taskExpected=@($taskInventory.files.path)
$taskInventoryPath=Join-Path $PSScriptRoot 'inventory.json'
[string[]]$taskActual=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object FullName -CNE $taskInventoryPath | ForEach-Object {$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if(($taskActual -join "`n") -cne ($taskExpected -join "`n")){throw 'Unlisted, missing or unsorted member'}
foreach($taskMode in @('windows','194','188','repository')){
    Receipt (ReadJson "$taskMode-receipt.json") $taskMode
    $taskIdentity=ReadJson "$taskMode-identity.json"
    if($taskIdentity.source -cne $taskSource -or $taskIdentity.mode -cne $taskMode -or $taskIdentity.recipe -cne $taskRecipe -or $taskIdentity.local_topology_approved -isnot [bool] -or $taskIdentity.local_topology_approved -cne $true -or $taskIdentity.static_build_authority -isnot [bool] -or $taskIdentity.static_build_authority -cne $true){throw 'Identity binding'}
    foreach($taskFlag in @('actual_role_build','original_join','original_send','server','supervisor','product','new_live_or_fault_matrix','clock_changed')){
        if($taskIdentity.$taskFlag -isnot [bool] -or $taskIdentity.$taskFlag -cne $false){throw 'Identity scope upgrade'}
    }
    if($taskMode -ceq 'repository'){continue}
    foreach($taskName in @('held_build','diagnostic_artifacts','diagnostic_receipts')){
        $taskTests=if($taskName -ceq 'held_build'){16}elseif($taskName -ceq 'diagnostic_artifacts'){if($taskMode -ceq 'windows'){17}else{31}}else{if($taskMode -ceq 'windows'){8}else{9}}
        Image (ReadJson "$taskMode-$taskName-image.json") $taskMode $taskTests
        TestLog "$taskMode-$taskName.log" $taskTests
    }
    TestLog "$taskMode-docs.log" 2
}
TestLog 'repository-source-guards.log' 8
TestLog 'repository-old-bridge.log' 3
$taskSummary=ReadJson 'summary.json'
if($taskSummary.source -cne $taskSource -or $taskSummary.recipe -cne $taskRecipe -or $taskSummary.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or $taskSummary.old_sources_reconstructed -ne 437 -or $taskSummary.named_runtime_additions -ne 1){throw 'Summary binding'}
foreach($taskFlag in @('actual_role_build','original_join','original_send','production_enrollment','server','supervisor','product','new_live_or_fault_matrix','full_321_rerun','clock_changed')){
    if($taskSummary.$taskFlag -isnot [bool] -or $taskSummary.$taskFlag -cne $false){throw 'Summary scope upgrade'}
}
# In-memory metadata negative controls; no logs or sealed members are edited.
$taskNegative=0
$taskBase=ReadJson 'windows-receipt.json'
foreach($taskField in @('source','mode','recipe','passed','original_join','original_send','actual_role_build','production_enrollment','server','supervisor','product','new_live_or_fault_matrix','clock_changed')){
    foreach($taskKind in @('change','missing')){
        $taskBad=$taskBase | ConvertTo-Json | ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}
        elseif($taskField -ceq 'passed'){$taskBad.$taskField=$false}
        elseif($taskBad.$taskField -is [bool]){$taskBad.$taskField=$true}
        else{$taskBad.$taskField='wrong'}
        $taskRefused=$false
        try{Receipt $taskBad 'windows'}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted metadata mutation: $taskField/$taskKind"}
        $taskNegative++
    }
}
[ordered]@{verified=$true;files=$taskPaths.Count;metadata_mutation_refusals=$taskNegative;inventory_sha256=(Get-FileHash -LiteralPath $taskInventoryPath -Algorithm SHA256).Hash.ToLowerInvariant();static_build_authority_only=$true;actual_role_build=$false;original_join=$false;server=$false;product=$false} | ConvertTo-Json
