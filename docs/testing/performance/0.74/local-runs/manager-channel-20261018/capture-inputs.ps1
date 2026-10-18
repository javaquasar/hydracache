$ErrorActionPreference='Stop'
$packet=$PSScriptRoot
$root=(Resolve-Path (Join-Path $packet '../../../../../..')).Path
function HashText([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
$paths=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
$paths+=@(Get-ChildItem (Join-Path $root 'tools/long-run-supervisor-074/src') -File -Filter '*.rs' | ForEach-Object {'tools/long-run-supervisor-074/src/'+$_.Name})
$inputs=@($paths | Sort-Object | ForEach-Object {
    $file=Join-Path $root $_
    @{path=$_;raw_sha256=(Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant();lf_sha256=HashText ([IO.File]::ReadAllText($file).Replace("`r`n","`n"))}
})
$changed=@(git -C $root diff --name-only 27192139 -- Cargo.lock crates/xtask/tests tools/long-run-supervisor-074 docs/COMPAT.md docs/testing/gated-test-registry.toml)
if ($LASTEXITCODE -ne 0) {throw 'source diff'}
$changed+=@(git -C $root ls-files --others --exclude-standard -- crates/xtask/tests/support tools/long-run-supervisor-074)
if ($LASTEXITCODE -ne 0) {throw 'untracked inventory'}
$changed=@($changed | Sort-Object -Unique)
if ($changed.Count -ne 14 -or $inputs.Count -ne 333) {throw "unexpected inventory: sources=$($changed.Count) inputs=$($inputs.Count)"}
New-Item -ItemType Directory -Path (Join-Path $packet 'source-capsule') -Force | Out-Null
foreach ($path in $changed) {Copy-Item -LiteralPath (Join-Path $root $path) -Destination (Join-Path $packet ('source-capsule/'+$path.Replace('/','__')+'.source'))}
[IO.File]::WriteAllText((Join-Path $packet 'build-bindings.json'),($inputs | ConvertTo-Json -Depth 5).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $packet 'build-inputs.log'),((@($inputs | ForEach-Object {"$($_.raw_sha256)  $($_.path)"}) -join "`n")+"`n"),[Text.UTF8Encoding]::new($false))
'HC074-MANAGER-INPUTS sources=14 inputs=333 base=27192139 admission=false'
