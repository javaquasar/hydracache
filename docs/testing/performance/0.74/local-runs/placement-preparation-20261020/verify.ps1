param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Json([string]$Path){Raw $Path | ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.source -cne '8e976d769e00f0e818711ab4fc041a1b15b61a03' -or $Meta.baseline -cne 'ddbfe9edee97624fb6ab6a8bcb25db93045ea58d' -or $Meta.original_files -ne 430 -or $Meta.source_contracts -ne 315 -or $Meta.effect_brackets -ne 46 -or ($Meta.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Source/schedule identity drift'}
    foreach($taskClosed in @('runtime_changed','held_projection','original_send','completion','server','supervisor','product','qualification','preparation_speedup_claimed','allocations_claimed','historical_failing_tuple_recovered')){if($Meta.$taskClosed -isnot [bool] -or $Meta.$taskClosed -cne $false){throw "Scope drift: $taskClosed"}}
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -cne $Meta.manifest_sha256){throw 'Qualification drift'}
    if($Meta.prior_inventory_sha256 -cne 'f7d3784659c43e0f1f325b828fbc350476037b4f9b55793fd562f198ef562c06' -or (Hash (Join-Path $PSScriptRoot '../owned-placement-20261019/inventory.json')) -cne $Meta.prior_inventory_sha256){throw 'Prior packet drift'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object FullName -CNE (Join-Path $PSScriptRoot 'inventory.json'))
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing/duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    foreach($taskTest in @(@('baseline194.log',8),@('baseline188.log',8),@('source-screen.log',3),@('source-full.log',315),@('windows-unit.log',60),@('windows-doc.log',20))){if(-not (Raw $taskTest[0]).Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Test receipt missing'}}
    foreach($taskLint in @('windows-clippy.log','xtask-clippy.log')){if(-not (Raw $taskLint).Contains('Finished `dev` profile')){throw 'Strict lint missing'}}
    if(-not (Raw 'red-missing-trace.log').Contains('couldn') -or -not (Raw 'red-missing-trace.log').Contains('diagnostic_placement_preparation_trace.rs') -or -not (Raw 'green-check.log').Contains('Finished `dev` profile')){throw 'Construction evidence missing'}
    if((Raw 'fmt-check.log').Trim() -cne 'Formatting passed' -or -not (Raw 'registry.log').Contains('184 discovered gates, all registered') -or -not (Raw 'docs.log').Contains('doc-check: OK') -or -not (Raw 'local-performance.log').Contains('OK (local, non-promotable)')){throw 'Local checks missing'}
    foreach($taskCompiler in @('194','188')){
        $taskBuild=Json "gnu$taskCompiler/receipt.json"
        $taskVersion=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
        if($taskBuild.source -cne $Meta.source -or $taskBuild.compiler -cne $taskVersion -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'build.ps1')) -or $taskBuild.trace_units -ne 5 -or $taskBuild.selected_units -ne 66 -or $taskBuild.doc_tests -ne 247){throw 'Build identity'}
        foreach($taskClosed in @('profile','runtime_changed','original_send','server','supervisor','product','clock_changed','new_fault_cohort')){if($taskBuild.$taskClosed -isnot [bool] -or $taskBuild.$taskClosed -cne $false){throw 'Build scope'}}
        foreach($taskTest in @(@('trace-units',5),@('placement-units',8),@('launch-units',11),@('enrollment-units',5),@('owned-units',2),@('origin-units',22),@('closing-units',4),@('root-units',9),@('doc',247))){if(-not (Raw "gnu$taskCompiler/$($taskTest[0]).log").Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Linux units missing'}}
        if(-not (Raw "gnu$taskCompiler/clippy.log").Contains('Finished `dev` profile') -or -not (Raw "gnu$taskCompiler/compiler.log").Contains("release: $taskVersion")){throw 'Linux lint/compiler missing'}
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
    if($Meta.local_live_gate_passed -isnot [bool] -or $Meta.local_live_gate_passed -cne $taskPassed -or $taskOutcome.local_live_gate_passed -cne $taskPassed -or $Meta.observed_time_refusals -ne $taskRefusals){throw 'False full matrix or refusal attribution'}
    $taskAnalysis=@(Json 'analysis.json')
    if($taskAnalysis.Count -ne 8 -or $taskAnalysis[0].compiler -cne '194' -or $taskAnalysis[0].case -cne 'healthy' -or $taskAnalysis[0].returned -cne $false -or @($taskAnalysis | Where-Object compiler -CEQ '188').Count -ne 7){throw 'Attribution case drift'}
    $taskExpiry=$taskAnalysis[0].kernel_first_refusal
    if($taskExpiry.issued_ns -ne 884474515163 -or $taskExpiry.now_ns -ne 894476360325 -or $taskExpiry.age_ns -ne 10001845162 -or $taskExpiry.age_ns -ne ($taskExpiry.now_ns-$taskExpiry.issued_ns) -or $taskExpiry.diagnostic_reason -cne 'expired' -or $taskExpiry.issued_boot -cne 'c56f74ec-65b5-4a21-964d-cd04ce280385' -or $taskExpiry.now_boot -cne $taskExpiry.issued_boot){throw 'Actual GNU expiry changed'}
    $taskOriginalRow=([regex]::Match((Raw 'live-1/live194.log'),'HC074-PLACEMENT-PREPARATION (\{[^\r\n]+\})').Groups[1].Value | ConvertFrom-Json)
    if(($taskOriginalRow.causal.first_refusal | ConvertTo-Json -Compress) -cne ($taskExpiry | ConvertTo-Json -Compress) -or ($taskOriginalRow.age.sites[1].first_refusal | ConvertTo-Json -Compress) -cne ($taskAnalysis[0].raw_refusals[0] | ConvertTo-Json -Compress)){throw 'Analysis differs from original captured arguments'}
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
    foreach($taskField in @('source','baseline','original_files','source_contracts','effect_brackets','runtime_changed','held_projection','original_send','completion','server','product','qualification','local_live_gate_passed','observed_time_refusals','preparation_speedup_claimed','allocations_claimed','historical_failing_tuple_recovered')){
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
    Write-Output "placement preparation verifier mutations: $taskRejected refused"
}
Write-Output "placement preparation evidence: $($taskInventory.Count) raw files verified; original execution remains closed"
