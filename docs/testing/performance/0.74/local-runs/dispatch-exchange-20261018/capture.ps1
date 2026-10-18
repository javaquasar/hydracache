param([Parameter(Mandatory=$true)][string]$SourceCommit)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
if ($SourceCommit -cnotmatch '^[a-f0-9]{40}$') {throw 'exact source commit required'}
git -C $root cat-file -e "$SourceCommit`^{commit}"
if ($LASTEXITCODE -ne 0) {throw 'source unavailable'}
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
foreach ($compiler in @('194','188')) {
    $raw=Get-Content (Join-Path $packet "gnu-$compiler.log") -Raw
    $hashes=[regex]::Matches($raw,'(?m)^([a-f0-9]{64})  /home/artur/[^\r\n]+')
    if ($hashes.Count -ne 2 -or $hashes[0].Groups[1].Value -cne $hashes[1].Groups[1].Value) {throw 'image changed'}
    [IO.File]::WriteAllText((Join-Path $packet "binary-$compiler.sha256"),$hashes[0].Groups[1].Value+"`n",[Text.UTF8Encoding]::new($false))
}
$sources = @(Get-ChildItem (Join-Path $packet 'source-capsule') -File | Sort-Object Name | ForEach-Object {
    @{path=$_.Name.Substring(0,$_.Name.Length-7).Replace('__','/');
      raw_sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();
      lf_sha256=HashText ([IO.File]::ReadAllText($_.FullName).Replace("`r`n","`n"))}
})
$files = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Sort-Object FullName | ForEach-Object {
    $name = [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
    if ($name -ne 'manifest.json') {@{path=$name;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}}
})
$manifest = [ordered]@{
    schema='hc074-dispatch-exchange-v1';base_commit=(git -C $root rev-parse 6af23de5).Trim()
    source_commit=$SourceCommit;status='local-message-mechanics-only'
    implementation_commit='642265063c8bba31f4f71894b366f5cb53d4638e'
    admission=$false;original_start=$false;manager_authenticated=$false;writer_cessation=$false
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    unit_tests_per_compiler=14;source_contracts=229
    source_files=$sources
    build_inputs=@(Get-Content (Join-Path $packet 'build-bindings.json') -Raw | ConvertFrom-Json)
    files=$files
}
[IO.File]::WriteAllText((Join-Path $packet 'manifest.json'),
    ($manifest | ConvertTo-Json -Depth 9).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
'HC074-DISPATCH capture=true admission=false'
