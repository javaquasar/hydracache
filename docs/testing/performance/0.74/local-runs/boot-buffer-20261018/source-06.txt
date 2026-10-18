param([ValidateSet('windows','194','188','final')][string]$Lane='windows')
$ErrorActionPreference='Stop'
$root=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $root
$out=Join-Path $root 'target/boot-buffer-20261018/checks'
New-Item -ItemType Directory -Force -Path $out | Out-Null
function Run([string]$Name,[scriptblock]$Command) {
    $log=Join-Path $out "$Name.log"
    if (Test-Path -LiteralPath $log) {throw "retain first receipt: $log"}
    & $Command 2>&1 | Tee-Object -FilePath $log
    if ($LASTEXITCODE -ne 0) {throw "$Name failed: $LASTEXITCODE"}
}
if ($Lane -ceq 'windows') {
    Run 'format' {cargo fmt -p hydracache-long-run-supervisor-074 -p xtask -- --check}
    Run 'windows-check' {cargo check -p hydracache-long-run-supervisor-074 -p xtask --all-targets --locked}
    Run 'windows-clippy' {cargo clippy -p hydracache-long-run-supervisor-074 -p xtask --all-targets --all-features --locked -- -D warnings}
    Run 'windows-tests' {cargo test -p hydracache-long-run-supervisor-074 --lib --locked}
    Run 'windows-allocation' {cargo test --release -p hydracache-long-run-supervisor-074 --test diagnostic_boot_buffer_allocations --locked -- --nocapture --test-threads=1}
    Run 'contracts' {cargo test -p xtask --test performance_contract_074 --locked -- --test-threads=1}
} elseif ($Lane -cin @('194','188')) {
    $version=if ($Lane -ceq '194') {'1.94.0'} else {'1.88.0'}
    $cache="/home/artur/.cache/hydracache-074-$Lane-check"
    Run "gnu-$Lane-check" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" check -p hydracache-long-run-supervisor-074 --all-targets --locked}
    Run "gnu-$Lane-clippy" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings}
    Run "gnu-$Lane-allocation" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --test diagnostic_boot_buffer_allocations --locked -- --nocapture --test-threads=1}
    Run "gnu-$Lane-boot" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --lib --locked boot_buffer_ -- --nocapture --test-threads=1}
    $image=if ($Lane -ceq '194') {"$cache/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098"} else {"$cache/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234"}
    foreach ($filter in @('kernel_walk::tests::','live_walk::tests::','validation_trace_tests::','manager_channel::','dispatch_exchange::','original_pipe_message_','worker_context_','diagnostic_kernel_admission::tests::','diagnostic_kernel_admission::origin::tests::')) {
        $name=$filter.Replace(':','').Replace('_','-')
        Run "gnu-$Lane-$name" {wsl -d Ubuntu -- $image $filter --test-threads=1}
    }
    Run "gnu-$Lane-image" {wsl -d Ubuntu -- sha256sum -- $image}
} else {
    Run 'registry' {cargo xtask gated-test-check}
    Run 'docs' {cargo xtask doc-check}
    Run 'performance-contract' {cargo xtask performance-contract-check --release 0.74}
    Run 'diff' {git diff --check}
    Run 'staged-diff' {git diff --cached --check}
}
