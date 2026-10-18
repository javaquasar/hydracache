param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$out = Join-Path $PSScriptRoot $Label
if (Test-Path -LiteralPath $out) { throw "Receipt cohort already exists: $out" }
$selected = @('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests/prearmed_watchdog.py','tools/long-run-supervisor-074/tests/original_custody_owner_loss.py','crates/xtask/tests/support/performance_contract_074_prearmed.rs','crates/xtask/tests/support/performance_contract_074_two_phase_launch.rs','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/prearmed-watchdog-contract.toml')
Push-Location $root
try {
    $dirty = & git status --porcelain -- @selected
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Commit selected sources before capture' }
    $source = (& git rev-parse HEAD).Trim()
    $uid = (& wsl.exe -d Ubuntu -- id -u).Trim()
    if ($LASTEXITCODE -ne 0 -or $uid -notmatch '^[1-9][0-9]*$') { throw 'Nonroot WSL required' }
    New-Item -ItemType Directory -Path $out | Out-Null
    $suffix = if ($Compiler -eq '1.94.0') { '194' } else { '188' }
    $cache = "/home/artur/.cache/hydracache-074-$suffix-check"
    $image = if ($Compiler -eq '1.94.0') { "$cache/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098" } else { "$cache/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234" }
    function Run([string]$Name, [string[]]$Arguments) {
        $log = Join-Path $out "$Name.log"
        if (Test-Path -LiteralPath $log) { throw 'Receipt overwrite refused' }
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath $log
        if ($LASTEXITCODE -ne 0) { throw "Failed: $Name" }
    }
    $cargo = @('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=$cache",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    Run 'check' ($cargo + @('check','-p','hydracache-long-run-supervisor-074','--all-targets','--locked'))
    Run 'clippy' ($cargo + @('clippy','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--locked','--','-D','warnings'))
    Run 'units' ($cargo + @('test','--release','-p','hydracache-long-run-supervisor-074','--lib','--locked','diagnostic_two_phase_launch::','--','--nocapture','--test-threads=1'))
    $before = (& wsl.exe -d Ubuntu -- sha256sum $image).Trim()
    if ($LASTEXITCODE -ne 0 -or $before -notmatch '^[0-9a-f]{64} ') { throw 'Image missing' }
    Run 'old-barrier' @('-d','Ubuntu','--',$image,'two_phase_barrier_owned_kernel_fault_matrix','--ignored','--nocapture','--test-threads=1')
    Run 'owner-matrix' @('-d','Ubuntu','--','env',"HC074_PREARMED_TEST_BINARY=$image",'python3','tools/long-run-supervisor-074/tests/prearmed_watchdog.py','--binary-env','HC074_PREARMED_TEST_BINARY')
    Run 'old-exchange' @('-d','Ubuntu','--',$image,'enrollment_frame::dispatch_exchange::tests::','--nocapture','--test-threads=1')
    $after = (& wsl.exe -d Ubuntu -- sha256sum $image).Trim()
    $dirty = & git status --porcelain -- @selected
    if ($LASTEXITCODE -ne 0 -or $dirty -or $before -cne $after -or (& git rev-parse HEAD).Trim() -cne $source) { throw 'Source or image drift' }
    foreach ($gate in @(@('units',14),@('old-barrier',1),@('old-exchange',30))) {
        $log = Get-Content -Raw -LiteralPath (Join-Path $out "$($gate[0]).log")
        if ($log -notmatch "test result: ok\. $($gate[1]) passed; 0 failed") { throw "Gate missing: $($gate[0])" }
    }
    $matrix = Get-Content -Raw -LiteralPath (Join-Path $out 'owner-matrix.log')
    if ($matrix -notmatch 'completed=true cases=12' -or
        @([regex]::Matches($matrix,'HC074-PREARMED case=')).Count -ne 12 -or
        $matrix -notmatch 'case=guardian-death[^\r\n]*fence_retained=False' -or
        $matrix -notmatch 'case=guardian-death-after-release[^\r\n]*fence_retained=False' -or
        $matrix -notmatch 'case=guardian-stopped-after-release[^\r\n]*fence_retained=True') { throw 'Matrix or retained negative missing' }
    $qualification = (Get-FileHash -Algorithm SHA256 -LiteralPath 'docs/testing/performance/0.74/qualification-manifest.toml').Hash.ToLowerInvariant()
    if ($qualification -ne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') { throw 'Qualification drift' }
    [ordered]@{ source_commit=$source; compiler=$Compiler; image_before=$before; image_after=$after; qualification_sha256=$qualification; nonroot_uid=$uid; owner_cases=12; original_authority=$false; whole_cgroup=$false; product=$false } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $out 'receipt.json') -Encoding utf8NoBOM
}
finally { Pop-Location }
