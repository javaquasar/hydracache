$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
$base = (git -C $root rev-parse 3bb825b5).Trim()
if ($LASTEXITCODE -ne 0) {throw 'base commit unavailable'}
$sourceFiles = @(Get-ChildItem (Join-Path $packet 'source-capsule') -File | Sort-Object Name | ForEach-Object {
    $name = $_.Name.Substring(0,$_.Name.Length-7).Replace('__','/')
    @{path=$name;raw_sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();
      lf_sha256=HashText ([IO.File]::ReadAllText($_.FullName).Replace("`r`n","`n"))}
})
if ($sourceFiles.Count -ne 13) {throw 'expected thirteen withdrawn source files'}
$inputs = @(Get-Content (Join-Path $packet 'build-bindings.json') -Raw | ConvertFrom-Json)
if ($inputs.Count -ne 323) {throw 'expected 323 recorded candidate inputs'}
$files = @(Get-ChildItem -LiteralPath $packet -File -Recurse | Sort-Object FullName | ForEach-Object {
    $relative = [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
    if ($relative -ne 'manifest.json') {
        @{path=$relative;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    }
})
$hostLog = [IO.File]::ReadAllText((Join-Path $packet 'host-first-red.log'))
$manifest = [ordered]@{
    schema='hc074-original-collector-loss-stopped-v1';base_commit=$base
    candidate_committed=$false;status='withdrawn-msrv-healthy-refusal'
    scratch='/tmp/hc074-collector-loss-matrix-20261018.mPInXP'
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    admission=$false;msrv_faults_completed=$false
    completed_owner_cases=[regex]::Matches($hostLog,'HC074-COLLECTOR-LOSS-OWNER seed=').Count
    lost_b_controls=[regex]::Matches($hostLog,'HC074-COLLECTOR-LOSS-NOTICE seed=').Count
    source_files=$sourceFiles;build_inputs=$inputs;files=$files
}
[IO.File]::WriteAllText((Join-Path $packet 'manifest.json'),
    ($manifest | ConvertTo-Json -Depth 9).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
"HC074-COLLECTOR-LOSS-STOPPED capture=true source_files=$($sourceFiles.Count) inputs=$($inputs.Count) admission=false"
