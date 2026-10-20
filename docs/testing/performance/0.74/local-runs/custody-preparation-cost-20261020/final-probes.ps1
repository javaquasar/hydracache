param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)){throw 'Selected source changed'}
function Run([string]$Name,[string[]]$Arguments){
    if(Test-Path "$PSScriptRoot/$Name-exit.json"){throw 'Never replay an invocation'}
    & wsl.exe @Arguments 2>&1|Tee-Object "$PSScriptRoot/$Name.log"|Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$Source;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/$Name-exit.json"
    $taskExit
}
$taskTest='diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::privileged_root::cgroup::held_live::privileged_consuming_held_live_matrix'
foreach($taskLane in '194','188'){
    $taskImage=Get-Content -Raw "$PSScriptRoot/Final-$taskLane-plain-image.json"|ConvertFrom-Json
    if($taskImage.source -cne $Source){throw 'Wrong image source'}
    if((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64) -cne $taskImage.sha256){throw 'Image changed before observations'}
    if((Run "fresh-$taskLane" @('-d','Ubuntu','--',$taskImage.path,'fresh_kernel_observation_attribution','--ignored','--nocapture','--test-threads=1')) -ne 0){throw 'Fresh count contract refused'}
    $taskExit=Run "matrix-$taskLane" @('-d','Ubuntu','-u','root','--','env','-u','HC074_CONSUMING_HELD_LIVE_CASE','-u','HC074_CONSUMING_HELD_COST','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017','/usr/bin/timeout','--signal=TERM','--kill-after=5s','1200s',$taskImage.path,$taskTest,'--exact','--ignored','--nocapture','--test-threads=1')
    Write-Output "lane=$taskLane full_matrix_exit=$taskExit"
    if((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64) -cne $taskImage.sha256){throw 'Image changed after observations'}
}
