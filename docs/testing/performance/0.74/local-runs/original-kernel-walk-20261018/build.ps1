$ErrorActionPreference='Stop'
$images=@()
foreach ($compiler in @('194','188')) {
    $version=if ($compiler -ceq '194') {'1.94.0'} else {'1.88.0'}
    $cache="/home/artur/.cache/hydracache-074-$compiler-check"
    $raw=& wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --lib --locked --no-run --message-format=json 2> (Join-Path $PSScriptRoot "build-$compiler.log")
    $exit=$LASTEXITCODE
    $raw | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "build-$compiler.jsonl")
    if ($exit -ne 0) {throw 'build failed'}
    $artifacts=@($raw | Where-Object {$_.StartsWith('{')} | ForEach-Object {$_ | ConvertFrom-Json} | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
    if ($artifacts.Count -ne 1) {throw 'ambiguous image'}
    $path=[string]$artifacts[0].executable
    $hash=(& wsl -d Ubuntu -- sha256sum -- $path) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $hash -cnotmatch '^([a-f0-9]{64})  ') {throw 'image hash'}
    $images+=@{compiler=$compiler;version=$version;image=$path;sha256=$Matches[1]}
    foreach ($filter in @('kernel_walk::tests::','live_walk::tests::','validation_trace_tests::','manager_channel::','dispatch_exchange::','original_pipe_message_','worker_context_','diagnostic_kernel_admission::tests::')) {
        & wsl -d Ubuntu -- $path $filter --test-threads=1 2>&1 | Tee-Object -Append -FilePath (Join-Path $PSScriptRoot "units-$compiler.log")
        if ($LASTEXITCODE -ne 0) {throw "unit $filter"}
    }
}
$images | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot 'images.json')
