param([Parameter(Mandatory)][ValidateSet('Capture')][string]$Stage,
      [Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask docs/testing/gated-test-registry.toml docs/testing/performance/0.74/authority-owners-contract.toml)){throw 'Source drift'}
$taskPrefix=Join-Path $PSScriptRoot "$Stage-$Lane"
if((Test-Path "$taskPrefix-exit.json") -or (Test-Path "$taskPrefix-breakdown.log") -or (Test-Path "$taskPrefix-inspect.log")){throw 'Never replace observations'}
$taskReceipt=Get-Content (Join-Path $PSScriptRoot "$Stage-check-$Lane-exit.json") -Raw|ConvertFrom-Json
if($taskReceipt.failure -or @($taskReceipt.steps|Where-Object exit -ne 0).Count){throw 'Checks failed'}
$taskImages=@(Get-Content (Join-Path $PSScriptRoot "$Stage-check-$Lane-build.log")|ForEach-Object{
    if($_.StartsWith('{')){
        $taskItem=$_|ConvertFrom-Json
        if($taskItem.reason -ceq 'compiler-artifact' -and $taskItem.target.name -ceq 'hydracache_long_run_supervisor_074' -and $taskItem.profile.test -and $taskItem.executable){$taskItem.executable}
    }
})
if($taskImages.Count -ne 1){throw 'Ambiguous test image'}
$taskImage=$taskImages[0]
$taskHash=(& wsl.exe -d Ubuntu -- sha256sum -- $taskImage).Split(' ')[0]
if($LASTEXITCODE -ne 0){throw 'Image hash failed'}
$taskInputs=@(foreach($taskPath in @('tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs','tools/long-run-supervisor-074/tests/support/authority_owners_controls.rs','tools/long-run-supervisor-074/tests/support/authority_owners_alloc.rs','tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs','docs/testing/performance/0.74/authority-owners-contract.toml','docs/testing/performance/0.74/qualification-manifest.toml')){
    $taskBlob=(& git rev-parse "${Source}:$taskPath").Trim()
    if($LASTEXITCODE -ne 0){throw 'Missing committed input'}
    $taskRaw=[IO.File]::ReadAllText((Join-Path $taskRepo $taskPath)).Replace("`r",'')
    $taskGit=((& git show "${Source}:$taskPath") -join "`n")+"`n"
    if($LASTEXITCODE -ne 0 -or $taskRaw -cne $taskGit){throw 'Input bytes differ after LF normalization'}
    @{path=$taskPath;blob=$taskBlob;normalized_sha256=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($taskRaw))).ToLowerInvariant()}
})
$taskSteps=@()
foreach($taskCase in @(@('breakdown','authority_owners_allocation_attribution'),@('inspect','validation_leaf_requests_attribution'))){
    & wsl.exe -d Ubuntu -- /usr/bin/timeout --signal=TERM --kill-after=5s 1200s $taskImage $taskCase[1] --ignored --nocapture --test-threads=1 *> "$taskPrefix-$($taskCase[0]).log"
    $taskCode=[int]$LASTEXITCODE
    $taskSteps+=@{name=$taskCase[0];exit=$taskCode}
    if($taskCode -ne 0){break}
}
$taskAfterHash=(& wsl.exe -d Ubuntu -- sha256sum -- $taskImage).Split(' ')[0]
if($LASTEXITCODE -ne 0){throw 'Image rehash failed'}
[ordered]@{stage=$Stage;lane=$Lane;source=$Source;image=$taskImage;image_sha256=$taskHash;image_after_sha256=$taskAfterHash;inputs=$taskInputs;steps=$taskSteps;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false;native_gate_open=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskSteps.Count -ne 2 -or @($taskSteps|Where-Object exit -ne 0).Count -or $taskHash -cne $taskAfterHash){throw 'Profile failed; never retry into healthy cohort'}
