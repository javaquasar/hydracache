param([switch]$SelfTest,[switch]$SelfTestOnly)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$cases=@('bare-receiver','healthy','fragmented','lost-ack','foreign-sender','wrong-serial','disconnect','manager-error','no-capability')
function Matrix([string]$Raw) {
    $rows=@([regex]::Matches($Raw,'(?m)HC074-NEGOTIATED case=([^\s]+) (.+)\r?$'))
    if ($rows.Count -ne 9 -or $Raw -notmatch 'test result: ok\. 1 passed; 0 failed;' -or $Raw -cmatch 'FAILED|panicked at') {throw 'matrix incomplete/refused'}
    for ($i=0;$i -lt 9;$i++) {
        if ($rows[$i].Groups[1].Value -cne $cases[$i]) {throw 'matrix case drift'}
        $expected=if ($cases[$i] -ceq 'bare-receiver') {'result=ok accepted=false cloexec=false retry=false original=false'} elseif ($cases[$i] -ceq 'no-capability') {'result=ok sends=0 retry=false original=false'} else {'result=ok sends=1 retry=false retained_writer=true original=false'}
        if ($rows[$i].Groups[2].Value.TrimEnd("`r") -cne $expected) {throw 'matrix result drift'}
    }
}
function State([string]$Raw) {
    if ($Raw -notmatch 'test result: ok\. 30 passed; 0 failed; 1 ignored;' -or $Raw -cmatch 'FAILED|panicked at') {throw 'dispatch units incomplete/refused'}
    $rows=@([regex]::Matches($Raw,'HC074-REPLY-STATE seed=0740e518 sequences=512 random_steps=16384 checks=17150 coverage=\[([0-9, ]+)\]'))
    if ($rows.Count -ne 1) {throw 'seed/model report drift'}
    $counts=@($rows[0].Groups[1].Value.Split(',') | ForEach-Object {[long]$_.Trim()})
    if ($counts.Count -ne 6 -or ($counts | Where-Object {$_ -le 0}).Count -ne 0 -or ($counts | Measure-Object -Sum).Sum -ne 17150) {throw 'model coverage drift'}
}
function Frame([string]$Raw) {
    $expected=@('ok','reply-missing','reply-manager','reply-client','reply-serial','closing-reply-missing','closing-reply-manager','closing-reply-client','closing-reply-serial','names-before','names-after')
    $rows=@([regex]::Matches($Raw,'HC074-ORIGINAL-REPLY case=([^\s]+) (.+)\r?$',[Text.RegularExpressions.RegexOptions]::Multiline))
    if ($rows.Count -ne 11 -or $Raw -notmatch 'test result: ok\. 1 passed; 0 failed;' -or $Raw -cmatch 'FAILED|panicked at') {throw 'full frame incomplete/refused'}
    for ($i=0;$i -lt 11;$i++) {
        $case=$expected[$i]
        $closing=$case -ceq 'ok' -or $case.StartsWith('closing-') -or $case -ceq 'names-after'
        $auth=if ($closing) {34} else {0}
        $checks=if ($closing) {2} else {1}
        if ($rows[$i].Groups[1].Value -cne $case -or $rows[$i].Groups[2].Value.TrimEnd("`r") -cne "frame_auth=$auth channel_checks=$checks original_bounds=true test_channel_only=true start=false") {throw 'full frame case/bound drift'}
    }
}
function FrameNegatives([string]$Raw) {
    $missing=[regex]::Replace($Raw,'(?m)^HC074-ORIGINAL-REPLY case=reply-missing .+\r?\n','')
    foreach ($mutation in @($missing,$Raw.Replace('frame_auth=34','frame_auth=33'),$Raw.Replace('channel_checks=2','channel_checks=1'),$Raw.Replace('original_bounds=true','original_bounds=false'),$Raw.Replace('start=false','start=true'),$Raw.Replace('case=names-after','case=names-before'))) {
        $refused=$false;try {Frame $mutation} catch {$refused=$true}
        if (!$refused) {throw 'frame parser negative accepted'}
    }
    Write-Output 'frame parser negatives=6'
}
function Negatives([string]$Raw,[string]$Units) {
    $missing=[regex]::Replace($Raw,'(?m)^HC074-NEGOTIATED case=healthy .+\r?\n','')
    $duplicate=$Raw+"`nHC074-NEGOTIATED case=healthy result=ok sends=1 retry=false retained_writer=true original=false`n"
    $matrixMutations=@($Raw.Replace('case=healthy','case=bare-receiver'),$Raw.Replace('retry=false','retry=true'),$Raw.Replace('original=false','original=true'),$Raw.Replace('1 passed; 0 failed','0 passed; 1 failed'),$Raw.Replace('cloexec=false','cloexec=true'),$Raw.Replace('sends=0','sends=1'),$missing,$duplicate,$Raw.Replace('case=wrong-serial','case=unknown'))
    foreach ($mutation in $matrixMutations) {
        $rejected=$false;try {Matrix $mutation} catch {$rejected=$true}
        if (-not $rejected) {throw 'matrix negative accepted'}
    }
    $coverage=[regex]::Replace($Units,'coverage=\[[0-9, ]+\]','coverage=[0, 0, 0, 0, 0, 17150]')
    foreach ($mutation in @($Units.Replace('0740e518','deadbeef'),$Units.Replace('checks=17150','checks=17149'),$Units.Replace('30 passed; 0 failed','29 passed; 1 failed'),$coverage)) {
        $rejected=$false;try {State $mutation} catch {$rejected=$true}
        if (-not $rejected) {throw 'state negative accepted'}
    }
    Write-Output 'parser negatives=13'
}
if ($SelfTestOnly) {
    $out=Join-Path $repo 'target/original-reply-binding-20261018'
    $raw=Get-Content -LiteralPath (Join-Path $out 'gnu-194-matrix.log') -Raw
    $units=Get-Content -LiteralPath (Join-Path $out 'gnu-194-dispatch-exchange.log') -Raw
    Matrix $raw; State $units; Negatives $raw $units
    $frame=Get-Content -LiteralPath (Join-Path $out 'full-frame-194.log') -Raw
    Frame $frame; FrameNegatives $frame
    exit
}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function GitText([string]$Ref,[string]$Path) {
    $text=((& git show "${Ref}:$Path") -join "`n")+"`n"
    if ($LASTEXITCODE -ne 0) {throw "Git source missing $Path"}
    $text
}
$manifest=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json
if ($manifest.schema -cne 'hc074-original-reply-binding-v1' -or $manifest.scope -cne 'local-original-correlation-preparation' -or $manifest.original_start -or $manifest.system_bus -or $manifest.server_workload -or $manifest.promotion) {throw 'scope drift'}
$actual=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | Where-Object {$_.FullName -cne (Join-Path $PSScriptRoot 'manifest.json')} | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Sort-Object)
if (($actual -join "`n") -cne (($manifest.files.path | Sort-Object) -join "`n")) {throw 'file inventory drift'}
foreach ($item in $manifest.files) {if ((Hash (Join-Path $PSScriptRoot $item.path)) -cne $item.sha256) {throw "hash $($item.path)"}}
foreach ($item in $manifest.compressed_receipts) {
    $file=[IO.File]::OpenRead((Join-Path $PSScriptRoot $item.path))
    $gzip=[IO.Compression.GZipStream]::new($file,[IO.Compression.CompressionMode]::Decompress)
    $sha=[Security.Cryptography.SHA256]::Create()
    try {$hash=[Convert]::ToHexString($sha.ComputeHash($gzip)).ToLowerInvariant()} finally {$sha.Dispose();$gzip.Dispose();$file.Dispose()}
    if ($hash -cne $item.raw_sha256) {throw 'compressed raw receipt drift'}
}
if ($manifest.base -cne (& git rev-parse ba017a7d).Trim()) {throw 'baseline drift'}
foreach ($item in $manifest.inputs) {if ((Hash $item.path) -cne $item.sha256) {throw 'manifest/lock input drift'}}
if ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') {throw 'qualification drift'}
if ($manifest.sources.Count -ne 14) {throw 'source census drift'}
foreach ($item in $manifest.sources) {
    $text=GitText $manifest.source $item.path
    $hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($text))).ToLowerInvariant()
    if ($hash -cne $item.normalized_sha256 -or (Hash (Join-Path $PSScriptRoot $item.capsule)) -cne $item.sha256) {throw 'source binding drift'}
}
$path='tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange.rs'
$old=GitText $manifest.base $path
$new=GitText $manifest.source $path
$marker='    // Call before any possible transport side effect.'
$endMarker="`n}`n`nimpl DispatchExchange {"
$before=$old.Substring($old.IndexOf($marker),$old.IndexOf($endMarker)-$old.IndexOf($marker))
$after=$new.Substring($new.IndexOf($marker),$new.IndexOf($endMarker)-$new.IndexOf($marker))
if ($before -cne $after) {throw 'original classifier/transition body drift'}
$marker='fn request_matches('
$endMarker='impl std::ops::Deref'
if ($old.Substring($old.IndexOf($marker),$old.IndexOf($endMarker)-$old.IndexOf($marker)) -cne $new.Substring($new.IndexOf($marker),$new.IndexOf($endMarker)-$new.IndexOf($marker))) {throw 'request predicate drift'}
foreach ($name in @('diagnostic_manager_channel.rs','diagnostic_manager_channel_kernel.rs','diagnostic_original_enrollment_frame.rs','diagnostic_original_enrollment_frame_build.rs','diagnostic_original_custody_join.rs','diagnostic_original_descriptor_plan.rs','diagnostic_autonomous_watchdog.rs')) {
    $path="tools/long-run-supervisor-074/src/$name"
    if ((GitText $manifest.base $path) -cne (GitText $manifest.source $path)) {throw "original live source drift $name"}
}
$changed=@(& git diff --name-only $manifest.base $manifest.source -- tools/long-run-supervisor-074/src crates/xtask/tests/support docs/testing/gated-test-registry.toml)
if ($LASTEXITCODE -ne 0 -or ($changed -join ',') -cne ($manifest.sources.path -join ',')) {throw 'source slice drift'}
if (($manifest.images.lane -join ',') -cne '194,188') {throw 'image census drift'}
foreach ($image in $manifest.images) {
    foreach ($boundary in @('before','after')) {
        $line=(Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/gnu-$($image.lane)-image-$boundary.log") -Raw).Trim()
        if ($line -cne "$($image.sha256)  $($image.path)") {throw 'execution image drift'}
    }
    Matrix (Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/gnu-$($image.lane)-matrix.log") -Raw)
    State (Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/gnu-$($image.lane)-dispatch-exchange.log") -Raw)
}
if ((Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/contracts.log') -Raw) -notmatch 'test result: ok\. 259 passed; 0 failed;') {throw 'source contracts incomplete'}
if ($SelfTest) {Negatives (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/gnu-194-matrix.log') -Raw) (Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/gnu-194-dispatch-exchange.log') -Raw)}
foreach ($lane in @('194','188')) {
    $frame=Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/full-frame-$lane.log") -Raw
    Frame $frame
    if ($SelfTest) {FrameNegatives $frame}
    $image=@($manifest.images | Where-Object {$_.lane -ceq $lane})[0]
    foreach ($boundary in @('before','after')) {
        $line=(Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/full-frame-image-$boundary-$lane.log") -Raw).Trim()
        if ($line -cne "$($image.sha256)  $($image.path)") {throw 'full frame image drift'}
    }
    $bundleBefore=Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/full-frame-bundle-before-$lane.log") -Raw
    $bundleAfter=Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/full-frame-bundle-after-$lane.log") -Raw
    if ($bundleBefore -cne $bundleAfter -or @([regex]::Matches($bundleBefore,'(?m)^[a-f0-9]{64}  [^\r\n]+\r?$')).Count -ne 9) {throw 'frame bundle inventory/drift'}
    foreach ($entry in @(@('manager-channel','18 passed; 0 failed; 3 ignored;'),@('original-pipe-message-','5 passed; 0 failed;'),@('worker-context-','10 passed; 0 failed;'))) {
        $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/gnu-$lane-$($entry[0]).log") -Raw
        if (!$raw.Contains("test result: ok. $($entry[1])") -or $raw -cmatch 'FAILED|panicked at') {throw 'unit receipt drift'}
    }
}
Write-Output "verified original correlation preparation packet source=$($manifest.source) files=$($actual.Count)"
