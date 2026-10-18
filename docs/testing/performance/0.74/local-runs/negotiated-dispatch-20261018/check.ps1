param([Parameter(Mandatory)][ValidateSet('194','188','final')][string]$Lane,[string]$Round='lint-final')
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$out=Join-Path $repo 'target/negotiated-dispatch-20261018'
function Run([string]$Name,[scriptblock]$Command) {
    $path=Join-Path $out "$Name.log"
    if (Test-Path -LiteralPath $path) {throw "retain existing receipt $Name"}
    & $Command 2>&1 | Tee-Object -FilePath $path
    if ($LASTEXITCODE -ne 0) {throw "failed $Name"}
}
if ($Lane -ceq 'final') {
    Run "$Round-format" {cargo fmt -p hydracache-long-run-supervisor-074 -p xtask -- --check}
    Run "$Round-registry" {cargo xtask gated-test-check}
    Run "$Round-docs" {cargo xtask doc-check}
    Run "$Round-performance-contract" {cargo xtask performance-contract-check --release 0.74}
    Run "$Round-diff" {git diff --cached --check}
    exit
}
$version=if ($Lane -ceq '194') {'1.94.0'} else {'1.88.0'}
$cache="/home/artur/.cache/hydracache-074-$Lane-check"
$suffix=if ($Lane -ceq '194') {'b26b156d3dcd0098'} else {'d2aba7f58d773234'}
$image="$cache/release/deps/hydracache_long_run_supervisor_074-$suffix"
Run "$Round-gnu-$Lane-check" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" check -p hydracache-long-run-supervisor-074 --all-targets --locked}
Run "$Round-gnu-$Lane-clippy" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings}
foreach ($filter in @('dispatch_exchange::','manager_channel::','original_pipe_message_','worker_context_')) {
    $name=$filter.Replace(':','').Replace('_','-')
    Run "$Round-gnu-$Lane-$name" {wsl -d Ubuntu -- $image $filter --test-threads=1}
}
