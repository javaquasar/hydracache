param([Parameter(Mandatory)][ValidateSet('windows','194','188','final')][string]$Lane)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$out=Join-Path $repo 'target/boot-buffer-plain-20261018/checks'
New-Item -ItemType Directory -Force -Path $out | Out-Null
function Run([string]$Name,[scriptblock]$Command) {
    $path=Join-Path $out "$Name.log"
    if (Test-Path -LiteralPath $path) {throw "retain first receipt $Name"}
    & $Command 2>&1 | Tee-Object -FilePath $path
    if ($LASTEXITCODE -ne 0) {throw "failed $Name"}
}
if ($Lane -ceq 'windows') {
    Run 'format' {cargo fmt -p hydracache-long-run-supervisor-074 -p xtask -- --check}
    Run 'windows-check' {cargo check -p hydracache-long-run-supervisor-074 -p xtask --all-targets --locked}
    Run 'windows-clippy' {cargo clippy -p hydracache-long-run-supervisor-074 -p xtask --all-targets --all-features --locked -- -D warnings}
    Run 'windows-tests' {cargo test -p hydracache-long-run-supervisor-074 --lib --locked}
    Run 'contracts' {cargo test -p xtask --test performance_contract_074 --locked -- --test-threads=1}
} elseif ($Lane -cin @('194','188')) {
    $version=if ($Lane -ceq '194') {'1.94.0'} else {'1.88.0'}
    $cache="/home/artur/.cache/hydracache-074-$Lane-check"
    Run "gnu-$Lane-check" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" check -p hydracache-long-run-supervisor-074 --all-targets --locked}
    Run "gnu-$Lane-clippy" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings}
    $image=(Get-Content -LiteralPath (Join-Path $repo "target/boot-buffer-plain-20261018/Candidate-$Lane.json") -Raw | ConvertFrom-Json).image
    foreach ($filter in @('boot_buffer_','kernel_walk::tests::','live_walk::tests::','validation_trace_tests::','manager_channel::','dispatch_exchange::','original_pipe_message_','worker_context_','diagnostic_kernel_admission::tests::','diagnostic_kernel_admission::origin::tests::')) {
        $name=$filter.Replace(':','').Replace('_','-')
        Run "gnu-$Lane-$name" {wsl -d Ubuntu -- $image $filter --test-threads=1}
    }
    Run "gnu-$Lane-allocation" {wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --test diagnostic_boot_buffer_allocations --locked -- --nocapture --test-threads=1}
} else {
    Run 'registry' {cargo xtask gated-test-check}
    Run 'docs' {cargo xtask doc-check}
    Run 'performance-contract' {cargo xtask performance-contract-check --release 0.74}
    Run 'diff' {git diff --check}
}
