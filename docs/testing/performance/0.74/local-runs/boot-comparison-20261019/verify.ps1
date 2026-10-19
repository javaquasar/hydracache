param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Json([string]$Path){Raw $Path | ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.baseline -cne 'aa84aeb8' -or $Meta.source_contracts -ne 306 -or $Meta.original_files -ne 420 -or $Meta.original_effect_brackets -ne 46 -or ($Meta.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Original source/check schedule'}
    foreach($taskClosed in @('product_speedup_claimed','preparation_wall_speedup_claimed','production_enrollment','original_send','completion','unlock','server','supervisor','product','qualification','cross_compiler_functional_matrix_claimed')){if($Meta.$taskClosed -cne $false){throw "Boundary changed: $taskClosed"}}
    if($Meta.source -cne '22b7a297a49665d5b7f85b0253dd982c2ab9f5c7' -or $Meta.allocation_reduction_confirmed -cne $true){throw 'Source or allocation evidence'}
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -cne $Meta.manifest_sha256){throw 'Qualification manifest drift'}
    if((Hash (Join-Path $PSScriptRoot '../closing-validation-20261019/inventory.json')) -cne '7b1c508b1cdd83118765e3400872c0a0e8378d1e0cd7bbb05b22c65da1e9b23e'){throw 'Sealed attribution changed'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object FullName -CNE (Join-Path $PSScriptRoot 'inventory.json'))
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing/duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    foreach($taskTest in @(@('source-full-final.log',306),@('source-screen-final.log',3),@('windows-unit-final.log',54),@('windows-doc-final.log',18),@('counter194.log',2),@('counter188.log',2))){if(-not (Raw $taskTest[0]).Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Missing test receipt'}}
    if(-not (Raw 'registry.log').Contains('gated-test-check: OK') -or -not (Raw 'docs-post-results.log').Contains('doc-check: OK') -or -not (Raw 'local-performance-post-results.log').Contains('OK (local, non-promotable)') -or (Raw 'fmt-result.log').Trim() -cne 'Formatting passed'){throw 'Local checks'}
    if(-not (Raw 'harness-self-test.log').Contains('capsule mutation controls: 8 passed') -or -not (Raw 'red-before-implementation.log').Contains('E0433') -or -not (Raw 'red-before-implementation.log').Contains('boot_comparison')){throw 'Harness or red-test evidence'}
    if((Raw 'fixture-before.json') -cne (Raw 'fixture-after.json') -or @(Json 'fixture-before.json').Count -ne 9){throw 'Signed fixture data drift'}
    foreach($taskLabel in @('baseline-194','candidate-194-final','candidate-188-final')){
        $taskReceipt=Json "$taskLabel/receipt.json"
        $taskSource=if($taskLabel -ceq 'baseline-194'){'d54c25261c9bcc9c4035d7ea951ba984e4224ebe'}else{$Meta.source}
        $taskVersion=if($taskLabel -ceq 'candidate-188-final'){'1.88.0'}else{'1.94.0'}
        if($taskReceipt.compiler -cne $taskVersion -or $taskReceipt.samples -ne 3 -or $taskReceipt.iterations -ne 1024){throw 'Allocation compiler/workload'}
        if($taskReceipt.source -cne $taskSource -or $taskReceipt.profile -cne $true -or $taskReceipt.functional_gate -cne $false -or $taskReceipt.server -cne $false -or $taskReceipt.product -cne $false -or $taskReceipt.recipe -cne (Hash (Join-Path $PSScriptRoot 'allocations.ps1'))){throw 'Allocation identity'}
        $taskRows=[regex]::Matches((Raw "$taskLabel/allocations.log"),'HC074-BOOT-COMPARE-ALLOC (\{[^\r\n]+\})')
        if($taskRows.Count -ne 6){throw 'Allocation repetitions'}
        $taskSeen=@()
        foreach($taskMatch in $taskRows){
            $taskRow=$taskMatch.Groups[1].Value | ConvertFrom-Json
            if($taskRow.sample -notin @(0,1,2) -or $taskRow.operation -notin @('revalidate','sample') -or $taskRow.iterations -ne 1024 -or $taskRow.rust_requests_only -cne $true -or $taskRow.product -cne $false){throw 'Allocation workload'}
            $taskSeen+="$($taskRow.sample):$($taskRow.operation)"
            $taskBefore=if($taskRow.operation -ceq 'revalidate'){2048}else{5120}
            $taskAfter=if($taskLabel -ceq 'baseline-194'){$taskBefore}elseif($taskRow.operation -ceq 'revalidate'){0}else{1024}
            if($taskRow.reference.allocations -ne $taskBefore -or $taskRow.reference.requested_bytes -ne 36*$taskBefore -or $taskRow.candidate.allocations -ne $taskAfter -or $taskRow.candidate.requested_bytes -ne 36*$taskAfter -or $taskRow.reference.reallocations -ne 0 -or $taskRow.candidate.reallocations -ne 0){throw 'Allocation reduction not reproduced'}
        }
        if(@($taskSeen | Sort-Object -Unique).Count -ne 6){throw 'Duplicate allocation samples'}
    }
    $taskImages=@()
    foreach($taskCompiler in @('194','188')){
        $taskDir="gnu$taskCompiler-final"
        $taskBuild=Json "$taskDir/receipt.json"
        $taskVersion=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
        if($taskBuild.source -cne $Meta.source -or $taskBuild.compiler -cne $taskVersion -or $taskBuild.profile -cne $false -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'build.ps1'))){throw 'Build identity'}
        if($taskBuild.server -cne $false -or $taskBuild.product -cne $false){throw 'Build scope'}
        foreach($taskTest in @(@('owned-units',2),@('origin-units',22),@('closing-units',4),@('root-units',9))){if(-not (Raw "$taskDir/$($taskTest[0]).log").Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Linux units'}}
        if(-not (Raw "$taskDir/clippy.log").Contains('Finished `dev` profile')){throw 'Strict lint missing'}
        $taskImages+=$taskBuild.image
    }
    $taskPositive=Test-Path -LiteralPath (Join-Path $PSScriptRoot 'msrv-gate/receipt.json')
    $taskRun=if($taskPositive){Json 'msrv-gate/receipt.json'}else{Json 'msrv-gate/outcome.json'}
    if($taskRun.source -cne $Meta.source -or $taskRun.stage -cne 'remaining' -or $taskRun.recipe -cne (Hash (Join-Path $PSScriptRoot 'run.ps1')) -or ($taskRun.images -join ',') -cne ($taskImages -join ',') -or $taskRun.clock_before -cne 'active' -or $taskRun.clock_after -cne 'active' -or (Raw 'msrv-gate/clock-restored.log').Trim() -cne 'active'){throw 'Functional source/image/restoration'}
    if($taskRun.original_send -cne $false -or $taskRun.completion -cne $false -or $taskRun.server -cne $false -or $taskRun.supervisor -cne $false -or $taskRun.product -cne $false){throw 'Functional scope'}
    $taskCleanup=Json 'fixture-cleanup.json'
    if($taskCleanup.exit_code -ne 0 -or @($taskCleanup.roots).Count -ne 0 -or ($taskCleanup.command -join ' ') -cne 'find /sys/fs/cgroup -maxdepth 1 -name hc074-owned-root-* -print' -or $taskCleanup.server -cne $false -or $taskCleanup.supervisor -cne $false -or $taskCleanup.product -cne $false -or (Raw 'clock-final.log').Trim() -cne 'active'){throw 'Final local restoration'}
    $taskProbe=Json 'msrv-gate/clock-probe.log'
    if($taskProbe.backward_samples -ne 0 -or $taskProbe.samples -le 0){throw 'Clock probe'}
    $taskAnalysis=Json 'msrv-gate-analysis.json'
    if($taskAnalysis.source -cne $Meta.source -or $taskAnalysis.stage -cne 'remaining' -or $taskAnalysis.rows.Count -ne $taskRun.rows.Count -or $taskAnalysis.historical_failing_tuple_recovered -cne $false -or $taskAnalysis.product_speedup_claimed -cne $false -or $taskAnalysis.production_admission -cne $false){throw 'Analysis scope'}
    $taskCases=@('second-dead','first-stopped','second-stopped','outside-alias','owner','first-dead')
    if($taskRun.rows.Count -lt 1 -or $taskRun.rows.Count -gt 6){throw 'Cohort size'}
    $taskPassed=0
    for($taskIndex=0;$taskIndex -lt $taskRun.rows.Count;$taskIndex++){
        $taskRow=$taskRun.rows[$taskIndex]
        if($taskRow.case -cne $taskCases[$taskIndex] -or $taskRow.label -cne $taskRow.case -or $taskRow.diagnostic_mode -cne 'facts'){throw 'Fixed fault order/mode'}
        $taskRoot="msrv-gate/$($taskRow.label)/$($taskRow.case)"
        $taskRaw=Raw "$taskRoot/creator.log"
        $taskFacts=[regex]::Matches($taskRaw,'HC074-CLOSING (\{[^\r\n]+\})')
        if($taskRow.exit_code -eq 0){
            $taskSuccess=Raw "msrv-gate/$($taskRow.label).log"
            if(-not $taskSuccess.Contains("case=$($taskRow.case) seed=0x074c2025 creator_sigkill=true writers_sigkill=3 before_original_deadline=true sinks_shed=true drained_eof=true fence_held=true test_issuers=true original_send=false completion=false product=false") -or -not $taskSuccess.Contains('completed=true cases=1') -or $taskFacts.Count -ne 1 -or $taskRow.actual_creator_sigkill -cne $true){throw 'Actual fault proof missing'}
            $taskReport=$taskFacts[0].Groups[1].Value | ConvertFrom-Json
            $taskAnalyzed=$taskAnalysis.rows[$taskIndex]
            if($taskAnalyzed.case -cne $taskRow.case -or $taskAnalyzed.exit_code -ne $taskRow.exit_code -or ($taskAnalyzed.report | ConvertTo-Json -Depth 30 -Compress) -cne ($taskReport | ConvertTo-Json -Depth 30 -Compress)){throw 'Analysis differs from actual comparison arguments'}
            $taskReady=Json "$taskRoot/ready.json"
            $taskCustody=$taskReport.age.sites[0]
            if($taskReport.mode -cne 'facts' -or $taskReport.success -cne $true -or $null -ne $taskReport.cost -or $taskReport.schedule_changed -cne $false -or $taskReport.deadline_changed -cne $false -or $taskCustody.last.issued_ns -ne $taskReady.origins[2] -or $taskCustody.last.age_ns -ge 10000000000 -or $null -ne $taskCustody.first_refusal){throw 'Original successful custody frame'}
            if($taskCustody.calls -ne 15 -or $taskCustody.last.reason -cne 'within-window' -or $taskCustody.last.age_ns -ne ($taskCustody.last.now_ns - $taskCustody.last.issued_ns) -or -not $taskCustody.last.file.EndsWith('diagnostic_original_custody_join.rs') -or $taskCustody.last.line -ne 121 -or $taskReport.causal.last.issued_boot -cne $taskProbe.boot -or $taskReport.causal.last.now_boot -cne $taskProbe.boot){throw 'Original observation frequency/site/boot'}
            if($taskReady.bounds[0]-$taskReady.origins[0] -ne 300000000000 -or $taskReady.bounds[1]-$taskReady.origins[3] -ne 60000000000 -or $taskReady.bounds[2]-$taskReady.origins[1] -ne 10000000000 -or $taskReady.bounds[3] -ne ($taskReady.bounds | Measure-Object -Minimum).Minimum){throw 'Original expiry origins'}
            $taskPassed++
        }else{
            if($taskPositive -or $taskIndex -ne $taskRun.rows.Count-1 -or $taskRow.actual_creator_sigkill -cne $false -or $taskRaw -notmatch 'Time' -or $taskRaw -match 'panic in a destructor' -or (Test-Path -LiteralPath (Join-Path $PSScriptRoot "$taskRoot/ready.json"))){throw 'Negative incorrectly promoted'}
        }
    }
    $taskFull=$taskPassed -eq 6
    if($Meta.actual_creator_sigkill_cases -ne $taskPassed -or $Meta.fresh_full_msrv_gate_passed -cne $taskFull -or $taskRun.functional_gate_passed -cne $taskFull -or $taskPositive -ne $taskFull){throw 'Functional gate promoted'}
    $taskUnexecuted=@($taskCases | Select-Object -Skip $taskRun.rows.Count)
    if(($taskRun.unexecuted_cases -join ',') -cne ($taskUnexecuted -join ',')){throw 'Unexecuted cases lost'}
}
$taskMeta=Json 'metadata.json'
$taskInventory=if($Seal){
    if(Test-Path -LiteralPath (Join-Path $PSScriptRoot 'inventory.json')){throw 'Already sealed'}
    @(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Sort-Object FullName | ForEach-Object {[pscustomobject]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}})
}else{@(Json 'inventory.json')}
Check $taskMeta $taskInventory
if($SelfTest){
    foreach($taskFault in @('source','server','product','qualification','send','completion','wall','allocation','bounds','schedule','contracts','cross-source','hash','missing','promote-msrv','cases')){
        $taskCopy=$taskMeta | ConvertTo-Json -Depth 10 | ConvertFrom-Json
        $taskEntries=@($taskInventory | ConvertTo-Json -Depth 10 | ConvertFrom-Json)
        switch($taskFault){
            source {$taskCopy.source='0'*40}
            server {$taskCopy.server=$true}
            product {$taskCopy.product=$true}
            qualification {$taskCopy.qualification=$true}
            send {$taskCopy.original_send=$true}
            completion {$taskCopy.completion=$true}
            wall {$taskCopy.preparation_wall_speedup_claimed=$true}
            allocation {$taskCopy.allocation_reduction_confirmed=$false}
            bounds {$taskCopy.original_offsets_ns[2]++}
            schedule {$taskCopy.original_effect_brackets++}
            contracts {$taskCopy.source_contracts--}
            cross-source {$taskCopy.cross_compiler_functional_matrix_claimed=$true}
            hash {$taskEntries[0].sha256='0'*64}
            missing {$taskEntries=@($taskEntries | Select-Object -Skip 1)}
            promote-msrv {$taskCopy.fresh_full_msrv_gate_passed=-not $taskCopy.fresh_full_msrv_gate_passed}
            cases {$taskCopy.actual_creator_sigkill_cases++}
        }
        $taskRejected=$false
        try{Check $taskCopy $taskEntries}catch{$taskRejected=$true}
        if(-not $taskRejected){throw "Mutation accepted: $taskFault"}
    }
    'Verifier mutation checks passed: 16'
}
if($Seal){$taskInventory | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') -Encoding utf8NoBOM}
"Boot comparison evidence verified; files=$($taskInventory.Count); full_msrv_gate=$($taskMeta.fresh_full_msrv_gate_passed); server=false; product=false"
