param([Parameter(Mandatory)][ValidateSet('194','188')][string]$Lane)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$out=Join-Path $repo 'target/original-reply-binding-20261018'
$receipt=Join-Path $out "full-frame-$Lane.log"
if (Test-Path -LiteralPath $receipt) {throw 'retain existing frame cohort'}
$suffix=if ($Lane -ceq '194') {'b26b156d3dcd0098'} else {'d2aba7f58d773234'}
$image="/home/artur/.cache/hydracache-074-$Lane-check/release/deps/hydracache_long_run_supervisor_074-$suffix"
$bundle=Join-Path $repo 'target/diagnostic-start-material-bundle-20261017'
function BundleHashes {
    foreach ($name in @('timing-controls-074','build-receipt-v1.json','build-log.jsonl','Cargo.lock.root','Cargo.lock.observer','embedded.json','direct.json','resp2.json','resp3.json')) {
        "$((Get-FileHash -LiteralPath (Join-Path $bundle $name) -Algorithm SHA256).Hash.ToLowerInvariant())  $name"
    }
}
BundleHashes | Tee-Object -FilePath (Join-Path $out "full-frame-bundle-before-$Lane.log")
& wsl -d Ubuntu -- sha256sum $image 2>&1 | Tee-Object -FilePath (Join-Path $out "full-frame-image-before-$Lane.log")
if ($LASTEXITCODE -ne 0) {throw 'frame image missing'}
& wsl -d Ubuntu -- env HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017 $image original_reply_binding_frame_controls_and_refusals --ignored --nocapture --test-threads=1 2>&1 | Tee-Object -FilePath $receipt
$frameExit=$LASTEXITCODE
"HC074-FRAME-EXIT code=$frameExit" | Tee-Object -FilePath (Join-Path $out "full-frame-exit-$Lane.log")
& wsl -d Ubuntu -- sha256sum $image 2>&1 | Tee-Object -FilePath (Join-Path $out "full-frame-image-after-$Lane.log")
if ($LASTEXITCODE -ne 0) {throw 'frame image missing after test'}
BundleHashes | Tee-Object -FilePath (Join-Path $out "full-frame-bundle-after-$Lane.log")
if ((Get-Content (Join-Path $out "full-frame-bundle-before-$Lane.log") -Raw) -cne (Get-Content (Join-Path $out "full-frame-bundle-after-$Lane.log") -Raw)) {throw 'frame bundle changed'}
if ((Get-Content (Join-Path $out "full-frame-image-before-$Lane.log") -Raw) -cne (Get-Content (Join-Path $out "full-frame-image-after-$Lane.log") -Raw)) {throw 'frame image changed'}
if ($frameExit -ne 0) {throw "frame failed without admission renewal: $Lane"}
