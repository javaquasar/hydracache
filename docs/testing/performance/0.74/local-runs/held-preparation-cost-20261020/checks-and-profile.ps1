param([Parameter(Mandatory)][ValidateSet('instrumented','candidate')][string]$Cohort,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
Push-Location $taskRepo
try{
    if((& git rev-parse HEAD).Trim() -cne $Source){throw 'Wrong source'}
    $taskSelected=@('tools/long-run-supervisor-074','crates/xtask','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    if(& git status --porcelain -- @taskSelected){throw 'Uncommitted selected source'}
    function Run([string]$Name,[string[]]$Arguments){
        $taskLog="$PSScriptRoot/$Cohort-$Name.log"
        if(Test-Path $taskLog){throw 'Do not replay a recorded invocation'}
        & wsl.exe @Arguments 2>&1 | Tee-Object $taskLog | Out-Null
        $taskExit=$LASTEXITCODE
        [ordered]@{source=$Source;cohort=$Cohort;selection=$Name;exit=$taskExit;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$Cohort-$Name-exit.json"
        return $taskExit
    }
    $taskImages=@{}
    foreach($taskLane in '194','188'){
        $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
        $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskLane-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
        if((Run "$taskLane-build" ($taskCargo+@('test','--locked','--release','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json'))) -ne 0){throw 'Build refused'}
        $taskRows=@(Get-Content "$PSScriptRoot/$Cohort-$taskLane-build.log"|Where-Object{$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object{$_|ConvertFrom-Json})
        $taskImage=@($taskRows|Where-Object{$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
        if($taskImage.Count -ne 1){throw 'Image selection'}
        $taskPath=$taskImage[0].executable
        $taskHash=((& wsl.exe -d Ubuntu -- sha256sum $taskPath).Trim().Substring(0,64))
        $taskImages[$taskLane]=[ordered]@{source=$Source;lane=$taskLane;path=$taskPath;sha256=$taskHash;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}
        $taskImages[$taskLane]|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$Cohort-$taskLane-image.json"
        foreach($taskSelection in @(@('observer','preparation_cost_observer'),@('bridge','consuming_held'),@('trust','held_runtime::tests::'),@('kernel','diagnostic_kernel_admission::tests::'))){
            $taskArgs=@('-d','Ubuntu','--',$taskPath,$taskSelection[1])
            if($taskSelection[0] -ceq 'trust'){$taskArgs+=@('--skip','consuming_held_bridge')}
            if((Run "$taskLane-$($taskSelection[0])" ($taskArgs+@('--nocapture','--test-threads=1'))) -ne 0){throw "Targeted check refused: $taskLane $taskSelection"}
        }
        if((Run "$taskLane-lint" ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))) -ne 0){throw 'Strict lint refused'}
    }
    # All own builds/checks finish before any phase-cost attempt.
    foreach($taskLane in '194','188'){
        $taskImage=$taskImages[$taskLane]
        foreach($taskSample in 0..2){
            $taskArgs=@('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017','HC074_CONSUMING_HELD_LIVE_CASE=healthy','HC074_CONSUMING_HELD_COST=1','/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage.path,'privileged_consuming_held_live_matrix','--ignored','--nocapture','--test-threads=1')
            $taskExit=Run "$taskLane-profile-$taskSample" $taskArgs
            Write-Output "$Cohort lane=$taskLane sample=$taskSample exit=$taskExit"
        }
        if(((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64)) -cne $taskImage.sha256){throw 'Image changed during observations'}
    }
    if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- @taskSelected)){throw 'Source changed during control'}
}finally{Pop-Location}
