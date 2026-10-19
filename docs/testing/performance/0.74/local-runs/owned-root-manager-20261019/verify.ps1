param([switch]$Seal,[switch]$SelfTest)
$ErrorActionPreference='Stop'
function Raw([string]$Path){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Path)}
function Json([string]$Path){Raw $Path | ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Check($Meta,$Inventory){
    if($Meta.schema -ne 1 -or $Meta.baseline_source -cne '28bbba4a' -or $Meta.join_source -cne 'e917ab3e005ca4468aef30bfbedca1881c9b0814' -or $Meta.creator_source -cne '894e4df842da9a43a350259413f5dc654a0ee4d1'){throw 'Source identity'}
    foreach($taskClosed in @('production_enrollment','original_send','production_all_writer_cessation','completion','unlock','server','supervisor','product','qualification','speedup_claimed')){if($Meta.$taskClosed -cne $false){throw "Boundary not closed: $taskClosed"}}
    if($Meta.test_issuers -cne $true -or $Meta.actual_local_root -cne $true -or $Meta.combined_fixture_passed -cne $true -or $Meta.creator_sigkill_passed -cne $false -or $Meta.creator_gnu_matrix_passed -cne $true -or $Meta.creator_msrv_matrix_passed -cne $false -or $Meta.selected_creator_sigkill_cases -ne 9 -or $Meta.msrv_closing_frame_refused_before_fault -cne $true -or $Meta.source_contracts -ne 300 -or $Meta.original_effect_brackets -ne 46 -or ($Meta.original_offsets_ns -join ',') -cne '300000000000,60000000000,10000000000'){throw 'Scope or preservation'}
    if($Meta.manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash (Join-Path $PSScriptRoot '../../qualification-manifest.toml')) -cne $Meta.manifest_sha256){throw 'Frozen manifest'}
    $taskFiles=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse | Where-Object Name -ne 'inventory.json')
    if($Inventory.Count -ne $taskFiles.Count -or @($Inventory.path | Sort-Object -Unique).Count -ne $Inventory.Count){throw 'Missing or duplicate inventory'}
    foreach($taskFile in $taskFiles){
        $taskPath=[IO.Path]::GetRelativePath($PSScriptRoot,$taskFile.FullName).Replace('\','/')
        $taskEntry=@($Inventory | Where-Object path -CEQ $taskPath)
        if($taskEntry.Count -ne 1 -or $taskEntry[0].sha256 -cne (Hash $taskFile.FullName)){throw "Raw drift: $taskPath"}
    }
    foreach($taskTest in @(@('source-final-full.log',300),@('source-final-screen.log',3),@('windows-final-unit.log',54),@('windows-final-doc.log',18))){
        if(-not (Raw $taskTest[0]).Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Missing nonzero test receipt'}
    }
    if(-not (Raw 'registry-final.log').Contains('gated-test-check: OK') -or -not (Raw 'local-performance.log').Contains('OK (local, non-promotable)')){throw 'Structural checks'}
    if(-not (Raw 'first-debug-screen.log').Contains('PermissionDenied')){throw 'Preliminary negative lost'}
    if((Raw 'fixture-before.json') -cne (Raw 'fixture-after.json') -or @(Json 'fixture-before.json').Count -ne 9){throw 'Signed fixture changed'}
    foreach($taskCompiler in @('194','188')){
        foreach($taskStage in @('confirmed','creator-final')){
            $taskBuild=Json "build-$taskStage-gnu$taskCompiler/receipt.json"
            $taskSource=if($taskStage -ceq 'confirmed'){$Meta.join_source}else{$Meta.creator_source}
            $taskVersion=if($taskCompiler -ceq '194'){'1.94.0'}else{'1.88.0'}
            if($taskBuild.source -cne $taskSource -or $taskBuild.compiler -cne $taskVersion -or $taskBuild.profile -cne $false -or $taskBuild.server -cne $false -or $taskBuild.product -cne $false -or $taskBuild.owned_units -ne 2 -or $taskBuild.origin_units -ne 18 -or $taskBuild.recipe -cne (Hash (Join-Path $PSScriptRoot 'build.ps1'))){throw 'Wrong ordinary build'}
            $taskRootCount=if($taskStage -ceq 'confirmed'){8}else{9}
            foreach($taskTest in @(@('owned-units',2),@('origin-units',18),@('root-units',$taskRootCount))){
                if(-not (Raw "build-$taskStage-gnu$taskCompiler/$($taskTest[0]).log").Contains("test result: ok. $($taskTest[1]) passed; 0 failed;")){throw 'Wrong Linux units'}
            }
            if(-not (Raw "build-$taskStage-gnu$taskCompiler/clippy.log").Contains('Finished `dev` profile')){throw 'Strict lint missing'}
        }
        $taskRun=Json "matrix-gnu$taskCompiler/receipt.json"
        if($taskRun.source -cne $Meta.join_source -or $taskRun.image -cne (Json "build-confirmed-gnu$taskCompiler/receipt.json").image -or $taskRun.matrix -cne $true -or $taskRun.repeats -ne 1 -or $taskRun.recipe -cne (Hash (Join-Path $PSScriptRoot 'run.ps1')) -or $taskRun.clock_before -cne 'active' -or $taskRun.clock_after -cne 'active' -or (Raw "matrix-gnu$taskCompiler/clock-restored.log").Trim() -cne 'active'){throw 'Combined identity/restoration'}
        foreach($taskKey in @('original_send','completion','server','supervisor','product')){if($taskRun.$taskKey -cne $false){throw 'Combined boundary'}}
        $taskCases=@([regex]::Matches((Raw "matrix-gnu$taskCompiler/frame-1.log"),'HC074-OWNED-ROOT case=([a-z-]+) ') | ForEach-Object {$_.Groups[1].Value})
        if((($taskCases | Sort-Object) -join ',') -cne 'duplicate,expiry,guardian-dead,guardian-stopped,outside-alias,retire,trust-drift'){throw 'Combined cases'}
        if(-not (Raw "matrix-gnu$taskCompiler/frame-1.log").Contains('test result: ok. 1 passed; 0 failed;')){throw 'Combined failed'}
    }
    if(Test-Path -LiteralPath (Join-Path $PSScriptRoot 'creator-final/receipt.json')){throw 'Negative cohort cannot acquire a positive run receipt'}
    $taskCreator=Json 'creator-outcome.json'
    if($taskCreator.source -cne $Meta.creator_source -or $taskCreator.status -cne 'msrv-closing-frame-refused-before-fault' -or $taskCreator.creator_sigkill_cases -ne 9 -or $taskCreator.complete_matrix_passed -cne $false -or $taskCreator.recipe -cne (Hash (Join-Path $PSScriptRoot 'creator-run.ps1')) -or $taskCreator.clock_after -cne 'active' -or (Raw 'creator-final/clock-restored.log').Trim() -cne 'active'){throw 'Creator identity/restoration'}
    foreach($taskKey in @('original_send','completion','server','supervisor','product')){if($taskCreator.$taskKey -cne $false){throw 'Creator boundary'}}
    if(($taskCreator.images -join ',') -cne (@((Json 'build-creator-final-gnu194/receipt.json').image,(Json 'build-creator-final-gnu188/receipt.json').image) -join ',')){throw 'Creator images'}
    foreach($taskCohort in @('screen-gnu194','matrix-gnu194','matrix-gnu188')){
        $taskRaw=Raw "creator-final/$taskCohort.log"
        $taskCases=if($taskCohort.StartsWith('screen')){@('owner')}elseif($taskCohort -ceq 'matrix-gnu188'){@('owner','first-dead')}else{@('owner','first-dead','second-dead','first-stopped','second-stopped','outside-alias')}
        if(([regex]::Matches($taskRaw,'HC074-OWNED-CREATOR case=')).Count -ne $taskCases.Count){throw 'Creator cases'}
        if($taskCohort -ceq 'matrix-gnu188'){
            if($taskRaw.Contains('completed=true') -or -not $taskRaw.Contains('Challenge(Time)') -or -not $taskRaw.Contains('test result: FAILED. 0 passed; 1 failed;')){throw 'MSRV negative relabeled'}
            if(Test-Path -LiteralPath (Join-Path $PSScriptRoot 'creator-final/matrix-gnu188/second-dead/ready.json')){throw 'Refused MSRV case cannot be ready'}
            $taskFailed=Raw 'creator-final/matrix-gnu188/second-dead/creator.log'
            if(-not $taskFailed.Contains('age_ns":9724276622') -or -not $taskFailed.Contains('Challenge(Time)') -or $taskFailed.Contains('panic in a destructor')){throw 'MSRV refusal/cleanup mismatch'}
            foreach($taskMissing in @('first-stopped','second-stopped','outside-alias')){if(Test-Path -LiteralPath (Join-Path $PSScriptRoot "creator-final/matrix-gnu188/$taskMissing")){throw 'Unexecuted case fabricated'}}
        }elseif(-not $taskRaw.Contains("completed=true cases=$($taskCases.Count)")){throw 'Creator completion missing'}
        foreach($taskCase in $taskCases){
            if(-not $taskRaw.Contains("case=$taskCase seed=0x074c2025 creator_sigkill=true writers_sigkill=3 before_original_deadline=true sinks_shed=true drained_eof=true fence_held=true test_issuers=true original_send=false completion=false product=false")){throw 'Creator proof missing'}
            $taskPreparing=Json "creator-final/$taskCohort/$taskCase/preparing.json"
            $taskReady=Json "creator-final/$taskCohort/$taskCase/ready.json"
            if($taskPreparing.phase -cne 'preparing' -or $taskReady.phase -cne 'ready' -or $taskReady.schema -cne 'hc074-owned-root-creator-v1' -or $taskReady.root -cne $taskPreparing.root -or $taskReady.fence -cne $taskPreparing.fence -or $taskReady.bounds.Count -ne 4 -or $taskReady.origins.Count -ne 4 -or $taskReady.bounds[3] -ne ($taskReady.bounds[0..2] | Measure-Object -Minimum).Minimum){throw 'Original capsule identity'}
            $taskFacts=[regex]::Match((Raw "creator-final/$taskCohort/$taskCase/creator.log"),'HC074-OWNED-ENROLL success=true facts=(\{[^\r\n]+\})')
            if(-not $taskFacts.Success){throw 'Missing original clock observations'}
            $taskReport=$taskFacts.Groups[1].Value | ConvertFrom-Json
            if($taskReport.observations -le 0 -or $null -ne $taskReport.first_refusal -or $taskReport.last.diagnostic_reason -cne 'within-window' -or $taskReport.last.issued_boot -cne $taskReport.last.now_boot){throw 'Enrollment observation refused'}
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
    foreach($taskFault in @('source','server','product','scope','send','completion','deadline','contracts','hash','missing','promote','msrv','cases')){
        $taskCopy=$taskMeta | ConvertTo-Json -Depth 10 | ConvertFrom-Json
        $taskEntries=@($taskInventory | ConvertTo-Json -Depth 10 | ConvertFrom-Json)
        switch($taskFault){
            source {$taskCopy.creator_source='00000000'}
            server {$taskCopy.server=$true}
            product {$taskCopy.product=$true}
            scope {$taskCopy.production_all_writer_cessation=$true}
            send {$taskCopy.original_send=$true}
            completion {$taskCopy.completion=$true}
            deadline {$taskCopy.original_offsets_ns[2]=11}
            contracts {$taskCopy.source_contracts=0}
            hash {$taskEntries[0].sha256='0'*64}
            missing {$taskEntries=@($taskEntries | Select-Object -Skip 1)}
            promote {$taskCopy.creator_sigkill_passed=$true}
            msrv {$taskCopy.creator_msrv_matrix_passed=$true}
            cases {$taskCopy.selected_creator_sigkill_cases=13}
        }
        $taskRejected=$false
        try{Check $taskCopy $taskEntries}catch{$taskRejected=$true}
        if(-not $taskRejected){throw "Mutation accepted: $taskFault"}
    }
}
if($Seal){ConvertTo-Json -InputObject $taskInventory -Depth 10 | Set-Content -LiteralPath (Join-Path $PSScriptRoot 'inventory.json') -Encoding utf8NoBOM}
"Owned root packet verified; files=$($taskInventory.Count); GNU=passed; MSRV=incomplete; production=false; mutation_controls=13"
