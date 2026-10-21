param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain --untracked-files=no)){throw 'Require exact clean committed source'}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d HEAD -- tools/long-run-supervisor-074 crates Cargo.lock
if($LASTEXITCODE -ne 0){throw 'Runtime/observer source changed'}
$taskPrior='docs/testing/performance/0.74/local-runs/validation-round-residual-20261021'
& "$taskRepo/$taskPrior/preflight-review.ps1" -Mode Review
$taskPrefix=Join-Path $taskRepo 'target/single-session-rounds-20261021'
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/single-session-rounds-20261021'
if((Test-Path $taskPacket) -or @(Get-ChildItem -LiteralPath (Join-Path $taskRepo 'target') -Filter 'single-session-rounds-20261021-*').Count){throw 'Never retry or overwrite preparation/session'}
$taskLaunchIntent=ConvertTo-Json -InputObject @{source=$Source;server=$false;product=$false;single_profile_session=$true}
New-Item -ItemType File -Path "$taskPrefix-launch.intent.json" -Value $taskLaunchIntent -ErrorAction Stop|Out-Null
$taskPreparation=@{}
foreach($taskLane in @('194','188')){
    $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
    $taskBuildLog="$taskPrefix-$taskLane-build.log"
    & wsl.exe -d Ubuntu -- env RUSTUP_AUTO_INSTALL=0 "CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskLane-check" /home/artur/.cargo/bin/cargo "+$taskVersion" test --locked --release -p hydracache-long-run-supervisor-074 --lib --no-run --message-format=json *> $taskBuildLog
    $taskCode=[int]$LASTEXITCODE
    if($taskCode -ne 0){throw "Fresh default image build failed: $taskLane (raw log retained)"}
    $taskArtifacts=@(Get-Content $taskBuildLog|Where-Object {$_.StartsWith('{')}|ForEach-Object {$_|ConvertFrom-Json}|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
    if($taskArtifacts.Count -ne 1 -or $taskArtifacts[0].features.Count){throw 'Exactly one default-feature image required'}
    $taskImage=$taskArtifacts[0].executable
    $taskHashOutput=& wsl.exe -d Ubuntu -- sha256sum -- $taskImage
    if($LASTEXITCODE -ne 0){throw 'Cannot bind fresh image'}
    $taskPreparation[$taskLane]=@{source=$Source;exit=$taskCode;image=$taskImage;sha256=$taskHashOutput.Split(' ')[0];build_log_sha256=(Hash $taskBuildLog);build_log=[IO.Path]::GetFileName($taskBuildLog)}
}
$taskPreparationPath="$taskPrefix-preparation.json"
ConvertTo-Json -InputObject $taskPreparation -Depth 8|Set-Content -Encoding utf8NoBOM $taskPreparationPath
$taskWslRepo='/mnt/c/Workspace/prj/jq/cashe/hydracache-074'
$taskPins=@{source=$Source;bundle=(Get-Content "$taskPrior/context-Verified-Before.json" -Raw|ConvertFrom-Json).bundle;images=@{194=$taskPreparation['194'].sha256;188=$taskPreparation['188'].sha256};preparation="$taskWslRepo/target/single-session-rounds-20261021-preparation.json"}
ConvertTo-Json -InputObject $taskPins -Depth 8|Set-Content -Encoding utf8NoBOM "$taskPrefix-pins.json"
# This is the ONE WSL entry enclosing fresh metadata, all profiles, postflight.
# Outer containment includes prechecks; it never changes Rust admission windows.
& wsl.exe -d Ubuntu -u root -- /usr/bin/timeout --signal=TERM --kill-after=15s 7530s /usr/bin/python3 -B "$taskWslRepo/tools/local-profile-session-074/runner.py" --source $Source --pins "$taskWslRepo/target/single-session-rounds-20261021-pins.json" --output "$taskWslRepo/docs/testing/performance/0.74/local-runs/single-session-rounds-20261021" *> "$taskPrefix-session.log"
$taskExit=[int]$LASTEXITCODE
@{source=$Source;exit=$taskExit;single_profile_session=$true;session_log_sha256=(Hash "$taskPrefix-session.log");recipe_sha256=(Hash $PSCommandPath);server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-launch-exit.json"
Write-Output "Single session exited $taskExit; preserve all receipts, never rerun this packet."
if($taskExit -ne 0){exit $taskExit}
