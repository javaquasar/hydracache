param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskReceiptRoot=Join-Path $PSScriptRoot 'stable-clock'
$taskRecipe=(Get-FileHash (Join-Path $PSScriptRoot 'checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
$taskFlags=@('privileged_full_join','late_original_join','production_enrollment','production_role_receipt_issued','original_send','server','supervisor','product','full_joined_sigkill','clock_changed')
function ReadJson([string]$Name){$taskFolder=if($Name -in @('inventory.json','summary.json')){$PSScriptRoot}else{$taskReceiptRoot};Get-Content -Raw -LiteralPath (Join-Path $taskFolder $Name)|ConvertFrom-Json}
function Scope($Value,[string]$Mode){
    foreach($taskFlag in $taskFlags){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $false){throw "Scope upgrade: $taskFlag"}}
    foreach($taskFlag in @('typed_consuming_join','kernel_role_bridge')){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne ($Mode -in @('194','188'))){throw 'Consuming scope'}}
}
function Receipt($Value,[string]$Mode){
    $taskNew=if($Mode -in @('194','188')){17}else{0}
    $taskOld=if($Mode -ceq 'repository'){18}elseif($Mode -ceq 'windows'){28}else{38}
    $taskDocs=if($Mode -ceq 'repository'){0}elseif($Mode -ceq 'windows'){4}else{11}
    $taskRounds=if($Mode -ceq 'repository'){1}else{2}
    if($Value.source -cne $Source -or $Value.mode -cne $Mode -or $Value.recipe -cne $taskRecipe -or $Value.passed -isnot [bool] -or $Value.passed -cne $true -or $Value.new_tests -ne $taskNew -or $Value.old_tests -ne $taskOld -or $Value.docs -ne $taskDocs -or $Value.rounds -ne $taskRounds){throw 'Receipt binding'}
    Scope $Value $Mode
}
function TestLog([string]$Name,[int]$Count){
    $taskRaw=Get-Content -Raw (Join-Path $taskReceiptRoot $Name)
    if(-not $taskRaw.Contains("test result: ok. $Count passed; 0 failed; 0 ignored;") -or $taskRaw -match 'compiler unexpectedly panicked|internal compiler error|thread .rustc. .*panicked'){throw "Test selection or compiler panic: $Name"}
}
function FormatReceipt($Value){
    if($Value.source -cne $Source -or $Value.mode -cne 'repository' -or $Value.check -cne 'fmt' -or ($Value.native_exit -isnot [long] -and $Value.native_exit -isnot [int]) -or $Value.native_exit -ne 0 -or $Value.empty_output -isnot [bool] -or $Value.empty_output -cne $true){throw 'Silent format receipt'}
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
    if($taskIdentity.source -cne $Source -or $taskIdentity.mode -cne $taskMode -or $taskIdentity.recipe -cne $taskRecipe){throw 'Identity binding'}
    Scope $taskIdentity $taskMode
    if($taskMode -ceq 'repository'){continue}
    $taskSelections=if($taskMode -ceq 'windows'){@('old')}else{@('old','new')}
    foreach($taskSelection in $taskSelections){
        $taskImage=ReadJson "$taskMode-$taskSelection-image.json"
        if($taskImage.source -cne $Source -or $taskImage.mode -cne $taskMode -or $taskImage.recipe -cne $taskRecipe -or $taskImage.selection -cne $taskSelection -or $taskImage.sha256 -cnotmatch '^[0-9a-f]{64}$'){throw 'Image binding'}
        if($taskMode -ceq 'windows'){if($taskImage.path -notmatch '^[A-Za-z]:\\.+\.exe$'){throw 'Windows image path'}}
        elseif(-not $taskImage.path.StartsWith("/home/artur/.cache/hydracache-074-$taskMode-check/debug/",[StringComparison]::Ordinal)){throw 'Linux image path'}
        $taskCount=if($taskSelection -ceq 'new'){17}elseif($taskMode -ceq 'windows'){28}else{38}
        foreach($taskRound in 1..2){
            TestLog "$taskMode-$taskSelection-tests-$taskRound.log" $taskCount
            if($taskSelection -ceq 'new' -and -not (Get-Content -Raw (Join-Path $taskReceiptRoot "$taskMode-$taskSelection-tests-$taskRound.log")).Contains('consuming_held_join seed=0x0074c05e substitutions=128')){throw 'Missing seeded controls'}
        }
    }
    if($taskMode -cne 'windows'){foreach($taskRound in 1..2){TestLog "$taskMode-trust-$taskRound.log" 26;TestLog "$taskMode-fixed-$taskRound.log" 5}}
    TestLog "$taskMode-old-capability-docs.log" $(if($taskMode -ceq 'windows'){4}else{6})
    if($taskMode -cne 'windows'){TestLog "$taskMode-new-capability-docs.log" 5}
}
TestLog 'repository-source-guards.log' 18
TestLog 'repository-old-bridge.log' 3
FormatReceipt (ReadJson 'repository-fmt-empty-output.json')
$taskSummary=ReadJson 'summary.json'
if($taskSummary.source -cne $Source -or $taskSummary.recipe -cne $taskRecipe -or $taskSummary.old_sources_reconstructed -ne 443 -or $taskSummary.named_runtime_additions -ne 3 -or $taskSummary.full_321_rerun -isnot [bool] -or $taskSummary.full_321_rerun -cne $false -or $taskSummary.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Summary binding'}
Scope $taskSummary '194'
if($taskSummary.first_frozen_cohort_accepted -isnot [bool] -or $taskSummary.first_frozen_cohort_accepted -cne $false -or $taskSummary.preceding_refusal_attributed -isnot [bool] -or $taskSummary.preceding_refusal_attributed -cne $false){throw 'Do not erase the unclassified preceding refusal'}
if((Get-FileHash (Join-Path $PSScriptRoot 'first-checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant() -cne 'a782f74fcb92448b1f57b6d216e5a92fcb06f1d8528b04e89e64496d252462c6'){throw 'First recipe changed'}
if((Get-FileHash (Join-Path $PSScriptRoot 'second-checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant() -cne '56bf4f95a5c87d4ae8da8bc9c738cf4985d2a99e3aeb6372dc26ba8718dec0c7'){throw 'Second recipe changed'}
if(-not (Get-Content -Raw (Join-Path $PSScriptRoot 'confirmed/188-new-tests-1.log')).Contains('site: "clock-order"')){throw 'Recorded real clock refusal missing'}
$taskClock=Get-Content -Raw (Join-Path $PSScriptRoot 'clock-control-receipt.json')|ConvertFrom-Json
if($taskClock.source -cne $Source -or $taskClock.complete -isnot [bool] -or $taskClock.complete -cne $true -or $taskClock.local_timesyncd_before -cne 'active' -or $taskClock.local_timesyncd_during -cne 'inactive' -or $taskClock.local_timesyncd_after -cne 'active' -or $taskClock.service -cne 'systemd-timesyncd.service' -or $taskClock.distribution -cne 'Ubuntu' -or $taskClock.control_recipe -cne (Get-FileHash (Join-Path $PSScriptRoot 'clock-control.ps1') -Algorithm SHA256).Hash.ToLowerInvariant() -or $taskClock.probe_recipe -cne (Get-FileHash (Join-Path $PSScriptRoot 'clock-probe.py') -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Exact local clock control binding'}
foreach($taskFlag in @('manual_clock_changed','windows_touched','server','supervisor')){if($taskClock.$taskFlag -isnot [bool] -or $taskClock.$taskFlag -cne $false){throw 'Clock control scope'}}
foreach($taskSide in @('before','after')){$taskProbe=Get-Content -Raw (Join-Path $PSScriptRoot "clock-probe-$taskSide.log")|ConvertFrom-Json;if($taskProbe.samples -ne 3000 -or @($taskProbe.backwards).Count -ne 0 -or $taskProbe.manual_clock_change -isnot [bool] -or $taskProbe.manual_clock_change -cne $false){throw 'Clock probe refused'}}
if(-not (Get-Content -Raw (Join-Path $PSScriptRoot '194-trust-2.log')).Contains('test result: FAILED. 25 passed; 1 failed;')){throw 'First frozen refusal missing'}
foreach($taskMode in @('194','188')){foreach($taskRound in 1..2){$taskRaw=Get-Content -Raw (Join-Path $taskReceiptRoot "$taskMode-new-tests-$taskRound.log");foreach($taskSample in 0..7){if(-not $taskRaw.Contains("HC074-CONSUMING-MISMATCH-TRACE sample=$taskSample assertions_ok=true first=None")){throw 'Unchanged assertion diagnostic selection'}}}}
if(-not (Get-Content -Raw (Join-Path $PSScriptRoot 'missing-module-red.log')).Contains('diagnostic_held_role_provenance.rs')){throw 'Missing preimplementation refusal'}
# In-memory metadata controls never modify the sealed packet.
$taskNegative=0;$taskBase=ReadJson 'windows-receipt.json'
foreach($taskField in (@('source','mode','recipe','passed','new_tests','old_tests','docs','rounds','typed_consuming_join','kernel_role_bridge')+$taskFlags)){
    foreach($taskKind in @('change','missing')){
        $taskBad=$taskBase|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}
        elseif($taskField -ceq 'passed'){$taskBad.$taskField=$false}
        elseif($taskField -in @('new_tests','old_tests','docs','rounds')){$taskBad.$taskField=99}
        elseif($taskBad.$taskField -is [bool]){$taskBad.$taskField=$true}
        else{$taskBad.$taskField='wrong'}
        $taskRefused=$false;try{Receipt $taskBad 'windows'}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted metadata mutation: $taskField/$taskKind"};$taskNegative++
    }
}
$taskBase=ReadJson 'repository-fmt-empty-output.json'
foreach($taskField in @('native_exit','empty_output')){
    foreach($taskKind in @('change','missing')){
        $taskBad=$taskBase|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}elseif($taskField -ceq 'native_exit'){$taskBad.$taskField=1}else{$taskBad.$taskField=$false}
        $taskRefused=$false;try{FormatReceipt $taskBad}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted silent format mutation: $taskField/$taskKind"};$taskNegative++
    }
}
[ordered]@{verified=$true;files=$taskMembers.Count;receipt_metadata_mutations_refused=$taskNegative;inventory_sha256=(Get-FileHash $taskInventoryPath -Algorithm SHA256).Hash.ToLowerInvariant();typed_consuming_join=$true;kernel_role_bridge=$true;privileged_full_join=$false;late_original_join=$false;server=$false;product=$false}|ConvertTo-Json
