$ErrorActionPreference='Stop'
$taskSources=Get-Content -Raw "$PSScriptRoot/sources.json"|ConvertFrom-Json
if((& git rev-parse HEAD).Trim() -cne $taskSources.final -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)){throw 'Source drift'}
foreach($taskLane in '194','188'){
    if(Test-Path "$PSScriptRoot/fresh-$taskLane-exit.json"){throw 'Never replace fresh observations'}
    $taskImage=Get-Content -Raw "$PSScriptRoot/Final-$taskLane-plain-image.json"|ConvertFrom-Json
    if($taskImage.source -cne $taskSources.final -or (& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64) -cne $taskImage.sha256){throw 'Image drift'}
    & wsl.exe -d Ubuntu -- $taskImage.path fresh_kernel_observation_attribution --ignored --nocapture --test-threads=1 *> "$PSScriptRoot/fresh-$taskLane.log"
    $taskExit=[int]$LASTEXITCODE
    [ordered]@{source=$taskSources.final;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/fresh-$taskLane-exit.json"
    Write-Output "fresh $taskLane exit=$taskExit"
    if($taskExit -ne 0){throw 'Fresh control refused'}
}
