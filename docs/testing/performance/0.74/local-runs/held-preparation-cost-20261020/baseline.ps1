$ErrorActionPreference='Stop'
foreach($taskLane in '194','188'){
    $taskImage=Get-Content -Raw "$PSScriptRoot/../held-consuming-live-20261020/traced-$taskLane-image.json"|ConvertFrom-Json
    $taskHash=((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64))
    if($LASTEXITCODE -ne 0 -or $taskHash -cne $taskImage.sha256){throw 'Retained baseline image changed'}
    if(Test-Path "$PSScriptRoot/baseline-$taskLane.log"){throw 'Do not replay baseline'}
    $taskImage|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/baseline-$taskLane-image.json"
    & wsl.exe -d Ubuntu -u root -- env HC074_OWNED_ROOT_FIXTURE=local-only-v1 HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017 HC074_CONSUMING_HELD_LIVE_CASE=healthy HC074_CONSUMING_HELD_COST=1 /usr/bin/timeout --signal=TERM --kill-after=5s 1200s $taskImage.path privileged_consuming_held_live_matrix --ignored --nocapture --test-threads=1 2>&1 | Tee-Object "$PSScriptRoot/baseline-$taskLane.log" | Out-Null
    $taskExit=$LASTEXITCODE
    [ordered]@{source=$taskImage.source;image_sha256=$taskHash;lane=$taskLane;exit=$taskExit;healthy_gate=$false;server=$false;product=$false;deadline_changed=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/baseline-$taskLane-exit.json"
    Write-Output "baseline lane=$taskLane exit=$taskExit"
}
