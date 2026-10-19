param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Json([string]$Path){Raw $Path | ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.source -cne '659855a3c83f5185cd290ab26fdd43122e7cb396' -or $Meta.baseline -cne 'eefcd449' -or $Meta.source_contracts -ne 303 -or $Meta.original_files -ne 418){throw 'Source/check identity'}
    if($Meta.missing_four_msrv_cases_passed -cne $true -or $Meta.fresh_full_msrv_gate_passed -cne $false -or $Meta.remaining_actual_sigkill_cases -ne 5){throw 'Partial MSRV outcome promoted'}
    foreach($taskClosed in @('historical_failing_tuple_recovered','product_speedup_claimed','production_enrollment','original_send','completion','unlock','server','supervisor','product','qualification','new_cross_compiler_matrix_claimed')){if($Meta.$taskClosed -cne $false){throw "Boundary changed: $taskClosed"}}
    if($Meta.original_effect_brackets -ne 46 -or ($Meta.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Original schedule or bounds'}
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -cne $Meta.manifest_sha256){throw 'Qualification manifest drift'}
    if((Hash (Join-Path $PSScriptRoot '../owned-root-manager-20261019/inventory.json')) -cne 'a328705a7165f178dbf068a764f107585324d2bcec0630b41dc8ed658c5e9a7f'){throw 'Historical packet changed'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object Name -ne 'inventory.json')
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing/duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    foreach($taskTest in @(@('source-full.log',303),@('source-screen.log',3),@('windows-unit.log',54),@('windows-doc.log',18))){if(-not (Raw $taskTest[0]).Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Missing test receipt'}}
    if(-not (Raw 'registry.log').Contains('gated-test-check: OK') -or -not (Raw 'docs.log').Contains('doc-check: OK') -or -not (Raw 'local-performance.log').Contains('OK (local, non-promotable)') -or (Raw 'fmt-result.log').Trim() -cne 'Formatting passed'){throw 'Local checks'}
    if((Raw 'fixture-before.json') -cne (Raw 'fixture-after.json') -or @(Json 'fixture-before.json').Count -ne 9){throw 'Signed fixture data drift'}
    $taskImages=@()
    foreach($taskCompiler in @('194','188')){
        $taskDir="gnu$taskCompiler-confirmed"
        $taskBuild=Json "$taskDir/receipt.json"
        $taskVersion=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
        if($taskBuild.source -cne $Meta.source -or $taskBuild.compiler -cne $taskVersion -or $taskBuild.profile -cne $false -or $taskBuild.server -cne $false -or $taskBuild.product -cne $false -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'build.ps1'))){throw 'Build identity'}
        foreach($taskTest in @(@('owned-units',2),@('origin-units',18),@('closing-units',4),@('root-units',9))){if(-not (Raw "$taskDir/$($taskTest[0]).log").Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Linux unit selection'}}
        if(-not (Raw "$taskDir/clippy.log").Contains('Finished `dev` profile')){throw 'Strict lint missing'}
        $taskImages+= $taskBuild.image
    }
    foreach($taskStage in @('cost','remaining')){
        if($taskStage -ceq 'remaining'){
            if(Test-Path -LiteralPath (Join-Path $PSScriptRoot 'remaining/receipt.json')){throw 'Incomplete cohort cannot have a positive receipt'}
            $taskRun=Json 'remaining/outcome.json'
            if($taskRun.analysis_not_positive_run_receipt -cne $true -or $taskRun.missing_four_cases_passed -cne $true -or $taskRun.actual_creator_sigkill_cases -ne 5){throw 'Negative cohort scope'}
        }else{$taskRun=Json "$taskStage/receipt.json"}
        if($taskRun.source -cne $Meta.source -or $taskRun.stage -cne $taskStage -or $taskRun.recipe -cne (Hash (Join-Path $PSScriptRoot 'run.ps1')) -or ($taskRun.images -join ',') -cne ($taskImages -join ',') -or $taskRun.clock_before -cne 'active' -or $taskRun.clock_after -cne 'active' -or (Raw "$taskStage/clock-restored.log").Trim() -cne 'active' -or $taskRun.rows.Count -ne 6){throw 'Run identity/restoration'}
        foreach($taskClosed in @('original_send','completion','server','supervisor','product')){if($taskRun.$taskClosed -cne $false){throw 'Run boundary'}}
        $taskAnalysis=Json "$taskStage-analysis.json"
        if($taskAnalysis.source -cne $Meta.source -or $taskAnalysis.stage -cne $taskStage -or $taskAnalysis.rows.Count -ne 6 -or $taskAnalysis.historical_failing_tuple_recovered -cne $false -or $taskAnalysis.product_speedup_claimed -cne $false -or $taskAnalysis.production_admission -cne $false){throw 'Analysis scope'}
        if($taskStage -ceq 'cost'){
            if($taskRun.functional_gate_passed -cne $false -or (($taskRun.rows.label | Sort-Object) -join ',') -cne 'cost-gnu188-0,cost-gnu188-1,cost-gnu188-2,cost-gnu194-0,cost-gnu194-1,cost-gnu194-2'){throw 'Cost repetitions/promotion'}
        }else{
            if($taskRun.functional_gate_passed -cne $false -or (($taskRun.rows.case | Sort-Object) -join ',') -cne 'first-dead,first-stopped,outside-alias,owner,second-dead,second-stopped'){throw 'Missing MSRV cases or false promotion'}
        }
        for($taskIndex=0;$taskIndex -lt 6;$taskIndex++){
            $taskRow=$taskRun.rows[$taskIndex]
            $taskReported=$taskAnalysis.rows[$taskIndex]
            if($taskRow.label -cne $taskReported.label -or $taskRow.exit_code -ne $taskReported.exit_code){throw 'Analysis row identity'}
            $taskRoot="$taskStage/$($taskRow.label)/$($taskRow.case)"
            $taskRaw=Raw "$taskRoot/creator.log"
            $taskFacts=[regex]::Matches($taskRaw,'HC074-CLOSING (\{[^\r\n]+\})')
            if($taskFacts.Count -eq 0){
                if($taskRow.exit_code -eq 0 -or $taskReported.closing_not_reached -cne $true){throw 'Missing closing report'}
                if($taskStage -ceq 'remaining'){
                    if($taskRow.case -cne 'first-dead' -or (Test-Path -LiteralPath (Join-Path $PSScriptRoot "$taskRoot/ready.json")) -or $taskRaw -notmatch 'Causal\(Time\)' -or $taskRaw.Contains('panic in a destructor')){throw 'Preclosing refusal relabeled'}
                    $taskEnrolled=[regex]::Match($taskRaw,'HC074-OWNED-ENROLL success=false facts=(\{[^\r\n]+\})')
                    if(-not $taskEnrolled.Success){throw 'Missing original enrollment observations'}
                    $taskEnrollment=$taskEnrolled.Groups[1].Value | ConvertFrom-Json
                    if(($taskEnrollment | ConvertTo-Json -Depth 10 -Compress) -cne ($taskReported.enrollment_report | ConvertTo-Json -Depth 10 -Compress) -or $taskEnrollment.first_refusal.age_ns -ne 10024697034 -or $taskEnrollment.first_refusal.diagnostic_reason -cne 'expired' -or $taskEnrollment.first_refusal.issued_boot -cne $taskEnrollment.first_refusal.now_boot){throw 'Original enrollment refusal changed'}
                }
                continue
            }
            if($taskFacts.Count -ne 1){throw 'Closing call frequency'}
            $taskReport=$taskFacts[0].Groups[1].Value | ConvertFrom-Json
            if(($taskReport | ConvertTo-Json -Depth 30 -Compress) -cne ($taskReported.report | ConvertTo-Json -Depth 30 -Compress)){throw 'Analysis facts changed'}
            $taskMode=if($taskStage -ceq 'cost'){'cost'}else{'facts'}
            if($taskReport.mode -cne $taskMode -or $taskReport.schedule_changed -cne $false -or $taskReport.deadline_changed -cne $false -or $taskReport.product -cne $false){throw 'Report scope'}
            if($taskStage -ceq 'cost'){
                if($taskReport.cost.wall_not_cpu -cne $true -or $taskReport.cost.overlapping_spans -cne $true -or $taskReport.cost.live.Count -ne 6 -or $taskReport.cost.kernel.Count -ne 10){throw 'Cost units'}
            }elseif($null -ne $taskReport.cost){throw 'Functional cohort profiled'}
            if($taskRow.exit_code -eq 0){
                $taskSuccess=Raw "$taskStage/$($taskRow.label).log"
                if(-not $taskSuccess.Contains("case=$($taskRow.case) seed=0x074c2025 creator_sigkill=true writers_sigkill=3 before_original_deadline=true sinks_shed=true drained_eof=true fence_held=true test_issuers=true original_send=false completion=false product=false") -or -not $taskSuccess.Contains('completed=true cases=1')){throw 'Actual fault proof missing'}
                $taskReady=Json "$taskRoot/ready.json"
                $taskCustody=$taskReport.age.sites[0]
                if($taskReport.success -cne $true -or $taskCustody.calls -le 0 -or $null -ne $taskCustody.first_refusal -or $taskCustody.last.reason -cne 'within-window' -or $taskCustody.last.issued_ns -ne $taskReady.origins[2] -or $taskCustody.last.age_ns -ge 10000000000){throw 'Original custody deadline/binding'}
                if(-not $taskCustody.last.file.EndsWith('diagnostic_original_custody_join.rs') -or $taskCustody.last.line -ne 121){throw 'Comparison site changed'}
            }elseif($taskStage -cne 'cost' -or $taskReport.success -cne $false -or $taskRaw -notmatch 'Time') {throw 'Negative incorrectly promoted'}
        }
    }
}
$taskMeta=Json 'metadata.json'
$taskInventory=if($Seal){
    if(Test-Path -LiteralPath (Join-Path $PSScriptRoot 'inventory.json')){throw 'Already sealed'}
    @(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Sort-Object FullName | ForEach-Object {[pscustomobject]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}})
}else{@(Json 'inventory.json')}
Check $taskMeta $taskInventory
if($SelfTest){
    foreach($taskFault in @('source','server','product','qualification','send','completion','historical','speedup','bounds','schedule','contracts','cross-source','hash','missing','promote-msrv','cases')){
        $taskCopy=$taskMeta | ConvertTo-Json -Depth 10 | ConvertFrom-Json
        $taskEntries=@($taskInventory | ConvertTo-Json -Depth 10 | ConvertFrom-Json)
        switch($taskFault){
            source {$taskCopy.source='00000000'}
            server {$taskCopy.server=$true}
            product {$taskCopy.product=$true}
            qualification {$taskCopy.qualification=$true}
            send {$taskCopy.original_send=$true}
            completion {$taskCopy.completion=$true}
            historical {$taskCopy.historical_failing_tuple_recovered=$true}
            speedup {$taskCopy.product_speedup_claimed=$true}
            bounds {$taskCopy.original_offsets_ns[2]=11000000000}
            schedule {$taskCopy.original_effect_brackets=45}
            contracts {$taskCopy.source_contracts=0}
            cross-source {$taskCopy.new_cross_compiler_matrix_claimed=$true}
            hash {$taskEntries[0].sha256='0'*64}
            missing {$taskEntries=@($taskEntries | Select-Object -Skip 1)}
            promote-msrv {$taskCopy.fresh_full_msrv_gate_passed=$true}
            cases {$taskCopy.remaining_actual_sigkill_cases=6}
        }
        $taskRejected=$false
        try{Check $taskCopy $taskEntries}catch{$taskRejected=$true}
        if(-not $taskRejected){throw "Mutation accepted: $taskFault"}
    }
}
if($Seal){ConvertTo-Json -InputObject $taskInventory -Depth 10 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') -Encoding utf8NoBOM}
"Closing packet verified; files=$($taskInventory.Count); missing MSRV cases=4 passed; fresh gate=partial; production=false; mutation_controls=16"
