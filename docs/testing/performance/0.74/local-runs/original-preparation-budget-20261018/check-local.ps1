param([switch]$Finalize)
$ErrorActionPreference='Stop'
function Run([string]$Label,[scriptblock]$Command) {
    & $Command 2>&1 | Tee-Object -FilePath (Join-Path $PSScriptRoot "$Label.log")
    if ($LASTEXITCODE -ne 0) {throw "$Label failed"}
}
if ($Finalize) {
    foreach ($name in @('windows-check','windows-clippy','windows-tests','contracts','gnu-194','gnu-188')) {
        $raw=Get-Content (Join-Path $PSScriptRoot "$name.log") -Raw
        if ($raw -cmatch 'FAILED|panicked|error:|Exception:') {throw "unfinished prior $name"}
    }
    foreach ($compiler in @('194','188')) {
        if (!(Get-Content (Join-Path $PSScriptRoot "gnu-$compiler.log") -Raw).Contains("HC074-ORIGINAL-MANAGER-GNU compiler=$compiler complete=true admission=false")) {throw 'missing GNU completion'}
    }
}
Run 'format' {cargo fmt -p hydracache-long-run-supervisor-074 -p xtask -- --check}
if (!$Finalize) {
Run 'windows-check' {cargo check -p hydracache-long-run-supervisor-074 -p xtask --all-targets --locked}
Run 'windows-clippy' {cargo clippy -p hydracache-long-run-supervisor-074 -p xtask --all-targets --all-features --locked -- -D warnings}
Run 'windows-tests' {cargo test -p hydracache-long-run-supervisor-074 --lib --locked}
Run 'contracts' {cargo test -p xtask --test performance_contract_074 --locked -- --test-threads=1}
foreach ($compiler in @('194','188')) {
    $version=if ($compiler -ceq '194') {'1.94.0'} else {'1.88.0'}
    $cache="/home/artur/.cache/hydracache-074-$compiler-check"
    Run "gnu-$compiler" {
        wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" check -p hydracache-long-run-supervisor-074 --all-targets --locked
        if ($LASTEXITCODE -ne 0) {throw 'GNU check'}
        wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" clippy -p hydracache-long-run-supervisor-074 --all-targets --all-features --locked -- -D warnings
        if ($LASTEXITCODE -ne 0) {throw 'GNU Clippy'}
        "HC074-ORIGINAL-MANAGER-GNU compiler=$compiler complete=true admission=false"
    }
}
}
Run 'registry' {cargo xtask gated-test-check}
Run 'docs' {cargo xtask doc-check}
Run 'performance-contract' {cargo xtask performance-contract-check --release 0.74}
Run 'diff' {git diff --check}
Run 'staged-diff' {git diff --cached --check}
'HC074-ORIGINAL-MANAGER-CHECKS complete=true admission=false' | Tee-Object -FilePath (Join-Path $PSScriptRoot 'final-checks.log')
