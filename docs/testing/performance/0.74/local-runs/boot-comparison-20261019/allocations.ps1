param([Parameter(Mandatory)][ValidateSet('1.94.0','1.88.0')][string]$Compiler,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    $taskSource=(& git rev-parse HEAD).Trim()
    if(& git status --porcelain -- tools/long-run-supervisor-074/src){throw 'Dirty measured sources'}
    $taskSuffix=if($Compiler -ceq '1.94.0'){'194'}else{'188'}
    & wsl.exe -d Ubuntu -- env "CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-$taskSuffix-check" /home/artur/.cargo/bin/cargo "+$Compiler" test --release --locked -p hydracache-long-run-supervisor-074 --features fixed-leaf-alloc-probe --lib --no-run --message-format=json 2> (Join-Path $taskOut 'build.log') | Set-Content -LiteralPath (Join-Path $taskOut 'artifacts.jsonl') -Encoding utf8NoBOM
    if($LASTEXITCODE -ne 0){throw 'Profile image build failed'}
    $taskArtifacts=@(Get-Content -LiteralPath (Join-Path $taskOut 'artifacts.jsonl') | ForEach-Object {$_ | ConvertFrom-Json} | Where-Object {$_.reason -ceq 'compiler-artifact' -and $_.profile.test -and $_.executable -and $_.target.name -ceq 'hydracache_long_run_supervisor_074'})
    if($taskArtifacts.Count -ne 1){throw 'Ambiguous profile image'}
    $taskImage=$taskArtifacts[0].executable
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Missing image'}
    & wsl.exe -d Ubuntu -- $taskImage boot_comparison_allocation_attribution --nocapture --test-threads=1 2>&1 | Tee-Object -FilePath (Join-Path $taskOut 'allocations.log')
    if($LASTEXITCODE -ne 0){throw 'Allocation attribution failed'}
    $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut 'allocations.log')
    if(-not $taskRaw.Contains('test result: ok. 1 passed; 0 failed;') -or ([regex]::Matches($taskRaw,'HC074-BOOT-COMPARE-ALLOC ')).Count -ne 6){throw 'Wrong sample count'}
    if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $taskImage).Trim() -or $taskSource -cne (& git rev-parse HEAD).Trim() -or (& git status --porcelain -- tools/long-run-supervisor-074/src)){throw 'Measured source/image changed'}
    [ordered]@{source=$taskSource;compiler=$Compiler;image=$taskHash;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();samples=3;iterations=1024;profile=$true;functional_gate=$false;server=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
