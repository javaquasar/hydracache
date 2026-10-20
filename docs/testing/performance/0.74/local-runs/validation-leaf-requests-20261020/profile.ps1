param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain -- tools/long-run-supervisor-074 crates/xtask docs/testing/gated-test-registry.toml)){throw 'Source drift'}
$taskPrefix=Join-Path $PSScriptRoot "profile-$Lane"
if((Test-Path "$taskPrefix.log") -or (Test-Path "$taskPrefix-exit.json")){throw 'Never replace profile'}
$taskReceipt=Get-Content (Join-Path $PSScriptRoot "check-$Lane-exit.json") -Raw|ConvertFrom-Json
if($taskReceipt.failure -or @($taskReceipt.steps|Where-Object exit -ne 0).Count){throw 'Checks failed'}
$taskImages=@(Get-Content (Join-Path $PSScriptRoot "check-$Lane-build.log")|ForEach-Object{
    if($_.StartsWith('{')){
        $taskItem=$_|ConvertFrom-Json
        if($taskItem.reason -ceq 'compiler-artifact' -and $taskItem.target.name -ceq 'hydracache_long_run_supervisor_074' -and $taskItem.profile.test -and $taskItem.executable){$taskItem.executable}
    }
})
if($taskImages.Count -ne 1){throw 'Ambiguous image'}
$taskImage=$taskImages[0]
$taskImageHash=(& wsl.exe -d Ubuntu -- sha256sum -- $taskImage).Split(' ')[0]
if($LASTEXITCODE -ne 0){throw 'Cannot hash image'}
$taskInputs=@(foreach($taskPath in @('tools/long-run-supervisor-074/src/diagnostic_fixed_leaf_alloc_tests.rs','tools/long-run-supervisor-074/tests/support/validation_leaf_requests.rs','tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs','tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs','docs/testing/performance/0.74/qualification-manifest.toml')){
    @{path=$taskPath;sha256=(Get-FileHash $taskPath -Algorithm SHA256).Hash.ToLowerInvariant()}
})
& wsl.exe -d Ubuntu -- /usr/bin/timeout --signal=TERM --kill-after=5s 1200s $taskImage validation_leaf_requests_attribution --ignored --nocapture --test-threads=1 *> "$taskPrefix.log"
$taskExit=[int]$LASTEXITCODE
$taskAfterHash=(& wsl.exe -d Ubuntu -- sha256sum -- $taskImage).Split(' ')[0]
if($LASTEXITCODE -ne 0){throw 'Cannot rehash image'}
[ordered]@{source=$Source;exit=$taskExit;lane=$Lane;image=$taskImage;image_sha256=$taskImageHash;image_after_sha256=$taskAfterHash;inputs=$taskInputs;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false;native_gate_open=$false;runtime_candidate=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
if($taskExit -ne 0 -or $taskAfterHash -cne $taskImageHash){throw 'Profile failed or image changed; never retry into healthy cohort'}
