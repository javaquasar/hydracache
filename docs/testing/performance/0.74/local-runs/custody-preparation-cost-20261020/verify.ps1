param([switch]$Review)
$ErrorActionPreference='Stop'
. "$PSScriptRoot/facts.ps1"
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function ReadJson([string]$Name){Get-Content -Raw "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Json($Value){ConvertTo-Json -Depth 30 -Compress -InputObject $Value}
function Summary($Value,$Expected){Require ((Json $Value) -ceq (Json $Expected)) 'Summary differs from raw facts'}
function Inventory([string]$Directory,[string]$Expected){
    $Directory=(Resolve-Path -LiteralPath $Directory).Path
    if($Expected){Require ((Hash "$Directory/inventory.json") -ceq $Expected) 'Prior inventory changed'}
    Require (@(Get-ChildItem $Directory -Recurse -Force|Where-Object{($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0}).Count -eq 0) 'No reparse members'
    $taskInventory=Get-Content -Raw "$Directory/inventory.json"|ConvertFrom-Json
    Require ($taskInventory.schema -ceq 'sha256-local-file-inventory-v1') 'Inventory schema'
    $taskActual=[string[]]@(Get-ChildItem $Directory -File -Recurse -Force|Where-Object{$_.FullName -cne "$Directory\inventory.json"}|ForEach-Object{$_.FullName.Substring($Directory.Length+1).Replace('\','/')})
    [Array]::Sort($taskActual,[StringComparer]::Ordinal)
    $taskPaths=[string[]]@($taskInventory.files.path)
    Require ((Json $taskActual) -ceq (Json $taskPaths) -and @($taskPaths|Select-Object -Unique).Count -eq $taskPaths.Count) 'Missing extra duplicate or unsorted member'
    foreach($taskEntry in $taskInventory.files){
        Require ($taskEntry.path -cmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -and $taskEntry.path -cnotmatch '(^|/)\.\.(/|$)' -and $taskEntry.path -cne 'inventory.json') 'Unsafe member'
        $taskPath=Join-Path $Directory $taskEntry.path
        Require ((Get-Item -LiteralPath $taskPath).Length -eq $taskEntry.bytes -and (Hash $taskPath) -ceq $taskEntry.sha256) "Changed member: $($taskEntry.path)"
    }
    $taskPaths.Count
}
Push-Location $taskRepo
try {
    $taskInputs=ReadJson 'inputs-after.json';$taskBefore=ReadJson 'inputs-before.json'
    Require ((Json $taskBefore.bundle) -ceq (Json $taskInputs.bundle) -and $taskInputs.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Frozen inputs changed'
    $taskAnchors=@(@('held-native-recheck-20261020','2a7be06fe6f93e3e8761b29afebaf89ac6266d7c405b457ef8ea26d60259bd63'),@('held-composition-recheck-20261020','0bfa7cef4d3ebd69c9e751fe6e4304a8bade3133122dcc48249c9d1396bfe1c0'),@('pid-path-cost-20261020','982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05'))
    Require ((Json $taskInputs.prior_inventories) -ceq (Json $taskAnchors) -and (Json $taskBefore.prior_inventories) -ceq (Json $taskAnchors)) 'Prior anchors changed'
    foreach($taskAnchor in $taskAnchors){[void](Inventory (Join-Path $PSScriptRoot "../$($taskAnchor[0])") $taskAnchor[1])}
    $taskSource=ReadJson 'review.json'
    & git diff --quiet $taskSource.review HEAD -- tools/long-run-supervisor-074 crates/xtask
    Require ($LASTEXITCODE -eq 0) 'Reviewed source drift'
    & git diff --quiet $taskSource.runtime HEAD -- tools/long-run-supervisor-074
    Require ($LASTEXITCODE -eq 0) 'Runtime changed after measurements'
    & git diff --quiet 72efce2401e753b857f7fc03f3300b6aae19bd5d HEAD -- crates/hydracache Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
    Require ($LASTEXITCODE -eq 0 -and -not (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)) 'Frozen product/input or dirty source'
    foreach($taskCohort in 'Baseline','Candidate'){
        foreach($taskLane in '194','188'){
            $taskImage=ReadJson "$taskCohort-$taskLane-image.json"
            $taskExpected=if($taskCohort -ceq 'Baseline'){'b28a29ebe465f27762828689ce759a6716fda5b4'}else{$taskSource.runtime}
            Require ($taskImage.source -ceq $taskExpected -and $taskImage.recipe_sha256 -ceq (Hash "$PSScriptRoot/profile.ps1")) 'Profile image binding'
            foreach($taskSample in 0..2){
                $taskReceipt=ReadJson "$taskCohort-$taskLane-profile-$taskSample-exit.json"
                Require ($taskReceipt.source -ceq $taskExpected -and $taskReceipt.exit -eq 0 -and $taskReceipt.recipe_sha256 -ceq $taskImage.recipe_sha256 -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Profile cohort refused'
            }
        }
    }
    foreach($taskCohort in 'Prototype','Final'){
        foreach($taskLane in '194','188'){
            $taskImage=ReadJson "$taskCohort-$taskLane-alloc-image.json"
            $taskRecipe=if($taskCohort -ceq 'Prototype'){'scratch.ps1'}else{'checked-layout.ps1'}
            $taskExpected=if($taskCohort -ceq 'Prototype'){'4952b0f28fd752d9e479ed5f5ed34e6184bbb836'}else{$taskSource.runtime}
            Require ($taskImage.source -ceq $taskExpected -and $taskImage.recipe_sha256 -ceq (Hash "$PSScriptRoot/$taskRecipe")) 'Image/recipe binding'
            foreach($taskName in 'allocation','alloc-build','alloc-differential','runtime-binding'){
                $taskReceipt=ReadJson "$taskCohort-$taskLane-$taskName-exit.json"
                $taskExit=if($taskCohort -ceq 'Prototype' -and $taskName -ceq 'runtime-binding'){101}else{0}
                Require ($taskReceipt.source -ceq $taskExpected -and $taskReceipt.exit -eq $taskExit -and -not $taskReceipt.server -and -not $taskReceipt.product -and $taskReceipt.recipe_sha256 -ceq $taskImage.recipe_sha256) 'Allocation/test-first cohort changed'
            }
        }
    }
    foreach($taskLane in '194','188'){
        foreach($taskSelection in @(@('kernel',58),@('bridge',20),@('trust',26),@('observer',3),@('boundaries',1),@('plain-differential',3))){
            $taskReceipt=ReadJson "Final-$taskLane-$($taskSelection[0])-exit.json"
            Require ($taskReceipt.exit -eq 0 -and $taskReceipt.source -ceq $taskSource.runtime) 'Targeted regression refused'
            Require ((Get-Content -Raw "$PSScriptRoot/Final-$taskLane-$($taskSelection[0]).log").Contains("test result: ok. $($taskSelection[1]) passed; 0 failed;")) 'Targeted selection count'
        }
        Require ((ReadJson "Final-$taskLane-lint-exit.json").exit -eq 0) 'Strict tool lint refused'
        $taskFresh=Get-Content -Raw "$PSScriptRoot/fresh-$taskLane.log"
        Require ((ReadJson "fresh-$taskLane-exit.json").exit -eq 0 -and ([regex]::Matches($taskFresh,'HC074-FRESH-KERNEL ')).Count -eq 18) 'Fresh observation contract missing'
    }
    $taskContracts=ReadJson 'windows-final-source-contracts-exit.json'
    Require ($taskContracts.exit -eq 0 -and $taskContracts.source -ceq $taskSource.review) 'Reviewed source contracts refused'
    $taskCounts=@([regex]::Matches((Get-Content -Raw "$PSScriptRoot/windows-final-source-contracts.log"),'test result: ok\. ([0-9]+) passed; 0 failed; 0 ignored;')|ForEach-Object{[int]$_.Groups[1].Value}|Sort-Object)
    Require ((Json $taskCounts) -ceq (Json @(2,2,3,3,4,20,321))) 'All 355 source/contract assertions required'
    foreach($taskName in 'format','xtask-lint','doc-check','registry','contract-check'){
        $taskReceipt=ReadJson "$taskName-exit.json"
        Require ($taskReceipt.exit -eq 0 -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Local repository gate refused'
    }
    # The first observer wrapper recorded zero despite the failed cargo result.
    # Preserve that receipt and require the actual failed selection, never a pass.
    $taskObserverRed=Get-Content -Raw "$PSScriptRoot/observer-guard-red.log"
    Require ((ReadJson 'observer-guard-red-exit.json').exit -eq 0 -and $taskObserverRed.Contains('test result: FAILED. 0 passed; 2 failed;') -and $taskObserverRed.Contains('error: test failed, to rerun pass')) 'Retain observer refusal and recorder discrepancy'
    Require ((ReadJson 'adoption-source-audit-exit.json').exit -eq 101 -and (ReadJson 'source-contracts-exit.json').exit -eq 101 -and (ReadJson 'final-source-contracts-exit.json').exit -eq 101 -and (ReadJson 'Adopted-194-bridge-exit.json').exit -eq 101) 'Retain preceding refusals'
    Require ((ReadJson 'shared-view-lint-exit.json').exit -eq 0 -and (ReadJson 'shared-view-audit-exit.json').exit -eq 0) 'Shared implementation review refused'
    Require ((ReadJson 'completed-xtask-lint-exit.json').exit -eq 101 -and (Get-Content -Raw "$PSScriptRoot/completed-xtask-lint.log").Contains('duplicate_mod')) 'Retain strict duplicate-module refusal'
    $taskLinkRed=Get-Content -Raw "$PSScriptRoot/sealed-source-contracts.log"
    Require ((ReadJson 'sealed-source-contracts-exit.json').exit -eq 0 -and $taskLinkRed.Contains('LNK1104') -and $taskLinkRed.Contains('error: could not compile') -and -not $taskLinkRed.Contains('test result: ok.')) 'Retain pre-test linker refusal and recorder discrepancy'
    $taskStop=ReadJson 'source-check-stop.json'
    Require ((ReadJson 'post-lock-source-contracts-exit.json').exit -ne 0 -and $taskStop.process_group -eq 287 -and $taskStop.action -ceq 'SIGTERM' -and -not $taskStop.runtime_or_fault_test_stopped -and -not $taskStop.server -and -not $taskStop.product) 'Retain explicitly interrupted duplicate source check'
    $taskPost=@(Get-Content "$PSScriptRoot/local-postflight.log"|Where-Object{$_})
    Require ($taskPost.Count -eq 2 -and $taskPost[0].StartsWith('uid=0(root)') -and $taskPost[1] -cmatch '^[0-9a-f-]{36}$') 'Local residue/postflight check incomplete'
    $taskFacts=CostFacts
    if($Review){Require (-not (Test-Path "$PSScriptRoot/inventory.json")) 'Cannot bypass seal';$taskFacts|ConvertTo-Json -Depth 30;exit}
    $taskCount=Inventory $PSScriptRoot ''
    Summary (ReadJson 'summary.json') $taskFacts
    $taskMutations=0
    foreach($taskKey in $taskFacts.Keys){
        $taskBad=Json $taskFacts|ConvertFrom-Json
        if($taskBad.$taskKey -is [bool]){$taskBad.$taskKey=-not $taskBad.$taskKey}else{$taskBad.$taskKey='changed'}
        $taskRefused=$false
        try{Summary $taskBad $taskFacts}catch{if($_.Exception.Message -cne 'Summary differs from raw facts'){throw};$taskRefused=$true}
        Require $taskRefused 'Summary mutation accepted';$taskMutations++
    }
    [ordered]@{evidence_verified=$true;inventory_checked=$true;files=$taskCount;summary_mutations_refused=$taskMutations;allocation_effect_confirmed=$true;local_matrix_passed=$taskFacts.local_matrix_passed;native_execution_gate_passed=$false;server=$false;product=$false}|ConvertTo-Json
} finally {Pop-Location}
