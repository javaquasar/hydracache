$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
$paths = @('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
$paths += @(Get-ChildItem (Join-Path $root 'tools/long-run-supervisor-074/src') -File -Filter '*.rs' |
    ForEach-Object {'tools/long-run-supervisor-074/src/'+$_.Name})
$inputs = @($paths | Sort-Object | ForEach-Object {
    $path = Join-Path $root $_
    @{path=$_;raw_sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant();
      lf_sha256=HashText ([IO.File]::ReadAllText($path).Replace("`r`n","`n"))}
})
$changed = @(git -C $root diff --name-only 27dbbac7)
if ($LASTEXITCODE -ne 0) {throw 'diff failed'}
$changed += @(git -C $root ls-files --others --exclude-standard -- crates/xtask/tests/support tools/long-run-supervisor-074)
if ($LASTEXITCODE -ne 0) {throw 'untracked inventory failed'}
$changed = @($changed | Sort-Object -Unique)
if ($changed.Count -ne 14 -or $inputs.Count -ne 326) {throw "unexpected inventories sources=$($changed.Count) inputs=$($inputs.Count)"}
New-Item -ItemType Directory -Path (Join-Path $packet 'source-capsule') -Force | Out-Null
foreach ($path in $changed) {
    Copy-Item -LiteralPath (Join-Path $root $path) -Destination (Join-Path $packet ('source-capsule/'+$path.Replace('/','__')+'.source'))
}
[IO.File]::WriteAllText((Join-Path $packet 'build-bindings.json'),
    ($inputs | ConvertTo-Json -Depth 5).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $packet 'build-inputs.log'),
    ((@($inputs | ForEach-Object {"$($_.raw_sha256)  $($_.path)"}) -join "`n")+"`n"),[Text.UTF8Encoding]::new($false))
"HC074-ENROLLMENT-INPUTS sources=$($changed.Count) inputs=$($inputs.Count) base=27dbbac7 candidate_committed=false"
