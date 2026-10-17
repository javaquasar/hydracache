$ErrorActionPreference = 'Stop'
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$frozen = '4f5c8a6fd12a1f2bddf9642b4499db60f16ab6b3'
$prior = 'docs/testing/performance/0.74/local-runs/original-fence-custody-20261017'
& git -C $repo diff --exit-code $frozen -- $prior
if ($LASTEXITCODE -ne 0) { throw 'Prior evidence was changed' }
$manifest = Get-Content -Raw (Join-Path $repo "$prior/manifest.json") | ConvertFrom-Json
$qualification = 'docs/testing/performance/0.74/qualification-manifest.toml'
$paths = @(@($qualification) + @($manifest.sources | ForEach-Object { $_.path }) | Sort-Object -Unique)
$index = (& git -C $repo rev-parse --git-path index).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Index unavailable' }
if (-not [IO.Path]::IsPathRooted($index)) { $index = Join-Path $repo $index }
$indexBefore = (Get-FileHash -LiteralPath $index).Hash
$parent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
$name = 'hc074-custody-join-prior-' + [Guid]::NewGuid().ToString('N')
$scratch = Join-Path $parent $name
$view = Join-Path $scratch 'source'
$saved = @{}
foreach ($key in @('GIT_DIR','GIT_WORK_TREE','GIT_OPTIONAL_LOCKS','GIT_INDEX_FILE')) { $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process') }
New-Item -ItemType Directory -Path $scratch | Out-Null
try {
    $env:GIT_OPTIONAL_LOCKS = '0'
    $env:GIT_INDEX_FILE = Join-Path $scratch 'index'
    Copy-Item -LiteralPath $index -Destination $env:GIT_INDEX_FILE
    $archive = Join-Path $scratch 'source.zip'
    & git -C $repo archive --format=zip --output=$archive $frozen -- @paths
    if ($LASTEXITCODE -ne 0) { throw 'Frozen source archive failed' }
    Expand-Archive -LiteralPath $archive -DestinationPath $view
    New-Item -ItemType Directory -Path (Join-Path $view 'docs/testing/performance/0.74/local-runs') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repo $prior) -Destination (Join-Path $view $prior) -Recurse
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
    if ((Get-FileHash -LiteralPath $index).Hash -cne $indexBefore) { throw 'Active index changed' }
} finally {
    foreach ($key in $saved.Keys) {
        if ($null -eq $saved[$key]) { Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue }
        else { [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process') }
    }
    $resolved = (Resolve-Path -LiteralPath $scratch).Path
    if ([IO.Path]::GetFullPath($resolved) -ne [IO.Path]::GetFullPath($scratch) -or
        [IO.Path]::GetDirectoryName($resolved).TrimEnd('\') -ne $parent -or
        [IO.Path]::GetFileName($resolved) -ne $name) { throw 'Temporary cleanup target refused' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
}
foreach ($key in $saved.Keys) {
    if ([Environment]::GetEnvironmentVariable($key, 'Process') -cne $saved[$key] -or
        ($null -eq $saved[$key] -and (Test-Path -LiteralPath "Env:$key"))) { throw "Environment restoration failed: $key" }
}
$restored = (& git -C $repo rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or $restored.Replace('\','/') -cne $repo.Replace('\','/')) { throw 'Git environment not restored' }
Write-Output "Prior original custody evidence verified at exact source $frozen; active index and environment unchanged"
