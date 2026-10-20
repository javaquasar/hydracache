$ErrorActionPreference='Stop'
Push-Location (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
try {
    $taskImage=Get-Content -Raw (Join-Path $PSScriptRoot 'bound-194-image.json')|ConvertFrom-Json
    & git diff --exit-code $taskImage.source -- tools/long-run-supervisor-074 Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
    if($LASTEXITCODE -ne 0){throw 'Image inputs differ'}
    $taskHash=((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64))
    if($taskHash -cne $taskImage.sha256){throw 'Image changed'}
    $taskCommit=(& git rev-parse HEAD).Trim()
    [ordered]@{image_source=$taskImage.source;control_commit=$taskCommit;image_sha256=$taskHash;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();serial_control=$true;preceding_cohort_accepted=$false;server=$false;product=$false;clock_changed=$false;deadline_changed=$false}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'serial-identity.json') -Encoding utf8NoBOM
    $taskPrefix=@('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017')
    $taskResults=[ordered]@{}
    foreach($taskSelection in 'live','attribution'){
        $taskLog=Join-Path $PSScriptRoot "serial-194-$taskSelection.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Do not retry a completed control'}
        $taskArgs=if($taskSelection -ceq 'attribution'){@('HC074_CONSUMING_HELD_LIVE_CASE=healthy','HC074_CONSUMING_HELD_COST=1')}else{@()}
        & wsl.exe @taskPrefix @taskArgs /usr/bin/timeout --signal=TERM --kill-after=5s 1200s $taskImage.path privileged_consuming_held_live_matrix --ignored --nocapture --test-threads=1 2>&1|Tee-Object -FilePath $taskLog|Out-Null
        $taskResults[$taskSelection]=$LASTEXITCODE
    }
    if(((& wsl.exe -d Ubuntu -- sha256sum $taskImage.path).Trim().Substring(0,64)) -cne $taskHash -or (& git rev-parse HEAD).Trim() -cne $taskCommit){throw 'Control identity changed'}
    [ordered]@{image_source=$taskImage.source;control_commit=$taskCommit;results=$taskResults;serial_control=$true;live_matrix_passed=($taskResults.live -eq 0);preceding_cohort_accepted=$false;server=$false;product=$false;original_send=$false;full_joined_sigkill=$false;real_role_image_cost=$false}|ConvertTo-Json|Set-Content (Join-Path $PSScriptRoot 'serial-194-result.json') -Encoding utf8NoBOM
}finally{Pop-Location}
