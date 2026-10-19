param(
    [Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,
    [Parameter(Mandatory)][ValidateSet('build','frame')][string]$Stage
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$out = Join-Path $PSScriptRoot $Label
$selected = @('tools/long-run-supervisor-074/src','tools/long-run-supervisor-074/tests/redundant_watchdog.py','tools/long-run-supervisor-074/tests/prearmed_watchdog.py','tools/long-run-supervisor-074/tests/original_custody_owner_loss.py','crates/xtask/tests/support/performance_contract_074_original_watchdog_context.rs','crates/xtask/tests/support/performance_contract_074_redundant.rs','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/original-watchdog-context-contract.toml')
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
        Run 'units' ($cargo + @('test','--release','-p','hydracache-long-run-supervisor-074','--lib','--locked','original_watchdog_context_','--','--nocapture','--test-threads=1'))
        $before = ImageHash
        Run 'old-exchange' @('-d','Ubuntu','--',$image,'enrollment_frame::dispatch_exchange::tests::','--nocapture','--test-threads=1')
        Run 'guardian-matrix' @('-d','Ubuntu','--','env',"HC074_REDUNDANT_TEST_BINARY=$image",'python3','tools/long-run-supervisor-074/tests/redundant_watchdog.py','--binary-env','HC074_REDUNDANT_TEST_BINARY')
        $after = ImageHash
        CheckSource
        if ($before -cne $after) { throw 'Build image drift' }
        [ordered]@{ source_commit=$source; compiler=$Compiler; image=$after; nonroot_uid=$uid } |
            ConvertTo-Json | Set-Content -LiteralPath (Join-Path $out 'build.json') -Encoding utf8NoBOM
        return
    }
    # Frame stages MUST run sequentially: one WSL clock-control owner at a time.
    foreach ($name in @('frame.log','bundle-before.log','bundle-after.log','receipt.json')) {
        if (Test-Path (Join-Path $out $name)) { throw "Retain partial/completed frame cohort: $name" }
    }
    $build = Get-Content -Raw -LiteralPath (Join-Path $out 'build.json') | ConvertFrom-Json
    if ($build.source_commit -cne $source -or $build.compiler -cne $Compiler -or
        $build.nonroot_uid -cne $uid -or (Test-Path (Join-Path $out 'receipt.json'))) { throw 'Build mismatch or completed cohort' }
    $before = ImageHash
    if ($before -cne $build.image) { throw 'Image changed after build' }
    $bundleBefore = @(BundleHashes)
    $bundleBefore | Set-Content -LiteralPath (Join-Path $out 'bundle-before.log') -Encoding utf8NoBOM
    $active = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if ($LASTEXITCODE -ne 0 -or $active -cne 'active') { throw 'Require sole clock-control ownership and initially active local service' }
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if ($LASTEXITCODE -ne 0) { throw 'Stop exact local timesyncd failed' }
    try {
        Run 'frame' @('-d','Ubuntu','--','env','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017',$image,'original_watchdog_context_frame_controls','--ignored','--nocapture','--test-threads=1')
    }
    finally {
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if ($LASTEXITCODE -ne 0) { throw 'Restore exact local timesyncd failed' }
        $restored = (& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        if ($LASTEXITCODE -ne 0 -or $restored -cne 'active') { throw 'Local clock restoration unverified' }
    }
    $after = ImageHash
    $bundleAfter = @(BundleHashes)
    $bundleAfter | Set-Content -LiteralPath (Join-Path $out 'bundle-after.log') -Encoding utf8NoBOM
    CheckSource
    if ($before -cne $after -or (Compare-Object -CaseSensitive $bundleBefore $bundleAfter)) { throw 'Input or image drift' }
    foreach ($gate in @(@('units',4),@('old-exchange',30),@('frame',1))) {
        $log = Get-Content -Raw -LiteralPath (Join-Path $out "$($gate[0]).log")
        if ($log -notmatch "test result: ok\. $($gate[1]) passed; 0 failed") { throw "Missing gate: $($gate[0])" }
    }
    $frame = Get-Content -Raw -LiteralPath (Join-Path $out 'frame.log')
    $guardians = Get-Content -Raw -LiteralPath (Join-Path $out 'guardian-matrix.log')
    if (@([regex]::Matches($frame,'HC074-ORIGINAL-WATCHDOG case=')).Count -ne 14 -or
        $guardians -notmatch 'completed=true cases=17') { throw 'Matrix missing' }
    $qualification = (Get-FileHash -Algorithm SHA256 -LiteralPath 'docs/testing/performance/0.74/qualification-manifest.toml').Hash.ToLowerInvariant()
    if ($qualification -ne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') { throw 'Qualification drift' }
    [ordered]@{ source_commit=$source; compiler=$Compiler; image_before=$before; image_after=$after; qualification_sha256=$qualification; nonroot_uid=$uid; frame_cases=14; clock_service_before=$active; clock_service_after=$restored; test_channel_only=$true; watchdog_spawn=$false; original_send=$false; product=$false } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $out 'receipt.json') -Encoding utf8NoBOM
}
finally { Pop-Location }
