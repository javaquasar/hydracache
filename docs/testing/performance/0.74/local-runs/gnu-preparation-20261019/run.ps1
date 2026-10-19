param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,
    [Parameter(Mandatory)][ValidateSet('build','frame')][string]$Stage
)
$ErrorActionPreference = 'Stop'
$taskRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut = Join-Path $PSScriptRoot $Label
$taskSelected = @('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests','crates/xtask/tests/support/performance_contract_074_gnu_preparation.rs','crates/xtask/tests/support/performance_contract_074_root_retirement.rs','docs/COMPAT.md','docs/testing/gated-test-registry.toml')
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
    if ($LASTEXITCODE -ne 0 -or $taskUid -cne '1000') { throw 'Nonroot UID1000 required' }
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
    function BundleHashes {
        foreach ($taskName in @('timing-controls-074','build-receipt-v1.json','build-log.jsonl','Cargo.lock.root','Cargo.lock.observer','embedded.json','direct.json','resp2.json','resp3.json')) {
            "$((Get-FileHash -LiteralPath (Join-Path $taskRoot "target/diagnostic-start-material-bundle-20261017/$taskName") -Algorithm SHA256).Hash.ToLowerInvariant())  $taskName"
        }
    }
    if ($Stage -eq 'build') {
        if (Test-Path -LiteralPath $taskOut) { throw 'Cohort overwrite refused' }
        New-Item -ItemType Directory -Path $taskOut | Out-Null
        $taskCargo = @('-d','Ubuntu','--','env',"CARGO_TARGET_DIR=$taskCache",'/home/artur/.cargo/bin/cargo',"+$Compiler")
        Run 'clippy' ($taskCargo + @('clippy','--locked','-p','hydracache-long-run-supervisor-074','--all-targets','--all-features','--','-D','warnings'))
        Run 'causal-units' ($taskCargo + @('test','--release','--locked','-p','hydracache-long-run-supervisor-074','--lib','causal_facts_','--','--nocapture','--test-threads=1')) 2
        $taskHash = ImageHash
        Run 'compiler' @('-d','Ubuntu','--','/home/artur/.cargo/bin/rustc',"+$Compiler",'-vV')
        CheckSource
        [ordered]@{source_commit=$taskSource;compiler=$Compiler;image=$taskHash;runner_sha256=$taskRecipe;uid=$taskUid;unit_tests=2;server=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'build.json') -Encoding utf8NoBOM
        return
    }
    $taskBuild = Get-Content -LiteralPath (Join-Path $taskOut 'build.json') -Raw | ConvertFrom-Json
    if ($taskBuild.source_commit -cne $taskSource -or $taskBuild.compiler -cne $Compiler -or $taskBuild.runner_sha256 -cne $taskRecipe -or $taskBuild.image -cne (ImageHash)) { throw 'Build identity mismatch' }
    if (Test-Path -LiteralPath (Join-Path $taskOut 'receipt.json')) { throw 'Closed cohort' }
    $taskBundleBefore = @(BundleHashes)
    $taskBundleBefore | Set-Content -LiteralPath (Join-Path $taskOut 'bundle-before.log') -Encoding utf8NoBOM
    $taskActive = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if ($LASTEXITCODE -ne 0 -or $taskActive -cne 'active') { throw 'Require initially active local timesyncd' }
    # Previously authorized local WSL environment stabilization only. No attempt exists yet.
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
        Run 'clock-probe' @('-d','Ubuntu','--','python3','-u','-c',"import time,pathlib,json
start=time.monotonic(); prev=time.time_ns(); count=0
while time.monotonic()-start < 65:
 now=time.time_ns()
 if now < prev: raise RuntimeError('CLOCK_REALTIME moved backwards')
 prev=now; count+=1; time.sleep(0.01)
print(json.dumps(dict(clock_probe_complete=True,backward_samples=0,samples=count,realtime_ns=time.time_ns(),boottime_ns=time.clock_gettime_ns(time.CLOCK_BOOTTIME),boot=pathlib.Path('/proc/sys/kernel/random/boot_id').read_text().strip())),flush=True)")
        foreach ($taskRepeat in 1..3) {
            Run "preparation-$taskRepeat" @('-d','Ubuntu','--','env','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017',$taskImage,'gnu_preparation_cost_attribution','--ignored','--nocapture','--test-threads=1') 1
            $taskRaw = Get-Content -Raw -LiteralPath (Join-Path $taskOut "preparation-$taskRepeat.log")
            if (-not $taskRaw.Contains('HC074-PREPARATION-CLOSED admission=false renewal=false send=false server=false product=false')) { throw 'No closed diagnostic marker' }
        }
    } finally {
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if ($LASTEXITCODE -ne 0) { throw 'Restore exact local timesyncd failed' }
        $taskRestored = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        $taskRestored | Set-Content -LiteralPath (Join-Path $taskOut 'clock-restored.log') -Encoding utf8NoBOM
        if ($LASTEXITCODE -ne 0 -or $taskRestored -cne 'active') { throw 'Local time service restoration unverified' }
    }
    $taskBundleAfter = @(BundleHashes)
    $taskBundleAfter | Set-Content -LiteralPath (Join-Path $taskOut 'bundle-after.log') -Encoding utf8NoBOM
    CheckSource
    if ($taskBuild.image -cne (ImageHash) -or (Compare-Object -CaseSensitive $taskBundleBefore $taskBundleAfter)) { throw 'Image/input drift' }
    [ordered]@{source_commit=$taskSource;compiler=$Compiler;image=$taskBuild.image;runner_sha256=$taskRecipe;repeats=3;clock_service_before=$taskActive;clock_service_after=$taskRestored;diagnostic_only=$true;functional_gate_passed=$false;kernel_simulated=$true;end_to_end_original=$false;server=$false;product=$false;original_send=$false;completion_allowed=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
} finally { Pop-Location }
