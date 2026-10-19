param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Json([string]$Path){Raw $Path | ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.source -cne 'bb426070f7bcccab8e182fca100b62cba420aa21' -or $Meta.baseline -cne 'ad0347cb259d46b3356797f81fda84f01d323626' -or $Meta.original_files -ne 431 -or $Meta.source_contracts -ne 318 -or $Meta.effect_brackets -ne 46 -or ($Meta.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Source/schedule identity drift'}
    foreach($taskClosed in @('runtime_changed','held_projection','original_send','completion','server','supervisor','product','qualification','preparation_speedup_claimed','allocations_claimed','historical_failing_tuple_recovered')){if($Meta.$taskClosed -isnot [bool] -or $Meta.$taskClosed -cne $false){throw "Scope drift: $taskClosed"}}
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -cne $Meta.manifest_sha256){throw 'Qualification drift'}
    if($Meta.prior_inventory_sha256 -cne 'a2df2a1122fc24201d31aa56b6bcd971e3edcda735309523924b6c18c9cbe43a' -or (Hash (Join-Path $PSScriptRoot '../placement-preparation-20261020/inventory.json')) -cne $Meta.prior_inventory_sha256){throw 'Prior packet drift'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object FullName -CNE (Join-Path $PSScriptRoot 'inventory.json'))
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing/duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    foreach($taskTest in @(@('source-screen.log',3),@('source-full.log',318),@('windows-unit.log',60),@('windows-doc.log',20))){if(-not (Raw $taskTest[0]).Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Test receipt missing'}}
    foreach($taskLint in @('windows-clippy.log','xtask-clippy.log')){if(-not (Raw $taskLint).Contains('Finished `dev` profile')){throw 'Strict lint missing'}}
    if(-not (Raw 'first-check.log').Contains('Finished `dev` profile') -or -not (Raw 'baseline-verifier.log').Contains('57 raw files verified')){throw 'Construction/baseline evidence missing'}
    if((Raw 'fmt-check.log').Trim() -cne 'Formatting passed' -or -not (Raw 'registry.log').Contains('184 discovered gates, all registered') -or -not (Raw 'docs.log').Contains('doc-check: OK') -or -not (Raw 'local-performance.log').Contains('OK (local, non-promotable)')){throw 'Local checks missing'}
    foreach($taskCompiler in @('194','188')){
        $taskBuild=Json "gnu$taskCompiler/receipt.json"
        $taskVersion=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
        if($taskBuild.source -cne $Meta.source -or $taskBuild.compiler -cne $taskVersion -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'build.ps1')) -or $taskBuild.trace_units -ne 5 -or $taskBuild.selected_units -ne 68 -or $taskBuild.doc_tests -ne 247){throw 'Build identity'}
        foreach($taskClosed in @('profile','runtime_changed','original_send','server','supervisor','product','clock_changed','new_fault_cohort')){if($taskBuild.$taskClosed -isnot [bool] -or $taskBuild.$taskClosed -cne $false){throw 'Build scope'}}
        foreach($taskTest in @(@('trace-units',5),@('cost-units',2),@('placement-units',8),@('launch-units',11),@('enrollment-units',5),@('owned-units',2),@('origin-units',22),@('closing-units',4),@('root-units',9),@('doc',247))){if(-not (Raw "gnu$taskCompiler/$($taskTest[0]).log").Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Linux units missing'}}
        if(-not (Raw "gnu$taskCompiler/clippy.log").Contains('Finished `dev` profile') -or -not (Raw "gnu$taskCompiler/compiler.log").Contains("release: $taskVersion")){throw 'Linux lint/compiler missing'}
    }

    foreach($taskClosed in @('healthy_gate','local_live_gate_passed','guard_clock_added')){if($Meta.$taskClosed -isnot [bool] -or $Meta.$taskClosed -cne $false){throw 'Cost promoted to admission'}}
    $taskIdentity=Json 'cost-1/identity.json';$taskOutcome=Json 'cost-1/outcome.json';$taskAttempts=@(Json 'cost-1/attempt-outcomes.json')
    if($taskIdentity.source -cne $Meta.source -or $taskOutcome.source -cne $Meta.source -or $taskIdentity.recipe -cne (Hash (Join-Path $PSScriptRoot 'cost.ps1')) -or $taskIdentity.clock_before -cne 'active' -or $taskOutcome.clock_after -cne 'active' -or (Raw 'cost-1/clock-restored.log').Trim() -cne 'active' -or ($taskIdentity.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Cost identity/restoration'}
    foreach($taskClosed in @('actual_systemd_placement','original_send','completion','server','supervisor','product')){if($taskIdentity.$taskClosed -isnot [bool] -or $taskIdentity.$taskClosed -cne $false){throw 'Cost identity scope'}}
    if($taskOutcome.cost_observations -ne 3 -or $taskOutcome.local_live_gate_passed -cne $false -or $taskOutcome.healthy_gate -cne $false -or $taskAttempts.Count -ne 3 -or ($taskAttempts.attempt -join ',') -cne '0,1,2' -or $Meta.cost_observations -ne 3){throw 'Preregistered cohort count/admission'}
    $taskStable=Json 'cost-1/clock-stable.log'
    if($taskStable.stable_seconds -ne 65 -or $taskStable.samples -lt 1 -or $taskStable.attempt -cne $false){throw 'Clock preflight'}
    foreach($taskIndex in 0..1){
        $taskCompiler=if($taskIndex -eq 0){'194'}else{'188'}
        if((Json "gnu$taskCompiler/receipt.json").image -cne $taskIdentity.images[$taskIndex]){throw 'Cost image drift'}
    }
    $taskNames=@('enroll-join-cleanup','cold-setup-original-admission','fence-reserve-publish-challenge','response-consent','outputs','enrollment-frame','fixed-manager-message','owned-root-preparation')
    $taskRefusals=0
    foreach($taskAttempt in $taskAttempts){
        $taskIndex=$taskAttempt.attempt;$taskRaw=Raw "cost-1/cost-$taskIndex.log"
        $taskRows=[regex]::Matches($taskRaw,'HC074-PLACEMENT-COST (\{[^\r\n]+\})')
        if($taskRows.Count -ne 1 -or $taskAttempt.fixture_scan_exit_code -ne 0 -or @($taskAttempt.fixture_roots).Count -ne 0 -or $taskAttempt.healthy_gate -cne $false){throw 'Cost observation/cleanup'}
        $taskRow=$taskRows[0].Groups[1].Value | ConvertFrom-Json
        if($taskRow.case -cne 'healthy' -or $taskRow.returned -isnot [bool] -or $taskRow.returned -cne $taskAttempt.returned -or ($taskRow.phases.phase -join ',') -cne ($taskNames -join ',') -or $taskRow.kernel.Count -ne 10 -or $taskRow.live.Count -ne 6){throw 'Cost shape'}
        foreach($taskClosed in @('guard_clock_added','schedule_changed','deadline_changed','healthy_gate','original_send','server','product')){if($taskRow.$taskClosed -isnot [bool] -or $taskRow.$taskClosed -cne $false){throw 'Observer promoted'}}
        foreach($taskFact in @('wall_not_cpu','overlapping_spans','instrumented')){if($taskRow.$taskFact -isnot [bool] -or $taskRow.$taskFact -cne $true){throw 'Misrepresented instrumentation'}}
        $taskSum=($taskRow.phases | Measure-Object wall_ns -Sum).Sum
        if($taskRow.wall_ns -lt 1 -or $taskSum -gt $taskRow.wall_ns){throw 'Invalid elapsed scope identity'}
        if($null -ne $taskRow.causal.first_refusal -or @($taskRow.age.sites | Where-Object {$null -ne $_.first_refusal}).Count -ne 0){$taskRefusals++}
        if($taskRow.returned){
            if($taskAttempt.wsl_exit_code -ne 0 -or -not $taskRaw.Contains('test result: ok. 1 passed; 0 failed;') -or -not $taskRaw.Contains('HC074-OWNED-PLACEMENT-LIVE case=healthy accepted=true')){throw 'Missing successful original call'}
        }else{
            if($taskAttempt.wsl_exit_code -eq 0 -or -not $taskRaw.Contains('test result: FAILED. 0 passed; 1 failed;')){throw 'Failure promoted'}
        }
        $taskUsage=Raw "cost-1/usage-$taskIndex.log"
        foreach($taskField in @('privileged_placement_preparation_cost','User time (seconds):','System time (seconds):','Maximum resident set size (kbytes):',('Exit status: '+$taskAttempt.wsl_exit_code))){if(-not $taskUsage.Contains($taskField)){throw 'Whole-probe resource receipt missing'}}
    }
    if($Meta.observed_time_refusals -ne $taskRefusals){throw 'Cost refusal attribution drift'}
    $taskAnalysis=@(Json 'cost-analysis.json')
    if($taskAnalysis.Count -ne 3){throw 'Cost analysis count'}
    foreach($taskIndex in 0..2){
        $taskRow=([regex]::Match((Raw "cost-1/cost-$taskIndex.log"),'HC074-PLACEMENT-COST (\{[^\r\n]+\})').Groups[1].Value | ConvertFrom-Json)
        $taskDerived=$taskAnalysis[$taskIndex]
        if($taskDerived.attempt -ne $taskIndex -or $taskDerived.returned -cne $taskRow.returned -or $taskDerived.wall_ns -ne $taskRow.wall_ns -or ($taskDerived.phases | ConvertTo-Json -Depth 8 -Compress) -cne ($taskRow.phases | ConvertTo-Json -Depth 8 -Compress) -or ($taskDerived.kernel_counts -join ',') -cne ((@($taskRow.kernel | ForEach-Object {$_[0]})) -join ',') -or ($taskDerived.live_counts -join ',') -cne ((@($taskRow.live | ForEach-Object {$_[0]})) -join ',') -or $taskDerived.last_custody_age_ns -ne $taskRow.age.sites[0].last.age_ns){throw 'Derived cost facts drift'}
        if($taskRow.kernel[5][0] -ne 320831 -or $taskRow.kernel[6][0] -ne 641664 -or $taskRow.kernel[4][0] -ne 160415 -or $taskRow.phases[4].authentication_calls -ne 782){throw 'Observed operation counts drift'}
        $taskUsage=Raw "cost-1/usage-$taskIndex.log"
        foreach($taskField in @(@('user_seconds','User time \(seconds\): ([0-9.]+)'),@('system_seconds','System time \(seconds\): ([0-9.]+)'),@('max_rss_kbytes','Maximum resident set size \(kbytes\): ([0-9]+)'),@('major_faults','Major \(requiring I/O\) page faults: ([0-9]+)'))){
            $taskValue=[double]::Parse([regex]::Match($taskUsage,$taskField[1]).Groups[1].Value,[cultureinfo]::InvariantCulture)
            if($taskDerived.($taskField[0]) -ne $taskValue){throw 'Derived resource usage drift'}
        }
    }
    $taskIdentity=Json 'live-1/identity.json';$taskOutcome=Json 'live-1/outcome.json';$taskCompilers=@(Json 'live-1/compiler-outcomes.json')
    if($taskIdentity.source -cne $Meta.source -or $taskOutcome.source -cne $Meta.source -or $taskIdentity.recipe -cne (Hash (Join-Path $PSScriptRoot 'live.ps1')) -or $taskIdentity.clock_before -cne 'active' -or $taskOutcome.clock_after -cne 'active' -or (Raw 'live-1/clock-restored.log').Trim() -cne 'active' -or ($taskIdentity.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Live identity/restoration drift'}
    foreach($taskClosed in @('actual_systemd_placement','original_send','completion','server','supervisor','product')){if($taskIdentity.$taskClosed -isnot [bool] -or $taskIdentity.$taskClosed -cne $false){throw 'Live identity scope'}}
    $taskStable=Json 'live-1/clock-stable.log'
    if($taskStable.stable_seconds -ne 65 -or $taskStable.samples -lt 1 -or $taskStable.attempt -cne $false){throw 'Clock preflight missing'}
    if($taskCompilers.Count -ne 2 -or ($taskCompilers.compiler -join ',') -cne '194,188'){throw 'Missing/duplicate compiler cohort'}
    $taskCases=@('healthy','revalidate','unarmed','wrong-key','different-consent','foreign-root','trust-drift');$taskRefusals=0
    for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
        $taskCompiler=$taskCompilers[$taskIndex];$taskSuffix=$taskCompiler.compiler
        if((Json "gnu$taskSuffix/receipt.json").image -cne $taskIdentity.images[$taskIndex] -or $taskCompiler.fixture_scan_exit_code -ne 0 -or @($taskCompiler.fixture_roots).Count -ne 0){throw 'Live image/cleanup drift'}
        $taskRaw=Raw "live-1/live$taskSuffix.log"
        $taskRows=@([regex]::Matches($taskRaw,'HC074-PLACEMENT-PREPARATION (\{[^\r\n]+\})') | ForEach-Object {$_.Groups[1].Value | ConvertFrom-Json})
        if($taskRows.Count -lt 1 -or $taskRows.Count -gt 7 -or ($taskRows.case -join ',') -cne ($taskCases[0..($taskRows.Count-1)] -join ',')){throw 'Preparation case order/count'}
        foreach($taskRow in $taskRows){
            foreach($taskClosed in @('schedule_changed','deadline_changed','new_clock','original_send','server','product')){if($taskRow.$taskClosed -isnot [bool] -or $taskRow.$taskClosed -cne $false){throw 'Observer promoted'}}
            if($null -ne $taskRow.causal.first_refusal -or @($taskRow.age.sites | Where-Object {$null -ne $_.first_refusal}).Count -ne 0){$taskRefusals++}
        }
        $taskCompleted=@($taskRows | Where-Object returned -CEQ $true | ForEach-Object case)
        $taskFailed=@($taskRows | Where-Object returned -CNE $true | ForEach-Object case)
        if(($taskCompiler.completed_cases -join ',') -cne ($taskCompleted -join ',') -or ($taskCompiler.failed_cases -join ',') -cne ($taskFailed -join ',')){throw 'Declared case result drift'}
        $taskJoins=@([regex]::Matches($taskRaw,'HC074-OWNED-PLACEMENT-LIVE case=([^ ]+) accepted=(true|false) ([^\r\n]+) facts=(\{[^\r\n]+\})'))
        if($taskJoins.Count -ne $taskCompleted.Count){throw 'Missing completed custody proof'}
        foreach($taskJoin in $taskJoins){
            $taskCase=$taskJoin.Groups[1].Value;$taskExpected=if($taskCase -cin @('healthy','revalidate')){'true'}else{'false'}
            if($taskJoin.Groups[2].Value -cne $taskExpected -or $taskCase -cnotin $taskCompleted -or $null -ne ($taskJoin.Groups[4].Value | ConvertFrom-Json).first_refusal){throw 'Intended negative replaced by Time'}
            foreach($taskFact in @('original_bounds=true','eof=true','fence_held_while_owned=true','timer_live_before_drop=true','actual_systemd_placement=false','original_send=false','completion=false','product=false')){if(-not $taskJoin.Groups[3].Value.Contains($taskFact)){throw 'Custody safety proof missing'}}
            $taskArmed=if($taskCase -ceq 'unarmed'){'false'}else{'true'};$taskReplay=if($taskCase -ceq 'unarmed'){'true'}else{'false'}
            if(-not $taskJoin.Groups[3].Value.Contains("fence_held_after_drop=$taskArmed") -or -not $taskJoin.Groups[3].Value.Contains("retained_replay_refused=$taskReplay")){throw 'Armed/unarmed custody drift'}
        }
        $taskPassed=$taskCompiler.wsl_exit_code -eq 0 -and $taskRaw.Contains('test result: ok. 1 passed; 0 failed;') -and $taskRows.Count -eq 7 -and $taskFailed.Count -eq 0
        if($taskCompiler.full_cohort_passed -isnot [bool] -or $taskCompiler.full_cohort_passed -cne $taskPassed){throw 'False full compiler promotion'}
        if(-not $taskPassed -and -not $taskRaw.Contains('test result: FAILED. 0 passed; 1 failed;')){throw 'Missing actual failed cohort'}
    }
    $taskPassed=@($taskCompilers | Where-Object full_cohort_passed -CNE $true).Count -eq 0
    if($Meta.post_profile_functional_gate_passed -isnot [bool] -or $Meta.post_profile_functional_gate_passed -cne $taskPassed -or $taskOutcome.local_live_gate_passed -cne $taskPassed -or $Meta.functional_time_refusals -ne $taskRefusals){throw 'False full matrix or refusal attribution'}
    $taskAnalysis=@(Json 'functional-analysis.json')
    if($taskAnalysis.Count -ne 14){throw 'Functional analysis incomplete'}
    foreach($taskCompiler in @('194','188')){
        $taskRaw=Raw "live-1/live$taskCompiler.log"
        if($taskRaw.Contains('HC074-PLACEMENT-COST')){throw 'Functional source ran with cost capture'}
        foreach($taskMatch in [regex]::Matches($taskRaw,'HC074-PLACEMENT-PREPARATION (\{[^\r\n]+\})')){
            $taskRow=$taskMatch.Groups[1].Value | ConvertFrom-Json
            $taskDerived=@($taskAnalysis | Where-Object {$_.compiler -ceq $taskCompiler -and $_.case -ceq $taskRow.case})
            if($taskDerived.Count -ne 1 -or $taskDerived[0].returned -cne $taskRow.returned -or $taskDerived[0].first_custody_age_ns -ne $taskRow.age.sites[0].first.age_ns -or $taskDerived[0].last_custody_age_ns -ne $taskRow.age.sites[0].last.age_ns -or $taskDerived[0].boot -cne $taskRow.causal.first.issued_boot -or $null -ne $taskDerived[0].causal_first_refusal -or @($taskDerived[0].raw_first_refusals).Count -ne 0){throw 'Derived functional facts drift'}
        }
    }
    if((Raw 'fixture-before.json') -cne (Raw 'fixture-after.json') -or @(Json 'fixture-before.json').Count -ne 9){throw 'Signed fixture drift'}
    $taskCleanup=Json 'cleanup.json'
    if(@($taskCleanup.roots).Count -ne 0 -or $taskCleanup.scan_exit_code -ne 0 -or $taskCleanup.clock -cne 'active'){throw 'Final restoration drift'}
}
$taskMeta=Json 'metadata.json';$taskInventoryPath=Join-Path $PSScriptRoot 'inventory.json'
if($Seal){
    if(Test-Path -LiteralPath $taskInventoryPath){throw 'Never reseal retained evidence'}
    $taskInventory=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Sort-Object FullName | ForEach-Object {[ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}})
    Check $taskMeta $taskInventory
    $taskInventory | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $taskInventoryPath -Encoding utf8NoBOM
}
$taskInventory=@(Json 'inventory.json');Check $taskMeta $taskInventory
if($SelfTest){
    $taskRejected=0
    foreach($taskField in @('source','baseline','original_files','source_contracts','effect_brackets','runtime_changed','held_projection','original_send','completion','server','product','qualification','local_live_gate_passed','observed_time_refusals','preparation_speedup_claimed','allocations_claimed','post_profile_functional_gate_passed')){
        $taskChanged=$taskMeta | ConvertTo-Json -Depth 30 | ConvertFrom-Json;$taskValue=$taskChanged.$taskField
        $taskChanged.$taskField=if($taskValue -is [bool]){-not $taskValue}elseif($taskValue -is [string]){"${taskValue}x"}else{$taskValue+1}
        $taskRefused=$false;try{Check $taskChanged $taskInventory}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted metadata mutation: $taskField"};$taskRejected++
    }
    foreach($taskKind in @('missing','duplicate','hash')){
        $taskChanged=@($taskInventory | ConvertTo-Json -Depth 30 | ConvertFrom-Json)
        switch($taskKind){'missing'{$taskChanged=$taskChanged[1..($taskChanged.Count-1)]};'duplicate'{$taskChanged+=,$taskChanged[0]};'hash'{$taskChanged[0].sha256='0'*64}}
        $taskRefused=$false;try{Check $taskMeta $taskChanged}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted inventory mutation: $taskKind"};$taskRejected++
    }
    if($taskRejected -ne 20){throw 'Mutation selection drift'}
    Write-Output "placement stage cost verifier mutations: $taskRejected refused"
}
Write-Output "placement stage cost evidence: $($taskInventory.Count) raw files verified; original execution remains closed"
