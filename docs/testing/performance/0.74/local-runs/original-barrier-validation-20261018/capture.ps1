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
$base = '5ea0d060'
$source = (GitText @('rev-parse', $SourceCommit)).Trim()
$base = (GitText @('rev-parse', $base)).Trim()
$paths = @((GitText @('diff', '--name-only', $base, $source)).Trim() -split "`n")
if ($paths.Count -ne 6) { throw 'Expected exactly six source/registry files' }
$patch = GitText (@('diff', '--binary', $base, $source, '--') + $paths)
[IO.File]::WriteAllText((Join-Path $packet 'implementation.patch'), $patch, [Text.UTF8Encoding]::new($false))
$sourceFiles = @($paths | ForEach-Object {
    @{ path = $_; sha256 = HashText (GitText @('show', "${source}:$_")) }
})
$files = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Where-Object Name -ne 'manifest.json' | Sort-Object FullName | ForEach-Object {
    @{ path = [IO.Path]::GetRelativePath($packet, $_.FullName).Replace('\','/'); sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
})
$manifest = [ordered]@{
    schema = 'hc074-original-barrier-validation-v1'
    base_commit = $base
    source_commit = $source
    scratch = '/tmp/hc074-original-barrier-20261018.2lcSZE'
    qualification_sha256 = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    rust194_binary_sha256 = 'cdb2f261e5d509f42b27413b7e0410372467462fe78df31a6082f9af7dcfc667'
    rust188_binary_sha256 = 'fea257627d79d899e920ff8d5c0dd37734134131cba944d1ddedff3ae4ac0ad9'
    admission = $false
    source_files = $sourceFiles
    files = $files
}
$json = ($manifest | ConvertTo-Json -Depth 8).Replace("`r`n", "`n") + "`n"
[IO.File]::WriteAllText((Join-Path $packet 'manifest.json'), $json, [Text.UTF8Encoding]::new($false))
Write-Output "HC074-ORIGINAL-BARRIER capture=true source=$source files=$($files.Count) admission=false"
