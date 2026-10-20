$ErrorActionPreference='Stop'
function PacketJson([string]$Name){Get-Content -Raw -LiteralPath "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Rows([string]$Name,[string]$Prefix){
    @(Get-Content -LiteralPath "$PSScriptRoot/$Name"|ForEach-Object {
        $taskAt=$_.IndexOf($Prefix,[StringComparison]::Ordinal)
        if($taskAt -ge 0){$_.Substring($taskAt+$Prefix.Length)|ConvertFrom-Json}
    })
}
function KernelFacts {
    $taskPhases=@();$taskAllocations=0;$taskBytes=@()
    foreach($taskCohort in 'Baseline','Candidate'){
        $taskSources=@()
        foreach($taskLane in '194','188'){
            foreach($taskFlavor in 'plain','alloc'){
                $taskImage=PacketJson "$taskCohort-$taskLane-$taskFlavor-image.json"
                if($taskImage.source -notmatch '^[0-9a-f]{40}$' -or $taskImage.sha256 -notmatch '^[0-9a-f]{64}$' -or $taskImage.recipe_sha256 -cne (Get-FileHash "$PSScriptRoot/run.ps1" -Algorithm SHA256).Hash.ToLowerInvariant() -or $taskImage.server -cne $false -or $taskImage.product -cne $false){throw 'Image binding'}
                $taskSources+=$taskImage.source
            }
            foreach($taskSelection in 'plain-build','alloc-build','attribution','runtime-binding','allocation','pid_path_signed_edges_and_seeded_values_match_complete_decimal','pid_path_open_differential_preserves_identity_flags_and_refusals'){
                $taskReceipt=PacketJson "$taskCohort-$taskLane-$taskSelection-exit.json"
                $taskExpected=if($taskCohort -ceq 'Baseline' -and $taskSelection -ceq 'runtime-binding'){101}else{0}
                if($taskReceipt.exit -ne $taskExpected -or $taskReceipt.source -cne $taskImage.source -or $taskReceipt.server -cne $false -or $taskReceipt.product -cne $false){throw 'Invocation binding'}
            }
            if($taskCohort -ceq 'Baseline'){
                if(-not (Get-Content -Raw "$PSScriptRoot/$taskCohort-$taskLane-runtime-binding.log").Contains('unimplemented PID candidate')){throw 'Missing red refusal'}
            }
            $taskRows=Rows "$taskCohort-$taskLane-attribution.log" 'HC074-FRESH-KERNEL '
            if($taskRows.Count -ne 18){throw 'Incomplete attribution'}
            foreach($taskPhase in 'origin','sender','pidfd','status-document','status-metadata','status-filesystem'){
                $taskSamples=@($taskRows|Where-Object{$_.phase -ceq $taskPhase})
                if(($taskSamples.sample -join ',') -cne '0,1,2'){throw 'Wrong sample order'}
                $taskExpectedCalls=switch($taskPhase){
                    'origin'{@(0,0,0,0,64,128,256,0,0,128)}
                    'sender'{@(0,64,128,128,128,256,512,0,0,512)}
                    'pidfd'{@(0,0,64,0,0,0,0,0,0,64)}
                    'status-document'{@(0,0,0,0,0,0,0,0,0,64)}
                    default{@(0,0,0,0,0,0,0,0,0,0)}
                }
                foreach($taskRow in $taskSamples){
                    if($taskRow.iterations -ne 64 -or $taskRow.outcome -cne 'ok' -or $null -ne $taskRow.first_refusal -or ($taskRow.nodes.calls -join ',') -cne ($taskExpectedCalls -join ',')){throw 'Fresh calls/refusal changed'}
                    foreach($taskFlag in 'guard_clock_added','deadline_changed','server','product'){
                        if($taskRow.$taskFlag -isnot [bool] -or $taskRow.$taskFlag -cne $false){throw 'Scope changed'}
                    }
                }
                $taskPhases+=[ordered]@{cohort=$taskCohort;lane=$taskLane;phase=$taskPhase;iterations=64;wall_ns=@($taskSamples.wall_ns);process_cpu_ns=@($taskSamples.process_cpu_ns);nested_origin_ns=@($taskSamples|ForEach-Object{$_.nodes[4].inclusive_ns});nested_boot_ns=@($taskSamples|ForEach-Object{$_.nodes[5].inclusive_ns});nested_namespace_ns=@($taskSamples|ForEach-Object{$_.nodes[6].inclusive_ns});nested_document_ns=@($taskSamples|ForEach-Object{$_.nodes[9].inclusive_ns})}
            }
            $taskRows=Rows "$taskCohort-$taskLane-allocation.log" 'HC074-PID-ALLOC '
            $taskTiming=Rows "$taskCohort-$taskLane-allocation.log" 'HC074-PID-TIME '
            if(($taskRows.sample -join ',') -cne '0,1,2' -or ($taskTiming.sample -join ',') -cne '0,1,2'){throw 'Incomplete paired control'}
            foreach($taskRow in $taskRows){
                if($taskRow.iterations -ne 1024 -or $taskRow.reference.allocations -ne 2048 -or $taskRow.reference.reallocations -ne 0 -or $taskRow.reference.requested_bytes -le 0 -or $taskRow.reference.requested_bytes%1024 -ne 0 -or $taskRow.pid -le 0 -or $taskRow.digits -ne "$($taskRow.pid)".Length -or $taskRow.pid -ne $taskRows[0].pid -or $taskRow.reference.requested_bytes -ne $taskRows[0].reference.requested_bytes -or $taskRow.candidate.allocations -ne 0 -or $taskRow.candidate.reallocations -ne 0 -or $taskRow.candidate.requested_bytes -ne 0 -or $taskRow.candidate_first -cne ($taskRow.sample%2 -eq 1) -or $taskRow.rust_requests_only -cne $true -or $taskRow.server -cne $false -or $taskRow.product -cne $false){throw 'Allocation control changed'}
                $taskAllocations++
            }
            $taskBytes+=[ordered]@{cohort=$taskCohort;lane=$taskLane;pid=$taskRows[0].pid;digits=$taskRows[0].digits;reference_requested_bytes_per_open=$taskRows[0].reference.requested_bytes/1024}
            foreach($taskRow in $taskTiming){
                if($taskRow.iterations -ne 4096 -or $taskRow.allocation_counter_active -cne $false -or $taskRow.candidate_first -cne ($taskRow.sample%2 -eq 1) -or $taskRow.server -cne $false -or $taskRow.product -cne $false){throw 'Timing scope changed'}
            }
        }
        if(@($taskSources|Select-Object -Unique).Count -ne 1){throw 'Mixed cohort sources'}
    }
    [ordered]@{schema='pid-path-cost-local-v1';baseline_source=(PacketJson 'Baseline-194-plain-image.json').source;candidate_source=(PacketJson 'Candidate-194-plain-image.json').source;paired_allocation_samples=$taskAllocations;reference_allocations_per_open=2;candidate_allocations_per_open=0;reference_requested_bytes_per_open=$taskBytes;candidate_requested_bytes_per_open=0;fresh_calls_unchanged=$true;allocation_candidate_accepted=$true;phases=$taskPhases;latency_claim=$false;full_functional_matrix_retested=$false;native_execution_gate_passed=$false;sigkill_retested=$false;lost_ack_retested=$false;deadlines_changed=$false;clock_services_changed=$false;server=$false;product=$false}
}
