#requires -Version 7.2
param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function WriteNew([string]$Path,$Value){
    $taskBytes=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject $Value -Depth 30)+"`n")
    $taskStream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
    try{$taskStream.Write($taskBytes);$taskStream.Flush($true)}finally{$taskStream.Dispose()}
}
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain --untracked-files=no)){throw 'Exact clean committed source required'}
$taskName='held-material-live-20261021'
$taskPrefix=Join-Path $taskRepo "target/$taskName"
$taskPacket=Join-Path $taskRepo "docs/testing/performance/0.74/local-runs/$taskName"
if((Test-Path $taskPacket) -or @('launch.intent.json','194-build.log','188-build.log','preparation.json','pins.json','session.log','launch-exit.json'|Where-Object {Test-Path -LiteralPath "$taskPrefix-$_"}).Count){throw 'Never resume or overwrite this session'}
WriteNew "$taskPrefix-launch.intent.json" @{source=$Source;server=$false;product=$false;single_profile_session=$true}
$taskPreparation=@{}
foreach($taskLane in @('194','188')){
    $taskVersion=if($taskLane -ceq '194'){'1.94.0'}else{'1.88.0'}
    $taskBuildLog="$taskPrefix-$taskLane-build.log"
    & wsl.exe -d Ubuntu -u artur -- env RUSTUP_AUTO_INSTALL=0 "CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskLane-check" /home/artur/.cargo/bin/cargo "+$taskVersion" test --offline --locked --release -p hydracache-long-run-supervisor-074 --lib --no-run --message-format=json *> $taskBuildLog
    $taskCode=[int]$LASTEXITCODE
    if($taskCode -ne 0){throw "Default image build refused: $taskLane; preserve log, no profiles"}
    $taskArtifacts=@(Get-Content $taskBuildLog|Where-Object {$_.StartsWith('{')}|ForEach-Object {$_|ConvertFrom-Json}|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
    if($taskArtifacts.Count -ne 1 -or $taskArtifacts[0].features.Count){throw 'Exactly one default-feature image required'}
    $taskImage=$taskArtifacts[0].executable
    $taskHashOutput=& wsl.exe -d Ubuntu -u artur -- sha256sum -- $taskImage
    if($LASTEXITCODE -ne 0){throw 'Cannot bind fresh image'}
    $taskPreparation[$taskLane]=@{source=$Source;exit=$taskCode;image=$taskImage;sha256=$taskHashOutput.Split(' ')[0];build_log_sha256=(Hash $taskBuildLog);build_log=[IO.Path]::GetFileName($taskBuildLog)}
}
WriteNew "$taskPrefix-preparation.json" $taskPreparation
$taskWslRepo='/mnt/c/Workspace/prj/jq/cashe/hydracache-074'
$taskBundle=(Get-Content docs/testing/performance/0.74/local-runs/validation-round-residual-20261021/context-Verified-Before.json -Raw|ConvertFrom-Json).bundle
WriteNew "$taskPrefix-pins.json" @{source=$Source;bundle=$taskBundle;images=@{'194'=$taskPreparation['194'].sha256;'188'=$taskPreparation['188'].sha256};preparation="$taskWslRepo/target/$taskName-preparation.json"}
& wsl.exe -d Ubuntu -u root -- /usr/bin/timeout --signal=TERM --kill-after=15s 7530s /usr/bin/python3 -B "$taskWslRepo/tools/local-profile-session-074/run_material_live.py" --source $Source --pins "$taskWslRepo/target/$taskName-pins.json" --output "$taskWslRepo/docs/testing/performance/0.74/local-runs/$taskName" *> "$taskPrefix-session.log"
$taskExit=[int]$LASTEXITCODE
WriteNew "$taskPrefix-launch-exit.json" @{source=$Source;exit=$taskExit;single_profile_session=$true;session_log_sha256=(Hash "$taskPrefix-session.log");recipe_sha256=(Hash $PSCommandPath);server=$false;product=$false}
Write-Output "Original material session exited $taskExit; retain every outcome, never rerun this packet."
if($taskExit -ne 0){exit $taskExit}
