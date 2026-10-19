param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,
    [Parameter(Mandatory)][ValidateSet('build','frame')][string]$Stage
)
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut = Join-Path $PSScriptRoot $Label
$taskSelected = @('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/tests/support/performance_contract_074_original_root_join.rs','crates/xtask/tests/support/performance_contract_074_root_creator.rs','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/original-root-join-contract.toml','docs/testing/performance/0.74/root-creator-contract.toml')
$taskRecipe = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
Push-Location $taskRoot
try {
    $taskSource = (& git rev-parse HEAD).Trim()
    function CheckSource {
        $taskDirty = & git status --porcelain -- @taskSelected
        if ($LASTEXITCODE -ne 0 -or $taskDirty -or (& git rev-parse HEAD).Trim() -cne $taskSource) { throw 'Selected source drift' }
    }
    CheckSource
    $taskUid = (& wsl.exe -d Ubuntu -- id -u).Trim()
    if ($LASTEXITCODE -ne 0 -or $taskUid -cne '1000') { throw 'Build/frame UID1000 required' }
    $taskSuffix = if ($Compiler -eq '1.94.0') { '194' } else { '188' }
    $taskCache = "/home/artur/.cache/hydracache-074-$taskSuffix-check"
    $taskImage = if ($Compiler -eq '1.94.0') { "$taskCache/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098" } else { "$taskCache/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234" }
    function Run([string]$Name,[string[]]$Arguments,[int]$Passed=-1) {
        $taskLog = Join-Path $taskOut "$Name.log"
        if (Test-Path -LiteralPath $taskLog) { throw "Retain existing evidence: $Name" }
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath $taskLog
        if ($LASTEXITCODE -ne 0) { throw "Failed: $Name" }
        if ($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath $taskLog).Contains("test result: ok. $Passed passed; 0 failed;")) { throw "Wrong/zero test selection: $Name" }
    }
    function ImageHash {
        $taskHash = (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
        if ($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /') { throw 'Image missing' }
        $taskHash
    }
    function Quiescent {
        foreach ($taskPrefix in @('hc074-root-fixture-*','hc074-transport-fixture-*','hc074-creator-*')) {
            $taskNames = & wsl.exe -d Ubuntu -u root -- find /sys/fs/cgroup -mindepth 1 -maxdepth 1 -type d -name $taskPrefix -print
            if ($LASTEXITCODE -ne 0 -or $taskNames) { throw 'Existing fixture; never adopt or delete it' }
        }
    }
    function BundleHashes {
        foreach ($taskName in @('timing-controls-074','build-receipt-v1.json','build-log.jsonl','Cargo.lock.root','Cargo.lock.observer','embedded.json','direct.json','resp2.json','resp3.json')) {
            "$((Get-FileHash -LiteralPath (Join-Path $taskRoot "target/diagnostic-start-material-bundle-20261017/$taskName") -Algorithm SHA256).Hash.ToLowerInvariant())  $taskName"
        }
    }
    if ($Stage -eq 'build') {
        if (Test-Path -LiteralPath $taskOut) { throw 'Cohort overwrite refused' }
        New-Item -ItemType Directory -Path $taskOut | Out-Null
        $taskCargo = @('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=$taskCache",'/home/artur/.cargo/bin/cargo',"+$Compiler")
        Run 'check' ($taskCargo + @('check','--locked','-p','hydracache-long-run-supervisor-074','--all-targets'))
        Run 'clippy' ($taskCargo + @('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
        Run 'root-units' ($taskCargo + @('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--lib','diagnostic_two_phase_launch::root_enrollment::','--','--nocapture','--test-threads=1')) 18
        $taskBefore = ImageHash
        Run 'join-units' @('-d','Ubuntu','--',$taskImage,'original_root_guardian_','--nocapture','--test-threads=1') 3
        Run 'two-phase' @('-d','Ubuntu','--',$taskImage,'diagnostic_two_phase_launch::','--nocapture','--test-threads=1') 40
        Run 'exchange' @('-d','Ubuntu','--',$taskImage,'enrollment_frame::dispatch_exchange::tests::','--nocapture','--test-threads=1') 30
        Run 'old-guardians' @('-d','Ubuntu','--','env',"HC074_REDUNDANT_TEST_BINARY=$taskImage",'python3','tools/long-run-supervisor-074/tests/redundant_watchdog.py','--binary-env','HC074_REDUNDANT_TEST_BINARY')
        foreach ($taskRepeat in 1..3) {
            Quiescent
            Run "creator-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_CREATOR_FIXTURE=local-only-v1','python3','tools/long-run-supervisor-074/tests/root_creator_sigkill.py','--binary',$taskImage)
            Quiescent
            Run "join-kernel-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_ROOT_JOIN_FIXTURE=local-only-v1',$taskImage,'root_guardian_original_kernel_controls','--ignored','--nocapture','--test-threads=1') 1
            Quiescent
            Run "root-pair-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_ROOT_GUARDIAN_FIXTURE=local-only-v1',$taskImage,'cgroup_root_guardian_fault_matrix','--ignored','--nocapture','--test-threads=1') 1
            Quiescent
            Run "transport-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_ROOT_TRANSPORT_FIXTURE=local-only-v1',$taskImage,'cgroup_root_transport_role_matrix','--ignored','--nocapture','--test-threads=1') 1
            Quiescent
            Run "prior-kernel-$taskRepeat" @('-d','Ubuntu','-u','root','--','env','HC074_ROOT_FAULT_FIXTURE=local-only-v1',$taskImage,'cgroup_root_kernel_fault_matrix','--ignored','--nocapture','--test-threads=1') 1
            Quiescent
        }
        $taskAfter = ImageHash
        CheckSource
        if ($taskBefore -cne $taskAfter) { throw 'Image drift' }
        [ordered]@{source_commit=$taskSource;compiler=$Compiler;image_before=$taskBefore;image_after=$taskAfter;runner_sha256=$taskRecipe;build_uid=$taskUid;fixture_uid=0;creator_cases=12;join_kernel_cases=4;root_pair_cases=16;transport_cases=12;prior_kernel_cases=11;repeats=3;fixture_roots_before=0;fixture_roots_after=0;creator_sigkill_tested=$true;server=$false;product=$false;original_send=$false;completion_allowed=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'build.json') -Encoding utf8NoBOM
        return
    }
    $taskBuild = Get-Content -LiteralPath (Join-Path $taskOut 'build.json') -Raw | ConvertFrom-Json
    if ($taskBuild.source_commit -cne $taskSource -or $taskBuild.compiler -cne $Compiler -or $taskBuild.runner_sha256 -cne $taskRecipe -or $taskBuild.image_after -cne (ImageHash)) { throw 'Build identity mismatch' }
    if (Test-Path -LiteralPath (Join-Path $taskOut 'receipt.json')) { throw 'Closed frame cohort' }
    $taskBundleBefore = @(BundleHashes)
    $taskBundleBefore | Set-Content -LiteralPath (Join-Path $taskOut 'bundle-before.log') -Encoding utf8NoBOM
    $taskActive = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if ($LASTEXITCODE -ne 0 -or $taskActive -cne 'active') { throw 'Require initially active local timesyncd' }
    # Previously authorized local WSL environment stabilization, never server or
    # supervisor maintenance; no attempt exists before both clock observations.
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if ($LASTEXITCODE -ne 0) { throw 'Stop exact local timesyncd failed' }
    try {
        Run 'clock-settle' @('-d','Ubuntu','--','python3','-u','-c',"import time
start=time.monotonic(); stable=start; prev=time.time_ns(); steps=0
while time.monotonic()-start < 180:
 now=time.time_ns(); mono=time.monotonic()
 if now < prev:
  steps+=1; stable=mono; print('backward_ns='+str(prev-now),flush=True)
 if mono-stable >= 65:
  print('settled=true stable_seconds=65 steps='+str(steps)+' admission_created=false',flush=True); break
 prev=now; time.sleep(0.01)
else: raise RuntimeError('clock did not settle; no admission')")
        Run 'clock-probe' @('-d','Ubuntu','--','python3','-u','-c',"import time
start=time.monotonic(); prev=time.time_ns(); count=0
while time.monotonic()-start < 65:
 now=time.time_ns()
 if now < prev: raise RuntimeError('CLOCK_REALTIME moved backwards')
 prev=now; count+=1; time.sleep(0.01)
print('clock_probe_complete=true backward_samples=0 samples='+str(count),flush=True)")
        $taskFrame = @('-d','Ubuntu','--','env','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017',$taskImage)
        foreach ($taskRepeat in 1..3) { Run "original-frame-$taskRepeat" ($taskFrame + @('original_root_guardian_frame_controls','--ignored','--nocapture','--test-threads=1')) 1 }
        Run 'prior-pending-frame' ($taskFrame + @('original_pending_watchdog_frame_controls','--ignored','--nocapture','--test-threads=1')) 1
    } finally {
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if ($LASTEXITCODE -ne 0) { throw 'Restore exact local timesyncd failed' }
        $taskRestored = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        if ($LASTEXITCODE -ne 0 -or $taskRestored -cne 'active') { throw 'Local time service restoration unverified' }
    }
    $taskBundleAfter = @(BundleHashes)
    $taskBundleAfter | Set-Content -LiteralPath (Join-Path $taskOut 'bundle-after.log') -Encoding utf8NoBOM
    CheckSource
    if ($taskBuild.image_after -cne (ImageHash) -or (Compare-Object -CaseSensitive $taskBundleBefore $taskBundleAfter)) { throw 'Image/input drift' }
    [ordered]@{source_commit=$taskSource;compiler=$Compiler;image=$taskBuild.image_after;runner_sha256=$taskRecipe;frame_cases=8;frame_repeats=3;prior_pending_cases=10;clock_service_before=$taskActive;clock_service_after=$taskRestored;kernel_simulated=$true;end_to_end_original=$false;server=$false;product=$false;original_send=$false;completion_allowed=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
} finally { Pop-Location }
