param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label
)
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut = Join-Path $PSScriptRoot $Label
$taskSelected = @('tools/long-run-supervisor-074/src','crates/xtask/tests/support/performance_contract_074_root_guardian.rs','crates/xtask/tests/support/performance_contract_074_cgroup_root_transport.rs','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/root-guardian-contract.toml')
$taskRecipe = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
Push-Location $taskRoot
try {
    if (Test-Path -LiteralPath $taskOut) { throw 'Refuse cohort overwrite' }
    $taskDirty = & git status --porcelain -- @taskSelected
    if ($LASTEXITCODE -ne 0 -or $taskDirty) { throw 'Commit selected sources first' }
    $taskSource = (& git rev-parse HEAD).Trim()
    $taskUid = (& wsl.exe -d Ubuntu -- id -u).Trim()
    if ($LASTEXITCODE -ne 0 -or $taskUid -cne '1000') { throw 'Require nonroot build UID1000' }
    $taskNtp = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if ($LASTEXITCODE -ne 0 -or $taskNtp -cne 'active') { throw 'Require unchanged active local time service' }
    New-Item -ItemType Directory -Path $taskOut | Out-Null
    function Run([string]$Name, [string[]]$Arguments) {
        $taskLog = Join-Path $taskOut "$Name.log"
        if (Test-Path -LiteralPath $taskLog) { throw "Receipt overwrite: $Name" }
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath $taskLog
        if ($LASTEXITCODE -ne 0) { throw "Failed: $Name" }
    }
    function RequireNoFixtureRoots {
        foreach ($taskPrefix in @('hc074-root-fixture-*','hc074-transport-fixture-*')) {
            $taskNames = & wsl.exe -d Ubuntu -u root -- find /sys/fs/cgroup -mindepth 1 -maxdepth 1 -type d -name $taskPrefix -print
            if ($LASTEXITCODE -ne 0 -or $taskNames) { throw 'Existing fixture context; never adopt/delete it' }
        }
    }
    function ImageHash {
        $taskHash = (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
        if ($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /') { throw 'Image hash unavailable' }
        $taskHash
    }
    $taskSuffix = if ($Compiler -eq '1.94.0') { '194' } else { '188' }
    $taskCache = "/home/artur/.cache/hydracache-074-$taskSuffix-check"
    $taskImage = if ($Compiler -eq '1.94.0') { "$taskCache/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098" } else { "$taskCache/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234" }
    $taskCargo = @('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=$taskCache",'/home/artur/.cargo/bin/cargo',"+$Compiler")
    Run 'check' ($taskCargo + @('check','--locked','-p','hydracache-long-run-supervisor-074','--all-targets'))
    Run 'clippy' ($taskCargo + @('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
    Run 'new-controls' ($taskCargo + @('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--lib','diagnostic_two_phase_launch::root_enrollment::','--','--nocapture','--test-threads=1'))
    $taskBefore = ImageHash
    Run 'two-phase-regression' @('-d','Ubuntu','--',$taskImage,'diagnostic_two_phase_launch::','--nocapture','--test-threads=1')
    Run 'exchange-regression' @('-d','Ubuntu','--',$taskImage,'enrollment_frame::dispatch_exchange::tests::','--nocapture','--test-threads=1')
    Run 'guardian-regression' @('-d','Ubuntu','--','env',"HC074_REDUNDANT_TEST_BINARY=$taskImage",'python3','tools/long-run-supervisor-074/tests/redundant_watchdog.py','--binary-env','HC074_REDUNDANT_TEST_BINARY')
    # Compiler cohorts execute sequentially, preserving global quiescence checks.
    for ($taskRepeat=1; $taskRepeat -le 3; $taskRepeat++) {
        RequireNoFixtureRoots
        Run "root-pair-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_ROOT_GUARDIAN_FIXTURE=local-only-v1',$taskImage,'cgroup_root_guardian_fault_matrix','--ignored','--nocapture','--test-threads=1')
        RequireNoFixtureRoots
        Run "transport-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_ROOT_TRANSPORT_FIXTURE=local-only-v1',$taskImage,'cgroup_root_transport_role_matrix','--ignored','--nocapture','--test-threads=1')
        RequireNoFixtureRoots
        Run "kernel-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_ROOT_FAULT_FIXTURE=local-only-v1',$taskImage,'cgroup_root_kernel_fault_matrix','--ignored','--nocapture','--test-threads=1')
        RequireNoFixtureRoots
    }
    $taskAfter = ImageHash
    $taskDirty = & git status --porcelain -- @taskSelected
    if ($LASTEXITCODE -ne 0 -or $taskDirty -or (& git rev-parse HEAD).Trim() -cne $taskSource -or $taskBefore -cne $taskAfter) { throw 'Source/image drift' }
    $taskNtpAfter = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if ($LASTEXITCODE -ne 0 -or $taskNtpAfter -cne $taskNtp) { throw 'Local time service drift' }
    [ordered]@{ source_commit=$taskSource; compiler=$Compiler; image_before=$taskBefore; image_after=$taskAfter;
        runner_sha256=$taskRecipe; build_uid=$taskUid; fixture_uid=0; root_pair_cases_per_repeat=16;
        transport_cases_per_repeat=12; prior_kernel_cases_per_repeat=11; repeats=3;
        fixture_roots_before=0; fixture_roots_after=0; local_timesyncd_before=$taskNtp; local_timesyncd_after=$taskNtpAfter;
        server=$false; product=$false; original_send=$false; original_armed=$false; creator_sigkill_tested=$false;
        completion_allowed=$false } | ConvertTo-Json |
        Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
} finally { Pop-Location }
