param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$taskExpected=[ordered]@{
    schema='fixed-slice-namespace-local-evidence-v1'
    source='2f27453f48e038457b3591e98c43ce1f40a91e31'
    source_contracts_reference='75f3aeabde2c461bec9c4574c691b13e2bc1cc99'
    source_contracts_stable_head=$false
    baseline='9724e428b9850df8ff5aae34caa47ad8ba18e702'
    kernel_cases_per_compiler=9;ordinary_tests_per_compiler=9;docs_per_compiler=247
    source_contracts=321;fixture_source_guards=6;windows_lib_tests=60;windows_doc_tests=20
    qualification_manifest_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    host_anchors_unchanged=$true;cleanup=$true;production_runtime_changed=$false
    original_join=$false;manager_ownership=$false;exclusive_population=$false
    artifact_admission=$false;original_send=$false;server=$false;supervisor=$false
    product=$false;qualification=$false;clock_changed=$false
}
function CheckMetadata($Value){
    foreach($taskEntry in $taskExpected.GetEnumerator()){
        if($Value.PSObject.Properties.Name -cnotcontains $taskEntry.Key -or $Value.($taskEntry.Key) -cne $taskEntry.Value){throw "Scope mismatch: $($taskEntry.Key)"}
        if($taskEntry.Value -is [bool] -and $Value.($taskEntry.Key) -isnot [bool]){throw 'Boolean scope must not be text'}
        if($taskEntry.Value -is [string] -and $Value.($taskEntry.Key) -isnot [string]){throw 'Identity must be text'}
        if($taskEntry.Value -is [int] -and $Value.($taskEntry.Key) -isnot [int] -and $Value.($taskEntry.Key) -isnot [long]){throw 'Count must be an integer'}
    }
}
$taskInventory=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') | ConvertFrom-Json
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
$taskSeen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
$taskPaths=@()
foreach($taskMember in $taskInventory.files){
    $taskPath=$taskMember.path
    if(-not $taskPath -or $taskPath -match '[:\\]' -or $taskPath.StartsWith('/') -or $taskPath.Split('/') -contains '..' -or $taskPath.Split('/') -contains '.' -or $taskPath -ceq 'inventory.json' -or -not $taskSeen.Add($taskPath)){throw 'Unsafe or duplicate member'}
    $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskPath)
    if($taskFile.PSIsContainer -or $taskFile.Attributes -band [IO.FileAttributes]::ReparsePoint -or $taskFile.Length -ne $taskMember.bytes -or (Get-FileHash -LiteralPath $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskMember.sha256){throw "Changed member: $taskPath"}
    $taskPaths+=,$taskPath
}
[string[]]$taskSorted=$taskPaths.Clone()
[Array]::Sort($taskSorted,[StringComparer]::Ordinal)
if(($taskSorted -join "`n") -cne ($taskPaths -join "`n")){throw 'Inventory order'}
[string[]]$taskActual=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object Name -CNE 'inventory.json' | ForEach-Object {$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if(($taskActual -join "`n") -cne ($taskSorted -join "`n")){throw 'Unlisted or missing member'}
$taskMetadata=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'metadata.json') | ConvertFrom-Json
CheckMetadata $taskMetadata
$taskRecipe=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'run.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
foreach($taskPair in @(@('gnu194-final','1.94.0'),@('gnu188-final','1.88.0'))){
    $taskFolder=Join-Path $PSScriptRoot $taskPair[0]
    $taskReceipt=Get-Content -Raw -LiteralPath (Join-Path $taskFolder 'receipt.json') | ConvertFrom-Json
    $taskImage=Get-Content -Raw -LiteralPath (Join-Path $taskFolder 'image.json') | ConvertFrom-Json
    if($taskImage.source -cne $taskExpected.source -or $taskImage.compiler -cne $taskPair[1] -or $taskImage.recipe -cne $taskRecipe){throw 'Pre-execution image identity'}
    if($taskReceipt.source -cne $taskExpected.source -or $taskReceipt.compiler -cne $taskPair[1] -or $taskReceipt.recipe -cne $taskRecipe -or $taskReceipt.image -cne $taskImage.image -or $taskReceipt.image -notmatch '^[0-9a-f]{64}  /home/artur/.cache/hydracache-074-(194|188)-check/release/deps/' -or $taskReceipt.kernel_cases -ne 9 -or $taskReceipt.ordinary_tests -ne 9 -or $taskReceipt.doc_tests -ne 247){throw 'Compiler/image/recipe binding'}
    foreach($taskFlag in @('host_anchors_unchanged','cleanup')){if($taskReceipt.$taskFlag -cne $true){throw 'Kernel completion'}}
    foreach($taskFlag in @('production_runtime_changed','original_join','manager_ownership','exclusive_population','artifact_admission','original_send','server','supervisor','product','qualification','clock_changed')){if($taskReceipt.$taskFlag -cne $false){throw 'Compiler scope upgrade'}}
    $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskFolder 'kernel.log')
    if([regex]::Matches($taskRaw,'HC074-FIXED-NAMESPACE case=[a-z-]+ positive_pin=true checked=true observer_readonly=true original_join=false manager_ownership=false send=false').Count -ne 9 -or -not $taskRaw.Contains('HC074-FIXED-NAMESPACE cases=9 host_anchors_unchanged=true cleanup=true')){throw 'Kernel case record'}
}
$taskChecks=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'selected-checks/receipt.json') | ConvertFrom-Json
if($taskChecks.source -cne $taskExpected.source -or $taskChecks.source_tests -ne 6 -or $taskChecks.windows_lib_tests -ne 60 -or $taskChecks.windows_doc_tests -ne 20 -or $taskChecks.recipe -cne (Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'checks-selected.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Local check binding'}
foreach($taskFlag in @('strict_lint','format','registry','docs','local_contract')){if($taskChecks.$taskFlag -cne $true){throw 'Missing local check'}}
if(-not (Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'source-full-corrected.log')).Contains('test result: ok. 321 passed; 0 failed; 0 ignored;')){throw 'Source audit incomplete'}
$taskScope=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'source-audit-scope.json') | ConvertFrom-Json
if($taskScope.reference -cne $taskExpected.source_contracts_reference -or $taskScope.selected -cne $taskExpected.source -or $taskScope.input_diff_empty -cne $true -or $taskScope.fresh_full_source_audit_on_selected -cne $false){throw 'Reference audit relabeled'}
foreach($taskOld in @('gnu194','gnu188')){
    if(Test-Path -LiteralPath (Join-Path $PSScriptRoot "$taskOld/receipt.json")){throw 'Failed initial cohort promoted'}
    if(-not (Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$taskOld/kernel.log")).Contains('assertion `left != right` failed')){throw 'Original thread identity refusal missing'}
}
foreach($taskOld in @('gnu194-corrected','gnu188-corrected')){
    if(Test-Path -LiteralPath (Join-Path $PSScriptRoot "$taskOld/receipt.json")){throw 'Partial replacement cohort promoted'}
    $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot "$taskOld/kernel.log")
    if(-not $taskRaw.Contains('case=healthy positive_pin=true') -or -not $taskRaw.Contains('Operation not permitted')){throw 'Kernel rename refusal missing'}
}
$taskRefusals=0
if($SelfTest){
    foreach($taskEntry in $taskExpected.GetEnumerator()){
        $taskMutant=$taskMetadata | ConvertTo-Json -Depth 8 | ConvertFrom-Json
        if($taskEntry.Value -is [bool]){$taskMutant.($taskEntry.Key)=-not $taskEntry.Value}
        elseif($taskEntry.Value -is [string]){$taskMutant.($taskEntry.Key)='changed'}
        else{$taskMutant.($taskEntry.Key)=[int]$taskEntry.Value+1}
        $taskRejected=$false
        try {CheckMetadata $taskMutant}catch{$taskRejected=$true}
        if(-not $taskRejected){throw 'Metadata mutation accepted'}
        $taskRefusals++
    }
    $taskMutant=$taskMetadata | ConvertTo-Json -Depth 8 | ConvertFrom-Json
    $taskMutant.original_join='False'
    $taskRejected=$false
    try {CheckMetadata $taskMutant}catch{$taskRejected=$true}
    if(-not $taskRejected){throw 'Text boolean accepted'}
    $taskRefusals++
}
[ordered]@{verified=$true;files=$taskPaths.Count;metadata_mutations_refused=$taskRefusals;inventory_sha256=(Get-FileHash -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') -Algorithm SHA256).Hash.ToLowerInvariant();original_join=$false;server=$false;product=$false} | ConvertTo-Json
