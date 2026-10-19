param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Json([string]$Path){Raw $Path | ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.baseline -cne 'f4d7f44c2dc39ef6aa19cfc26611bde32e97bdcc' -or $Meta.source -cne '67c2f41c1b9288297c95507986859883f5645949' -or $Meta.original_files -ne 425 -or $Meta.source_contracts -ne 312 -or $Meta.effect_brackets -ne 46 -or ($Meta.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Original source/check schedule'}
    if($Meta.seed -cne '0x074e6a1d' -or $Meta.constraint_swaps -ne 128 -or $Meta.consent_leaves -ne 36 -or $Meta.new_units -ne 8){throw 'Reader mutation coverage'}
    foreach($taskClosed in @('held_projection','original_send','barrier_release','completion','fence_release','production_enrollment','server','supervisor','product','qualification','actual_systemd_placement','artifact_admission','new_peer_authenticated','new_creator_sigkill','preparation_wall_speedup_claimed','allocation_reduction_claimed')){if($Meta.$taskClosed -isnot [bool] -or $Meta.$taskClosed -cne $false){throw "Boundary changed: $taskClosed"}}
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -cne $Meta.manifest_sha256){throw 'Qualification manifest drift'}
    if($Meta.prior_inventory_sha256 -cne 'f86588ec9eff80ddab1a5fc3acd027adb98082c1e16b314bbe9dfea02d2f7e82' -or (Hash (Join-Path $PSScriptRoot '../held-placement-20261019/inventory.json')) -cne $Meta.prior_inventory_sha256){throw 'Prior sealed evidence drift'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object FullName -CNE (Join-Path $PSScriptRoot 'inventory.json'))
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing/duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    foreach($taskTest in @(@('baseline194.log',11),@('baseline188.log',11),@('green194.log',8),@('source-screen-final.log',3),@('source-full-final.log',312),@('windows-unit-final.log',60),@('windows-doc-final.log',20))){if(-not (Raw $taskTest[0]).Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Missing local test receipt'}}
    foreach($taskLint in @('windows-clippy.log','xtask-clippy-final.log')){if(-not (Raw $taskLint).Contains('Finished `dev` profile')){throw 'Strict lint missing'}}
    if(-not (Raw 'registry-final.log').Contains('gated-test-check: OK (184 discovered gates, all registered)') -or -not (Raw 'docs-post-results.log').Contains('doc-check: OK') -or -not (Raw 'local-performance-post-results.log').Contains('OK (local, non-promotable)') -or (Raw 'fmt-final.log').Trim() -cne 'Formatting passed'){throw 'Local contracts'}
    if(-not (Raw 'red-missing-module.log').Contains('E0432') -or -not (Raw 'red-missing-module.log').Contains('placement')){throw 'Missing red-test evidence'}
    foreach($taskBaselineImage in @('50e893f538817253752f765ffdb7866e66c228880812127a3d0d9e06a68aa6b2','8f1bf369e83c0a515563b9e3b989272afd8ca4a03960eccfdbfae762fb82c8a2')){if(-not (Raw 'baseline-images.log').Contains($taskBaselineImage)){throw 'Baseline identity'}}
    foreach($taskCompiler in @('194','188')){
        $taskDir="gnu$taskCompiler-final"
        $taskBuild=Json "$taskDir/receipt.json"
        $taskVersion=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
        if($taskBuild.source -cne $Meta.source -or $taskBuild.compiler -cne $taskVersion -or $taskBuild.image -cnotmatch '^[0-9a-f]{64}  /home/artur/\.cache/hydracache-074-' -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'build.ps1')) -or $taskBuild.placement_units -ne 8 -or $taskBuild.launch_units -ne 11 -or $taskBuild.enrollment_units -ne 5 -or $taskBuild.doc_tests -ne 247){throw 'Build identity'}
        foreach($taskClosed in @('profile','original_send','server','supervisor','product','clock_changed','new_fault_cohort')){if($taskBuild.$taskClosed -isnot [bool] -or $taskBuild.$taskClosed -cne $false){throw 'Build scope'}}
        foreach($taskTest in @(@('placement-units',8),@('launch-units',11),@('enrollment-units',5),@('owned-units',2),@('origin-units',22),@('closing-units',4),@('root-units',9),@('doc',247))){if(-not (Raw "$taskDir/$($taskTest[0]).log").Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Linux units'}}
        if(-not (Raw "$taskDir/placement-units.log").Contains('HC074-OWNED-PLACEMENT seed=0x074e6a1d identities=128 execution=false') -or -not (Raw "$taskDir/clippy.log").Contains('Finished `dev` profile') -or -not (Raw "$taskDir/compiler.log").Contains("release: $taskVersion")){throw 'Linux recipe receipt'}
    }
    if(-not (Raw 'clippy-initial188.log').Contains('result_large_err')){throw 'Initial strict lint negative missing'}
    if($Meta.local_live_gate_passed -isnot [bool] -or $Meta.local_live_gate_passed -cne $false -or $Meta.original_full_rounds_per_join -ne 1 -or $Meta.new_signature_checks_per_join -ne 2 -or $Meta.kernel_observations_per_join -ne 4){throw 'Join schedule or false full-gate promotion'}
    foreach($taskStage in @('baseline')){
        $taskDir='baseline-live';$taskRun=Json "$taskDir/receipt.json"
        if($taskRun.source -cne $Meta.baseline -or $taskRun.stage -cne $taskStage -or $taskRun.recipe -cne (Hash (Join-Path $PSScriptRoot 'live.ps1')) -or $taskRun.clock_before -cne 'active' -or $taskRun.clock_after -cne 'active' -or (Raw "$taskDir/clock-restored.log").Trim() -cne 'active' -or ($taskRun.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Live identity/clock/deadlines'}
        foreach($taskClosed in @('actual_systemd_placement','original_send','completion','server','supervisor','product')){if($taskRun.$taskClosed -isnot [bool] -or $taskRun.$taskClosed -cne $false){throw 'Live scope'}}
        $taskStable=Json "$taskDir/clock-stable.log"
        if($taskStable.stable_seconds -ne 65 -or $taskStable.samples -lt 1 -or $taskStable.attempt -cne $false){throw 'Clock stability receipt'}
        for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
            $taskCompiler=if($taskIndex -eq 0){'194'}else{'188'}
            $taskRaw=Raw "$taskDir/live$taskCompiler.log"
            if(-not $taskRaw.Contains('test result: ok. 1 passed; 0 failed;')){throw 'Live selection'}
            if(-not $taskRaw.Contains('HC074-OWNED-ROOT case=retire') -or -not (Raw 'baseline-images.log').Contains($taskRun.images[$taskIndex])){throw 'Baseline actual root'}
        }
    }
    $taskFinal=Json 'final-live-outcome.json'
    if($taskFinal.source -cne $Meta.source -or $taskFinal.recipe -cne (Hash (Join-Path $PSScriptRoot 'live.ps1')) -or ($taskFinal.completed_cases -join ',') -cne 'healthy,revalidate,unarmed' -or $taskFinal.failed_case -cne 'wrong-key' -or $taskFinal.failed_stage -cne 'prepare_owned_root_guardians' -or $taskFinal.reason -cne 'Causal(Time) during original preparation before signed placement binding'){throw 'Final partial result changed'}
    foreach($taskFalse in @('failing_tuple_recorded','native_test_exit_code_recorded','msrv_attempted','local_live_gate_passed','original_send','completion','server','product')){if($taskFinal.$taskFalse -isnot [bool] -or $taskFinal.$taskFalse -cne $false){throw 'Final partial promoted'} }
    if($taskFinal.native_test_failed -cne $true -or $taskFinal.wrapper_exit_code -ne 1 -or $taskFinal.clock_restored -cne $true -or @($taskFinal.fixture_roots).Count -ne 0 -or (Raw 'candidate-live-final/clock-restored.log').Trim() -cne 'active'){throw 'Final failure/restoration drift'}
    $taskStable=Json 'candidate-live-final/clock-stable.log'
    if($taskStable.stable_seconds -ne 65 -or $taskStable.samples -lt 1 -or $taskStable.attempt -cne $false){throw 'Final clock stability missing'}
    for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
        $taskCompiler=if($taskIndex -eq 0){'194'}else{'188'}
        if((Json "gnu$taskCompiler-final/receipt.json").image -cne $taskFinal.images[$taskIndex]){throw 'Final image drift'}
    }
    $taskPostImages=Json 'final-image-check.json'
    if($taskPostImages.source -cne $Meta.source -or ($taskPostImages.images -join ',') -cne ($taskFinal.images -join ',') -or $taskPostImages.runtime_diff_exit_code -ne 0 -or $taskPostImages.runtime_dirty -cne $false -or $taskPostImages.recipe -cne $taskFinal.recipe){throw 'Post-failure source/image drift'}
    $taskRaw=Raw 'candidate-live-final/live194.log'
    if(-not $taskRaw.Contains('test result: FAILED. 0 passed; 1 failed;') -or -not $taskRaw.Contains('diagnostic_owned_placement_live_tests.rs:107:77') -or -not $taskRaw.Contains('Causal(Time)') -or (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'candidate-live-final/live188.log')) -or (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'candidate-live-final/receipt.json'))){throw 'Final failed cohort promoted'}
    $taskRows=[regex]::Matches($taskRaw,'HC074-OWNED-PLACEMENT-LIVE case=([^ ]+) accepted=(true|false) ([^\r\n]+) facts=(\{[^\r\n]+\})')
    if(($taskRows | ForEach-Object {$_.Groups[1].Value}) -join ',' -cne 'healthy,revalidate,unarmed'){throw 'Final completed cases drift'}
    foreach($taskRow in $taskRows){
        $taskCase=$taskRow.Groups[1].Value
        $taskExpected=if($taskCase -ceq 'unarmed'){'false'}else{'true'}
        if($taskRow.Groups[2].Value -cne $taskExpected){throw 'Partial live acceptance'}
        foreach($taskFact in @('seed=0x074e6a1d','fixed_manager=true','actual_root=true','original_bounds=true','eof=true','fence_held_while_owned=true','timer_live_before_drop=true','test_issuers=true','actual_systemd_placement=false','original_send=false','completion=false','product=false')){if(-not $taskRow.Groups[3].Value.Contains($taskFact)){throw 'Partial live safety fact'}}
        $taskArmed=if($taskCase -ceq 'unarmed'){'false'}else{'true'}
        $taskReplay=if($taskCase -ceq 'unarmed'){'true'}else{'false'}
        if(-not $taskRow.Groups[3].Value.Contains("fence_held_after_drop=$taskArmed") -or -not $taskRow.Groups[3].Value.Contains("retained_replay_refused=$taskReplay") -or $null -ne ($taskRow.Groups[4].Value | ConvertFrom-Json).first_refusal){throw 'Partial custody/timing proof drift'}
        $taskFacts=$taskRow.Groups[4].Value | ConvertFrom-Json
        if($taskCase -cne 'unarmed'){
            foreach($taskPoint in @($taskFacts.first,$taskFacts.last)){
                if($null -eq $taskPoint -or $taskPoint.issued_ns -le 0 -or $taskPoint.now_ns -lt $taskPoint.issued_ns -or $taskPoint.age_ns -ne ($taskPoint.now_ns-$taskPoint.issued_ns) -or $taskPoint.age_ns -ge 10000000000 -or $taskPoint.issued_boot -cne $taskPoint.now_boot -or $taskPoint.diagnostic_reason -cne 'within-window'){throw 'Completed call original time evidence drift'}
            }
        }elseif($taskFacts.observations -ne 0 -or $null -ne $taskFacts.first -or $null -ne $taskFacts.last){throw 'Invented unarmed causal observation'}
    }
    $taskInitial=Json 'initial-live-outcome.json'
    if($taskInitial.source -cne '08253cca073bfe56b5ed6b8d30cbcae8042b2937' -or $taskInitial.failed_case -cne 'unarmed' -or ($taskInitial.completed_cases -join ',') -cne 'healthy,revalidate' -or $taskInitial.native_test_failed -cne $true -or $taskInitial.native_test_exit_code_recorded -cne $false -or $taskInitial.wrapper_exit_code -ne 1 -or $taskInitial.msrv_attempted -cne $false -or $taskInitial.clock_restored -cne $true -or @($taskInitial.fixture_roots).Count -ne 0){throw 'Initial partial live result changed'}
    if(-not (Raw 'candidate-live/live194.log').Contains('test result: FAILED. 0 passed; 1 failed;') -or (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'candidate-live/live188.log')) -or (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'candidate-live/receipt.json'))){throw 'Partial initial promoted to full live gate'}
    if($taskInitial.recipe -cne (Hash (Join-Path $PSScriptRoot 'live.ps1')) -or (Raw 'candidate-live/clock-restored.log').Trim() -cne 'active'){throw 'Initial recipe/restoration drift'}
    for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
        $taskCompiler=if($taskIndex -eq 0){'194'}else{'188'}
        $taskInitialBuild=Json "gnu$taskCompiler/receipt.json"
        if($taskInitialBuild.source -cne $taskInitial.source -or $taskInitialBuild.image -cne $taskInitial.images[$taskIndex]){throw 'Initial source/image drift'}
    }
    if((Raw 'fixture-before.json') -cne (Raw 'fixture-after.json') -or @(Json 'fixture-before.json').Count -ne 9){throw 'Signed fixture inputs drift'}
    $taskCleanup=Json 'cleanup-final.json'
    if($taskCleanup.exit_code -ne 0 -or @($taskCleanup.roots).Count -ne 0 -or $taskCleanup.clock -cne 'active' -or $taskCleanup.server -cne $false -or $taskCleanup.product -cne $false){throw 'Final local restoration'}

}
$taskInventoryPath=Join-Path $PSScriptRoot 'inventory.json'
$taskMeta=Json 'metadata.json'
if($Seal){
    if(Test-Path -LiteralPath $taskInventoryPath){throw 'Never reseal retained evidence'}
    $taskInventory=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Sort-Object FullName | ForEach-Object {[ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}})
    Check $taskMeta $taskInventory
    $taskInventory | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $taskInventoryPath -Encoding utf8NoBOM
}
$taskInventory=@(Json 'inventory.json')
Check $taskMeta $taskInventory
if($SelfTest){
    $taskRejected=0
    foreach($taskField in @('source','baseline','original_files','source_contracts','effect_brackets','held_projection','original_send','barrier_release','completion','fence_release','server','product','preparation_wall_speedup_claimed','actual_systemd_placement','artifact_admission','new_peer_authenticated','local_live_gate_passed')){
        $taskChanged=$taskMeta | ConvertTo-Json -Depth 30 | ConvertFrom-Json
        $taskValue=$taskChanged.$taskField
        $taskChanged.$taskField=if($taskValue -is [bool]){-not $taskValue}elseif($taskValue -is [string]){"${taskValue}x"}else{$taskValue+1}
        $taskRefused=$false;try{Check $taskChanged $taskInventory}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted mutation: $taskField"};$taskRejected++
    }
    foreach($taskKind in @('missing','duplicate','hash')){
        $taskChanged=@($taskInventory | ConvertTo-Json -Depth 30 | ConvertFrom-Json)
        switch($taskKind){'missing'{$taskChanged=$taskChanged[1..($taskChanged.Count-1)]};'duplicate'{$taskChanged+=,$taskChanged[0]};'hash'{$taskChanged[0].sha256='0'*64}}
        $taskRefused=$false;try{Check $taskMeta $taskChanged}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted inventory mutation: $taskKind"};$taskRejected++
    }
    if($taskRejected -ne 20){throw 'Mutation selection drift'}
    Write-Output "owned-placement verifier mutations: $taskRejected refused"
}
Write-Output "owned-placement evidence: $($taskInventory.Count) raw files verified; runtime admission remains closed"
