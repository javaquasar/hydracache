param([switch]$Review)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
function ReadJson([string]$Name){Get-Content -Raw "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Json($Value){ConvertTo-Json -Depth 30 -Compress -InputObject $Value}
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function CheckSummary($Summary,$Expected){Require ((Json $Summary) -ceq (Json $Expected)) 'Summary does not match original logs'}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Inventory([string]$Directory,[string]$Expected){
    if($Expected){Require ((Hash "$Directory/inventory.json") -ceq $Expected) 'Prior seal changed'}
    $taskInventory=Get-Content -Raw "$Directory/inventory.json"|ConvertFrom-Json
    Require ($taskInventory.schema -ceq 'sha256-local-file-inventory-v1') 'Inventory schema'
    $taskActual=@(Get-ChildItem -LiteralPath $Directory -Recurse -Force|Where-Object{-not $_.PSIsContainer -and $_.FullName -cne "$Directory\inventory.json"}|ForEach-Object{$_.FullName.Substring($Directory.Length+1).Replace('\','/')})
    Require (@(Get-ChildItem -LiteralPath $Directory -Recurse -Force|Where-Object{($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0}).Count -eq 0) 'No reparse members'
    $taskPaths=[string[]]@($taskInventory.files.path)
    $taskSorted=[string[]]$taskPaths.Clone();[Array]::Sort($taskSorted,[StringComparer]::Ordinal)
    Require ((Json $taskPaths) -ceq (Json $taskSorted)) 'Inventory ordering'
    Require (@($taskPaths|Select-Object -Unique).Count -eq $taskPaths.Count) 'Duplicate members'
    Require ($taskActual.Count -eq $taskPaths.Count -and @($taskActual|Where-Object{$_ -cnotin $taskPaths}).Count -eq 0) 'Missing or extra member'
    foreach($taskFile in $taskInventory.files){
        Require ($taskFile.path -cmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -and $taskFile.path -cnotmatch '(^|/)\.\.(/|$)' -and $taskFile.path -cne 'inventory.json') 'Unsafe inventory path'
        $taskPath=Join-Path $Directory $taskFile.path
        Require ((Get-Item -LiteralPath $taskPath).Length -eq $taskFile.bytes -and (Hash $taskPath) -ceq $taskFile.sha256) "Member changed: $($taskFile.path)"
    }
    $taskPaths.Count
}
function Facts {
    $taskBinding=ReadJson 'bindings.json'
    Require ($taskBinding.source -ceq '3ffa6e91b0ab0b03b82206a2dc3760c752679092' -and $taskBinding.image_source -ceq 'b5f1b8fce6b3046239087873cb85ff1bfc557f0f') 'Source binding'
    Require (-not $taskBinding.server -and -not $taskBinding.product) 'Forbidden scope'
    Require ((Hash "$PSScriptRoot/run.ps1") -ceq $taskBinding.run_sha256 -and (Hash "$PSScriptRoot/matrix.sh") -ceq $taskBinding.matrix_sha256) 'Original recipe binding'
    $taskDispatch=ReadJson 'dispatch.json'
    Require ((Hash "$PSScriptRoot/dispatch.ps1") -ceq $taskDispatch.recipe_sha256 -and $taskDispatch.continued_pre_dispatch_only -and -not $taskDispatch.runtime_changed -and -not $taskDispatch.server -and -not $taskDispatch.product) 'Dispatch binding'
    $taskPost=ReadJson 'post.json'
    Require ($taskPost.exit -eq 0 -and -not $taskPost.server -and -not $taskPost.product -and (Json @($taskBinding.bundle_before)) -ceq (Json @($taskPost.bundle_after))) 'Postflight or bundle drift'
    $taskOldBundle=Get-Content -Raw "$taskRepo/docs/testing/performance/0.74/local-runs/held-preparation-cost-20261020/bundle-before.json"|ConvertFrom-Json
    Require ($taskBinding.bundle_before.Count -eq $taskOldBundle.files.Count) 'Original bundle member count'
    foreach($taskFile in $taskOldBundle.files){
        $taskMatch=@($taskBinding.bundle_before|Where-Object{$_.path -ceq $taskFile.name})
        Require ($taskMatch.Count -eq 1 -and $taskMatch[0].sha256 -ceq $taskFile.sha256 -and $taskMatch[0].bytes -eq $taskFile.bytes) 'Original bundle changed'
    }
    $taskPre=Get-Content -Raw "$PSScriptRoot/preflight.log"
    $taskAfter=Get-Content -Raw "$PSScriptRoot/postflight.log"
    $taskBoot=[regex]::Match($taskPre,'(?m)^boot=([0-9a-f-]{36})\r?$').Groups[1].Value
    Require ($taskBoot.Length -eq 36 -and [regex]::Match($taskAfter,'(?m)^boot=([0-9a-f-]{36})\r?$').Groups[1].Value -ceq $taskBoot) 'Session boot changed'
    foreach($taskText in $taskPre,$taskAfter){
        Require ($taskText -match 'uid=0\(root\)' -and $taskText -notmatch '(?m)^/(tmp|sys/fs/cgroup)/hc074-owned-root-') 'Fixture residue or wrong uid'
        Require ($taskText -match '(?m)^clocksource=tsc\r?$' -and $taskText -match '(?m)^timesyncd=active\r?$') 'Observed environment changed'
    }
    $taskExitRows=@(Get-Content "$PSScriptRoot/exits.txt")
    Require ($taskExitRows.Count -eq 2) 'Missing/extra matrix invocation'
    $taskExpected=@('healthy','revalidate','role-before','role-after','trust-after')
    $taskLanes=@()
    for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
        $taskLane=@('194','188')[$taskIndex]
        $taskExitMatch=[regex]::Match($taskExitRows[$taskIndex],"^$taskLane (0|101)$")
        Require $taskExitMatch.Success 'Unexpected original exit'
        $taskExit=[int]$taskExitMatch.Groups[1].Value
        $taskImage=$taskBinding.images[$taskIndex]
        $taskPrior=Get-Content -Raw "$taskRepo/docs/testing/performance/0.74/local-runs/pid-path-cost-20261020/Candidate-$taskLane-plain-image.json"|ConvertFrom-Json
        Require ((Json $taskImage) -ceq (Json $taskPrior)) 'Prior image binding'
        $taskLog=Get-Content -Raw "$PSScriptRoot/matrix-$taskLane.log"
        $taskOriginal=@([regex]::Matches($taskLog,'HC074-HELD-LIVE-ORIGINAL ([^\r\n]+)')|ForEach-Object{$_.Groups[1].Value|ConvertFrom-Json})
        $taskCompleted=@([regex]::Matches($taskLog,'HC074-HELD-LIVE case=(\S+) ([^\r\n]+)')|ForEach-Object{
            Require ($_.Groups[2].Value -ceq 'seed=0x074c051e actual_fixed=true actual_consuming=true original_bounds=true eof=true fence_retained=true custodians_terminal=true synthetic_images=true original_send=false completion=false server=false product=false') 'Changed success assertions'
            $_.Groups[1].Value
        })
        Require ($taskOriginal.Count -ge 1 -and $taskOriginal.Count -le 5) 'Original record count'
        Require ((Json @($taskOriginal.case)) -ceq (Json @($taskExpected|Select-Object -First $taskOriginal.Count))) 'Cases spliced or reordered'
        $taskWantCount=$taskOriginal.Count-$(if($taskExit -eq 0){0}else{1})
        Require ((Json $taskCompleted) -ceq (Json @($taskExpected|Select-Object -First $taskWantCount))) 'Completed case mismatch'
        Require (($taskExit -ne 0 -or $taskOriginal.Count -eq 5) -and $taskLog.Contains($(if($taskExit -eq 0){'test result: ok.'}else{'test result: FAILED.'}))) 'Matrix exit/status mismatch'
        $taskCases=@()
        for($taskCaseIndex=0;$taskCaseIndex -lt $taskOriginal.Count;$taskCaseIndex++){
            $taskRecord=$taskOriginal[$taskCaseIndex]
            Require (-not $taskRecord.guard_clock_added -and -not $taskRecord.deadline_changed -and -not $taskRecord.server -and -not $taskRecord.product) 'Changed admission semantics'
            Require ($taskRecord.returned -eq ($taskCaseIndex -lt $taskWantCount)) 'Original return mismatch'
            $taskRefusals=@($taskRecord.age.sites|ForEach-Object{$_.first_refusal}|Where-Object{$null -ne $_})
            if($taskRecord.returned){Require ($null -eq $taskRecord.kernel -and $null -eq $taskRecord.causal.first_refusal -and $taskRefusals.Count -eq 0) 'Successful case contains original refusal'}
            if($null -ne $taskRecord.kernel -and $taskRecord.kernel.site -ceq 'clock-order'){
                $taskClock=$taskRecord.kernel.clock
                Require ($taskClock.Count -eq 6 -and $taskClock[1] -gt $taskClock[0] -and $taskClock[3] -lt $taskClock[2] -and $taskClock[4] -eq 1 -and $taskClock[5] -eq 60000000000) 'Clock-order classification not supported'
            }
            $taskCases+=,[ordered]@{case=$taskRecord.case;returned=$taskRecord.returned;kernel=$taskRecord.kernel;causal_refusal=$taskRecord.causal.first_refusal;age_refusals=$taskRefusals}
        }
        $taskLanes+=,[ordered]@{lane=$taskLane;exit=$taskExit;completed_cases=$taskCompleted;original_cases=$taskCases}
    }
    [ordered]@{schema='local-consuming-composition-recheck-v1';source=$taskBinding.source;image_source=$taskBinding.image_source;boot=$taskBoot;same_session_boot=$true;lanes=$taskLanes;local_composition_gate=(@($taskLanes|Where-Object{$_.exit -ne 0}).Count -eq 0);full_functional_matrix_retested=$true;native_execution_gate_passed=$false;joined_sigkill_lost_ack_retested=$false;original_delivery_retested=$false;all_writer_cessation_proved=$false;later_cells_retested=$false;reconnect_retested=$false;production_enrollment=$false;server=$false;product=$false}
}
Push-Location $taskRepo
try {
    $taskPrior=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/pid-path-cost-20261020'
    Require ((Inventory $taskPrior '982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05') -eq 116) 'Prior packet count'
    Require ((Inventory (Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/held-preparation-cost-20261020') '4800b14816d9ca01a0ad04aa4082c37fc576f0957efe46644c85c01164b2b65f') -eq 166) 'Preparation seal/member count'
    $taskSelected=@('tools/long-run-supervisor-074','crates/xtask','crates/hydracache','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    foreach($taskSource in '3ffa6e91b0ab0b03b82206a2dc3760c752679092','b5f1b8fce6b3046239087873cb85ff1bfc557f0f'){
        & git diff --quiet $taskSource HEAD -- @taskSelected
        Require ($LASTEXITCODE -eq 0) 'Frozen source drift'
    }
    Require (-not (& git status --porcelain -- @taskSelected)) 'Dirty selected source'
    Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
    $taskFacts=Facts
    if($Review){
        Require (-not (Test-Path "$PSScriptRoot/inventory.json")) 'Cannot bypass sealed inventory'
        $taskFacts|ConvertTo-Json -Depth 30
        exit
    }
    $taskCount=Inventory $PSScriptRoot ''
    CheckSummary (ReadJson 'summary.json') $taskFacts
    foreach($taskCheck in 'format','doc-check','registry','contract-check'){
        $taskReceipt=ReadJson "$taskCheck-exit.json"
        Require ($taskReceipt.exit -eq 0 -and -not $taskReceipt.server -and -not $taskReceipt.product) 'Local check refused'
    }
    $taskMutations=0
    foreach($taskName in @($taskFacts.Keys)){
        $taskBad=Json $taskFacts|ConvertFrom-Json
        if($taskBad.$taskName -is [bool]){$taskBad.$taskName=-not $taskBad.$taskName}else{$taskBad.$taskName='changed'}
        $taskRefused=$false
        try {CheckSummary $taskBad $taskFacts}catch{if($_.Exception.Message -cne 'Summary does not match original logs'){throw};$taskRefused=$true}
        Require $taskRefused 'Summary mutation accepted'
        $taskMutations++
    }
    [ordered]@{evidence_verified=$true;inventory_checked=$true;files=$taskCount;summary_mutations_refused=$taskMutations;local_composition_gate=$taskFacts.local_composition_gate;native_execution_gate_passed=$false;server=$false;product=$false}|ConvertTo-Json
} finally {Pop-Location}
