param([Parameter(Mandatory)][ValidateSet('Baseline','Candidate')][string]$Role,[Parameter(Mandatory)][ValidateSet('194','188')][string]$Compiler)
$ErrorActionPreference='Stop'
$repo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $repo
$out=Join-Path $repo 'target/boot-buffer-plain-20261018'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$tag="$Role-$Compiler"
if (Test-Path -LiteralPath (Join-Path $out "$tag.json")) {throw 'retain first image receipt'}
function LF([string]$Text) {$Text.Replace("`r`n","`n")}
function TextHash([string]$Text) {[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Text))).ToLowerInvariant()}
function Hash([string]$Path) {(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$origin='tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs'
$candidate=(& git show "dc00ebf9139c61d26750fe7875952483330c0453:$origin") -join "`n"
if ($LASTEXITCODE -ne 0) {throw 'candidate source lookup'}
$old=(& git show "3b8e73feec8412e4090bf05575f06a05901e1010:$origin") -join "`n"
if ($LASTEXITCODE -ne 0) {throw 'baseline source lookup'}
$start=$candidate.IndexOf('fn boot(root:')
$end=$candidate.IndexOf('fn boot_read(', $start)
$before=$old.IndexOf('fn boot(root:')
$after=$old.IndexOf('fn clock(', $before)
if ($start -lt 0 -or $end -lt $start -or $before -lt 0 -or $after -lt $before) {throw 'boot boundaries'}
$oldFunction=$old.Substring($before,$after-$before)
$newFunction=$candidate.Substring($start,$end-$start)
$expected=if ($Role -ceq 'Baseline') {$candidate.Replace($newFunction,$oldFunction)} else {$candidate}
if ((LF ([IO.File]::ReadAllText((Join-Path $repo $origin)))).TrimEnd("`n") -cne $expected.TrimEnd("`n")) {throw 'unexpected role source'}
Copy-Item -LiteralPath (Join-Path $repo $origin) -Destination (Join-Path $out "$tag-origin.source")
function Inputs {
    $src=@(Get-ChildItem -LiteralPath (Join-Path $repo 'tools/long-run-supervisor-074/src') -Recurse -File -Filter '*.rs')
    if ($src.Count -ne 356) {throw 'Rust census'}
    $paths=@($src | ForEach-Object {[IO.Path]::GetRelativePath($repo,$_.FullName).Replace('\','/')})
    $paths+=@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml')
    $calls=0
    $embedded=@(foreach ($f in $src) {
        $raw=[IO.File]::ReadAllText($f.FullName)
        $matches=[regex]::Matches($raw,'\binclude_(?:str|bytes)!\s*\(\s*"([^"]+)"\s*,?\s*\)')
        if ($matches.Count -ne [regex]::Matches($raw,'\binclude_(?:str|bytes)!\s*\(').Count) {throw 'nonliteral input'}
        $calls+=$matches.Count
        foreach ($m in $matches) {[IO.Path]::GetRelativePath($repo,[IO.Path]::GetFullPath((Join-Path $f.DirectoryName $m.Groups[1].Value))).Replace('\','/')}
    })
    $embedded=@($embedded | Sort-Object -Unique)
    if ($calls -ne 27 -or $embedded.Count -ne 12) {throw 'embedded census'}
    $paths=@($paths+$embedded | Sort-Object -Unique)
    if ($paths.Count -ne 367) {throw 'input census'}
    return ,@($paths | ForEach-Object {@{path=$_;raw_sha256=(Hash (Join-Path $repo $_));lf_sha256=(TextHash (LF ([IO.File]::ReadAllText((Join-Path $repo $_)))))}})
}
$inputs=Inputs
$inputs | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8NoBOM (Join-Path $out "$tag-inputs.json")
$version=if ($Compiler -ceq '194') {'1.94.0'} else {'1.88.0'}
$cache="/home/artur/.cache/hydracache-074-$Compiler-check"
$raw=& wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --lib --locked --no-run --message-format=json 2> (Join-Path $out "$tag-build.log")
$code=$LASTEXITCODE
$raw | Set-Content -Encoding utf8NoBOM (Join-Path $out "$tag-build.jsonl")
if ($code -ne 0) {throw 'build failed'}
$afterInputs=Inputs
if ((($inputs | ForEach-Object {"$($_.path) $($_.raw_sha256)"}) -join "`n") -cne (($afterInputs | ForEach-Object {"$($_.path) $($_.raw_sha256)"}) -join "`n")) {throw 'input drift during build'}
$selected=@($raw | ForEach-Object {$_ | ConvertFrom-Json} | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
if ($selected.Count -ne 1) {throw 'image inventory'}
$path=[string]$selected[0].executable
$linuxDestination="/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/boot-buffer-plain-20261018/tests-$Role-$Compiler"
wsl -d Ubuntu -- cp -- $path $linuxDestination
if ($LASTEXITCODE -ne 0) {throw 'image copy'}
$copy=Join-Path $out "tests-$Role-$Compiler"
$line=(& wsl -d Ubuntu -- sha256sum -- $path) -join "`n"
if ($LASTEXITCODE -ne 0 -or $line -cnotmatch '^([a-f0-9]{64})  ') {throw 'image hash'}
$hash=$Matches[1]
if ((Hash $copy) -cne $hash) {throw 'copied image hash'}
@{role=$Role;compiler=$Compiler;version=$version;image=$path;copy=$copy;sha256=$hash;origin_raw_sha256=(Hash (Join-Path $repo $origin));inputs=367} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $out "$tag.json")
"HC074-PLAIN-BUILD role=$Role compiler=$Compiler sha256=$hash inputs=367"
