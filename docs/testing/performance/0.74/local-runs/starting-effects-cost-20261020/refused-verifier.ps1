param([switch]$Review)
$ErrorActionPreference='Stop'
. "$PSScriptRoot/facts.ps1"
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
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
    Require ($taskSources.measured -ceq '9471afb4f2ce297c5c2d9ece77603c7e6a3e5bfe' -and $taskSources.baseline -ceq '4d0f461c638d158ff62dbd3e0f8f9914d523581e') 'Frozen source identity'
    & git diff --quiet $taskSources.measured HEAD -- tools/long-run-supervisor-074 crates/xtask
    Require ($LASTEXITCODE -eq 0 -and -not (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)) 'Measured source changed'
    & git diff --quiet $taskSources.baseline HEAD -- Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml crates/hydracache
    Require ($LASTEXITCODE -eq 0) 'Frozen inputs changed'
    $taskProduct=@(& git diff --name-only $taskSources.baseline HEAD -- crates|Where-Object{-not $_.StartsWith('crates/xtask/')})
    Require ($taskProduct.Count -eq 0) 'Product code changed'
    Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Current qualification drift'
    $taskBefore=ReadJson 'inputs-before.json';$taskAfter=ReadJson 'inputs-after.json'
    $taskRecorded=(& git show '9471afb4f2ce297c5c2d9ece77603c7e6a3e5bfe:docs/testing/performance/0.74/local-runs/starting-effects-cost-20261020/inputs-before.json'|Out-String)|ConvertFrom-Json
    Require ($LASTEXITCODE -eq 0 -and (Json $taskBefore) -ceq (Json $taskRecorded)) 'Predeclared inputs changed'
    Require ((Json $taskBefore.bundle) -ceq (Json $taskAfter.bundle) -and $taskAfter.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Bundle/qualification changed'
    Require ((Json $taskBefore.prior_inventories) -ceq (Json $taskAfter.prior_inventories) -and $taskBefore.prior_inventories.Count -eq 5) 'Prior anchors changed'
    foreach($taskAnchor in $taskBefore.prior_inventories){[void](Inventory (Join-Path $PSScriptRoot "../$($taskAnchor[0])") $taskAnchor[1])}
    foreach($taskName in 'kernel','tmp-filesystem','local-boot'){
        $taskReceipt=ReadJson "$taskName-exit.json"
        Require ($taskReceipt.source -ceq $taskSources.measured -and $taskReceipt.exit -eq 0 -and $taskReceipt.recipe_sha256 -ceq (Hash "$PSScriptRoot/environment.ps1") -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Environment binding'
    }
    foreach($taskLane in '194','188'){
        $taskImage=ReadJson "$taskLane-image.json"
        Require ($taskImage.source -ceq $taskSources.measured -and $taskImage.lane -ceq $taskLane -and $taskImage.sha256 -cmatch '^[0-9a-f]{64}$' -and $taskImage.recipe_sha256 -ceq (Hash "$PSScriptRoot/run.ps1") -and -not $taskImage.server -and -not $taskImage.product) 'Image binding'
        $taskBuild=Get-Content -Raw "$PSScriptRoot/$taskLane-build.log"
        $taskArtifacts=@([regex]::Matches($taskBuild,'(?m)^\{"reason":"compiler-artifact"[^\r\n]*')|ForEach-Object{$_.Value|ConvertFrom-Json}|Where-Object{$_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
        Require ($taskArtifacts.Count -eq 1 -and $taskArtifacts[0].executable -ceq $taskImage.path) 'Compiler-selected image'
        foreach($taskName in @('build','starting','bridge','kernel','trust','observer','boundaries','lint','Profile-0','Matrix-full')+$(if($taskLane -ceq '194'){@('Profile-1','Profile-2')}else{@()})){
            $taskReceipt=ReadJson "$taskLane-$taskName-exit.json"
            # Integrity verifies the actual refusal; it never marks a failed gate green.
            $taskExit=if(($taskLane -ceq '188' -and $taskName -ceq 'Profile-0') -or ($taskLane -ceq '194' -and $taskName -ceq 'Matrix-full')){101}else{0}
            Require ($taskReceipt.source -ceq $taskSources.measured -and $taskReceipt.selection -ceq $taskName -and $taskReceipt.exit -eq $taskExit -and $taskReceipt.recipe_sha256 -ceq $taskImage.recipe_sha256 -and -not $taskReceipt.server -and -not $taskReceipt.product) "Actual selection drift $taskLane/$taskName"
        }
        foreach($taskSelection in @(@('bridge',20),@('kernel',61),@('trust',26),@('observer',3),@('boundaries',1))){Require ((Get-Content -Raw "$PSScriptRoot/$taskLane-$($taskSelection[0]).log").Contains("test result: ok. $($taskSelection[1]) passed; 0 failed;")) 'Regression selection'}
        $taskStarting=Get-Content -Raw "$PSScriptRoot/$taskLane-starting.log"
        Require ($taskStarting.Contains('collector_starting_storage_exact_phase_order_and_old_inventory ... ok') -and $taskStarting.Contains('collector_starting_storage_exclusive_create_and_fd_mutations_refuse ... ok') -and $taskStarting.Contains('collector_starting_process_protocol_accepts_only_bounded_expected_records ... ok') -and -not $taskStarting.Contains('test result: FAILED')) 'Starting regression coverage'
    }
    foreach($taskName in 'format','xtask-lint','source-contracts','doc-check','registry','contract-check'){
        $taskReceipt=ReadJson "review-$taskName-exit.json"
        Require ($taskReceipt.source -ceq $taskSources.measured -and $taskReceipt.exit -eq 0 -and $taskReceipt.recipe_sha256 -ceq (Hash "$PSScriptRoot/review-checks.ps1") -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Repository gate refused'
    }
    $taskCounts=@([regex]::Matches((Get-Content -Raw "$PSScriptRoot/review-source-contracts.log"),'test result: ok\. ([0-9]+) passed; 0 failed; 0 ignored;')|ForEach-Object{[int]$_.Groups[1].Value}|Sort-Object)
    Require ((Json $taskCounts) -ceq (Json @(2,2,2,2,3,3,4,20,321))) 'All 359 source/contract assertions required'
    $taskPost=@(Get-Content "$PSScriptRoot/local-postflight.log"|Where-Object{$_})
    Require ($taskPost.Count -eq 1 -and $taskPost[0] -ceq (Get-Content -Raw "$PSScriptRoot/local-boot.log").Trim() -and (ReadJson 'local-postflight-exit.json').exit -eq 0) 'Local boot or fixture residue'
    $taskFacts=Facts
    if($Review){Require (-not (Test-Path "$PSScriptRoot/inventory.json")) 'Cannot bypass seal';$taskFacts|ConvertTo-Json -Depth 60;exit}
    $taskCount=Inventory $PSScriptRoot ''
    $taskSummary=ReadJson 'summary.json'
    Require ((Json $taskSummary) -ceq (Json $taskFacts)) 'Summary differs from raw facts'
    $taskMutations=0
    foreach($taskProperty in $taskSummary.PSObject.Properties){
        $taskChanged=Json $taskSummary|ConvertFrom-Json
        $taskChanged.($taskProperty.Name)='corrupt'
        Require ((Json $taskChanged) -cne (Json $taskFacts)) 'Mutation escaped exact summary comparison'
        $taskMutations++
    }
    [ordered]@{integrity_verified=$true;complete_profile_cohort=$false;both_local_matrices_passed=$false;runtime_candidate_adopted=$false;members=$taskCount;inventory_sha256=(Hash "$PSScriptRoot/inventory.json");summary_mutations_refused=$taskMutations;server=$false;product=$false}|ConvertTo-Json
}finally{Pop-Location}
