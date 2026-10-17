$ErrorActionPreference = 'Stop'
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$frozen = 'bb2c843f63fb941ea1011bf99221772cfbfc3052'
$prior = 'docs/testing/performance/0.74/local-runs/validation-leaf-cost-20261017'
$manifest = Get-Content -Raw (Join-Path $repo "$prior/manifest.json") | ConvertFrom-Json
$paths = @($prior, 'docs/testing/performance/0.74/qualification-manifest.toml')
$paths += @($manifest.sources | ForEach-Object { $_.path })
$paths = @($paths | Sort-Object -Unique)
$index = (& git -C $repo rev-parse --git-path index).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Index unavailable' }
if (-not [IO.Path]::IsPathRooted($index)) { $index = Join-Path $repo $index }
$indexBefore = (Get-FileHash -LiteralPath $index -Algorithm SHA256).Hash
$parent = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
$name = 'hc074-controller-prior-' + [Guid]::NewGuid().ToString('N')
$scratch = Join-Path $parent $name
$view = Join-Path $scratch 'source'
$saved = @{}
foreach ($key in @('GIT_DIR', 'GIT_WORK_TREE')) {
    $saved[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
}
New-Item -ItemType Directory -Path $scratch -ErrorAction Stop | Out-Null
try {
    $archive = Join-Path $scratch 'source.zip'
    & git -C $repo archive --format=zip --output=$archive $frozen -- @paths
    if ($LASTEXITCODE -ne 0) { throw 'Frozen source archive failed' }
    Expand-Archive -LiteralPath $archive -DestinationPath $view -ErrorAction Stop
    # The frozen verifier pins the original Windows checkout's raw CRLF digest.
    # git archive emits canonical LF. Copy only this immutable input after
    # proving its filtered Git blob is the exact frozen blob; never edit proof.
    $qualification = 'docs/testing/performance/0.74/qualification-manifest.toml'
    $originalQualification = Join-Path $repo $qualification
    $expectedBlob = (& git -C $repo rev-parse "${frozen}:$qualification").Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Qualification blob unavailable' }
    $actualBlob = (& git -C $repo hash-object "--path=$qualification" $originalQualification).Trim()
    if ($LASTEXITCODE -ne 0 -or $actualBlob -ne $expectedBlob) { throw 'Qualification source drift' }
    Copy-Item -LiteralPath $originalQualification -Destination (Join-Path $view $qualification)
    $gitdir = (& git -C $repo rev-parse --absolute-git-dir).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Git metadata unavailable' }
    $env:GIT_DIR = $gitdir
    $env:GIT_WORK_TREE = $view
    # The prior verifier uses only git show/rev-parse. No read-tree or index write.
    & (Join-Path $view "$prior/verify.ps1") -Root $view
    if ((Get-FileHash -LiteralPath $index -Algorithm SHA256).Hash -ne $indexBefore) {
        throw 'Active index changed during prior proof'
    }
    Write-Output "Prior leaf evidence verified at exact source $frozen; active index unchanged"
} finally {
    foreach ($key in $saved.Keys) {
        [Environment]::SetEnvironmentVariable($key, $saved[$key], 'Process')
    }
    $resolved = (Resolve-Path -LiteralPath $scratch -ErrorAction Stop).Path
    if ([IO.Path]::GetFullPath($resolved) -ne [IO.Path]::GetFullPath($scratch) -or
        [IO.Path]::GetDirectoryName($resolved).TrimEnd('\') -ne $parent -or
        [IO.Path]::GetFileName($resolved) -ne $name) { throw 'Temporary cleanup target refused' }
    Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction Stop
}
