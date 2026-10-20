param([Parameter(Mandatory)][ValidateSet('Baseline','Candidate')][string]$Cohort,
      [Parameter(Mandatory)][ValidateSet('Build','Probe')][string]$Stage,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
function Frozen {
    if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml)){throw 'Selected source changed'}
}
function Run([string]$Name,[string[]]$Arguments){
    $taskPrefix="$PSScriptRoot/$Cohort-$Name"
    if(Test-Path "$taskPrefix-exit.json"){throw 'Never replay a recorded invocation'}
    & wsl.exe @Arguments 2>&1|Tee-Object "$taskPrefix.log"|Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$Source;selection=$Name;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
    $taskExit
}
Frozen
foreach($taskLane in '194','188'){
    $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
    if($Stage -ceq 'Build'){
        $taskCargo=@('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskLane-check",'/home/artur/.cargo/bin/cargo',"+$taskVersion")
        if((Run "$taskLane-build" ($taskCargo+@('test','--locked','--release','-p','hydracache-long-run-supervisor-074','--lib','--no-run','--message-format=json'))) -ne 0){throw 'Build refused'}
        $taskRows=@(Get-Content "$PSScriptRoot/$Cohort-$taskLane-build.log"|Where-Object{$_.StartsWith('{',[StringComparison]::Ordinal)}|ForEach-Object{$_|ConvertFrom-Json})
        $taskImages=@($taskRows|Where-Object{$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
        if($taskImages.Count -ne 1){throw 'Image selection'}
        $taskPath=$taskImages[0].executable
        [ordered]@{source=$Source;lane=$taskLane;path=$taskPath;sha256=(& wsl.exe -d Ubuntu -- sha256sum $taskPath).Trim().Substring(0,64);recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$Cohort-$taskLane-image.json"
        foreach($taskSelection in 'custody_boundary_capture','preparation_cost_observer'){
            if((Run "$taskLane-$taskSelection" @('-d','Ubuntu','--',$taskPath,$taskSelection,'--nocapture','--test-threads=1')) -ne 0){throw 'Observer regression refused'}
        }
    }else{
        # No own build, lint, test or other cost job may overlap these probes.
        $taskImage=Get-Content -Raw "$PSScriptRoot/$Cohort-$taskLane-image.json"|ConvertFrom-Json
        if($taskImage.source -cne $Source -or $taskImage.recipe_sha256 -cne (Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()){throw 'Image recipe mismatch'}
        foreach($taskSample in 0..2){
            $taskExit=Run "$taskLane-profile-$taskSample" @('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017','HC074_CONSUMING_HELD_LIVE_CASE=healthy','HC074_CONSUMING_HELD_COST=1','/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage.path,'privileged_consuming_held_live_matrix','--ignored','--nocapture','--test-threads=1')
            Write-Output "$Cohort lane=$taskLane sample=$taskSample exit=$taskExit"
        }
        if((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64) -cne $taskImage.sha256){throw 'Image changed'}
    }
}
Frozen
