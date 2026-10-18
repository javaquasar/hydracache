param([Parameter(Mandatory=$true)][string]$SourceCommit)
$ErrorActionPreference='Stop'
$packet=$PSScriptRoot
$root=(Resolve-Path (Join-Path $packet '../../../../../..')).Path
if ($SourceCommit -cnotmatch '^[a-f0-9]{40}$') {throw 'exact source required'}
git -C $root cat-file -e "$SourceCommit`^{commit}"
if ($LASTEXITCODE -ne 0) {throw 'source unavailable'}
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
$sources=@(Get-ChildItem (Join-Path $packet 'source-capsule') -File | Sort-Object Name | ForEach-Object {
    @{path=$_.Name.Substring(0,$_.Name.Length-7).Replace('__','/');raw_sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();lf_sha256=HashText ([IO.File]::ReadAllText($_.FullName).Replace("`r`n","`n"))}
})
$files=@(Get-ChildItem -LiteralPath $packet -Recurse -File | Sort-Object FullName | ForEach-Object {
    $name=[IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
    if ($name -cne 'manifest.json') {@{path=$name;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}}
})
$m=[ordered]@{
    schema='hc074-fixed-manager-channel-v1';status='local-read-only-producer';selected_gate=$true
    source_commit=$SourceCommit;base_commit=(git -C $root rev-parse 27192139).Trim();first_base_commit=(git -C $root rev-parse ff4f3b7e).Trim()
    admission=$false;original_join=$false;original_start=$false;writer_cessation=$false;server_install=$false;product_workload=$false;original_host_context=$false
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    source_files=$sources;build_inputs=@(Get-Content (Join-Path $packet 'build-bindings.json') -Raw | ConvertFrom-Json)
    images=@(Get-Content (Join-Path $packet 'images.json') -Raw | ConvertFrom-Json)
    files=$files
}
[IO.File]::WriteAllText((Join-Path $packet 'manifest.json'),($m | ConvertTo-Json -Depth 10).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
'HC074-MANAGER-PACKET captured=true admission=false'
