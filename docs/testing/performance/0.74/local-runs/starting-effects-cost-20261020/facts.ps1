$ErrorActionPreference='Stop'
function ReadJson([string]$Name){Get-Content -Raw "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Require([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
function Json($Value){ConvertTo-Json -Depth 60 -Compress -InputObject $Value}
function Rows([string]$Path,[string]$Prefix){@([regex]::Matches((Get-Content -Raw $Path),[regex]::Escape($Prefix)+'([^\r\n]+)')|ForEach-Object{$_.Groups[1].Value|ConvertFrom-Json})}
function Range($Values){[ordered]@{min_ns=[long](($Values|Measure-Object -Minimum).Minimum);max_ns=[long](($Values|Measure-Object -Maximum).Maximum)}}
function ClockRefusal($Row){
    Require (-not $Row.returned -and $Row.case -ceq 'healthy' -and $Row.kernel.site -ceq 'clock-order' -and $Row.kernel.file -ceq 'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs' -and $Row.kernel.line -eq 259) 'Exact original refusal required'
    $taskClock=$Row.kernel.clock
    Require ($taskClock.Count -eq 6 -and $taskClock[1] -ge $taskClock[0] -and $taskClock[3] -lt $taskClock[2] -and $taskClock[4] -eq 1 -and $taskClock[5] -eq 60000000000 -and $null -eq $Row.causal.first_refusal -and @($Row.age.sites|Where-Object{$null -ne $_.first_refusal}).Count -eq 0) 'Clock-order refusal is not a causal deadline failure'
    foreach($taskFlag in 'guard_clock_added','deadline_changed','server','product'){Require ($Row.$taskFlag -is [bool] -and -not $Row.$taskFlag) 'Refusal scope'}
    [ordered]@{site=$Row.kernel.site;file=$Row.kernel.file;line=$Row.kernel.line;clock=$taskClock;boottime_forward_ns=[long]$taskClock[1]-[long]$taskClock[0];realtime_backward_ns=[long]$taskClock[2]-[long]$taskClock[3];same_boot=$true;causal_deadline_refused=$false;actor_attributed=$false}
}
function Facts {
    $taskSamples=@();$taskStages=@();$taskTotals=@();$taskMatrices=@()
    $taskNames=@('BeforeCreate','Created','CreateDirectorySynced','PrefixWritten','BytesWritten','Sealed','FileSynced','DirectorySynced','Acknowledging')
    $taskExpected=@('outputs-enter','st-enter','st-encoded','st-begun')
    foreach($taskIndex in 0..8){$taskExpected+=@('st-open','st-opened','st-store','st-inspected','st-effect','st-effected','st-stepped','st-checked','st-closed')}
    $taskExpected+=@('st-final','st-finaled','st-acked','outputs-starting','outputs-pipes','outputs-sinks')
    # Only complete GNU healthy profiles contribute costs. The stopped MSRV cohort
    # is retained below as a refusal, never accepted as three healthy observations.
    foreach($taskLane in @('194')){
        foreach($taskSample in 0..2){
            $taskName="$taskLane-Profile-$taskSample"
            Require ((ReadJson "$taskName-exit.json").exit -eq 0) 'Original profile failed; retain refusal'
            $taskRows=@(Rows "$PSScriptRoot/$taskName.log" 'HC074-HELD-PHASE ')
            $taskPrior=@(Rows "$PSScriptRoot/../remaining-preparation-cost-20261020/Final-$taskLane-Profile-$taskSample.log" 'HC074-HELD-PHASE ')
            Require ($taskRows.Count -eq 10 -and $taskPrior.Count -eq 10 -and (Json @($taskRows.phase)) -ceq (Json @($taskPrior.phase))) 'Incomplete or changed preparation'
            $taskOriginal=@(Rows "$PSScriptRoot/$taskName.log" 'HC074-HELD-LIVE-ORIGINAL ')
            Require ($taskOriginal.Count -eq 1 -and $taskOriginal[0].returned -and $null -eq $taskOriginal[0].kernel -and $null -eq $taskOriginal[0].causal.first_refusal -and @($taskOriginal[0].age.sites|Where-Object{$null -ne $_.first_refusal}).Count -eq 0) 'Original refusal is not healthy'
            foreach($taskFlag in 'guard_clock_added','deadline_changed','server','product'){Require ($taskOriginal[0].$taskFlag -is [bool] -and -not $taskOriginal[0].$taskFlag) 'Original scope'}
            for($taskIndex=0;$taskIndex -lt 10;$taskIndex++){
                $taskRow=$taskRows[$taskIndex];$taskOld=$taskPrior[$taskIndex]
                Require ($taskRow.returned -and $taskRow.instrumented -and -not $taskRow.includes_child_cpu) 'Observer scope'
                foreach($taskFlag in 'guard_clock_added','deadline_changed','healthy_gate','server','product'){Require ($taskRow.$taskFlag -is [bool] -and -not $taskRow.$taskFlag) 'Phase scope'}
                Require ((Json @($taskRow.kernel.calls)) -ceq (Json @($taskOld.kernel.calls)) -and (Json @($taskRow.live.calls)) -ceq (Json @($taskOld.live.calls)) -and $taskRow.authentication_calls -eq $taskOld.authentication_calls -and $taskRow.signature_calls -eq $taskOld.signature_calls) 'Fresh validation frequency changed'
            }
            Require ($taskOriginal[0].causal.first.issued_boot -ceq 'cf619cb0-6679-4f52-a5b8-f6a9802d9ec4' -and $taskOriginal[0].causal.last.now_boot -ceq 'cf619cb0-6679-4f52-a5b8-f6a9802d9ec4') 'Recorded sample boot identity'
            $taskOutput=@($taskRows|Where-Object phase -CEQ 'outputs')
            Require ($taskOutput.Count -eq 1) 'Unique outputs phase'
            $taskEvents=$taskOutput[0].boundaries
            Require ((Json @($taskEvents.name)) -ceq (Json $taskExpected)) 'Missing duplicate or reordered Starting boundary'
            for($taskIndex=1;$taskIndex -lt $taskEvents.Count;$taskIndex++){
                Require ($null -ne $taskEvents[$taskIndex].process_cpu_ns -and $taskEvents[$taskIndex].wall_ns -ge $taskEvents[$taskIndex-1].wall_ns -and $taskEvents[$taskIndex].process_cpu_ns -ge $taskEvents[$taskIndex-1].process_cpu_ns) 'Boundary clock order'
            }
            $taskAdd={param($Kind,$Before,$After)
                $taskTotals.Add([pscustomobject][ordered]@{lane=$taskLane;sample=$taskSample;kind=$Kind;wall_ns=[long]$After.wall_ns-[long]$Before.wall_ns;process_cpu_ns=[long]$After.process_cpu_ns-[long]$Before.process_cpu_ns})
            }
            # Mutable list is shared with the scriptblock; no grouped OrderedDictionary keys.
            if($taskTotals -isnot [Collections.Generic.List[object]]){$taskTotals=[Collections.Generic.List[object]]::new()}
            & $taskAdd 'encode' $taskEvents[1] $taskEvents[2]
            & $taskAdd 'begin' $taskEvents[2] $taskEvents[3]
            for($taskIndex=0;$taskIndex -lt 9;$taskIndex++){
                $taskOffset=4+$taskIndex*9;$taskStage=$taskNames[$taskIndex]
                $taskMap=@(@('opening-round',0,1),@('storage-opening',2,3),@('phase-checks',3,4),@('effect',4,5),@('storage-closing',5,6),@('checkpoint',6,7),@('closing-round',7,8))
                foreach($taskMapping in $taskMap){
                    $taskBefore=$taskEvents[$taskOffset+$taskMapping[1]];$taskAfter=$taskEvents[$taskOffset+$taskMapping[2]]
                    $taskStages+=,[pscustomobject][ordered]@{lane=$taskLane;sample=$taskSample;stage=$taskStage;kind=$taskMapping[0];wall_ns=[long]$taskAfter.wall_ns-[long]$taskBefore.wall_ns;process_cpu_ns=[long]$taskAfter.process_cpu_ns-[long]$taskBefore.process_cpu_ns}
                    & $taskAdd $taskMapping[0] $taskBefore $taskAfter
                    if($taskMapping[0] -ceq 'effect'){
                        $taskKind=if($taskIndex -in @(2,6,7)){'fsync'}elseif($taskIndex -in @(3,4,5)){'write-seal'}elseif($taskIndex -eq 1){'create'}else{'no-op'}
                        & $taskAdd $taskKind $taskBefore $taskAfter
                    }
                }
            }
            & $taskAdd 'final-round' $taskEvents[85] $taskEvents[86]
            & $taskAdd 'acknowledge' $taskEvents[86] $taskEvents[87]
            & $taskAdd 'starting-total' $taskEvents[1] $taskEvents[87]
            $taskSamples+=,[ordered]@{lane=$taskLane;sample=$taskSample;outputs_wall_ns=$taskOutput[0].wall_ns;starting_wall_ns=[long]$taskEvents[87].wall_ns-[long]$taskEvents[1].wall_ns;kernel_calls=@($taskOutput[0].kernel.calls);live_calls=@($taskOutput[0].live.calls);authentication_calls=$taskOutput[0].authentication_calls;complete_phase_count=$taskRows.Count}
        }
    }
    $taskRefused=@(Rows "$PSScriptRoot/188-Profile-0.log" 'HC074-HELD-LIVE-ORIGINAL ')
    Require ((ReadJson '188-Profile-0-exit.json').exit -eq 101 -and $taskRefused.Count -eq 1 -and -not (Test-Path "$PSScriptRoot/188-Profile-1.log") -and -not (Test-Path "$PSScriptRoot/188-Profile-2.log")) 'Retain stopped MSRV cohort without retry'
    $taskProfileRefusal=ClockRefusal $taskRefused[0]
    $taskPartial=@(Rows "$PSScriptRoot/188-Profile-0.log" 'HC074-HELD-PHASE ')
    Require ($taskPartial.Count -eq 3 -and (Json @($taskPartial.phase)) -ceq (Json @('prepare-consent','accept-consent','outputs')) -and -not $taskPartial[2].returned) 'Refused profile frontier'
    foreach($taskLane in '194','188'){
        $taskName="$taskLane-Matrix-full";$taskRaw=Get-Content -Raw "$PSScriptRoot/$taskName.log"
        $taskCases=@([regex]::Matches($taskRaw,'HC074-HELD-LIVE case=(\S+) seed=0x074c051e actual_fixed=true actual_consuming=true')|ForEach-Object{$_.Groups[1].Value})
        $taskOriginal=@(Rows "$PSScriptRoot/$taskName.log" 'HC074-HELD-LIVE-ORIGINAL ')
        Require (-not $taskRaw.Contains('HC074-HELD-PHASE')) 'Unprofiled matrix required'
        if($taskLane -ceq '194'){
            Require ((ReadJson "$taskName-exit.json").exit -eq 101 -and $taskCases.Count -eq 0 -and $taskOriginal.Count -eq 1) 'GNU stopped at first healthy refusal'
            $taskMatrices+=,[ordered]@{lane=$taskLane;cases=$taskCases;exit=101;passed=$false;first_case='healthy';refusal=(ClockRefusal $taskOriginal[0])}
            continue
        }
        Require ((ReadJson "$taskName-exit.json").exit -eq 0 -and (Json $taskCases) -ceq (Json @('healthy','revalidate','role-before','role-after','trust-after'))) 'Complete MSRV matrix required'
        Require ($taskOriginal.Count -eq 5) 'Complete matrix observations'
        foreach($taskRow in $taskOriginal){Require ($taskRow.returned -and $null -eq $taskRow.kernel -and $null -eq $taskRow.causal.first_refusal -and @($taskRow.age.sites|Where-Object{$null -ne $_.first_refusal}).Count -eq 0 -and -not $taskRow.deadline_changed -and -not $taskRow.guard_clock_added -and -not $taskRow.server -and -not $taskRow.product) 'Matrix refusal'}
        $taskMatrices+=,[ordered]@{lane=$taskLane;cases=$taskCases;exit=0;passed=$true}
    }
    $taskAggregates=@()
    foreach($taskLane in @('194')){
        foreach($taskSample in 0..2){
            foreach($taskGroup in @($taskTotals|Where-Object{$_.lane -ceq $taskLane -and $_.sample -eq $taskSample}|Group-Object kind)){
                $taskAggregates+=,[pscustomobject][ordered]@{lane=$taskLane;sample=$taskSample;kind=$taskGroup.Name;wall_ns=[long](($taskGroup.Group.wall_ns|Measure-Object -Sum).Sum);process_cpu_ns=[long](($taskGroup.Group.process_cpu_ns|Measure-Object -Sum).Sum)}
            }
        }
    }
    $taskRanges=@($taskAggregates|Group-Object kind|ForEach-Object{Require ($_.Count -eq 3) 'Three complete GNU observations, not six across compilers';[ordered]@{kind=$_.Name;samples=$_.Count;wall=(Range $_.Group.wall_ns);process_cpu=(Range $_.Group.process_cpu_ns)}})
    Require ($taskSamples.Count -eq 3 -and $taskStages.Count -eq 189 -and $taskAggregates.Count -eq 48 -and $taskRanges.Count -eq 16) 'Complete GNU attribution required'
    [ordered]@{schema='starting-effects-cost-v1';baseline='4d0f461c638d158ff62dbd3e0f8f9914d523581e';sources=(ReadJson 'sources.json');samples=$taskSamples;stages=$taskStages;aggregates=$taskAggregates;ranges=$taskRanges;matrices=$taskMatrices;msrv_profile_refusal=$taskProfileRefusal;initial_boot=(Get-Content -Raw "$PSScriptRoot/local-boot.log").Trim();sample_boot=(Get-Content -Raw "$PSScriptRoot/local-postflight.log").Trim();environment_boot_continuity=$false;complete_profile_cohort=$false;both_local_matrices_passed=$false;runtime_candidate_adopted=$false;live_checks_changed=$false;deadline_changed=$false;fsync_removed=$false;latency_gain_accepted=$false;native_gate_open=$false;joined_crash_retested=$false;all_writers_proved=$false;enrollment_admitted=$false;server=$false;product=$false}
}
