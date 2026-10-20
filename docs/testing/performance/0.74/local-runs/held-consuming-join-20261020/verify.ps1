param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRecipe=(Get-FileHash (Join-Path $PSScriptRoot 'checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
$taskModelRecipe=(Get-FileHash (Join-Path $PSScriptRoot 'compile-model-checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
$taskFlags=@('unrestricted_cpu_gate_passed','privileged_full_join','late_original_join','production_enrollment','production_role_receipt_issued','original_send','server','supervisor','product','full_joined_sigkill','clock_changed')
function ReadJson([string]$Name){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Name)|ConvertFrom-Json}
function FalseFlags($Value,[string[]]$Names){foreach($taskFlag in $Names){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $false){throw "Scope upgrade: $taskFlag"}}}
function Scope($Value,[string]$Mode){
    $taskExpected=if($Mode -in @('194','188')){'0'}else{''}
    if($Value.local_cpu_affinity -isnot [System.Array] -or ($Value.local_cpu_affinity -join ',') -cne $taskExpected){throw 'Affinity scope'}
    FalseFlags $Value $taskFlags
    foreach($taskFlag in @('typed_consuming_join','kernel_role_bridge')){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne ($Mode -in @('194','188'))){throw 'Consuming scope'}}
}
function Receipt($Value,[string]$Mode){
    if($Mode -notin @('windows','repository')){throw 'No accepted complete Linux execution cohort'}
    $taskOld=if($Mode -ceq 'repository'){18}else{28};$taskDocs=if($Mode -ceq 'repository'){0}else{4};$taskRounds=if($Mode -ceq 'repository'){1}else{2}
    if($Value.source -cne $Source -or $Value.mode -cne $Mode -or $Value.recipe -cne $taskRecipe -or $Value.passed -isnot [bool] -or $Value.passed -cne $true -or $Value.new_tests -ne 0 -or $Value.old_tests -ne $taskOld -or $Value.docs -ne $taskDocs -or $Value.rounds -ne $taskRounds){throw 'Receipt binding'}
    Scope $Value $Mode
}
function ModelReceipt($Value,[string]$Mode){
    if($Value.source -cne $Source -or $Value.mode -cne $Mode -or $Value.recipe -cne $taskModelRecipe -or $Value.models -ne 10 -or $Value.old_fixed_models -ne 5 -or $Value.rounds -ne 2 -or $Value.capability_docs -ne 11){throw 'Model receipt binding'}
    foreach($taskFlag in @('passed','model_only','strict_lint')){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $true){throw 'Model check incomplete'}}
    FalseFlags $Value @('real_join_gate_passed','unrestricted_cpu_gate_passed','server','supervisor','product')
}
function TestLog([string]$Name,[int]$Count){
    $taskRaw=Get-Content -Raw (Join-Path $PSScriptRoot $Name)
    if(-not $taskRaw.Contains("test result: ok. $Count passed; 0 failed; 0 ignored;") -or $taskRaw -match 'compiler unexpectedly panicked|internal compiler error|thread .rustc. .*panicked'){throw "Test selection or compiler panic: $Name"}
}
function FailedLog([string]$Name,[int]$Passed){
    if(-not (Get-Content -Raw (Join-Path $PSScriptRoot $Name)).Contains("test result: FAILED. $Passed passed; 1 failed;")){throw "Retained failed gate missing: $Name"}
}
function FormatReceipt($Value){
    if($Value.source -cne $Source -or $Value.mode -cne 'repository' -or $Value.check -cne 'fmt' -or ($Value.native_exit -isnot [long] -and $Value.native_exit -isnot [int]) -or $Value.native_exit -ne 0 -or $Value.empty_output -isnot [bool] -or $Value.empty_output -cne $true){throw 'Silent format receipt'}
}
$taskInventory=ReadJson 'inventory.json'
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
if(@(Get-ChildItem -LiteralPath $PSScriptRoot -Force -Recurse|Where-Object {$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Reparse member'}
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
if(($taskActual -join "\n") -cne ($taskExpected -join "\n")){throw 'Unlisted, missing or unsorted member'}
foreach($taskMode in @('windows','repository')){
    Receipt (ReadJson "affinity-smoke/$taskMode-receipt.json") $taskMode
    $taskIdentity=ReadJson "affinity-smoke/$taskMode-identity.json"
    if($taskIdentity.source -cne $Source -or $taskIdentity.mode -cne $taskMode -or $taskIdentity.recipe -cne $taskRecipe){throw 'Identity binding'}
    Scope $taskIdentity $taskMode
}
foreach($taskRound in 1..2){TestLog "affinity-smoke/windows-old-tests-$taskRound.log" 28}
TestLog 'affinity-smoke/windows-old-capability-docs.log' 4
TestLog 'affinity-smoke/repository-source-guards.log' 18
TestLog 'affinity-smoke/repository-old-bridge.log' 3
FormatReceipt (ReadJson 'affinity-smoke/repository-fmt-empty-output.json')
foreach($taskMode in @('194','188')){
    if(Test-Path (Join-Path $PSScriptRoot "affinity-smoke/$taskMode-receipt.json")){throw 'Do not promote the failed Linux cohort'}
    $taskIdentity=ReadJson "affinity-smoke/$taskMode-identity.json"
    if($taskIdentity.source -cne $Source -or $taskIdentity.mode -cne $taskMode -or $taskIdentity.recipe -cne $taskRecipe){throw 'Linux observation binding'}
    Scope $taskIdentity $taskMode
    foreach($taskSelection in @('old','new')){
        $taskImage=ReadJson "affinity-smoke/$taskMode-$taskSelection-image.json"
        if($taskImage.source -cne $Source -or $taskImage.mode -cne $taskMode -or $taskImage.recipe -cne $taskRecipe -or $taskImage.selection -cne $taskSelection -or $taskImage.sha256 -cnotmatch '^[0-9a-f]{64}$'){throw 'Observed image binding'}
    }
    foreach($taskRound in 1..2){TestLog "affinity-smoke/$taskMode-old-tests-$taskRound.log" 38}
    TestLog "affinity-smoke/$taskMode-new-tests-1.log" 17
    ModelReceipt (ReadJson "compile-model/$taskMode-receipt.json") $taskMode
    $taskModel=ReadJson "compile-model/$taskMode-image.json"
    if($taskModel.source -cne $Source -or $taskModel.mode -cne $taskMode -or $taskModel.recipe -cne $taskModelRecipe -or $taskModel.image_sha256 -cnotmatch '^[0-9a-f]{64}$' -or $taskModel.model_only -isnot [bool] -or $taskModel.model_only -cne $true){throw 'Model image binding'}
    foreach($taskRound in 1..2){TestLog "compile-model/$taskMode-models-$taskRound.log" 10;TestLog "compile-model/$taskMode-fixed-$taskRound.log" 5}
    TestLog "compile-model/$taskMode-held-docs.log" 6;TestLog "compile-model/$taskMode-trust-docs.log" 5
}
FailedLog '194-trust-2.log' 25
FailedLog 'confirmed/188-new-tests-1.log' 16
FailedLog 'stable-clock/194-new-tests-1.log' 16
FailedLog 'held-session/194-new-tests-1.log' 16
FailedLog 'affinity-smoke/194-new-tests-2.log' 16
FailedLog 'affinity-smoke/188-trust-1.log' 25
foreach($taskPath in @('confirmed/188-new-tests-1.log','stable-clock/194-new-tests-1.log','held-session/194-new-tests-1.log')){
    if(-not (Get-Content -Raw (Join-Path $PSScriptRoot $taskPath)).Contains('site: "clock-order"')){throw 'Recorded clock-order refusal missing'}
}
$taskClock=ReadJson 'clock-control-receipt.json'
if($taskClock.source -cne 'eeca0d5e6faa1cbdbf9c3bb92f5a036db4eca2ba' -or $taskClock.complete -isnot [bool] -or $taskClock.complete -cne $false -or $taskClock.local_timesyncd_before -cne 'active' -or $taskClock.local_timesyncd_during -cne 'inactive' -or $taskClock.local_timesyncd_after -cne 'active' -or $taskClock.service -cne 'systemd-timesyncd.service' -or $taskClock.distribution -cne 'Ubuntu'){throw 'Retained clock control/restoration'}
FalseFlags $taskClock @('manual_clock_changed','windows_touched','server','supervisor')
$taskSession=ReadJson 'held-clock-session-receipt.json'
if($taskSession.source -cne '542face4dccfab11fff073a7b1f896dbebcb254e' -or $taskSession.complete -isnot [bool] -or $taskSession.complete -cne $false -or $taskSession.observer_exit -ne 0 -or $taskSession.test_phase_backwards -ne 0){throw 'Do not promote the coarse observer'}
FalseFlags $taskSession @('service_changed','manual_clock_changed','windows_clock_changed','server','supervisor')
$taskSummary=ReadJson 'summary.json'
if($taskSummary.source -cne $Source -or $taskSummary.recipe -cne $taskRecipe -or $taskSummary.model_recipe -cne $taskModelRecipe -or $taskSummary.old_sources_reconstructed -ne 443 -or $taskSummary.named_runtime_additions -ne 3 -or $taskSummary.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Summary binding'}
if($taskSummary.evidence_only -isnot [bool] -or $taskSummary.evidence_only -cne $true){throw 'Evidence-only status'}
FalseFlags $taskSummary @('local_join_gate_passed','unrestricted_cpu_gate_passed','privileged_full_join','full_321_rerun','first_frozen_cohort_accepted','first_refusal_attributed','clock_cause_established','server','supervisor','product','full_joined_sigkill')
# Metadata controls validate evidence integrity, NOT a successful execution gate.
$taskNegative=0
function Mutations($Base,[string[]]$Fields,[scriptblock]$Validate){
    foreach($taskField in $Fields){foreach($taskKind in @('change','missing')){
        $taskBad=$Base|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}
        elseif($taskBad.$taskField -is [bool]){$taskBad.$taskField=-not $taskBad.$taskField}
        elseif($taskBad.$taskField -is [long] -or $taskBad.$taskField -is [int]){$taskBad.$taskField=99}
        else{$taskBad.$taskField='wrong'}
        $taskRefused=$false;try{& $Validate $taskBad}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted metadata mutation: $taskField/$taskKind"}
        $script:taskNegative++
    }}
}
Mutations (ReadJson 'affinity-smoke/windows-receipt.json') (@('source','mode','recipe','passed','new_tests','old_tests','docs','rounds','typed_consuming_join','kernel_role_bridge','local_cpu_affinity')+$taskFlags) {param($v) Receipt $v 'windows'}
Mutations (ReadJson 'affinity-smoke/repository-fmt-empty-output.json') @('native_exit','empty_output') {param($v) FormatReceipt $v}
Mutations (ReadJson 'compile-model/194-receipt.json') @('source','mode','recipe','passed','models','rounds','capability_docs','model_only','real_join_gate_passed','unrestricted_cpu_gate_passed','strict_lint','server') {param($v) ModelReceipt $v '194'}
[ordered]@{evidence_verified=$true;files=$taskMembers.Count;metadata_mutations_refused=$taskNegative;inventory_sha256=(Get-FileHash $taskInventoryPath -Algorithm SHA256).Hash.ToLowerInvariant();local_join_gate_passed=$false;unrestricted_cpu_gate_passed=$false;privileged_full_join=$false;clock_cause_established=$false;server=$false;product=$false}|ConvertTo-Json
