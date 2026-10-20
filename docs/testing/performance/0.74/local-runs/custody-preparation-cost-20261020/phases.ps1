$ErrorActionPreference='Stop'
$taskRows=@()
foreach($taskCohort in 'Baseline','Candidate'){
    foreach($taskLane in '194','188'){
        foreach($taskSample in 0..2){
            $taskPrefix="$PSScriptRoot/$taskCohort-$taskLane-profile-$taskSample"
            if(-not (Test-Path "$taskPrefix-exit.json")){continue}
            $taskReceipt=Get-Content -Raw "$taskPrefix-exit.json"|ConvertFrom-Json
            foreach($taskLine in Get-Content "$taskPrefix.log"){
                $taskAt=$taskLine.IndexOf('HC074-HELD-PHASE ',[StringComparison]::Ordinal)
                if($taskAt -lt 0){continue}
                $taskPhase=$taskLine.Substring($taskAt+17)|ConvertFrom-Json
                foreach($taskFlag in 'guard_clock_added','deadline_changed','includes_child_cpu','healthy_gate','server','product'){
                    if($taskPhase.$taskFlag -isnot [bool] -or $taskPhase.$taskFlag){throw 'Phase scope upgraded'}
                }
                $taskRows+=,[ordered]@{cohort=$taskCohort;lane=$taskLane;sample=$taskSample;source=$taskReceipt.source;exit=$taskReceipt.exit;phase=$taskPhase.phase;returned=$taskPhase.returned;wall_ns=$taskPhase.wall_ns;process_cpu_ns=$taskPhase.process_cpu_ns;authentication_calls=$taskPhase.authentication_calls;kernel=$taskPhase.kernel;live=$taskPhase.live;boundaries=$taskPhase.boundaries}
            }
        }
    }
}
$taskRows|ConvertTo-Json -Depth 20
