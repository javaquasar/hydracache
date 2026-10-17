$ErrorActionPreference = 'Stop'
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$frozen = 'fa680f3d4220bb0e49bd730c990ff07b9edfe8fa'
$prior = 'docs/testing/performance/0.74/local-runs/original-collector-start-material-20261017'
$paths = @(
    'tools/long-run-supervisor-074',
    'crates/xtask/tests',
    'Cargo.toml', 'Cargo.lock', 'docs/COMPAT.md', '.cargo', '.gitattributes',
    'docs/testing/gated-test-registry.toml',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'docs/testing/performance/0.74/local-runs/original-collector-sinks-20261017',
    'docs/testing/performance/0.74/local-runs/original-collector-sinks-join-20261017',
    $prior
)
$runtime = Get-Content -LiteralPath (Join-Path $repo 'docs/testing/performance/0.74/local-runs/original-collector-sinks-join-20261017/unchanged-runtime.json') -Raw | ConvertFrom-Json
$paths += @($runtime | ForEach-Object { $_.path } | Where-Object { -not $_.StartsWith('tools/long-run-supervisor-074/') })
$paths = @($paths | Sort-Object -Unique)
$parent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
$name = 'hc074-prior-view-' + [Guid]::NewGuid().ToString('N')
$scratch = Join-Path $parent $name
$view = Join-Path $scratch 'source'
$saved = @{}
foreach ($key in @('GIT_DIR','GIT_WORK_TREE','GIT_INDEX_FILE')) {
    $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
New-Item -ItemType Directory -Path $scratch -ErrorAction Stop | Out-Null
try {
    $archive = Join-Path $scratch 'source.zip'
    & git -C $repo archive --format=zip --output=$archive $frozen -- @paths
    if ($LASTEXITCODE -ne 0) { throw 'Frozen source archive failed' }
    Expand-Archive -LiteralPath $archive -DestinationPath $view -ErrorAction Stop
    $gitdir = (& git -C $repo rev-parse --absolute-git-dir).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Git metadata unavailable' }
    $env:GIT_DIR = $gitdir
    $env:GIT_WORK_TREE = $view
    $env:GIT_INDEX_FILE = Join-Path $scratch 'isolated-index'
    & git -C $view read-tree $frozen
    if ($LASTEXITCODE -ne 0) { throw 'Isolated index failed' }
    & (Join-Path $view ($prior + '/verify.ps1'))
    Write-Output ('Prior material proof verified at exact immutable source ' + $frozen + '; active checkout/index unchanged')
} finally {
    foreach ($key in $saved.Keys) {
        [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process')
    }
    $resolved = (Resolve-Path -LiteralPath $scratch -ErrorAction Stop).Path
    if ([IO.Path]::GetFullPath($resolved) -ne [IO.Path]::GetFullPath($scratch) -or
        [IO.Path]::GetDirectoryName($resolved).TrimEnd('\') -ne $parent -or
        [IO.Path]::GetFileName($resolved) -ne $name) { throw 'Temporary view cleanup target refused' }
    Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction Stop
}
