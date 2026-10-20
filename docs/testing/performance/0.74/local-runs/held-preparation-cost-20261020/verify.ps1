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
if(@(Get-ChildItem $PSScriptRoot -Force -Recurse|Where-Object{$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Reparse member'}
if($UnsealedReview){if(Test-Path "$PSScriptRoot/inventory.json"){throw 'Cannot bypass an existing seal'};$taskCount=@(Get-ChildItem $PSScriptRoot -Recurse -File).Count}else{$taskCount=Inventory $PSScriptRoot}
foreach($taskPrior in @(@('held-consuming-live-20261020','46956c62bddd8def65e8132d905d6235d41f51572f9571573ecf92bbe5aca710'),@('held-consuming-privileged-server-20261020','4d592e07dcc6a8dc461827801b724bea1ca803d22d8a61f50915988e28fec079'))){
    $taskDirectory=(Resolve-Path "$PSScriptRoot/../$($taskPrior[0])").Path
    if((Hash "$taskDirectory/inventory.json") -cne $taskPrior[1]){throw 'Preceding seal changed'}
    $null=Inventory $taskDirectory
}
$taskExpected=(PreparationFacts)|ConvertTo-Json -Depth 20|ConvertFrom-Json
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
    $taskBaseline=PacketJson "baseline-$taskLane-image.json"
    if($taskBaseline.sha256 -cne (Get-Content -Raw "$PSScriptRoot/../held-consuming-live-20261020/traced-$taskLane-image.json"|ConvertFrom-Json).sha256 -or $taskBaseline.source -cne '1eac3bb37814d5aaf44f84370606218719c01c2f' -or (PacketJson "baseline-$taskLane-exit.json").exit -ne 101){throw 'Retained-image baseline changed'}
    foreach($taskCohort in 'instrumented','candidate','reviewed/candidate'){
        foreach($taskSelection in 'build','observer','bridge','trust','kernel','lint'){
            if((PacketJson "$taskCohort-$taskLane-$taskSelection-exit.json").exit -ne 0){throw 'Required compiler check refused'}
        }
    }
    $taskImage=PacketJson "reviewed/candidate-$taskLane-image.json"
    if($taskImage.source -cne $taskExpected.reviewed_run_source -or $taskImage.sha256 -cne (PacketJson "candidate-$taskLane-image.json").sha256 -or $taskImage.recipe_sha256 -cne (Hash "$PSScriptRoot/reviewed/candidate-stage.ps1")){throw 'Reviewed image source, bytes or recipe differ'}
}
if((Hash "$PSScriptRoot/candidate-stage.ps1") -cne (Hash "$PSScriptRoot/reviewed/candidate-stage.ps1")){throw 'Reviewed recipe silently changed'}
$taskReady=PacketJson 'profiles-ready.json'
if($taskReady.format_exit -ne 0 -or $taskReady.lint_exit -ne 0 -or $taskReady.all_own_build_and_test_jobs_finished -cne $true){throw 'Profiling started without checks'}
$taskBundleBefore=PacketJson 'bundle-before.json';$taskBundleAfter=PacketJson 'bundle-after.json'
if(@($taskBundleBefore.files).Count -ne 9 -or ($taskBundleBefore.files|ConvertTo-Json -Depth 5 -Compress) -cne ($taskBundleAfter.files|ConvertTo-Json -Depth 5 -Compress)){throw 'Material workload input changed'}
foreach($taskEntry in $taskBundleBefore.files){
    if(-not (Get-Content -Raw "$PSScriptRoot/../held-consuming-privileged-server-20261020/inputs.sha256").Contains("$($taskEntry.sha256)  bundle/$($taskEntry.name)")){throw 'Wrong frozen material input'}
}
if((PacketJson 'source-contracts-exit.json').exit -ne 101 -or -not (Get-Content -Raw "$PSScriptRoot/source-contracts.log").Contains('fatal: not a git repository:')){throw 'Retain Windows Git pointer refusal in Linux'}
if((PacketJson 'source-contracts-git-context-exit.json').exit -ne 101 -or -not (Get-Content -Raw "$PSScriptRoot/source-contracts-git-context.log").Contains('left: 462') -or -not (Get-Content -Raw "$PSScriptRoot/source-contracts-git-context.log").Contains('right: 450')){throw 'Retain initial audit count error'}
if((PacketJson 'source-contracts-reviewed-exit.json').exit -ne 0){throw 'Reviewed source guards refused'}
$taskReviewed=Get-Content -Raw "$PSScriptRoot/source-contracts-reviewed.log"
foreach($taskTests in 4,321){if(-not $taskReviewed.Contains("test result: ok. $taskTests passed; 0 failed; 0 ignored;")){throw 'Incomplete reviewed source/contract selection'}}
if(-not (Get-Content -Raw "$PSScriptRoot/source-contracts-git-context.log").Contains('test result: ok. 20 passed; 0 failed; 0 ignored;')){throw 'Old guards not checked'}
foreach($taskCheck in 'format','xtask-lint','doc-check','registry','contract-check'){
    if((PacketJson "$taskCheck-exit.json").exit -ne 0){throw 'Local repository check refused'}
}
$taskFinal=PacketJson 'final-checks.json'
if($taskFinal.doc_check_exit -ne 0 -or $taskFinal.contract_check_exit -ne 0 -or $taskFinal.qualification_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (PacketJson 'recipe-syntax.json').syntax_passed -cne $true){throw 'Final document, contract or recipe check missing'}
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
if((Hash "$taskRepo/docs/testing/performance/0.74/qualification-manifest.toml") -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Qualification changed'}
Push-Location $taskRepo
try{
    & git diff --exit-code $taskExpected.runtime_candidate_source -- tools/long-run-supervisor-074
    if($LASTEXITCODE -ne 0){throw 'Unreviewed runtime/test source change'}
}finally{Pop-Location}
[ordered]@{evidence_verified=(-not $UnsealedReview);inventory_checked=(-not $UnsealedReview);files=$taskCount;metadata_mutations_refused=$taskNegative;allocation_candidate_accepted=$taskExpected.allocation_candidate_accepted;full_local_matrix_passed=$taskExpected.full_local_matrix_passed;native_execution_gate_passed=$false;server=$false;product=$false}|ConvertTo-Json
