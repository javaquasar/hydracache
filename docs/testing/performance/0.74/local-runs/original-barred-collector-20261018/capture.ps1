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
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
$base = (GitText @('rev-parse','70e000c5')).Trim()
$source = (GitText @('rev-parse',$SourceCommit)).Trim()
$paths = @((GitText @('diff','--name-only',$base,$source)).Trim() -split "`n")
if ($paths.Count -ne 10) { throw 'Expected exactly ten source/fixture/registry files' }
$sourceFiles = @($paths | ForEach-Object {
    @{path=$_;sha256=HashText (GitText @('show',"${source}:$_"))}
})
$inputs = @(Get-Content (Join-Path $packet 'build-inputs-final.log') | ForEach-Object {
    if ($_ -notmatch '^([0-9a-f]{64})  (.+)$') { throw 'Invalid build input' }
    $digest = $Matches[1]
    $path = $Matches[2]
    $local = Join-Path $root $path
    if ((Get-FileHash -LiteralPath $local -Algorithm SHA256).Hash.ToLowerInvariant() -ne $digest) {
        throw "Build input changed: $path"
    }
    $lf = [IO.File]::ReadAllText($local).Replace("`r`n","`n")
    if ((HashText $lf) -ne (HashText (GitText @('show',"${source}:$path")))) { throw "Uncommitted input: $path" }
    @{path=$path;raw_sha256=$digest;lf_sha256=HashText $lf}
})
if ($inputs.Count -ne 320) { throw 'Expected complete 320-input source inventory' }
$patch = GitText @('diff','--binary',$base,$source)
[IO.File]::WriteAllText((Join-Path $packet 'implementation.patch'),$patch,[Text.UTF8Encoding]::new($false))
$files = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Sort-Object FullName | ForEach-Object {
    $relative = [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
    if ($relative -ne 'manifest.json') {
        @{path=$relative;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    }
})
$manifest = [ordered]@{
    schema='hc074-original-barred-collector-v1';base_commit=$base;source_commit=$source
    scratch='/tmp/hc074-barred-collector-20261018.ET1hWc'
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    admission=$false;source_files=$sourceFiles;build_inputs=$inputs;files=$files
}
[IO.File]::WriteAllText((Join-Path $packet 'manifest.json'),
    ($manifest | ConvertTo-Json -Depth 9).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
"HC074-BARRED-COLLECTOR capture=true source=$source files=$($files.Count) inputs=$($inputs.Count) admission=false"
