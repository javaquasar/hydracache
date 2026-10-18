param([ValidateSet('Baseline','Red','Candidate')][string]$Stage)
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
if ((Get-Location).Path -cne $root) { throw 'run from repository root' }
if ($Stage -ceq 'Baseline') {
    $images=Get-Content (Join-Path $PSScriptRoot '../manager-channel-20261018/images.json') -Raw | ConvertFrom-Json
} else {
    $images=@()
    $compilers=if ($Stage -ceq 'Red') {@('194')} else {@('194','188')}
    foreach ($compiler in $compilers) {
        $version=if ($compiler -ceq '194') {'1.94.0'} else {'1.88.0'}
        $cache="/home/artur/.cache/hydracache-074-$compiler-check"
        $raw=& wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --lib --locked --no-run --message-format=json 2> (Join-Path $PSScriptRoot "$Stage-build-$compiler.log")
        if ($LASTEXITCODE -ne 0) {throw 'build failed'}
        $raw | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "$Stage-build-$compiler.jsonl")
        $artifacts=@($raw | Where-Object {$_.StartsWith('{')} | ForEach-Object {$_ | ConvertFrom-Json} |
            Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
        if ($artifacts.Count -ne 1) {throw 'ambiguous image'}
        $image=[string]$artifacts[0].executable
        if ($image -cnotmatch ('^'+[regex]::Escape($cache)+'/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$')) {throw 'image path'}
        $receipt=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
        if ($LASTEXITCODE -ne 0 -or $receipt -cnotmatch '^([a-f0-9]{64})  ') {throw 'image hash'}
        $images+=@{compiler=$compiler;version=$version;image=$image;sha256=$Matches[1]}
    }
    $images | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "$Stage-images.json")
    git diff --binary HEAD -- tools/long-run-supervisor-074 crates/xtask/tests/support/performance_contract_074_manager_channel.rs docs/COMPAT.md |
        Set-Content -Encoding utf8NoBOM (Join-Path $PSScriptRoot "$Stage-source.patch")
    if ($LASTEXITCODE -ne 0) {throw 'source diff'}
}
foreach ($entry in $images) {
    $image=[string]$entry.image
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $image") {throw 'image differs'}
    "HC074-EXCHANGE stage=$Stage compiler=$($entry.compiler) image_sha256=$($entry.sha256)"
    $filter=if ($Stage -ceq 'Red') {'dispatch_reply_flags_accept_only_zero_or_no_reply_expected_on_method_return'} else {'dispatch_exchange::'}
    & wsl -d Ubuntu -- $image $filter --nocapture --test-threads=1 2>&1 |
        Tee-Object -FilePath (Join-Path $PSScriptRoot "$Stage-$($entry.compiler).log")
    $code=$LASTEXITCODE
    if ($Stage -ceq 'Red') {
        $log=Get-Content (Join-Path $PSScriptRoot "$Stage-$($entry.compiler).log") -Raw
        if ($code -ne 101 -or !$log.Contains('bits=1 error=Reply')) {throw 'unexpected red result'}
    } elseif ($code -ne 0) {throw 'test failed'}
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cne "$($entry.sha256)  $image") {throw 'image changed'}
    "HC074-EXCHANGE stage=$Stage compiler=$($entry.compiler) exit=$code admission=false"
}
