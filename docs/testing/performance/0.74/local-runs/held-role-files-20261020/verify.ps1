$ErrorActionPreference='Stop'
$taskSource='82570db8589065df5711f2faf51d02406168cf79'
$taskRecipe=(Get-FileHash (Join-Path $PSScriptRoot 'final/checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
$taskCompletion=(Get-FileHash (Join-Path $PSScriptRoot 'final/complete-repository.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
$taskFlags=@('original_join','role_receipt_issued','original_send','server','supervisor','product','full_joined_sigkill','clock_changed')
function ReadJson([string]$Name){$taskPath=if($Name -in @('inventory.json','summary.json')){$Name}else{"final/$Name"};Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $taskPath)|ConvertFrom-Json}
function Scope($Value){foreach($taskFlag in $taskFlags){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $false){throw "Scope upgrade: $taskFlag"}}}
function Receipt($Value,[string]$Mode){
    $taskTests=if($Mode -ceq 'repository'){12}elseif($Mode -ceq 'windows'){16}else{26}
    $taskDocs=if($Mode -ceq 'repository'){0}elseif($Mode -ceq 'windows'){2}else{4}
    $taskRounds=if($Mode -ceq 'repository'){1}else{2}
    if($Value.source -cne $taskSource -or $Value.mode -cne $Mode -or $Value.recipe -cne $taskRecipe -or $Value.passed -isnot [bool] -or $Value.passed -cne $true -or $Value.tests -ne $taskTests -or $Value.docs -ne $taskDocs -or $Value.rounds -ne $taskRounds){throw 'Receipt binding'}
    Scope $Value
    if($Mode -ceq 'repository' -and ($Value.completion_recipe -cne $taskCompletion -or $Value.completion -cne 'silent-format-recheck-after-log-wrapper-refusal')){throw 'Repository completion binding'}
}
function TestLog([string]$Name,[int]$Count){
    $taskRaw=Get-Content -Raw (Join-Path $PSScriptRoot "final/$Name")
    if(-not $taskRaw.Contains("test result: ok. $Count passed; 0 failed; 0 ignored;") -or $taskRaw -match 'compiler unexpectedly panicked|internal compiler error|thread .rustc. .*panicked'){throw "Test selection or compiler panic: $Name"}
}
$taskInventory=ReadJson 'inventory.json'
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
if(@(Get-ChildItem -LiteralPath $PSScriptRoot -Force -Recurse | Where-Object {$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Reparse member'}
$taskMembers=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskEntry in $taskInventory.files){
    if(-not $taskEntry.path -or $taskEntry.path -match '[:\\\x00-\x1f]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskMembers.Add($taskEntry.path)){throw 'Invalid member'}
    $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskEntry.path)
    if($taskFile.PSIsContainer -or $taskFile.Attributes -band [IO.FileAttributes]::ReparsePoint -or $taskFile.Length -ne $taskEntry.bytes -or (Get-FileHash $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskEntry.sha256){throw "Changed member: $($taskEntry.path)"}
}
$taskInventoryPath=Join-Path $PSScriptRoot 'inventory.json'
[string[]]$taskExpected=@($taskInventory.files.path)
[string[]]$taskActual=@(Get-ChildItem -LiteralPath $PSScriptRoot -Force -File -Recurse|Where-Object FullName -CNE $taskInventoryPath|ForEach-Object {$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if(($taskActual -join "`n") -cne ($taskExpected -join "`n")){throw 'Unlisted, missing or unsorted member'}
foreach($taskMode in @('windows','194','188','repository')){
    Receipt (ReadJson "$taskMode-receipt.json") $taskMode
    $taskIdentity=ReadJson "$taskMode-identity.json"
    if($taskIdentity.source -cne $taskSource -or $taskIdentity.mode -cne $taskMode -or $taskIdentity.recipe -cne $taskRecipe){throw 'Identity binding'}
    Scope $taskIdentity
    if($taskMode -ceq 'repository'){continue}
    $taskImage=ReadJson "$taskMode-image.json"
    if($taskImage.source -cne $taskSource -or $taskImage.mode -cne $taskMode -or $taskImage.recipe -cne $taskRecipe -or $taskImage.sha256 -cnotmatch '^[0-9a-f]{64}$' -or $taskImage.role_receipt_issued -isnot [bool] -or $taskImage.role_receipt_issued -cne $false){throw 'Image binding'}
    if($taskMode -ceq 'windows'){if($taskImage.path -notmatch '^[A-Za-z]:\\.+\.exe$'){throw 'Windows image path'}}
    elseif(-not $taskImage.path.StartsWith("/home/artur/.cache/hydracache-074-$taskMode-check/debug/",[StringComparison]::Ordinal)){throw 'Linux compiler image path'}
    $taskCount=if($taskMode -ceq 'windows'){16}else{26}
    $taskDocCount=if($taskMode -ceq 'windows'){2}else{4}
    foreach($taskRound in 1..2){
        TestLog "$taskMode-tests-$taskRound.log" $taskCount
        if($taskMode -cne 'windows' -and -not (Get-Content -Raw (Join-Path $PSScriptRoot "final/$taskMode-tests-$taskRound.log")).Contains('held_role_files seed=0x0074f11e corruptions=32')){throw 'Missing corruption controls'}
    }
    TestLog "$taskMode-capability-docs.log" $taskDocCount
}
TestLog 'repository-source-guards.log' 12
TestLog 'repository-old-bridge.log' 3
$taskFormat=ReadJson 'repository-format-recheck.json'
if($taskFormat.source -cne $taskSource -or $taskFormat.recipe -cne $taskCompletion -or $taskFormat.format_exit -ne 0 -or $taskFormat.passed -isnot [bool] -or $taskFormat.passed -cne $true -or $taskFormat.selected_inputs_unchanged -isnot [bool] -or $taskFormat.selected_inputs_unchanged -cne $true){throw 'Format recheck binding'}
$taskSummary=ReadJson 'summary.json'
if($taskSummary.source -cne $taskSource -or $taskSummary.recipe -cne $taskRecipe -or $taskSummary.old_sources_reconstructed -ne 440 -or $taskSummary.named_runtime_additions -ne 1 -or $taskSummary.new_linux_tests -ne 10 -or $taskSummary.full_321_rerun -isnot [bool] -or $taskSummary.full_321_rerun -cne $false -or $taskSummary.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Summary binding'}
Scope $taskSummary
if(-not (Get-Content -Raw (Join-Path $PSScriptRoot 'missing-method-red.log')).Contains('no method named `pin_sealed_roles`')){throw 'Missing preimplementation refusal'}
# In-memory metadata controls never modify the sealed packet.
$taskNegative=0;$taskBase=ReadJson 'windows-receipt.json'
foreach($taskField in (@('source','mode','recipe','passed','tests','docs','rounds')+$taskFlags)){
    foreach($taskKind in @('change','missing')){
        $taskBad=$taskBase|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}
        elseif($taskField -ceq 'passed'){$taskBad.$taskField=$false}
        elseif($taskField -in @('tests','docs','rounds')){$taskBad.$taskField=99}
        elseif($taskBad.$taskField -is [bool]){$taskBad.$taskField=$true}
        else{$taskBad.$taskField='wrong'}
        $taskRefused=$false;try{Receipt $taskBad 'windows'}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted metadata mutation: $taskField/$taskKind"};$taskNegative++
    }
}
$taskBase=ReadJson 'repository-receipt.json'
foreach($taskField in @('completion_recipe','completion')){
    foreach($taskKind in @('change','missing')){
        $taskBad=$taskBase|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}else{$taskBad.$taskField='wrong'}
        $taskRefused=$false;try{Receipt $taskBad 'repository'}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted completion mutation: $taskField/$taskKind"};$taskNegative++
    }
}
[ordered]@{verified=$true;files=$taskMembers.Count;receipt_metadata_mutations_refused=$taskNegative;inventory_sha256=(Get-FileHash $taskInventoryPath -Algorithm SHA256).Hash.ToLowerInvariant();new_linux_tests=10;original_join=$false;server=$false;product=$false}|ConvertTo-Json
