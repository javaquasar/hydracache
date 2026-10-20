$ErrorActionPreference='Stop'
function ReadJson([string]$Name){Get-Content -Raw "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Require([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
function Json($Value){ConvertTo-Json -Depth 40 -Compress -InputObject $Value}
function Rows([string]$Name,[string]$Prefix){
    @([regex]::Matches((Get-Content -Raw "$PSScriptRoot/$Name.log"),[regex]::Escape($Prefix)+'([^\r\n]+)')|ForEach-Object{$_.Groups[1].Value|ConvertFrom-Json})
}
function Range($Values){[ordered]@{min_ns=[long](($Values|Measure-Object -Minimum).Minimum);max_ns=[long](($Values|Measure-Object -Maximum).Maximum)}}
function Facts {
    $taskPairs=@();$taskPhases=@();$taskSegments=@();$taskMatrices=@()
    foreach($taskCohort in 'Baseline','Final'){
        foreach($taskLane in '194','188'){
            $taskAlloc=@(Rows "$taskCohort-$taskLane-allocation" 'HC074-PROJECTION-ALLOC ')
            Require ($taskAlloc.Count -eq 3) 'Complete allocation cohort required'
            foreach($taskIndex in 0..2){
                $taskRow=$taskAlloc[$taskIndex];$taskOld=$taskRow.reference;$taskNew=$taskRow.candidate
                Require ($taskRow.sample -eq $taskIndex -and $taskRow.iterations -eq 1024 -and $taskRow.candidate_first -eq ($taskIndex%2 -eq 1) -and $taskRow.rust_requests_only -and -not $taskRow.server -and -not $taskRow.product) 'Allocation workload drift'
                Require ($taskOld.allocations-$taskNew.allocations -eq 8192 -and $taskOld.reallocations -eq $taskNew.reallocations -and $taskOld.requested_bytes -gt $taskNew.requested_bytes) 'Allocation mechanism not confirmed'
                $taskPairs+=,[ordered]@{cohort=$taskCohort;lane=$taskLane;sample=$taskIndex;reference_allocations=$taskOld.allocations/1024;candidate_allocations=$taskNew.allocations/1024;reference_requested_bytes=$taskOld.requested_bytes/1024;candidate_requested_bytes=$taskNew.requested_bytes/1024;reallocations=$taskNew.reallocations/1024}
            }
            foreach($taskSample in 0..2){
                $taskName="$taskCohort-$taskLane-Profile-$taskSample"
                $taskRows=@(Rows $taskName 'HC074-HELD-PHASE ')
                Require ($taskRows.Count -eq 10 -and (ReadJson "$taskName-exit.json").exit -eq 0) 'Profile selection refused'
                $taskOriginal=@(Rows $taskName 'HC074-HELD-LIVE-ORIGINAL ')
                Require ($taskOriginal.Count -eq 1 -and $taskOriginal[0].returned -and $null -eq $taskOriginal[0].kernel -and $null -eq $taskOriginal[0].causal.first_refusal -and @($taskOriginal[0].age.sites|Where-Object{$null -ne $_.first_refusal}).Count -eq 0) 'Retain original refusal, not a healthy cohort'
                foreach($taskFlag in 'guard_clock_added','deadline_changed','server','product'){Require ($taskOriginal[0].$taskFlag -is [bool] -and -not $taskOriginal[0].$taskFlag) 'Original profile scope'}
                foreach($taskRow in $taskRows){
                    foreach($taskFlag in 'guard_clock_added','deadline_changed','healthy_gate','server','product') {Require ($taskRow.$taskFlag -is [bool] -and -not $taskRow.$taskFlag) 'Profile scope changed'}
                    Require $taskRow.returned 'Profile panic'
                    if($taskRow.phase -notin @('outputs','owned-root-preparation')){continue}
                    $taskExpected=if($taskRow.phase -ceq 'outputs'){@('outputs-enter','outputs-starting','outputs-pipes','outputs-sinks')}else{@('owned-enter','owned-context','owned-opening','owned-controls','owned-closing')}
                    Require ((Json @($taskRow.boundaries.name)) -ceq (Json $taskExpected)) 'Missing or reordered boundary'
                    Require ($taskRow.kernel[4].calls -eq $(if($taskRow.phase -ceq 'outputs'){76544}else{10032})) 'Original observation frequency changed'
                    $taskPhases+=,[pscustomobject][ordered]@{cohort=$taskCohort;lane=$taskLane;sample=$taskSample;phase=$taskRow.phase;wall_ns=[long]$taskRow.wall_ns;process_cpu_ns=[long]$taskRow.process_cpu_ns;projection_calls=$taskRow.kernel[3].calls;projection_wall_ns=[long]$taskRow.kernel[3].inclusive_ns;origin_calls=$taskRow.kernel[4].calls;boot_calls=$taskRow.kernel[5].calls;namespace_calls=$taskRow.kernel[6].calls;kernel_calls=@($taskRow.kernel.calls);live_calls=@($taskRow.live.calls);authentication_calls=$taskRow.authentication_calls}
                    for($taskIndex=1;$taskIndex -lt $taskExpected.Count;$taskIndex++){
                        $taskBefore=$taskRow.boundaries[$taskIndex-1];$taskAfter=$taskRow.boundaries[$taskIndex]
                        Require ($taskAfter.wall_ns -ge $taskBefore.wall_ns -and $taskAfter.process_cpu_ns -ge $taskBefore.process_cpu_ns) 'Boundary counter order'
                        $taskSegments+=,[pscustomobject][ordered]@{cohort=$taskCohort;lane=$taskLane;sample=$taskSample;segment=$taskExpected[$taskIndex];wall_ns=[long]$taskAfter.wall_ns-[long]$taskBefore.wall_ns;process_cpu_ns=[long]$taskAfter.process_cpu_ns-[long]$taskBefore.process_cpu_ns}
                    }
                }
            }
        }
    }
    foreach($taskGroup in @($taskPhases|Group-Object phase)){
        $taskFirst=$taskGroup.Group[0]
        foreach($taskRow in $taskGroup.Group){Require ((Json $taskRow.kernel_calls) -ceq (Json $taskFirst.kernel_calls) -and (Json $taskRow.live_calls) -ceq (Json $taskFirst.live_calls) -and $taskRow.authentication_calls -eq $taskFirst.authentication_calls) 'Parent live/kernel/authentication frequency drift'}
    }
    foreach($taskLane in '194','188'){
        $taskName="Final-$taskLane-Matrix-full";$taskRaw=Get-Content -Raw "$PSScriptRoot/$taskName.log"
        $taskCases=@([regex]::Matches($taskRaw,'HC074-HELD-LIVE case=(\S+) seed=0x074c051e actual_fixed=true actual_consuming=true')|ForEach-Object{$_.Groups[1].Value})
        $taskOriginal=@(Rows $taskName 'HC074-HELD-LIVE-ORIGINAL ')
        Require ((ReadJson "$taskName-exit.json").exit -eq 0 -and (Json $taskCases) -ceq (Json @('healthy','revalidate','role-before','role-after','trust-after')) -and $taskOriginal.Count -eq 5 -and -not $taskRaw.Contains('HC074-HELD-PHASE')) 'Full unprofiled matrix required'
        foreach($taskRow in $taskOriginal){Require ($taskRow.returned -and $null -eq $taskRow.kernel -and $null -eq $taskRow.causal.first_refusal -and @($taskRow.age.sites|Where-Object{$null -ne $_.first_refusal}).Count -eq 0 -and -not $taskRow.deadline_changed -and -not $taskRow.guard_clock_added -and -not $taskRow.server -and -not $taskRow.product) 'Original matrix refusal'}
        foreach($taskRow in $taskOriginal){foreach($taskFlag in 'guard_clock_added','deadline_changed','server','product'){Require ($taskRow.$taskFlag -is [bool] -and -not $taskRow.$taskFlag) 'Original matrix scope'}}
        $taskMatrices+=,[ordered]@{lane=$taskLane;cases=$taskCases;exit=0}
    }
    $taskRanges=@()
    foreach($taskCohort in 'Baseline','Final'){
        foreach($taskGroup in @($taskSegments|Where-Object cohort -CEQ $taskCohort|Group-Object segment)){
            Require ($taskGroup.Count -eq 6) 'Six segment observations required'
            $taskRanges+=,[ordered]@{cohort=$taskCohort;segment=$taskGroup.Name;samples=$taskGroup.Count;wall=(Range $taskGroup.Group.wall_ns);process_cpu=(Range $taskGroup.Group.process_cpu_ns)}
        }
    }
    Require ($taskRanges.Count -eq 14 -and $taskPhases.Count -eq 24 -and $taskPairs.Count -eq 12) 'Complete attribution groups required'
    [ordered]@{schema='remaining-preparation-cost-v1';baseline_product='dea214a80aa59d3af7b8456a43c0763573a04546';sources=(ReadJson 'sources.json');allocation_pairs=$taskPairs;phase_samples=$taskPhases;boundary_ranges=$taskRanges;local_matrices=$taskMatrices;allocation_effect_confirmed=$true;numerical_latency_gain_accepted=$false;native_gate_open=$false;joined_crash_retested=$false;all_writers_proved=$false;enrollment_admitted=$false;deadline_changed=$false;live_checks_changed=$false;server=$false;product=$false}
}
