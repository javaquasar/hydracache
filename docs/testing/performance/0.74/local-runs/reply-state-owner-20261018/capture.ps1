param([Parameter(Mandatory)][string]$SourceCommit)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$out=Join-Path $repo 'target/reply-state-owner-20261018'
$manifestPath=Join-Path $PSScriptRoot 'manifest.json'
if (Test-Path -LiteralPath $manifestPath) {throw 'packet already captured'}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function NormalizedHash([string]$Path) {
    $bytes=[Text.Encoding]::UTF8.GetBytes([IO.File]::ReadAllText($Path).Replace("`r",''))
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}
$source=(& git rev-parse $SourceCommit).Trim()
if ($LASTEXITCODE -ne 0) {throw 'source commit missing'}
$base=(& git rev-parse 926a3b6f).Trim()
if ($LASTEXITCODE -ne 0) {throw 'baseline missing'}
$paths=@(& git diff --name-only $base $source -- tools/long-run-supervisor-074/src crates/xtask/tests/support)
if ($LASTEXITCODE -ne 0 -or $paths.Count -ne 6) {throw 'unexpected code slice'}
$sources=@(foreach ($path in $paths) {
    $destination=Join-Path $PSScriptRoot "source/$path"
    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo $path) -Destination $destination
    [ordered]@{path=$path;capsule="source/$path";sha256=Hash $destination;normalized_sha256=NormalizedHash $destination}
})
New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot 'receipts') | Out-Null
foreach ($log in Get-ChildItem -LiteralPath $out -Filter '*.log' -File) {
    Copy-Item -LiteralPath $log.FullName -Destination (Join-Path $PSScriptRoot "receipts/$($log.Name)")
}
$images=@(foreach ($lane in @('194','188')) {
    $line=(Get-Content -LiteralPath (Join-Path $out "gnu-$lane-image-after.log") -Raw).Trim()
    if ($line -notmatch '^([a-f0-9]{64})  (/home/artur/[^\r\n]+)$') {throw 'image identity'}
    [ordered]@{lane=$lane;sha256=$Matches[1];path=$Matches[2]}
})
$inputs=@(foreach ($path in @('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml','docs/testing/performance/0.74/qualification-manifest.toml')) {
    [ordered]@{path=$path;sha256=Hash $path}
})
$manifest=[ordered]@{
    schema='hc074-reply-state-evidence-v1';source=$source;base=$base
    scope='local-private-correlation-only';original_start=$false;system_bus=$false;server_workload=$false;promotion=$false
    sources=$sources;images=$images;inputs=$inputs
    files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | Sort-Object FullName | ForEach-Object {
        [ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}
    })
}
[IO.File]::WriteAllText($manifestPath,($manifest | ConvertTo-Json -Depth 12)+"`n",[Text.UTF8Encoding]::new($false))
Write-Output "captured source=$source files=$($manifest.files.Count)"
