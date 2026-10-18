param([Parameter(Mandatory)][string]$SourceCommit)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function GitText([string[]]$Arguments) {
    $start = [Diagnostics.ProcessStartInfo]::new('git')
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    $result = $process.StandardOutput.ReadToEnd()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "git failed: $Arguments" }
    $result
}
function HashText([string]$Text) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()
}
$base = 'a4f29f9d'
$source = (GitText @('rev-parse', $SourceCommit)).Trim()
$base = (GitText @('rev-parse', $base)).Trim()
$paths = @((GitText @('diff', '--name-only', $base, $source)).Trim() -split "`n")
if ($paths.Count -ne 9) { throw 'Expected exactly nine source/registry files' }
$patch = GitText (@('diff', '--binary', $base, $source, '--') + $paths)
[IO.File]::WriteAllText((Join-Path $packet 'implementation.patch'), $patch, [Text.UTF8Encoding]::new($false))
$sourceFiles = @($paths | ForEach-Object {
    @{ path = $_; sha256 = HashText (GitText @('show', "${source}:$_")) }
})
$files = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Where-Object Name -ne 'manifest.json' | Sort-Object FullName | ForEach-Object {
    @{ path = [IO.Path]::GetRelativePath($packet, $_.FullName).Replace('\','/'); sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
})
$manifest = [ordered]@{
    schema = 'hc074-original-barred-custody-v1'
    base_commit = $base
    source_commit = $source
    scratch = '/tmp/hc074-original-barred-custody-20261018.wI4c3h'
    qualification_sha256 = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    rust194_binary_sha256 = ([regex]::Match([IO.File]::ReadAllText((Join-Path $packet 'binaries.log')), '(?m)^([0-9a-f]{64})  tests-194').Groups[1].Value)
    rust188_binary_sha256 = ([regex]::Match([IO.File]::ReadAllText((Join-Path $packet 'binaries.log')), '(?m)^([0-9a-f]{64})  tests-188').Groups[1].Value)
    admission = $false
    source_files = $sourceFiles
    files = $files
}
$json = ($manifest | ConvertTo-Json -Depth 8).Replace("`r`n", "`n") + "`n"
[IO.File]::WriteAllText((Join-Path $packet 'manifest.json'), $json, [Text.UTF8Encoding]::new($false))
Write-Output "HC074-BARRED-CUSTODY capture=true source=$source files=$($files.Count) admission=false"
