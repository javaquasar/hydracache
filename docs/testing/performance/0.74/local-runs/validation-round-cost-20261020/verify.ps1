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
    $taskSource='f1e6ea87e0e67b3e1dcdde0bc563a79cc9e8644a'
    $taskSources=ReadJson 'sources.json'
    Require ($taskSources.measured -ceq $taskSource -and $taskSources.baseline -ceq '8ecb405b4068236970bcf72563a72ee3e7ca9c2e') 'Frozen source'
    & git diff --quiet $taskSource HEAD -- tools/long-run-supervisor-074 crates/xtask
    Require ($LASTEXITCODE -eq 0 -and -not (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)) 'Measured source drift'
    & git diff --quiet $taskSources.baseline HEAD -- Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
    Require ($LASTEXITCODE -eq 0) 'Frozen inputs changed'
    Require (@(& git diff --name-only $taskSources.baseline HEAD -- crates|Where-Object{-not $_.StartsWith('crates/xtask/')}).Count -eq 0) 'Product changed'
    $taskBefore=ReadJson 'inputs-before.json';$taskAfter=ReadJson 'inputs-after.json'
    Require ((Json $taskBefore.bundle) -ceq (Json $taskAfter.bundle) -and (Json $taskBefore.prior_inventories) -ceq (Json $taskAfter.prior_inventories) -and $taskBefore.prior_inventories.Count -eq 6) 'Original bundle/anchors changed'
    Require ($taskBefore.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -and $taskAfter.qualification_sha256 -ceq $taskBefore.qualification_sha256 -and (Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq $taskBefore.qualification_sha256) 'Qualification changed'
    $taskCommitted=(& git show 'f1e6ea87e0e67b3e1dcdde0bc563a79cc9e8644a:docs/testing/performance/0.74/local-runs/validation-round-cost-20261020/inputs-before.json'|Out-String)|ConvertFrom-Json
    Require ($LASTEXITCODE -eq 0 -and (Json $taskBefore) -ceq (Json $taskCommitted)) 'Initial inputs changed'
    foreach($taskAnchor in $taskBefore.prior_inventories){[void](Inventory (Join-Path $PSScriptRoot "../$($taskAnchor[0])") $taskAnchor[1])}
    foreach($taskLane in '194','188'){
        $taskImage=ReadJson "$taskLane-image.json"
        Require ($taskImage.source -ceq $taskSource -and $taskImage.lane -ceq $taskLane -and $taskImage.sha256 -cmatch '^[0-9a-f]{64}$' -and $taskImage.recipe_sha256 -ceq (Hash "$PSScriptRoot/run.ps1")) 'Image/recipe'
        $taskBuild=@(Get-Content "$PSScriptRoot/$taskLane-build.log"|Where-Object{$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object{$_|ConvertFrom-Json}|Where-Object{$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
        Require ($taskBuild.Count -eq 1 -and $taskBuild[0].executable -ceq $taskImage.path) 'Compiler-selected image'
        foreach($taskName in 'build','round-observer','frame','starting','bridge','kernel','trust','observer','boundaries','lint'){
            $taskReceipt=ReadJson "$taskLane-$taskName-exit.json"
            Require ($taskReceipt.exit -eq 0 -and $taskReceipt.source -ceq $taskSource -and $taskReceipt.recipe_sha256 -ceq $taskImage.recipe_sha256 -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Focused gate refused'
        }
        Require ((Get-Content -Raw "$PSScriptRoot/$taskLane-round-observer.log").Contains('test result: ok. 3 passed; 0 failed;')) 'Three observer controls'
        foreach($taskFile in @(Get-ChildItem $PSScriptRoot -Filter "$taskLane-Profile-*-exit.json")+@(Get-Item "$PSScriptRoot/$taskLane-Matrix-full-exit.json")){
            $taskReceipt=Get-Content -Raw $taskFile.FullName|ConvertFrom-Json
            Require ($taskReceipt.source -ceq $taskSource -and $taskReceipt.exit -in @(0,101) -and $taskReceipt.recipe_sha256 -ceq $taskImage.recipe_sha256 -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Original live receipt binding'
        }
    }
    foreach($taskLane in '194','188','xtask'){
        $taskReceipt=ReadJson "check-$taskLane-exit.json"
        Require ($taskReceipt.exit -eq 0 -and $taskReceipt.source -ceq $taskSource -and $taskReceipt.recipe_sha256 -ceq (Hash "$PSScriptRoot/check.ps1")) 'Affected cargo check'
    }
    foreach($taskName in 'format','xtask-lint','source-contracts','doc-check','registry','contract-check'){
        $taskReceipt=ReadJson "review-$taskName-exit.json"
        Require ($taskReceipt.exit -eq 0 -and $taskReceipt.source -ceq $taskSource -and $taskReceipt.recipe_sha256 -ceq (Hash "$PSScriptRoot/review-checks.ps1")) 'Repository check'
    }
    $taskCounts=@([regex]::Matches((Get-Content -Raw "$PSScriptRoot/review-source-contracts.log"),'test result: ok\. ([0-9]+) passed; 0 failed; 0 ignored;')|ForEach-Object{[int]$_.Groups[1].Value}|Sort-Object)
    Require ((Json $taskCounts) -ceq (Json @(2,2,2,2,2,3,3,4,20,321))) 'All 361 historical/current assertions'
    Require ((ReadJson 'bootstrap-observer-exit.json').exit -eq 0 -and (ReadJson 'bootstrap-lint-exit.json').exit -eq 0 -and (ReadJson 'bootstrap-source-refusal.json').failed -eq 1) 'Retain bootstrap outcomes'
    $taskRefused=(& git show 'f1e6ea87e0e67b3e1dcdde0bc563a79cc9e8644a:docs/testing/performance/0.74/local-runs/validation-round-cost-20261020/check.ps1'|Out-String).Replace([string][char]13,'').TrimEnd()
    Require ($LASTEXITCODE -eq 0 -and (Get-Content -Raw "$PSScriptRoot/check-refused.ps1").Replace([string][char]13,'').TrimEnd() -ceq $taskRefused -and -not (ReadJson 'check-wrapper-refusal.json').native_check_started) 'Retain refused recipe'
    $taskClock=ReadJson 'clock-observations-v2.json'
    Require ($taskClock.recipe_sha256 -ceq (Hash "$PSScriptRoot/probe-v2.ps1") -and $taskClock.probe_sha256 -ceq (Hash "$PSScriptRoot/clock_probe_v2.py")) 'Read-only observer source'
    Require ((ReadJson 'clock-observations.json').recipe_sha256 -ceq (Hash "$PSScriptRoot/diagnose.ps1") -and (ReadJson 'clock-observations.json').probe_sha256 -ceq (Hash "$PSScriptRoot/clock_probe.py")) 'Initial refusal source'
    $taskPost=@(Get-Content "$PSScriptRoot/local-postflight.log"|Where-Object{$_})
    Require ($taskPost.Count -eq 1 -and $taskPost[0] -cmatch '^[0-9a-f-]{36}$' -and (ReadJson 'local-postflight-exit.json').exit -eq 0) 'Owned fixture residue'
    $taskFacts=Facts
    if($Review){Require (-not (Test-Path "$PSScriptRoot/inventory.json")) 'No seal bypass';$taskFacts|ConvertTo-Json -Depth 70;exit}
    $taskCount=Inventory $PSScriptRoot ''
    $taskSummary=ReadJson 'summary.json'
    Require ((Json $taskSummary) -ceq (Json $taskFacts)) 'Summary differs from raw evidence'
    $taskMutations=0
    foreach($taskProperty in $taskSummary.PSObject.Properties){$taskChanged=(Json $taskSummary)|ConvertFrom-Json;$taskChanged.($taskProperty.Name)='corrupt';Require ((Json $taskChanged) -cne (Json $taskFacts)) 'Summary mutation escaped';$taskMutations++}
    [ordered]@{integrity_verified=$true;complete_profile_cohort=$taskFacts.complete_profile_cohort;both_local_matrices_passed=$taskFacts.both_local_matrices_passed;native_gate_open=$false;members=$taskCount;inventory_sha256=(Hash "$PSScriptRoot/inventory.json");summary_mutations_refused=$taskMutations;server=$false;product=$false}|ConvertTo-Json
}finally{Pop-Location}
