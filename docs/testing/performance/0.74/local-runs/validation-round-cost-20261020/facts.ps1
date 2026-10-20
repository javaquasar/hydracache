$ErrorActionPreference='Stop'
function ReadJson([string]$Name){Get-Content -Raw "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Require([bool]$Value,[string]$Message){if(-not $Value){throw $Message}}
function Json($Value){ConvertTo-Json -Depth 70 -Compress -InputObject $Value}
function Rows([string]$Path,[string]$Prefix){@([regex]::Matches((Get-Content -Raw $Path),[regex]::Escape($Prefix)+'([^\r\n]+)')|ForEach-Object{$_.Groups[1].Value|ConvertFrom-Json})}
function Range($Values){[ordered]@{min=[decimal](($Values|Measure-Object -Minimum).Minimum);max=[decimal](($Values|Measure-Object -Maximum).Maximum)}}
function Diagnosis {
    $taskFailed=ReadJson 'clock-observations.json'
    Require ($taskFailed.exit -eq 1 -and $taskFailed.samples.Count -eq 0 -and $taskFailed.stderr.Contains('AssertionError')) 'Retain initial argument-validation refusal'
    $taskData=ReadJson 'clock-observations-v2.json'
    Require ($taskData.exit -eq 0 -and $taskData.samples.Count -eq 180 -and $taskData.stderr -ceq '' -and $taskData.read_only -and -not $taskData.changes_time -and -not $taskData.changes_service -and -not $taskData.server) 'Bounded read-only observation'
    $taskFirst=$taskData.samples[0];$taskLast=$taskData.samples[-1];$taskDeltas=@()
    foreach($taskIndex in 0..179){
        $taskB=$taskData.samples[$taskIndex]
        Require ($taskB.linux.sample -eq $taskIndex -and $taskB.linux.boot -ceq $taskFirst.linux.boot -and $taskB.linux.pid -eq $taskFirst.linux.pid) 'Same observer and boot'
        if($taskIndex -eq 0){continue}
        $taskA=$taskData.samples[$taskIndex-1]
        $taskDeltas+=,[pscustomobject][ordered]@{sample=$taskIndex;realtime_ns=[long]$taskB.linux.realtime_ns-[long]$taskA.linux.realtime_ns;boottime_ns=[long]$taskB.linux.boottime_ns-[long]$taskA.linux.boottime_ns;monotonic_ns=[long]$taskB.linux.monotonic_ns-[long]$taskA.linux.monotonic_ns;raw_ns=[long]$taskB.linux.raw_ns-[long]$taskA.linux.raw_ns;windows_utc_ns=([long]$taskB.windows_utc_ticks-[long]$taskA.windows_utc_ticks)*100;windows_qpc_ns=([decimal]$taskB.windows_qpc-[decimal]$taskA.windows_qpc)*1000000000/$taskData.qpc_frequency}
    }
    $taskRanges=@();$taskNegative=[ordered]@{}
    foreach($taskClock in 'realtime_ns','boottime_ns','monotonic_ns','raw_ns','windows_utc_ns','windows_qpc_ns'){
        $taskRanges+=,[ordered]@{clock=$taskClock;delta=(Range @($taskDeltas.$taskClock))}
        $taskNegative[$taskClock]=@($taskDeltas|Where-Object {$_.$taskClock -lt 0}).Count
    }
    $taskElapsed=[ordered]@{linux_realtime_ns=[long]$taskLast.linux.realtime_ns-[long]$taskFirst.linux.realtime_ns;linux_boottime_ns=[long]$taskLast.linux.boottime_ns-[long]$taskFirst.linux.boottime_ns;linux_raw_ns=[long]$taskLast.linux.raw_ns-[long]$taskFirst.linux.raw_ns;windows_utc_ns=([long]$taskLast.windows_utc_ticks-[long]$taskFirst.windows_utc_ticks)*100;windows_qpc_ns=([decimal]$taskLast.windows_qpc-[decimal]$taskFirst.windows_qpc)*1000000000/$taskData.qpc_frequency}
    $taskBefore=(Get-Content -Raw "$PSScriptRoot/wsl-state-before.log").Replace([string][char]0,'')
    Require ($taskBefore -match 'Ubuntu\s+Stopped\s+2') 'Initial stopped distribution retained'
    $taskHistory=Get-Content -Raw "$PSScriptRoot/linux-before.log"
    Require ($taskHistory.Contains('9360d8a950f94818950921aca8f02f71') -and $taskHistory.Contains('cf619cb066794f52a5b8f6a9802d9ec4')) 'Prior boots retained in journal'
    $taskSync=Get-Content -Raw "$PSScriptRoot/previous-boot-timesync.log"
    Require ([regex]::Matches($taskSync,'Initial clock synchronization').Count -eq 2) 'Historical sync observations'
    [ordered]@{samples=180;boot=$taskFirst.linux.boot;pid=$taskFirst.linux.pid;initial_distro_stopped=$true;elapsed=$taskElapsed;delta_ranges=$taskRanges;negative_deltas=$taskNegative;raw_minus_boottime_elapsed_ns=$taskElapsed.linux_raw_ns-$taskElapsed.linux_boottime_ns;historical_sync_initializations=2;time_change_actor_proved=$false;past_time_stability_proved=$false;clock_correspondence_proved=$false;changes_time=$false;changes_service=$false;server=$false}
}
function Facts {
    $taskProfiles=@();$taskRefusals=@();$taskSteps=@();$taskMatrices=@();$taskNames=@('StorageBefore','ReservedLive','Progress','Causal','Prepared','Consent','StorageAfter','FinalClocks')
    foreach($taskLane in '194','188'){
        $taskStopped=$false
        foreach($taskSample in 0..2){
            $taskName="$taskLane-Profile-$taskSample";$taskExists=Test-Path "$PSScriptRoot/$taskName-exit.json"
            if($taskStopped){Require (-not $taskExists -and -not (Test-Path "$PSScriptRoot/$taskName.log")) 'No retry or extra sample after refusal';continue}
            Require $taskExists 'Scheduled profile receipt missing'
            $taskExit=(ReadJson "$taskName-exit.json").exit
            $taskOriginal=@(Rows "$PSScriptRoot/$taskName.log" 'HC074-HELD-LIVE-ORIGINAL ')
            Require ($taskOriginal.Count -eq 1) 'Original result required'
            $taskParent=@(Rows "$PSScriptRoot/$taskName.log" 'HC074-HELD-PHASE ')
            if($taskExit -ne 0){
                Require ($taskExit -eq 101 -and -not $taskOriginal[0].returned) 'Actual profile refusal'
                $taskRefusals+=,[ordered]@{lane=$taskLane;sample=$taskSample;exit=$taskExit;original=$taskOriginal[0];completed_phases=@($taskParent|Where-Object returned|ForEach-Object{$_.phase});cost_cohort=$false}
                $taskStopped=$true;continue
            }
            Require ($taskOriginal[0].returned -and $null -eq $taskOriginal[0].kernel -and $null -eq $taskOriginal[0].causal.first_refusal -and @($taskOriginal[0].age.sites|Where-Object{$null -ne $_.first_refusal}).Count -eq 0) 'Original healthy only'
            Require ($taskParent.Count -eq 10) 'Complete preparation, never spliced partial phases'
            $taskPrior=@(Rows "$PSScriptRoot/../remaining-preparation-cost-20261020/Final-$taskLane-Profile-$taskSample.log" 'HC074-HELD-PHASE ')
            Require ((Json @($taskParent.phase)) -ceq (Json @($taskPrior.phase))) 'Original phase order'
            for($taskPhase=0;$taskPhase -lt 10;$taskPhase++){
                $taskRow=$taskParent[$taskPhase];$taskOld=$taskPrior[$taskPhase]
                Require ($taskRow.returned -and $taskRow.instrumented -and -not $taskRow.healthy_gate -and -not $taskRow.guard_clock_added -and -not $taskRow.deadline_changed -and -not $taskRow.server -and -not $taskRow.product) 'Observation scope'
                Require ((Json @($taskRow.kernel.calls)) -ceq (Json @($taskOld.kernel.calls)) -and (Json @($taskRow.live.calls)) -ceq (Json @($taskOld.live.calls)) -and $taskRow.authentication_calls -eq $taskOld.authentication_calls -and $taskRow.signature_calls -eq $taskOld.signature_calls) 'Parent live frequency changed'
                Require ($taskRow.round_steps.Count -eq 8) 'All step reports required'
                for($taskStep=0;$taskStep -lt 8;$taskStep++){
                    $taskPart=$taskRow.round_steps[$taskStep]
                    Require ($taskPart.step -eq $taskStep -and $taskPart.calls -eq $taskPart.completed -and -not $taskPart.guard_clock_added) 'Incomplete or reordered step'
                    for($taskKind=0;$taskKind -lt 10;$taskKind++){
                        $taskNode=$taskPart.kernel[$taskKind]
                        $taskIncoming=($taskPart.edges|Where-Object child -EQ $taskKind|Measure-Object calls -Sum).Sum
                        $taskChildren=($taskPart.edges|Where-Object parent -EQ $taskKind|Measure-Object inclusive_ns -Sum).Sum
                        Require ($taskNode.calls -eq $taskPart.roots[$taskKind]+$taskIncoming -and $taskNode.inclusive_ns -eq $taskNode.outside_children_ns+$taskChildren) 'Kernel graph count/time conservation'
                    }
                }
                if($taskRow.phase -cne 'outputs'){continue}
                Require ($taskRow.authentication_calls -eq 782 -and @($taskRow.round_steps|Where-Object{$_.calls -ne 46}).Count -eq 0) '46 unchanged full output rounds'
                for($taskKind=0;$taskKind -lt 10;$taskKind++){
                    $taskSum=($taskRow.round_steps|ForEach-Object{$_.kernel[$taskKind].calls}|Measure-Object -Sum).Sum
                    Require ($taskSum -eq $taskRow.kernel[$taskKind].calls) 'All output kernel observations attributed once'
                }
                foreach($taskPart in $taskRow.round_steps){
                    $taskOutside=[long](($taskPart.kernel.outside_children_ns|Measure-Object -Sum).Sum)
                    Require ($taskOutside -le $taskPart.wall_ns) 'Kernel wall coverage cannot exceed step interval'
                    $taskSteps+=,[pscustomobject][ordered]@{lane=$taskLane;sample=$taskSample;step=$taskNames[$taskPart.step];calls=$taskPart.calls;wall_ns=[long]$taskPart.wall_ns;kernel_outside_children_ns=$taskOutside;outside_instrumented_kernel_ns=[long]$taskPart.wall_ns-$taskOutside;kernel=$taskPart.kernel;edges=$taskPart.edges;live=$taskPart.live;authentication_calls=$taskPart.authentication_calls;signature_calls=$taskPart.signature_calls;signature_ns=$taskPart.signature_ns}
                }
            }
            $taskProfiles+=,[ordered]@{lane=$taskLane;sample=$taskSample;phase_count=10;outputs_rounds=46;original=$taskOriginal[0]}
        }
        $taskName="$taskLane-Matrix-full";$taskRaw=Get-Content -Raw "$PSScriptRoot/$taskName.log"
        $taskExit=(ReadJson "$taskName-exit.json").exit
        $taskOriginal=@(Rows "$PSScriptRoot/$taskName.log" 'HC074-HELD-LIVE-ORIGINAL ')
        $taskCases=@([regex]::Matches($taskRaw,'HC074-HELD-LIVE case=(\S+) seed=0x074c051e actual_fixed=true actual_consuming=true')|ForEach-Object{$_.Groups[1].Value})
        Require (-not $taskRaw.Contains('HC074-HELD-PHASE')) 'Unprofiled functional gate'
        if($taskExit -eq 0){Require ((Json $taskCases) -ceq (Json @('healthy','revalidate','role-before','role-after','trust-after')) -and $taskOriginal.Count -eq 5 -and @($taskOriginal|Where-Object{-not $_.returned -or $null -ne $_.kernel -or $null -ne $_.causal.first_refusal}).Count -eq 0) 'Complete functional matrix'}else{Require ($taskExit -eq 101 -and $taskOriginal.Count -ge 1 -and -not $taskOriginal[-1].returned) 'Retained functional refusal'}
        $taskMatrices+=,[ordered]@{lane=$taskLane;exit=$taskExit;passed=($taskExit -eq 0);cases=$taskCases;original=$taskOriginal}
    }
    $taskRanges=@($taskSteps|Group-Object lane,step|ForEach-Object{[ordered]@{lane=$_.Group[0].lane;step=$_.Group[0].step;samples=$_.Count;wall_ns=(Range $_.Group.wall_ns);kernel_outside_children_ns=(Range $_.Group.kernel_outside_children_ns);outside_instrumented_kernel_ns=(Range $_.Group.outside_instrumented_kernel_ns)}})
    $taskShares=@($taskSteps|Group-Object lane,sample|ForEach-Object{
        $taskTotal=[long](($_.Group.wall_ns|Measure-Object -Sum).Sum)
        $taskReserved=@($_.Group|Where-Object step -CEQ 'ReservedLive')
        Require ($_.Count -eq 8 -and $taskReserved.Count -eq 1) 'Full eight-step attribution'
        [ordered]@{lane=$_.Group[0].lane;sample=$_.Group[0].sample;all_steps_wall_ns=$taskTotal;reserved_live_wall_ns=$taskReserved[0].wall_ns;reserved_live_share_percent=[math]::Round(100*$taskReserved[0].wall_ns/$taskTotal,6)}
    })
    $taskInitialConfig=(Get-Content -Raw "$PSScriptRoot/linux-before.log").Replace([string][char]13,'').Split("wsl.conf`n")[1].Split("clocksource`n")[0]
    $taskAfterRaw=(Get-Content -Raw "$PSScriptRoot/context-after.log").Replace([string][char]13,'')
    $taskAfterConfig=$taskAfterRaw.Split("wsl.conf`n")[1].Split("clocksource`n")[0]
    $taskWinBefore=ReadJson 'windows-wsl-config.json';$taskWinAfter=ReadJson 'context-after-exit.json'
    Require ($taskInitialConfig.Length -gt 0 -and $taskInitialConfig -ceq $taskAfterConfig -and $taskWinBefore.exists -eq $taskWinAfter.windows_wsl_config_exists -and $taskWinBefore.sha256 -ceq $taskWinAfter.windows_wsl_config_sha256) 'Configuration continuity'
    $taskBoots=@(@($taskProfiles|ForEach-Object{$_.original.causal.first.issued_boot})+@($taskRefusals|ForEach-Object{$_.original.causal.first.issued_boot})|Where-Object{$_}|Select-Object -Unique)
    [ordered]@{schema='validation-round-cost-v1';baseline='8ecb405b4068236970bcf72563a72ee3e7ca9c2e';sources=(ReadJson 'sources.json');diagnosis=(Diagnosis);profile_boots=$taskBoots;postflight_boot=(Get-Content -Raw "$PSScriptRoot/local-postflight.log").Trim();configuration_unchanged=$true;profiles=$taskProfiles;refusals=$taskRefusals;steps=$taskSteps;ranges=$taskRanges;shares=$taskShares;matrices=$taskMatrices;complete_profile_cohort=($taskProfiles.Count -eq 6 -and $taskRefusals.Count -eq 0);both_local_matrices_passed=(@($taskMatrices|Where-Object{-not $_.passed}).Count -eq 0);runtime_candidate_adopted=$false;deadline_changed=$false;live_checks_changed=$false;native_gate_open=$false;server=$false;product=$false}
}
