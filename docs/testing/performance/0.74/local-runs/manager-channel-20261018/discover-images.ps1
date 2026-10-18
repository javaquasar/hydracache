$ErrorActionPreference='Stop'
$packet=$PSScriptRoot
$images=@()
foreach ($compiler in @('194','188')) {
    $version=if ($compiler -ceq '194') {'1.94.0'} else {'1.88.0'}
    $cache="/home/artur/.cache/hydracache-074-$compiler-check"
    $raw=& wsl -d Ubuntu -- env "CARGO_TARGET_DIR=$cache" /home/artur/.cargo/bin/cargo "+$version" test --release -p hydracache-long-run-supervisor-074 --lib --locked --no-run --message-format=json 2> (Join-Path $packet "build-$compiler.log")
    if ($LASTEXITCODE -ne 0) {throw 'build discovery failed'}
    [IO.File]::WriteAllLines((Join-Path $packet "build-$compiler.jsonl"),[string[]]$raw,[Text.UTF8Encoding]::new($false))
    $artifacts=@($raw | Where-Object {$_.StartsWith('{')} | ForEach-Object {$_ | ConvertFrom-Json} |
        Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.target.name -ceq 'hydracache_long_run_supervisor_074' -and $_.profile.test -ceq $true -and $_.executable})
    if ($artifacts.Count -ne 1) {throw 'ambiguous test image'}
    $image=[string]$artifacts[0].executable
    if ($image -cnotmatch ('^'+[regex]::Escape($cache)+'/release/deps/hydracache_long_run_supervisor_074-[a-f0-9]{16}$')) {throw 'unexpected test image path'}
    $receipt=(& wsl -d Ubuntu -- sha256sum -- $image) -join "`n"
    if ($LASTEXITCODE -ne 0 -or $receipt -cnotmatch '^([a-f0-9]{64})  ') {throw 'image hash'}
    $hash=$Matches[1]
    $images+=@{compiler=$compiler;version=$version;image=$image;sha256=$hash}
    "HC074-MANAGER-IMAGE compiler=$compiler sha256=$hash image=$image"
}
[IO.File]::WriteAllText((Join-Path $packet 'images.json'),($images | ConvertTo-Json -Depth 5).Replace("`r`n","`n")+"`n",[Text.UTF8Encoding]::new($false))
