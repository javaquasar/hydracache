param(
    [string]$OutputDirectory = "target/moka-observer-spike/moka",
    [string]$PatchFile = "docs/testing/performance/0.73/moka-post-removal-observer-0.12.15.patch"
)

$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$destination = [IO.Path]::GetFullPath((Join-Path $repositoryRoot $OutputDirectory))
$targetRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot "target"))
if (-not $destination.StartsWith($targetRoot + [IO.Path]::DirectorySeparatorChar)) {
    throw "OutputDirectory must resolve below $targetRoot"
}
if (Test-Path -LiteralPath $destination) {
    throw "append-only spike directory already exists: $destination"
}

$cargoRoot = if ($env:CARGO_HOME) {
    $env:CARGO_HOME
} else {
    Join-Path $env:USERPROFILE ".cargo"
}
$source = Get-ChildItem -Path (Join-Path $cargoRoot "registry/src") -Directory -Recurse -Filter "moka-0.12.15" |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $source) {
    throw "moka-0.12.15 is absent from the Cargo registry; run cargo fetch --locked first"
}

New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
Copy-Item -LiteralPath $source -Destination $destination -Recurse
$manifest = Join-Path $destination "Cargo.toml"
$manifestText = [IO.File]::ReadAllText($manifest)
[IO.File]::WriteAllText($manifest, "[workspace]`n`n" + $manifestText)

$patch = Join-Path $repositoryRoot $PatchFile
if (-not (Test-Path -LiteralPath $patch -PathType Leaf)) {
    throw "patch file does not exist: $patch"
}
git -C $destination apply --unidiff-zero --check $patch
if ($LASTEXITCODE -ne 0) { throw "Moka observer patch does not apply" }
git -C $destination apply --unidiff-zero $patch
if ($LASTEXITCODE -ne 0) { throw "Moka observer patch failed" }

cargo check --manifest-path $manifest --features future --locked
if ($LASTEXITCODE -ne 0) { throw "patched Moka check failed" }
Write-Host "prepare-moka-observer-spike: OK ($destination)"
