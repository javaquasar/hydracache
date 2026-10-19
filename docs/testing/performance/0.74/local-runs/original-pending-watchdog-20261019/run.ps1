param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,
    [Parameter(Mandatory)][ValidateSet('build','frame')][string]$Stage
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$out = Join-Path $PSScriptRoot $Label
$runnerHash = (Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant()
$selected = @('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests/redundant_watchdog.py','tools/long-run-supervisor-074/tests/prearmed_watchdog.py','tools/long-run-supervisor-074/tests/original_custody_owner_loss.py','crates/xtask/tests/support/performance_contract_074_original_pending_watchdog.rs','crates/xtask/tests/support/performance_contract_074_original_watchdog_context.rs','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/original-pending-watchdog-contract.toml')
Push-Location $root
try {
    $dirty = & git status --porcelain -- @selected
    if ($LASTEXITCODE -ne 0 -or $dirty) { throw 'Commit selected sources before capture' }
    $source = (& git rev-parse HEAD).Trim()
    $uid = (& wsl.exe -d Ubuntu -- id -u).Trim()
    if ($LASTEXITCODE -ne 0 -or $uid -notmatch '^[1-9][0-9]*$') { throw 'Nonroot WSL required' }
    $suffix = if ($Compiler -eq '1.94.0') { '194' } else { '188' }
    $cache = "/home/artur/.cache/hydracache-074-$suffix-check"
    $image = if ($Compiler -eq '1.94.0') { "$cache/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098" } else { "$cache/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234" }
    function Run([string]$Name, [string[]]$Arguments) {
        $log = Join-Path $out "$Name.log"
        if (Test-Path -LiteralPath $log) { throw "Receipt overwrite refused: $Name" }
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath $log
        if ($LASTEXITCODE -ne 0) { throw "Failed: $Name" }
    }
    function ImageHash {
        $hash = (& wsl.exe -d Ubuntu -- sha256sum $image).Trim()
        if ($LASTEXITCODE -ne 0 -or $hash -notmatch '^[0-9a-f]{64}  /') { throw 'Image missing' }
        $hash
    }
    function CheckSource {
        $dirty = & git status --porcelain -- @selected
        if ($LASTEXITCODE -ne 0 -or $dirty -or (& git rev-parse HEAD).Trim() -cne $source) { throw 'Source drift' }
    }
    function BundleHashes {
        $bundle = Join-Path $root 'target/diagnostic-start-material-bundle-20261017'
        foreach ($name in @('timing-controls-074','build-receipt-v1.json','build-log.jsonl','Cargo.lock.root','Cargo.lock.observer','embedded.json','direct.json','resp2.json','resp3.json')) {
            "$((Get-FileHash -LiteralPath (Join-Path $bundle $name) -Algorithm SHA256).Hash.ToLowerInvariant())  $name"
        }
    }
    if ($Stage -eq 'build') {
        if (Test-Path -LiteralPath $out) { throw "Cohort already exists: $out" }
        New-Item -ItemType Directory -Path $out | Out-Null
        $cargo = @('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=$cache",'/home/artur/.cargo/bin/cargo',"+$Compiler")
        Run 'check' ($cargo + @('check','-p','hydracache-long-run-supervisor-074','--all-targets','--locked'))
        Run 'clippy' ($cargo + @('clippy','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--locked','--','-D','warnings'))
        Run 'kernel-units' ($cargo + @('test','--release','-p','hydracache-long-run-supervisor-074','--lib','--locked','inherited_pending_','--','--nocapture','--test-threads=1'))
        $before = ImageHash
        Run 'constructor' @('-d','Ubuntu','--',$image,'original_pending_watchdog_constructor_consumes_only_original_borrow','--nocapture','--test-threads=1')
        Run 'old-context-units' @('-d','Ubuntu','--',$image,'original_watchdog_context_','--nocapture','--test-threads=1')
        Run 'old-exchange' @('-d','Ubuntu','--',$image,'enrollment_frame::dispatch_exchange::tests::','--nocapture','--test-threads=1')
        Run 'guardian-matrix' @('-d','Ubuntu','--','env',"HC074_REDUNDANT_TEST_BINARY=$image",'python3','tools/long-run-supervisor-074/tests/redundant_watchdog.py','--binary-env','HC074_REDUNDANT_TEST_BINARY')
        $after = ImageHash
        CheckSource
        if ($before -cne $after) { throw 'Build image drift' }
        [ordered]@{ source_commit=$source; compiler=$Compiler; image=$after; nonroot_uid=$uid; runner_sha256=$runnerHash } |
            ConvertTo-Json | Set-Content -LiteralPath (Join-Path $out 'build.json') -Encoding utf8NoBOM
        return
    }
    # Shared local clock service: frame stages MUST run sequentially.
    foreach ($name in @('clock-settle.log','clock-probe.log','pending-frame.log','context-frame.log','bundle-before.log','bundle-after.log','receipt.json')) {
        if (Test-Path (Join-Path $out $name)) { throw "Retain partial/completed frame cohort: $name" }
    }
    $build = Get-Content -Raw -LiteralPath (Join-Path $out 'build.json') | ConvertFrom-Json
    if ($build.source_commit -cne $source -or $build.compiler -cne $Compiler -or $build.nonroot_uid -cne $uid -or $build.runner_sha256 -cne $runnerHash) { throw 'Build or runner mismatch' }
    $before = ImageHash
    if ($before -cne $build.image) { throw 'Image changed after build' }
    $bundleBefore = @(BundleHashes)
    $bundleBefore | Set-Content -LiteralPath (Join-Path $out 'bundle-before.log') -Encoding utf8NoBOM
    $active = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if ($LASTEXITCODE -ne 0 -or $active -cne 'active') { throw 'Require initially active local clock service' }
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if ($LASTEXITCODE -ne 0) { throw 'Stop exact local service failed' }
    try {
        # Environment stabilization only; no attempt/admission exists here.
        # Retain every early step. Never ignore one inside the strict probe below.
        Run 'clock-settle' @('-d','Ubuntu','--','python3','-u','-c',"import time; start=time.monotonic(); stable=start; prev=time.time_ns(); steps=0
print('environment_only=true max_seconds=180 stable_seconds=65 admission_created=false',flush=True)
while time.monotonic()-start < 180:
 now=time.time_ns(); mono=time.monotonic()
 if now < prev:
  steps+=1; stable=mono; print('noticed_backward_ns='+str(prev-now)+' elapsed_seconds='+str(mono-start),flush=True)
 if mono-stable >= 65:
  print('clock_settle_complete=true stable_seconds=65 noticed_backward_steps='+str(steps)+' admission_created=false',flush=True); break
 prev=now; time.sleep(0.01)
else: raise RuntimeError('Workstation clock never settled; no admission')")
        # Workstation observation BEFORE fixture/admission creation; no clock writes.
        Run 'clock-probe' @('-d','Ubuntu','--','python3','-u','-c',"import time; start=time.monotonic(); prev=time.time_ns(); count=0
while time.monotonic()-start < 65:
 now=time.time_ns()
 if now < prev: raise RuntimeError('CLOCK_REALTIME moved backwards by '+str(prev-now)+'ns')
 prev=now; count+=1; time.sleep(0.01)
print('clock_probe_complete=true backward_samples=0 samples='+str(count),flush=True)")
        $frame = @('-d','Ubuntu','--','env','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017',$image)
        Run 'pending-frame' ($frame + @('original_pending_watchdog_frame_controls','--ignored','--nocapture','--test-threads=1'))
        Run 'context-frame' ($frame + @('original_watchdog_context_frame_controls','--ignored','--nocapture','--test-threads=1'))
    }
    finally {
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if ($LASTEXITCODE -ne 0) { throw 'Restore exact local service failed' }
        $restored = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        if ($LASTEXITCODE -ne 0 -or $restored -cne 'active') { throw 'Local clock restoration unverified' }
    }
    $after = ImageHash
    $bundleAfter = @(BundleHashes)
    $bundleAfter | Set-Content -LiteralPath (Join-Path $out 'bundle-after.log') -Encoding utf8NoBOM
    CheckSource
    if ($before -cne $after -or (Compare-Object -CaseSensitive $bundleBefore $bundleAfter)) { throw 'Input or image drift' }
    foreach ($gate in @(@('kernel-units',4),@('constructor',1),@('old-context-units',4),@('old-exchange',30),@('pending-frame',1),@('context-frame',1))) {
        $log = Get-Content -Raw -LiteralPath (Join-Path $out "$($gate[0]).log")
        if ($log -notmatch "test result: ok\. $($gate[1]) passed; 0 failed") { throw "Missing gate: $($gate[0])" }
    }
    $qualification = (Get-FileHash -Algorithm SHA256 -LiteralPath 'docs/testing/performance/0.74/qualification-manifest.toml').Hash.ToLowerInvariant()
    if ($qualification -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') { throw 'Qualification drift' }
    if ((Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant() -cne $runnerHash) { throw 'Runner drift' }
    [ordered]@{ source_commit=$source; compiler=$Compiler; image_before=$before; image_after=$after; qualification_sha256=$qualification; runner_sha256=$runnerHash; nonroot_uid=$uid; pending_cases=10; context_cases=14; clock_service_before=$active; clock_service_after=$restored; test_channel_only=$true; pending_spawn=$true; target_enrollment=$false; original_send=$false; product=$false } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $out 'receipt.json') -Encoding utf8NoBOM
}
finally { Pop-Location }
