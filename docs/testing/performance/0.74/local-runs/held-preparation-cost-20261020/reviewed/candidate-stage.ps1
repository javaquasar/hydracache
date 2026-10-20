param([Parameter(Mandatory)][ValidateSet('Checks','Profiles')][string]$Stage,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
function Frozen {
    if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml)){throw 'Candidate source changed'}
}
function Run([string]$Name,[string[]]$Arguments){
    if(Test-Path "$PSScriptRoot/candidate-$Name-exit.json"){throw 'Never replay a recorded invocation'}
    & wsl.exe @Arguments 2>&1|Tee-Object "$PSScriptRoot/candidate-$Name.log"|Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$Source;selection=$Name;exit=$taskExit;server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/candidate-$Name-exit.json"
    return $taskExit
}
Frozen
if($Stage -ceq 'Checks'){
    foreach($taskLane in '194','188'){
        $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
        $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskLane-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
        if((Run "$taskLane-build" ($taskCargo+@('test','--locked','--release','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json'))) -ne 0){throw 'Build refused'}
        $taskRows=@(Get-Content "$PSScriptRoot/candidate-$taskLane-build.log"|Where-Object{$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object{$_|ConvertFrom-Json})
        $taskImage=@($taskRows|Where-Object{$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
        if($taskImage.Count -ne 1){throw 'Image selection'}
        $taskPath=$taskImage[0].executable
        $taskHash=((& wsl.exe -d Ubuntu -- sha256sum $taskPath).Trim().Substring(0,64))
        [ordered]@{source=$Source;lane=$taskLane;path=$taskPath;sha256=$taskHash;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/candidate-$taskLane-image.json"
        foreach($taskSelection in @(@('observer','preparation_cost_observer'),@('bridge','consuming_held'),@('trust','held_runtime::tests::'),@('kernel','diagnostic_kernel_admission::tests::'))){
            $taskArgs=@('-d','Ubuntu','--',$taskPath,$taskSelection[1])
            if($taskSelection[0] -ceq 'trust'){$taskArgs+=@('--skip','consuming_held_bridge')}
            if((Run "$taskLane-$($taskSelection[0])" ($taskArgs+@('--nocapture','--test-threads=1'))) -ne 0){throw 'Targeted check refused'}
        }
        if((Run "$taskLane-lint" ($taskCargo+@('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))) -ne 0){throw 'Strict lint refused'}
    }
}else{
    # Invoke only after all own compiler, source-contract and allocation jobs finish.
    $taskTest='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::privileged_root::cgroup::held_live::privileged_consuming_held_live_matrix'
    foreach($taskLane in '194','188'){
        $taskImage=Get-Content -Raw "$PSScriptRoot/candidate-$taskLane-image.json"|ConvertFrom-Json
        if($taskImage.source -cne $Source -or $taskImage.recipe_sha256 -cne (Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Image recipe/source mismatch'}
        foreach($taskSelection in 'build','observer','bridge','trust','kernel','lint'){
            if((Get-Content -Raw "$PSScriptRoot/candidate-$taskLane-$taskSelection-exit.json"|ConvertFrom-Json).exit -ne 0){throw 'Required source check refused'}
        }
        $taskRoot=@('-d','Ubuntu','-u','root','--','env','-u','HC074_CONSUMING_HELD_LIVE_CASE','-u','HC074_CONSUMING_HELD_COST','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017')
        foreach($taskSample in 0..2){
            $taskExit=Run "$taskLane-profile-$taskSample" ($taskRoot+@('HC074_CONSUMING_HELD_LIVE_CASE=healthy','HC074_CONSUMING_HELD_COST=1','/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage.path,$taskTest,'--exact','--ignored','--nocapture','--test-threads=1'))
            Write-Output "candidate lane=$taskLane sample=$taskSample exit=$taskExit"
        }
        $taskExit=Run "$taskLane-matrix" ($taskRoot+@('/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage.path,$taskTest,'--exact','--ignored','--nocapture','--test-threads=1'))
        Write-Output "candidate lane=$taskLane unprofiled_matrix_exit=$taskExit"
        if(((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64)) -cne $taskImage.sha256){throw 'Image changed'}
    }
}
Frozen
