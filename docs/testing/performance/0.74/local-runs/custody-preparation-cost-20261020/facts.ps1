$ErrorActionPreference='Stop'
function CostFacts {
    $taskAlloc=@()
    foreach($taskCohort in 'Prototype','Final'){
        foreach($taskLane in '194','188'){
            $taskRaw=Get-Content -Raw "$PSScriptRoot/$taskCohort-$taskLane-allocation.log"
            $taskSamples=@([regex]::Matches($taskRaw,'HC074-DOCUMENT-SCRATCH-ALLOC ([^\r\n]+)')|ForEach-Object{$_.Groups[1].Value|ConvertFrom-Json})
            if($taskSamples.Count -ne 3){throw 'Allocation cohort incomplete'}
            for($taskIndex=0;$taskIndex -lt 3;$taskIndex++){
                $taskSample=$taskSamples[$taskIndex]
                if($taskSample.sample -ne $taskIndex -or $taskSample.iterations -ne 1024 -or $taskSample.candidate_first -ne ($taskIndex%2 -eq 1) -or -not $taskSample.rust_requests_only -or $taskSample.server -or $taskSample.product){throw 'Allocation workload changed'}
                $taskOld=$taskSample.reference;$taskNew=$taskSample.candidate
                if($taskOld.allocations-$taskNew.allocations -ne 3072 -or $taskOld.requested_bytes-$taskNew.requested_bytes -ne 201329664 -or $taskOld.reallocations -ne $taskNew.reallocations){throw 'Unconfirmed allocation effect'}
                $taskAlloc+=,[ordered]@{cohort=$taskCohort;lane=$taskLane;sample=$taskIndex;reference_requests=$taskOld.allocations/1024;candidate_requests=$taskNew.allocations/1024;reference_bytes=$taskOld.requested_bytes/1024;candidate_bytes=$taskNew.requested_bytes/1024;reallocations=$taskNew.reallocations/1024}
            }
        }
    }
    $taskPhases=((& "$PSScriptRoot/phases.ps1")|Out-String)|ConvertFrom-Json
    $taskBase=@($taskPhases|Where-Object cohort -CEQ 'Baseline')
    if($taskBase.Count -ne 60 -or @($taskBase|Where-Object{$_.exit -ne 0 -or -not $_.returned}).Count){throw 'Baseline attribution incomplete'}
    $taskBounds=@($taskBase|Where-Object phase -CEQ 'enrollment-frame'|ForEach-Object{
        if(($_.boundaries.name -join ',') -cne 'before-first-custody,closing-enter,closing-exit'){throw 'Boundary order'}
        if($_.boundaries[2].wall_ns -lt $_.boundaries[1].wall_ns -or $_.boundaries[1].wall_ns -lt $_.boundaries[0].wall_ns){throw 'Boundary elapsed order'}
        [long]$_.boundaries[2].wall_ns-[long]$_.boundaries[1].wall_ns
    })
    $taskOutputs=@($taskBase|Where-Object phase -CEQ 'outputs')
    $taskMatrices=@()
    foreach($taskLane in '194','188'){
        $taskRaw=Get-Content -Raw "$PSScriptRoot/matrix-$taskLane.log"
        $taskExit=(Get-Content -Raw "$PSScriptRoot/matrix-$taskLane-exit.json"|ConvertFrom-Json).exit
        $taskCases=@([regex]::Matches($taskRaw,'HC074-HELD-LIVE case=(\S+) seed=0x074c051e actual_fixed=true actual_consuming=true')|ForEach-Object{$_.Groups[1].Value})
        $taskExpected=@('healthy','revalidate','role-before','role-after','trust-after')
        if(($taskCases -join ',') -cne (($taskExpected|Select-Object -First $taskCases.Count)-join ',') -or $taskRaw.Contains('HC074-HELD-PHASE')){throw 'Matrix case splicing or cost override'}
        $taskOriginal=@([regex]::Matches($taskRaw,'HC074-HELD-LIVE-ORIGINAL ([^\r\n]+)')|ForEach-Object{$_.Groups[1].Value|ConvertFrom-Json})
        foreach($taskRecord in $taskOriginal){foreach($taskFlag in 'guard_clock_added','deadline_changed','server','product'){if($taskRecord.$taskFlag -isnot [bool] -or $taskRecord.$taskFlag){throw 'Original semantics changed'}}}
        if($taskExit -eq 0){foreach($taskRecord in $taskOriginal){if($null -ne $taskRecord.kernel -or $null -ne $taskRecord.causal.first_refusal -or @($taskRecord.age.sites|ForEach-Object{$_.first_refusal}|Where-Object{$null -ne $_}).Count){throw 'Passing matrix contains original refusal'}}}
        if($taskExit -eq 0 -and ($taskCases.Count -ne 5 -or $taskOriginal.Count -ne 5 -or @($taskOriginal|Where-Object{-not $_.returned}).Count)){throw 'False matrix pass'}
        $taskMatrices+=,[ordered]@{lane=$taskLane;exit=$taskExit;completed_cases=$taskCases;original_refusals=@($taskOriginal|Where-Object{-not $_.returned}|ForEach-Object{[ordered]@{case=$_.case;kernel=$_.kernel;causal=$_.causal.first_refusal;age=@($_.age.sites|ForEach-Object{$_.first_refusal}|Where-Object{$null -ne $_})}})}
    }
    [ordered]@{schema='local-custody-preparation-cost-v1';baseline='72efce2401e753b857f7fc03f3300b6aae19bd5d';observer='b28a29ebe465f27762828689ce759a6716fda5b4';prototype='4952b0f28fd752d9e479ed5f5ed34e6184bbb836';runtime='1f3715498cb3b9b1328061a3647f90c1c3bd2e19';allocation_pairs=$taskAlloc;baseline_phase_rows=$taskBase.Count;candidate_phase_rows=@($taskPhases|Where-Object cohort -CEQ 'Candidate').Count;baseline_output_wall_min_ns=[long](($taskOutputs.wall_ns|Measure-Object -Minimum).Minimum);baseline_output_wall_max_ns=[long](($taskOutputs.wall_ns|Measure-Object -Maximum).Maximum);baseline_closing_wall_min_ns=[long](($taskBounds|Measure-Object -Minimum).Minimum);baseline_closing_wall_max_ns=[long](($taskBounds|Measure-Object -Maximum).Maximum);local_matrices=$taskMatrices;local_matrix_passed=(@($taskMatrices|Where-Object exit -NE 0).Count -eq 0);allocation_effect_confirmed=$true;numerical_latency_gain_accepted=$false;native_execution_gate_passed=$false;joined_sigkill_lost_ack_retested=$false;original_delivery_retested=$false;all_writer_cessation_proved=$false;later_cells_retested=$false;reconnect_retested=$false;production_enrollment=$false;earlier_refusals_retained=$true;deadline_changed=$false;live_checks_changed=$false;server=$false;product=$false}
}
