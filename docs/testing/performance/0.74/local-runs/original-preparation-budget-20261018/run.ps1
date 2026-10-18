param([ValidateSet('Baseline','Candidate')][string]$Stage)
$ErrorActionPreference='Stop'
$images=@()
if ($Stage -ceq 'Baseline') {
    $images=Get-Content (Join-Path $PSScriptRoot '../original-manager-grouped-message-20261018/Candidate-images.json') -Raw | ConvertFrom-Json
} else {
$compilers=if ($Stage -ceq 'Red') {@('194')} else {@('194','188')}
foreach ($compiler in $compilers) {
    $version=if ($compiler -ceq '194') {'1.94.0'} else {'1.88.0'}
    $cache="/home/artur/.cache/hydracache-074-$compiler-check"
    $raw=& wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --lib --locked --no-run --message-format=json 2> (Join-Path $PSScriptRoot "$Stage-build-$compiler.log")
    $exit=$LASTEXITCODE
    $raw | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "$Stage-build-$compiler.jsonl")
    if ($Stage -ceq 'Red') {
        $log=(Get-Content (Join-Path $PSScriptRoot "$Stage-build-$compiler.log") -Raw)+($raw -join "`n")
        if ($exit -ne 101 -or !$log.Contains('prepare_fixed_manager_message') -or !$log.Contains('E0599')) {throw 'unexpected red'}
        'HC074-PREPARATION-BUDGET expected_missing_constructor=true exit=101';exit 0
    }
    if ($exit -ne 0) {throw 'build failed'}
    $artifacts=@($raw | Where-Object {$_.StartsWith('{')} | ForEach-Object {$_ | ConvertFrom-Json} | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
    if ($artifacts.Count -ne 1) {throw 'ambiguous image'}
    $path=[string]$artifacts[0].executable
    $hash=(& wsl -d Ubuntu -- sha256sum -- $path) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $hash -cnotmatch '^([a-f0-9]{64})  ') {throw 'image hash'}
    $images+=@{compiler=$compiler;version=$version;image=$path;sha256=$Matches[1]}
}
}
$images | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "$Stage-images.json")
foreach ($entry in $images) {
    $hash=(& wsl -d Ubuntu -- sha256sum -- $entry.image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $hash -cne "$($entry.sha256)  $($entry.image)") {throw 'image drift'}
    "HC074-PREPARATION-BUDGET image=$($entry.sha256) compiler=$($entry.compiler)" | Tee-Object -FilePath (Join-Path $PSScriptRoot "$Stage-$($entry.compiler).log")
    foreach ($filter in @('manager_channel::','dispatch_exchange::','original_pipe_message_')) {
        & wsl -d Ubuntu -- $entry.image $filter --nocapture --test-threads=1 2>&1 | Tee-Object -Append -FilePath (Join-Path $PSScriptRoot "$Stage-$($entry.compiler).log")
        if ($LASTEXITCODE -ne 0) {throw "test $filter"}
    }
    $hash=(& wsl -d Ubuntu -- sha256sum -- $entry.image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $hash -cne "$($entry.sha256)  $($entry.image)") {throw 'image changed'}
}
