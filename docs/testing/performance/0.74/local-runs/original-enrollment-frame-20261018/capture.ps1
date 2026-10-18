param([Parameter(Mandatory=$true)][string]$SourceCommit)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
if ($SourceCommit -cnotmatch '^[a-f0-9]{40}$') {throw 'exact source commit required'}
git -C $root cat-file -e "$SourceCommit`^{commit}"
if ($LASTEXITCODE -ne 0) {throw 'source commit unavailable'}
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
$sources = @(Get-ChildItem (Join-Path $packet 'source-capsule') -File | Sort-Object Name | ForEach-Object {
    @{path=$_.Name.Substring(0,$_.Name.Length-7).Replace('__','/');
      raw_sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();
      lf_sha256=HashText ([IO.File]::ReadAllText($_.FullName).Replace("`r`n","`n"))}
})
$inputs = @(Get-Content (Join-Path $packet 'build-bindings.json') -Raw | ConvertFrom-Json)
$files = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Sort-Object FullName | ForEach-Object {
    $name = [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
    if ($name -ne 'manifest.json') {@{path=$name;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}}
})
$manifest = [ordered]@{
    schema='hc074-original-enrollment-frame-v1';base_commit=(git -C $root rev-parse 27dbbac7).Trim()
    source_commit=$SourceCommit;status='verified-barred-only';full_selected_gate=$true
    scratch='/tmp/hc074-enrollment-frame-20261018.jj6xm5'
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    admission=$false;original_start=$false;product_workload=$false
    healthy_controls=8;live_refusals=18;lost_b_controls=6;owner_cases=24;selected_sigkill_signals=72
    source_files=$sources;build_inputs=$inputs;files=$files
}
[IO.File]::WriteAllText((Join-Path $packet 'manifest.json'),
    ($manifest | ConvertTo-Json -Depth 9).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
'HC074-ENROLLMENT-FRAME capture=true admission=false'
