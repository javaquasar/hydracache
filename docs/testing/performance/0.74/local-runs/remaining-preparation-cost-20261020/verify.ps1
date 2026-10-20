param([switch]$Review)
$ErrorActionPreference='Stop'
. "$PSScriptRoot/facts.ps1"
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
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

$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
Push-Location $taskRepo
try {
    $taskSources=ReadJson 'sources.json'
    Require ($taskSources.baseline -ceq 'd62c8e7ae8ee551fde04081f999352c4cb551d2d') 'Frozen reference source changed'
    & git diff --quiet $taskSources.review HEAD -- tools/long-run-supervisor-074 crates/xtask
    Require ($LASTEXITCODE -eq 0 -and -not (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)) 'Measured source drift'
    & git diff --quiet $taskSources.final HEAD -- tools/long-run-supervisor-074
    Require ($LASTEXITCODE -eq 0) 'Runtime changed after measurement'
    & git diff --quiet dea214a80aa59d3af7b8456a43c0763573a04546 HEAD -- Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
    Require ($LASTEXITCODE -eq 0) 'Frozen inputs drift'
    Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Current qualification drift'
    $taskProduct=@(& git diff --name-only dea214a80aa59d3af7b8456a43c0763573a04546 HEAD -- crates|Where-Object{-not $_.StartsWith('crates/xtask/')})
    Require ($taskProduct.Count -eq 0) 'Product code changed'
    $taskBefore=ReadJson 'inputs-before.json';$taskAfter=ReadJson 'inputs-after.json'
    Require ((Json $taskBefore.bundle) -ceq (Json $taskAfter.bundle) -and $taskAfter.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Original bundle/qualification drift'
    $taskAnchors=@(@('custody-preparation-cost-20261020','3194244cb4e45c6f4f611b946afaa6a6a8b3aaf83179457c260d0a9e25f93864'),@('held-native-recheck-20261020','2a7be06fe6f93e3e8761b29afebaf89ac6266d7c405b457ef8ea26d60259bd63'),@('held-composition-recheck-20261020','0bfa7cef4d3ebd69c9e751fe6e4304a8bade3133122dcc48249c9d1396bfe1c0'),@('pid-path-cost-20261020','982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05'))
    Require ((Json $taskBefore.prior_inventories) -ceq (Json $taskAnchors) -and (Json $taskAfter.prior_inventories) -ceq (Json $taskAnchors)) 'Prior anchor changed'
    foreach($taskAnchor in $taskAnchors){[void](Inventory (Join-Path $PSScriptRoot "../$($taskAnchor[0])") $taskAnchor[1])}
    foreach($taskCohort in 'Baseline','Final'){
        $taskExpected=if($taskCohort -ceq 'Baseline'){$taskSources.baseline}else{$taskSources.final}
        foreach($taskLane in '194','188'){
            foreach($taskFlavor in 'plain','alloc'){
                $taskImage=ReadJson "$taskCohort-$taskLane-$taskFlavor-image.json"
                Require ($taskImage.source -ceq $taskExpected -and $taskImage.recipe_sha256 -ceq (Hash "$PSScriptRoot/run.ps1")) 'Image/source/recipe binding'
                foreach($taskName in "$taskFlavor-build","$taskFlavor-differential"){
                    $taskReceipt=ReadJson "$taskCohort-$taskLane-$taskName-exit.json"
                    Require ($taskReceipt.source -ceq $taskExpected -and $taskReceipt.exit -eq 0 -and $taskReceipt.recipe_sha256 -ceq $taskImage.recipe_sha256 -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Build/differential selection refused'
                }
                $taskRaw=Get-Content -Raw "$PSScriptRoot/$taskCohort-$taskLane-$taskFlavor-differential.log"
                Require ($taskRaw.Contains('cases=804 seed=0x074f1e1d') -and $taskRaw.Contains('test result: ok. 2 passed; 0 failed;')) 'Projection differential coverage missing'
            }
            foreach($taskName in @('allocation','runtime-binding','Profile-0','Profile-1','Profile-2')+$(if($taskCohort -ceq 'Final'){@('kernel','bridge','trust','observer','boundaries','lint','Matrix-full')}else{@()})){
                $taskReceipt=ReadJson "$taskCohort-$taskLane-$taskName-exit.json"
                $taskExit=if($taskCohort -ceq 'Baseline' -and $taskName -ceq 'runtime-binding'){101}else{0}
                Require ($taskReceipt.source -ceq $taskExpected -and $taskReceipt.exit -eq $taskExit -and $taskReceipt.recipe_sha256 -ceq (Hash "$PSScriptRoot/run.ps1") -and -not $taskReceipt.server -and -not $taskReceipt.product) "Refused selection: $taskCohort/$taskLane/$taskName"
            }
            $taskBinding=Get-Content -Raw "$PSScriptRoot/$taskCohort-$taskLane-runtime-binding.log"
            Require ($taskBinding.Contains($(if($taskCohort -ceq 'Baseline'){'test result: FAILED. 0 passed; 1 failed;'}else{'test result: ok. 1 passed; 0 failed;'}))) 'Wrong test-first binding'
            $taskTimes=@(Rows "$taskCohort-$taskLane-allocation" 'HC074-PROJECTION-TIME ')
            Require ($taskTimes.Count -eq 3 -and @($taskTimes|Where-Object{$_.iterations -ne 4096 -or $_.allocation_counter_active -or $_.server -or $_.product}).Count -eq 0) 'Timing workload changed'
        }
    }
    foreach($taskLane in '194','188'){
        foreach($taskSelection in @(@('kernel',61),@('bridge',20),@('trust',26),@('observer',3),@('boundaries',1))){
            Require ((Get-Content -Raw "$PSScriptRoot/Final-$taskLane-$($taskSelection[0]).log").Contains("test result: ok. $($taskSelection[1]) passed; 0 failed;")) 'Final regression selection'
        }
        $taskFresh=ReadJson "fresh-$taskLane-exit.json"
        Require ($taskFresh.exit -eq 0 -and $taskFresh.source -ceq $taskSources.final -and $taskFresh.recipe_sha256 -ceq (Hash "$PSScriptRoot/fresh.ps1") -and @(Rows "fresh-$taskLane" 'HC074-FRESH-KERNEL ').Count -eq 18) 'Fresh kernel control'
    }
    foreach($taskName in 'format','xtask-lint','source-contracts','doc-check','registry','contract-check'){
        $taskReceipt=ReadJson "review-$taskName-exit.json"
        Require ($taskReceipt.source -ceq $taskSources.review -and $taskReceipt.exit -eq 0 -and $taskReceipt.recipe_sha256 -ceq (Hash "$PSScriptRoot/review-checks.ps1") -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Local repository gate'
    }
    $taskCounts=@([regex]::Matches((Get-Content -Raw "$PSScriptRoot/review-source-contracts.log"),'test result: ok\. ([0-9]+) passed; 0 failed; 0 ignored;')|ForEach-Object{[int]$_.Groups[1].Value}|Sort-Object)
    Require ((Json $taskCounts) -ceq (Json @(2,2,2,3,3,4,20,321))) 'All 357 source/contract assertions required'
    Require ((ReadJson 'prototype-lint-exit.json').exit -eq 0) 'Prototype strict lint'
    Require ((ReadJson 'adoption-source-proof-exit.json').exit -eq 101 -and (Get-Content -Raw "$PSScriptRoot/adoption-source-proof.log").Contains('exact bounded projection parser')) 'Retain initial formatter-specific source refusal'
    Require ((ReadJson 'adoption-formatted-proof-exit.json').exit -eq 0 -and (ReadJson 'complete-sender-proof-exit.json').exit -eq 0) 'Corrected source proof'
    Require ((ReadJson 'source-contracts-exit.json').exit -eq 101 -and (Get-Content -Raw "$PSScriptRoot/source-contracts.log").Contains('exact projection test hook')) 'Retain non-idempotent predecessor-view refusal'
    $taskPost=@(Get-Content "$PSScriptRoot/local-postflight.log"|Where-Object{$_})
    Require ($taskPost.Count -eq 2 -and $taskPost[0].StartsWith('uid=0(root)') -and $taskPost[1] -cmatch '^[0-9a-f-]{36}$') 'Local fixture residue'
    $taskFacts=Facts
    if($Review){Require (-not (Test-Path "$PSScriptRoot/inventory.json")) 'Cannot bypass seal';$taskFacts|ConvertTo-Json -Depth 40;exit}
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
    [ordered]@{evidence_verified=$true;files=$taskCount;summary_mutations_refused=$taskMutations;allocation_effect_confirmed=$true;local_matrices_passed=$true;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
}finally{Pop-Location}
