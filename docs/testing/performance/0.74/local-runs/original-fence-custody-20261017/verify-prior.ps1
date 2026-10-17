$ErrorActionPreference = 'Stop'
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$frozen = '09b437435f6b2e2fac0ed9df475480c7e7c7910c'
$prior = 'docs/testing/performance/0.74/local-runs/autonomous-watchdog-20261017'
& git -C $repo diff --exit-code $frozen -- $prior
if ($LASTEXITCODE -ne 0) { throw 'Prior evidence was changed' }
$manifest = Get-Content -Raw (Join-Path $repo "$prior/manifest.json") | ConvertFrom-Json
$qualification = 'docs/testing/performance/0.74/qualification-manifest.toml'
$paths = @($qualification) + @($manifest.changed_paths) + @($manifest.sources | ForEach-Object { $_.path })
$paths = @($paths | Sort-Object -Unique)
$index = (& git -C $repo rev-parse --git-path index).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Index unavailable' }
if (-not [IO.Path]::IsPathRooted($index)) { $index = Join-Path $repo $index }
$indexBefore = (Get-FileHash -LiteralPath $index -Algorithm SHA256).Hash
$parent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
$name = 'hc074-fence-custody-prior-' + [Guid]::NewGuid().ToString('N')
$scratch = Join-Path $parent $name
$view = Join-Path $scratch 'source'
$saved = @{}
foreach ($key in @('GIT_DIR', 'GIT_WORK_TREE', 'GIT_OPTIONAL_LOCKS', 'GIT_INDEX_FILE')) {
    $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
New-Item -ItemType Directory -Path $scratch -ErrorAction Stop | Out-Null
try {
    $env:GIT_OPTIONAL_LOCKS = '0'
    $env:GIT_INDEX_FILE = Join-Path $scratch 'index'
    Copy-Item -LiteralPath $index -Destination $env:GIT_INDEX_FILE
    $archive = Join-Path $scratch 'source.zip'
    & git -C $repo archive --format=zip --output=$archive $frozen -- @paths
    if ($LASTEXITCODE -ne 0) { throw 'Frozen source archive failed' }
    Expand-Archive -LiteralPath $archive -DestinationPath $view -ErrorAction Stop
    # Unchanged prior packet is copied as raw bytes: its manifest pins checkout
    # line endings. Archived source files are checked by filtered Git blob.
    New-Item -ItemType Directory -Path (Join-Path $view 'docs/testing/performance/0.74/local-runs') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo $prior) -Destination (Join-Path $view $prior) -Recurse -Force
    $originalQualification = Join-Path $repo $qualification
    $expectedBlob = (& git -C $repo rev-parse "${frozen}:$qualification").Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Qualification blob unavailable' }
    $actualBlob = (& git -C $repo hash-object "--path=$qualification" $originalQualification).Trim()
    if ($LASTEXITCODE -ne 0 -or $actualBlob -cne $expectedBlob) { throw 'Qualification source drift' }
    Copy-Item -LiteralPath $originalQualification -Destination (Join-Path $view $qualification)
    $gitdir = (& git -C $repo rev-parse --absolute-git-dir).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Git metadata unavailable' }
    $env:GIT_DIR = $gitdir
    $env:GIT_WORK_TREE = $view
    & (Join-Path $view "$prior/verify.ps1")
    if ((Get-FileHash -LiteralPath $index -Algorithm SHA256).Hash -cne $indexBefore) { throw 'Active index changed' }
} finally {
    foreach ($key in $saved.Keys) {
        if ($null -eq $saved[$key]) {
            # A PowerShell string conversion can turn null into an existing
            # empty environment value. Absence must be restored explicitly.
            Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue
        } else {
            [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process')
        }
    }
    $resolved = (Resolve-Path -LiteralPath $scratch -ErrorAction Stop).Path
    if ([IO.Path]::GetFullPath($resolved) -ne [IO.Path]::GetFullPath($scratch) -or
        [IO.Path]::GetDirectoryName($resolved).TrimEnd('\') -ne $parent -or
        [IO.Path]::GetFileName($resolved) -ne $name) { throw 'Temporary cleanup target refused' }
    Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction Stop
}
foreach ($key in $saved.Keys) {
    $current = [Environment]::GetEnvironmentVariable($key, 'Process')
    if ($current -cne $saved[$key] -or
        ($null -eq $saved[$key] -and (Test-Path -LiteralPath "Env:$key"))) {
        throw "Environment restoration failed: $key"
    }
}
$restored = (& git -C $repo rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or $restored.Replace('\','/') -cne $repo.Replace('\','/')) { throw 'Git environment not restored' }
Write-Output "Prior autonomous evidence verified at exact source $frozen; active index and environment unchanged"
