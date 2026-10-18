param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$out = Join-Path $PSScriptRoot $Label
if (Test-Path -LiteralPath $out) { throw "Receipt cohort already exists: $out" }
$tracked = @('tools/long-run-supervisor-074/src', 'crates/xtask/tests/support/performance_contract_074_two_phase_launch.rs', 'crates/xtask/tests/support/performance_contract_074_original_reply_binding.rs', 'docs/COMPAT.md', 'docs/testing/gated-test-registry.toml', 'docs/testing/performance/0.74/two-phase-launch-contract.toml')
Push-Location $root
try {
    $dirty = & git status --porcelain -- @tracked
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Commit the selected sources before capture' }
    $source = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $source -notmatch '^[0-9a-f]{40}$') { throw 'Missing source commit' }
    $uid = (& wsl.exe -d Ubuntu -- id -u).Trim()
    if ($LASTEXITCODE -ne 0 -or $uid -notmatch '^[1-9][0-9]*$') { throw 'Nonroot WSL required' }
    New-Item -ItemType Directory -Path $out | Out-Null
    $suffix = if ($Compiler -eq '1.94.0') { '194' } else { '188' }
    $cache = "/home/artur/.cache/hydracache-074-$suffix-check"
    $image = if ($Compiler -eq '1.94.0') { "$cache/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098" } else { "$cache/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234" }
    function Run([string]$Name, [string[]]$Arguments) {
        $log = Join-Path $out "$Name.log"
        if (Test-Path -LiteralPath $log) { throw "Refusing overwrite: $log" }
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath $log
        if ($LASTEXITCODE -ne 0) { throw "Failed: $Name ($LASTEXITCODE)" }
    }
    $cargo = @('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=$cache",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    Run 'check' ($cargo + @('check','-p','hydracache-long-run-supervisor-074','--all-targets','--locked'))
    Run 'clippy' ($cargo + @('clippy','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--locked','--','-D','warnings'))
    Run 'units' ($cargo + @('test','--release','-p','hydracache-long-run-supervisor-074','--lib','--locked','two_phase','--','--nocapture','--test-threads=1'))
    $before = (& wsl.exe -d Ubuntu -- sha256sum $image).Trim()
    if ($LASTEXITCODE -ne 0 -or $before -notmatch '^[0-9a-f]{64} ') { throw 'Missing test image' }
    Run 'kernel-matrix' @('-d','Ubuntu','--',$image,'diagnostic_two_phase_launch::held_barrier::tests::two_phase_barrier_owned_kernel_fault_matrix','--ignored','--exact','--nocapture','--test-threads=1')
    Run 'old-exchange' @('-d','Ubuntu','--',$image,'enrollment_frame::dispatch_exchange::tests::','--nocapture','--test-threads=1')
    Run 'hidden-writer' @('-d','Ubuntu','--',$image,'dispatch_job_and_drop_do_not_prove_cessation_of_hidden_message_aliases','--nocapture','--test-threads=1')
    $after = (& wsl.exe -d Ubuntu -- sha256sum $image).Trim()
    if ($LASTEXITCODE -ne 0 -or $before -cne $after) { throw 'Test image changed during execution' }
    $dirtyAfter = & git status --porcelain -- @tracked
    if ($LASTEXITCODE -ne 0 -or $dirtyAfter -or (& git rev-parse HEAD).Trim() -cne $source) { throw 'Selected source changed during execution' }
    foreach ($name in @('units','kernel-matrix','old-exchange','hidden-writer')) {
        $log = Get-Content -Raw -LiteralPath (Join-Path $out "$name.log")
        if ($log -notmatch 'test result: ok\. [1-9][0-9]* passed; 0 failed') { throw "No positive test result: $name" }
    }
    $cases = @(Select-String -LiteralPath (Join-Path $out 'kernel-matrix.log') -Pattern 'two_phase_barrier case=')
    if ($cases.Count -ne 12) { throw "Expected 12 kernel cases, got $($cases.Count)" }
    $manifest = (Get-FileHash -Algorithm SHA256 -LiteralPath 'docs/testing/performance/0.74/qualification-manifest.toml').Hash.ToLowerInvariant()
    if ($manifest -ne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') { throw 'Qualification changed' }
    [ordered]@{ source_commit = $source; compiler = $Compiler; nonroot_uid = $uid; image_before = $before; image_after = $after; qualification_sha256 = $manifest; kernel_cases = $cases.Count; product = $false; systemd = $false; original_authority = $false } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $out 'receipt.json') -Encoding utf8NoBOM
}
finally { Pop-Location }
