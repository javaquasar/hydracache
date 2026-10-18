param([Parameter(Mandatory)][ValidateSet('194','188','windows','final')][string]$Lane)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$out=Join-Path $repo 'target/original-reply-binding-20261018'
function Run([string]$Name,[scriptblock]$Command) {
    $path=Join-Path $out "$Name.log"
    if (Test-Path -LiteralPath $path) {throw "retain existing receipt $Name"}
    & $Command 2>&1 | Tee-Object -FilePath $path
    if ($LASTEXITCODE -ne 0) {throw "failed $Name"}
}
if ($Lane -ceq 'windows') {
    Run 'windows-check' {cargo check -p hydracache-long-run-supervisor-074 -p xtask --all-targets --locked}
    Run 'windows-clippy' {cargo clippy -p hydracache-long-run-supervisor-074 -p xtask --all-targets --all-features --locked -- -D warnings}
    Run 'windows-units' {cargo test -p hydracache-long-run-supervisor-074 --lib --locked}
    exit
}
if ($Lane -ceq 'final') {
    Run 'final-format' {cargo fmt -p hydracache-long-run-supervisor-074 -p xtask -- --check}
    Run 'final-registry' {cargo xtask gated-test-check}
    Run 'final-docs' {cargo xtask doc-check}
    Run 'final-performance-contract' {cargo xtask performance-contract-check --release 0.74}
    Run 'final-diff' {git diff --cached --check}
    exit
}
$version=if ($Lane -ceq '194') {'1.94.0'} else {'1.88.0'}
$cache="/home/artur/.cache/hydracache-074-$Lane-check"
$suffix=if ($Lane -ceq '194') {'b26b156d3dcd0098'} else {'d2aba7f58d773234'}
$image="$cache/release/deps/hydracache_long_run_supervisor_074-$suffix"
Run "gnu-$Lane-check" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" check -p hydracache-long-run-supervisor-074 --all-targets --locked}
Run "gnu-$Lane-clippy" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings}
Run "gnu-$Lane-build" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --lib --no-run --locked}
Run "gnu-$Lane-image-before" {wsl -d Ubuntu -- sha256sum $image}
foreach ($filter in @('dispatch_exchange::','manager_channel::','original_pipe_message_','worker_context_')) {
    $name=$filter.Replace(':','').Replace('_','-')
    Run "gnu-$Lane-$name" {wsl -d Ubuntu -- $image $filter --nocapture --test-threads=1}
}
Run "gnu-$Lane-matrix" {wsl -d Ubuntu -- $image dispatch_negotiated_private_bus_fault_matrix --ignored --nocapture --test-threads=1}
Run "gnu-$Lane-image-after" {wsl -d Ubuntu -- sha256sum $image}
if ((Get-Content (Join-Path $out "gnu-$Lane-image-before.log") -Raw) -cne (Get-Content (Join-Path $out "gnu-$Lane-image-after.log") -Raw)) {throw 'image identity changed'}
