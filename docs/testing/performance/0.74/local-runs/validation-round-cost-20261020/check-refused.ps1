param([Parameter(Mandatory)][ValidateSet('194','188','xtask')][string]$Lane,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}
$ErrorActionPreference='Stop'
$taskSource=$Source
if((& git rev-parse HEAD).Trim() -cne $taskSource -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)){throw 'Source drift'}
$taskPrefix="$PSScriptRoot/check-$Lane"
if((Test-Path "$taskPrefix.log") -or (Test-Path "$taskPrefix-exit.json")){throw 'Never replace checks'}
if($Lane -ceq 'xtask'){
    & cargo check --locked -p xtask --all-targets *> "$taskPrefix.log"
    $taskExit=[int]$LASTEXITCODE
}else{
    $taskVersion=if($Lane -ceq '194'){'1.94.0'}else{'1.88.0'}
    & wsl.exe -d Ubuntu -- env "CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check" /home/artur/.cargo/bin/cargo "+$taskVersion" check --locked -p hydracache-long-run-supervisor-074 --all-targets *> "$taskPrefix.log"
    $taskExit=[int]$LASTEXITCODE
}
[ordered]@{source=$taskSource;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskExit -ne 0){throw 'Affected package check failed'})][string]$Source)
$ErrorActionPreference='Stop'
$taskSource='9471afb4f2ce297c5c2d9ece77603c7e6a3e5bfe'
if((& git rev-parse HEAD).Trim() -cne $taskSource -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask)){throw 'Source drift'}
$taskPrefix="$PSScriptRoot/check-$Lane"
if((Test-Path "$taskPrefix.log") -or (Test-Path "$taskPrefix-exit.json")){throw 'Never replace checks'}
if($Lane -ceq 'xtask'){
    & cargo check --locked -p xtask --all-targets *> "$taskPrefix.log"
    $taskExit=[int]$LASTEXITCODE
}else{
    $taskVersion=if($Lane -ceq '194'){'1.94.0'}else{'1.88.0'}
    & wsl.exe -d Ubuntu -- env "CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$Lane-check" /home/artur/.cargo/bin/cargo "+$taskVersion" check --locked -p hydracache-long-run-supervisor-074 --all-targets *> "$taskPrefix.log"
    $taskExit=[int]$LASTEXITCODE
}
[ordered]@{source=$taskSource;exit=$taskExit;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskExit -ne 0){throw 'Affected package check failed'}
