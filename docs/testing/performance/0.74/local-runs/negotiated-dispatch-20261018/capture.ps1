param([Parameter(Mandatory)][string]$SourceCommit)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$inputRoot=Join-Path $repo 'target/negotiated-dispatch-20261018'
$manifestPath=Join-Path $PSScriptRoot 'manifest.json'
if (Test-Path -LiteralPath $manifestPath) {throw 'immutable packet already captured'}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function NormalizedHash([string]$Path) {
    $bytes=[Text.Encoding]::UTF8.GetBytes([IO.File]::ReadAllText($Path).Replace("`r",''))
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()
}
$source=(& git rev-parse $SourceCommit).Trim()
if ($LASTEXITCODE -ne 0) {throw 'source commit missing'}
$base=(& git rev-parse a36ac6e8).Trim()
if ($LASTEXITCODE -ne 0) {throw 'baseline commit missing'}
$paths=@(& git diff --name-only $base $source)
if ($LASTEXITCODE -ne 0 -or $paths.Count -ne 8) {throw 'unexpected source slice'}
$sources=@(foreach ($path in $paths) {
    $destination=Join-Path $PSScriptRoot "source/$path"
    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo $path) -Destination $destination
    [ordered]@{path=$path;capsule="source/$path";sha256=Hash $destination;normalized_sha256=NormalizedHash $destination}
})
New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot 'receipts') | Out-Null
foreach ($log in Get-ChildItem -LiteralPath $inputRoot -Filter '*.log' -File) {
    Copy-Item -LiteralPath $log.FullName -Destination (Join-Path $PSScriptRoot "receipts/$($log.Name)")
}
$negative=@(foreach ($name in @('first','second','third')) {
    $directory=Join-Path $inputRoot $name
    if (-not (Test-Path -LiteralPath $directory)) {continue}
    $destination=Join-Path $PSScriptRoot "negative/$name"
    New-Item -ItemType Directory -Force $destination | Out-Null
    foreach ($file in Get-ChildItem -LiteralPath $directory -Filter '*.rs' -File) {
        Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $destination $file.Name)
    }
    $image=Join-Path $directory 'tests-194'
    [ordered]@{cohort=$name;image_sha256=Hash $image;image_retained_locally=$true}
})
$images=@(foreach ($lane in @('194','188')) {
    $suffix=if ($lane -ceq '194') {'b26b156d3dcd0098'} else {'d2aba7f58d773234'}
    $path="/home/artur/.cache/hydracache-074-$lane-check/release/deps/hydracache_long_run_supervisor_074-$suffix"
    $line=& wsl -d Ubuntu -- sha256sum $path
    if ($LASTEXITCODE -ne 0 -or $line -notmatch '^([a-f0-9]{64})\s') {throw 'image hash'}
    [ordered]@{lane=$lane;path=$path;sha256=$Matches[1]}
})
$preLint=Join-Path $inputRoot 'pre-lint'
$comparison=Join-Path $PSScriptRoot 'pre-lint'
New-Item -ItemType Directory -Force $comparison | Out-Null
foreach ($file in Get-ChildItem -LiteralPath $preLint -Filter '*.rs' -File) {
    Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $comparison $file.Name)
}
$preLintImages=@(foreach ($lane in @('194','188')) {
    [ordered]@{lane=$lane;sha256=Hash (Join-Path $preLint "tests-$lane")}
})
$manifest=[ordered]@{
    schema='hc074-private-negotiated-evidence-v1';source=$source;base=$base
    qualification_sha256=Hash 'docs/testing/performance/0.74/qualification-manifest.toml'
    original_start=$false;system_bus=$false;host_execution=$false;promotion=$false
    sources=$sources;images=$images;negative=$negative;pre_lint_images=$preLintImages
    files=@(Get-ChildItem -LiteralPath $PSScriptRoot -Recurse -File | Sort-Object FullName | ForEach-Object {
        [ordered]@{path=[IO.Path]::GetRelativePath($PSScriptRoot,$_.FullName).Replace('\','/');sha256=Hash $_.FullName}
    })
}
[IO.File]::WriteAllText($manifestPath,($manifest | ConvertTo-Json -Depth 12)+"`n",[Text.UTF8Encoding]::new($false))
Write-Output "captured source=$source files=$($manifest.files.Count)"
