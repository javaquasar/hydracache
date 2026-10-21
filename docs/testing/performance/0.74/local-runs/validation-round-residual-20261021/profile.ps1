param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane,[Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml)){throw 'Source drift'}
& git diff --quiet b074f8f9f3d8f9766b3b04c59a8291941a7e3b1d HEAD -- tools/long-run-supervisor-074 crates/xtask Cargo.lock
if($LASTEXITCODE -ne 0){throw 'Runtime/observer changed'}
$taskCheck=Get-Content (Join-Path $PSScriptRoot "check-Verified-$Lane-exit.json") -Raw|ConvertFrom-Json
if($taskCheck.failure -or $taskCheck.source -cne $Source -or $taskCheck.steps.Count -ne 11 -or @($taskCheck.steps|Where-Object exit -ne 0).Count){throw 'Checks incomplete'}
$taskContext=Get-Content (Join-Path $PSScriptRoot 'context-Verified-Before.json') -Raw|ConvertFrom-Json
$taskOldInputs=Get-Content (Join-Path $PSScriptRoot '../validation-round-cost-20261020/inputs-before.json') -Raw|ConvertFrom-Json
if(-not [System.Text.Json.Nodes.JsonNode]::DeepEquals([System.Text.Json.Nodes.JsonNode]::Parse(($taskContext.bundle|ConvertTo-Json -Depth 6 -Compress)),[System.Text.Json.Nodes.JsonNode]::Parse(($taskOldInputs.bundle|ConvertTo-Json -Depth 6 -Compress)))){throw 'Original bundle changed'}
& git merge-base --is-ancestor $taskContext.source $Source
if($LASTEXITCODE -ne 0 -or @($taskContext.commands|Where-Object exit -ne 0).Count -or ($taskContext.commands|Where-Object name -ceq 'fixtures').output.Trim()){throw 'Context refused'}
$taskImages=@(Get-Content (Join-Path $PSScriptRoot "check-Verified-$Lane-build.log")|Where-Object {$_.StartsWith('{')}|ForEach-Object {$_|ConvertFrom-Json}|Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -and $_.executable})
if($taskImages.Count -ne 1 -or $taskImages[0].features.Count){throw 'Require exactly one default-feature test image'}
$taskImage=$taskImages[0].executable
$taskStopped=$false
foreach($taskSample in 0..2){
    $taskPrefix=Join-Path $PSScriptRoot "$Lane-Profile-$taskSample"
    if((Test-Path "$taskPrefix.log") -or (Test-Path "$taskPrefix-exit.json")){throw 'Never replace a profile'}
    if($taskStopped){continue}
    $taskBefore=(& wsl.exe -d Ubuntu -- sha256sum -- $taskImage).Split(' ')[0]
    if($LASTEXITCODE -ne 0){throw 'Image unavailable'}
    & wsl.exe -d Ubuntu -u root -- env HC074_OWNED_ROOT_FIXTURE=local-only-v1 HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017 HC074_CONSUMING_HELD_LIVE_CASE=healthy HC074_CONSUMING_HELD_COST=1 /usr/bin/timeout --signal=TERM --kill-after=5s 1200s $taskImage privileged_consuming_held_live_matrix --ignored --nocapture --test-threads=1 *> "$taskPrefix.log"
    $taskCode=[int]$LASTEXITCODE
    $taskAfter=(& wsl.exe -d Ubuntu -- sha256sum -- $taskImage).Split(' ')[0]
    $taskHashExit=[int]$LASTEXITCODE
    @{source=$Source;lane=$Lane;sample=$taskSample;exit=$taskCode;image=$taskImage;image_sha256=$taskBefore;image_after_sha256=$taskAfter;image_hash_exit=$taskHashExit;log_sha256=(Hash "$taskPrefix.log");recipe_sha256=(Hash $PSCommandPath);allocator_probe=$false;full_matrix=$false;original_send=$false;server=$false;product=$false;native_gate_open=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
    if($taskCode -ne 0){$taskStopped=$true}
    if($taskHashExit -ne 0 -or $taskBefore -cne $taskAfter){throw 'Image changed; no continuation'}
}
@{source=$Source;lane=$Lane;stopped_on_refusal=$taskStopped;recipe_sha256=(Hash $PSCommandPath);server=$false;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "$Lane-cohort.json")
if($taskStopped){Write-Output "$Lane retained original refusal; later samples not run"}
