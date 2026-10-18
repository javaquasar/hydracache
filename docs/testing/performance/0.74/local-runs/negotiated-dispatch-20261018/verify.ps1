param([switch]$SelfTest)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$cases=@('bare-receiver','healthy','fragmented','lost-ack','foreign-sender','wrong-serial','disconnect','manager-error','no-capability')
function ParseMatrix([string]$Raw) {
    $rows=@([regex]::Matches($Raw,'(?m)HC074-NEGOTIATED case=([^\s]+) (.+)\r?$'))
    if ($rows.Count -ne $cases.Count -or $Raw -notmatch 'test result: ok\. 1 passed; 0 failed;' -or $Raw -cmatch 'FAILED|panicked at') {throw 'matrix incomplete or refused'}
    for ($i=0;$i -lt $cases.Count;$i++) {
        if ($rows[$i].Groups[1].Value -cne $cases[$i]) {throw 'matrix order/duplicate/unknown case'}
        $tail=$rows[$i].Groups[2].Value.TrimEnd("`r")
        $expected=if ($cases[$i] -ceq 'bare-receiver') {'result=ok accepted=false cloexec=false retry=false original=false'} elseif ($cases[$i] -ceq 'no-capability') {'result=ok sends=0 retry=false original=false'} else {'result=ok sends=1 retry=false retained_writer=true original=false'}
        if ($tail -cne $expected) {throw 'matrix outcome drift'}
    }
}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$manifest=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'manifest.json') -Raw | ConvertFrom-Json
if ($manifest.schema -cne 'hc074-private-negotiated-evidence-v1' -or $manifest.original_start -or $manifest.system_bus -or $manifest.host_execution -or $manifest.promotion) {throw 'scope drift'}
$actual=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | Where-Object {$_.FullName -cne (Join-Path $PSScriptRoot 'manifest.json')} | ForEach-Object {[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/')} | Sort-Object)
if (($actual -join "`n") -cne (($manifest.files.path | Sort-Object) -join "`n")) {throw 'closed inventory drift'}
foreach ($item in $manifest.files) {if ((Hash (Join-Path $PSScriptRoot $item.path)) -cne $item.sha256) {throw "hash $($item.path)"}}
$initial=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'parser-before-fix/manifest.json') -Raw | ConvertFrom-Json
foreach ($item in $initial.files) {
    $path=if ($item.path -cin @('verify.ps1','README.md')) {"parser-before-fix/$($item.path)"} else {$item.path}
    if ((Hash (Join-Path $PSScriptRoot $path)) -cne $item.sha256) {throw 'initial parser packet retention drift'}
}
if ($manifest.qualification_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or (Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -cne $manifest.qualification_sha256) {throw 'qualification drift'}
$base=(& git rev-parse a36ac6e8).Trim()
if ($LASTEXITCODE -ne 0 -or $manifest.base -cne $base) {throw 'baseline pin drift'}
$changed=@(& git diff --name-only $manifest.base $manifest.source -- tools/long-run-supervisor-074/src)
if ($LASTEXITCODE -ne 0) {throw 'Git source census failed'}
$allowed=@('tools/long-run-supervisor-074/src/diagnostic_dispatch_exchange_tests.rs','tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_recv_tests.rs','tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_tests.rs')
if (($changed -join "`n") -cne ($allowed -join "`n")) {throw 'runtime freeze drift'}
$path=$allowed[0]
$before=((& git show "$($manifest.base):$path") -join "`n")+"`n"
if ($LASTEXITCODE -ne 0) {throw 'old tests missing'}
$after=((& git show "$($manifest.source):$path") -join "`n")+"`n"
if ($LASTEXITCODE -ne 0) {throw 'new tests missing'}
$registration="`n#[path = `"diagnostic_dispatch_negotiated_tests.rs`"]`nmod negotiated_tests;`n"
if ([regex]::Matches($after,[regex]::Escape($registration)).Count -ne 1 -or $after.Replace($registration,'') -cne $before) {throw 'old test body drift'}
foreach ($item in $manifest.sources) {
    $text=((& git show "$($manifest.source):$($item.path)") -join "`n")+"`n"
    if ($LASTEXITCODE -ne 0) {throw 'source object missing'}
    $hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($text))).ToLowerInvariant()
    if ($hash -cne $item.normalized_sha256 -or (Hash (Join-Path $PSScriptRoot $item.capsule)) -cne $item.sha256) {throw "source binding $($item.path)"}
}
if (($manifest.images.lane -join ',') -cne '194,188') {throw 'image census drift'}
foreach ($image in $manifest.images) {
    foreach ($boundary in @('before','after')) {
        $line=(Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/image-$($image.lane)-lint-final-$boundary.log") -Raw).Trim()
        if ($line -cne "$($image.sha256)  $($image.path)") {throw 'execution image identity drift'}
    }
}
foreach ($lane in @('194','188')) {
    $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/matrix-$lane-lint-final.log") -Raw
    ParseMatrix $raw
    $units=Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/lint-final-units-$lane.log") -Raw
    if ($units -notmatch 'test result: ok\. 3 passed; 0 failed;') {throw 'atomic receiver units incomplete'}
}
$contracts=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/atomic-final-contracts.log') -Raw
if ($contracts -notmatch 'test result: ok\. 253 passed; 0 failed;') {throw 'source contracts incomplete'}
$focused=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/lint-final-source-contracts.log') -Raw
if ($focused -notmatch 'test result: ok\. 2 passed; 0 failed;') {throw 'final source guards incomplete'}
$oldHelper=[IO.File]::ReadAllText((Join-Path $PSScriptRoot 'pre-lint/diagnostic_dispatch_negotiated_recv_tests.rs')).Replace("`r",'')
$newHelper=[IO.File]::ReadAllText((Join-Path $PSScriptRoot 'source/tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_recv_tests.rs')).Replace("`r",'')
if ($newHelper.Replace('!length.is_multiple_of(size_of::<RawFd>())','length % size_of::<RawFd>() != 0') -cne $oldHelper) {throw 'lint-only predicate substitution drift'}
$oldFixture=[IO.File]::ReadAllText((Join-Path $PSScriptRoot 'pre-lint/diagnostic_dispatch_negotiated_tests.rs')).Replace("`r",'')
$newFixture=[IO.File]::ReadAllText((Join-Path $PSScriptRoot 'source/tools/long-run-supervisor-074/src/diagnostic_dispatch_negotiated_tests.rs')).Replace("`r",'')
if ($oldFixture -cne $newFixture) {throw 'pre-lint fixture drift'}
foreach ($image in $manifest.pre_lint_images) {
    foreach ($boundary in @('before','after')) {
        $line=(Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/image-$($image.lane)-$boundary.log") -Raw).Trim()
        $path=($manifest.images | Where-Object lane -CEQ $image.lane).path
        if ($line -cne "$($image.sha256)  $path") {throw 'pre-lint image binding drift'}
    }
    ParseMatrix (Get-Content -LiteralPath (Join-Path $PSScriptRoot "receipts/matrix-$($image.lane)-atomic-final.log") -Raw)
}
if ($SelfTest) {
    $raw=Get-Content -LiteralPath (Join-Path $PSScriptRoot 'receipts/matrix-194-lint-final.log') -Raw
    $missing=[regex]::Replace($raw,'(?m)^HC074-NEGOTIATED case=healthy .+\r?\n','')
    $duplicate=$raw+"`nHC074-NEGOTIATED case=healthy result=ok sends=1 retry=false retained_writer=true original=false`n"
    $mutations=@($raw.Replace('case=healthy','case=bare-receiver'),$raw.Replace('retry=false','retry=true'),$raw.Replace('original=false','original=true'),$raw.Replace('1 passed; 0 failed','0 passed; 1 failed'),$raw.Replace('cloexec=false','cloexec=true'),$raw.Replace('sends=0','sends=1'),$missing,$duplicate,$raw.Replace('case=wrong-serial','case=unknown'))
    foreach ($mutation in $mutations) {
        $rejected=$false
        try {ParseMatrix $mutation} catch {$rejected=$true}
        if (-not $rejected) {throw 'matrix parser accepted a negative control'}
    }
    Write-Output "parser negatives=$($mutations.Count)"
}
Write-Output "verified scoped private bus packet source=$($manifest.source) files=$($actual.Count)"
