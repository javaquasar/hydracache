param([switch]$UnsealedReview)
$ErrorActionPreference='Stop'
. "$PSScriptRoot/facts.ps1"
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Inventory([string]$Directory){
    if(@(Get-ChildItem $Directory -Force -Recurse|Where-Object{$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Reparse member'}
    $taskValue=Get-Content -Raw "$Directory/inventory.json"|ConvertFrom-Json
    if($taskValue.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
    $taskSeen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach($taskEntry in $taskValue.files){
        if(-not $taskEntry.path -or $taskEntry.path -match '[:\\\x00-\x1f]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskSeen.Add($taskEntry.path)){throw 'Invalid member'}
        $taskFile=Get-Item -LiteralPath "$Directory/$($taskEntry.path)"
        if($taskFile.PSIsContainer -or $taskFile.Length -ne $taskEntry.bytes -or (Hash $taskFile.FullName) -cne $taskEntry.sha256){throw "Changed member $($taskEntry.path)"}
    }
    [string[]]$taskActual=@(Get-ChildItem $Directory -Force -Recurse -File|Where-Object{$_.FullName -cne (Join-Path $Directory 'inventory.json')}|ForEach-Object{$_.FullName.Substring($Directory.Length+1).Replace('\','/')})
    [Array]::Sort($taskActual,[StringComparer]::Ordinal)
    if(($taskActual -join "`n") -cne ($taskValue.files.path -join "`n")){throw 'Unlisted, missing or unsorted member'}
    return $taskActual.Count
}
if($UnsealedReview){if(Test-Path "$PSScriptRoot/inventory.json"){throw 'Cannot bypass an existing seal'};$taskCount=@(Get-ChildItem $PSScriptRoot -Recurse -File).Count}else{$taskCount=Inventory $PSScriptRoot}
foreach($taskPrior in @(@('fresh-kernel-cost-20261020','cdd73b900d1697d64beef507f23e8b2bfbad20598dc6e984f53e1a9b4ed075cc'),@('held-preparation-cost-20261020','4800b14816d9ca01a0ad04aa4082c37fc576f0957efe46644c85c01164b2b65f'),@('held-consuming-live-20261020','46956c62bddd8def65e8132d905d6235d41f51572f9571573ecf92bbe5aca710'),@('held-consuming-privileged-server-20261020','4d592e07dcc6a8dc461827801b724bea1ca803d22d8a61f50915988e28fec079'))){
    $taskDirectory=(Resolve-Path "$PSScriptRoot/../$($taskPrior[0])").Path
    if((Hash "$taskDirectory/inventory.json") -cne $taskPrior[1]){throw 'Preceding seal changed'}
    $null=Inventory $taskDirectory
}
$taskExpected=KernelFacts|ConvertTo-Json -Depth 20|ConvertFrom-Json
if($taskExpected.baseline_source -cne '0d7717686a06cec7178a1965e02363fc43a63da8' -or $taskExpected.candidate_source -cne 'b5f1b8fce6b3046239087873cb85ff1bfc557f0f'){throw 'Frozen source lineage changed'}
if((PacketJson 'recipe-syntax.json').syntax_passed -cne $true){throw 'Recipe parse refused'}
$taskPreflight=PacketJson 'probe-preflight.json'
if($taskPreflight.source -cne $taskExpected.candidate_source -or $taskPreflight.own_builds_and_checks_finished -cne $true -or @($taskPreflight.images).Count -ne 4){throw 'Probe isolation/image preflight missing'}
if(($taskPreflight.images.image -join ',') -cne 'Candidate-188-alloc-image.json,Candidate-188-plain-image.json,Candidate-194-alloc-image.json,Candidate-194-plain-image.json'){throw 'Preflight image set changed'}
foreach($taskImage in $taskPreflight.images){
    if((PacketJson $taskImage.image).sha256 -cne $taskImage.sha256){throw 'Preflight image mismatch'}
}
function Summary($Value){
    [string[]]$taskKeys=@($taskExpected.PSObject.Properties.Name);[string[]]$taskGiven=@($Value.PSObject.Properties.Name)
    [Array]::Sort($taskKeys,[StringComparer]::Ordinal);[Array]::Sort($taskGiven,[StringComparer]::Ordinal)
    if(($taskKeys -join ',') -cne ($taskGiven -join ',')){throw 'Summary keys'}
    foreach($taskKey in $taskKeys){if(($Value.$taskKey|ConvertTo-Json -Depth 20 -Compress) -cne ($taskExpected.$taskKey|ConvertTo-Json -Depth 20 -Compress)){throw "Summary binding $taskKey"}}
}
Summary (PacketJson 'summary.json')
$taskNegative=0
foreach($taskKey in $taskExpected.PSObject.Properties.Name){
    foreach($taskMode in 'missing','wrong'){
        $taskMutant=$taskExpected|ConvertTo-Json -Depth 20|ConvertFrom-Json
        if($taskMode -ceq 'missing'){$taskMutant.PSObject.Properties.Remove($taskKey)}else{$taskMutant.$taskKey=if($taskMutant.$taskKey -is [bool]){-not $taskMutant.$taskKey}else{'changed'}}
        $taskRefused=$false;try{Summary $taskMutant}catch{$taskRefused=$true}
        if(-not $taskRefused){throw 'Metadata mutation accepted'};$taskNegative++
    }
}
$taskMutant=$taskExpected|ConvertTo-Json -Depth 20|ConvertFrom-Json
$taskMutant|Add-Member unexpected $true
$taskRefused=$false;try{Summary $taskMutant}catch{$taskRefused=$true}
if(-not $taskRefused){throw 'Unknown summary field accepted'};$taskNegative++
foreach($taskLane in '194','188'){
    foreach($taskSelection in 'fresh-tests','pid-tests','kernel','bridge','trust','observer','lint'){
        $taskReceipt=PacketJson "Candidate-$taskLane-$taskSelection-exit.json"
        if($taskReceipt.exit -ne 0 -or $taskReceipt.source -cne $taskExpected.candidate_source){throw 'Candidate regression failed'}
    }
}
foreach($taskCheck in 'source-contracts','format','xtask-lint','doc-check','registry','contract-check'){
    if((PacketJson "$taskCheck-exit.json").exit -ne 0){throw 'Local check refused'}
}
$taskFinal=PacketJson 'final-checks.json'
if($taskFinal.doc_check_exit -ne 0 -or $taskFinal.contract_check_exit -ne 0){throw 'Final documentation/contract check refused'}
$taskContracts=Get-Content -Raw "$PSScriptRoot/source-contracts.log"
foreach($taskCountExpected in 20,4,2,3,321){
    if(-not $taskContracts.Contains("test result: ok. $taskCountExpected passed; 0 failed; 0 ignored;")){throw 'Incomplete source/contract selection'}
}
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
if((Hash "$taskRepo/docs/testing/performance/0.74/qualification-manifest.toml") -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
$taskFrozen=PacketJson 'frozen-inputs.json'
if($taskFrozen.diff_exit -ne 0 -or $taskFrozen.reference_source -cne 'b63a0e81ad808d6f65e8c8d818ee4c090c3718ff' -or $taskFrozen.qualification_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Frozen contract evidence changed'}
Push-Location $taskRepo
try{
    & git diff --exit-code $taskExpected.candidate_source -- tools/long-run-supervisor-074 crates/xtask
    if($LASTEXITCODE -ne 0){throw 'Unreviewed runtime/test source change'}
    if(($taskFrozen.members.path -join ',') -cne 'Cargo.lock,docs/COMPAT.md,docs/testing/gated-test-registry.toml,docs/testing/performance/0.74/qualification-manifest.toml,crates/hydracache'){throw 'Frozen object set changed'}
    foreach($taskMember in $taskFrozen.members){
        if((& git rev-parse "$($taskFrozen.reference_source):$($taskMember.path)").Trim() -cne $taskMember.git_object -or (& git rev-parse "HEAD:$($taskMember.path)").Trim() -cne $taskMember.git_object){throw 'Frozen object changed'}
    }
    & git diff --exit-code $taskFrozen.reference_source -- @($taskFrozen.members.path)
    if($LASTEXITCODE -ne 0){throw 'Frozen product/contracts changed'}
}finally{Pop-Location}
[ordered]@{evidence_verified=(-not $UnsealedReview);inventory_checked=(-not $UnsealedReview);files=$taskCount;metadata_mutations_refused=$taskNegative;allocation_candidate_accepted=$true;full_functional_matrix_retested=$false;native_execution_gate_passed=$false;server=$false;product=$false}|ConvertTo-Json
