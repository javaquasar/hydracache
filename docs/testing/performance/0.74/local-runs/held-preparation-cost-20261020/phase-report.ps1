param([Parameter(Mandatory)][ValidateSet('instrumented','candidate')][string]$Cohort)
$ErrorActionPreference='Stop'
$taskPrefix='HC074-HELD-PHASE '
$taskResult=@()
foreach($taskLane in '194','188'){
    foreach($taskSample in 0..2){
        $taskExit=Get-Content -Raw "$PSScriptRoot/$Cohort-$taskLane-profile-$taskSample-exit.json"|ConvertFrom-Json
        $taskRaw=Get-Content "$PSScriptRoot/$Cohort-$taskLane-profile-$taskSample.log"
        foreach($taskLine in $taskRaw){
            $taskIndex=$taskLine.IndexOf($taskPrefix,[StringComparison]::Ordinal)
            if($taskIndex -lt 0){continue}
            $taskPhase=$taskLine.Substring($taskIndex+$taskPrefix.Length)|ConvertFrom-Json
            foreach($taskFlag in 'guard_clock_added','deadline_changed','includes_child_cpu','healthy_gate','server','product'){
                if($taskPhase.$taskFlag -isnot [bool] -or $taskPhase.$taskFlag -cne $false){throw 'Phase scope upgrade'}
            }
            $taskResult += [ordered]@{cohort=$Cohort;lane=$taskLane;sample=$taskSample;invocation_exit=$taskExit.exit;phase=$taskPhase.phase;returned=$taskPhase.returned;wall_ns=$taskPhase.wall_ns;process_cpu_ns=$taskPhase.process_cpu_ns;authentication_calls=$taskPhase.authentication_calls;authentication_ns=$taskPhase.authentication_ns;projection=$taskPhase.kernel[3];origin=$taskPhase.kernel[4];boot=$taskPhase.kernel[5];namespace=$taskPhase.kernel[6];document=$taskPhase.kernel[9]}
        }
    }
}
$taskResult|ConvertTo-Json -Depth 8
