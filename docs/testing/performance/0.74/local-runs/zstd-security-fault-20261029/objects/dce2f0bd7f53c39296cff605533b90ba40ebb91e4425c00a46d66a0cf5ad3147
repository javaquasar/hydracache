param([Parameter(Mandatory)][ValidateSet('Baseline','Candidate')][string]$Stage,
      [Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $taskRepo
$taskPacket=Join-Path $taskRepo 'docs/testing/performance/0.74/local-runs/boot-authority-operations-20261021'
if((& git rev-parse HEAD).Trim() -cne $Source -or (& git status --porcelain --untracked-files=no)){throw 'Exact clean source required'}
if(Test-Path (Join-Path $taskPacket "$Stage-intent.json")){throw 'Never repeat or resume this stage'}
if($Stage -ceq 'Candidate' -and -not(Test-Path (Join-Path $taskPacket 'baseline-facts.json'))){throw 'Baseline must be reviewed before candidate'}
$taskMethod=Join-Path $taskPacket "method-$Stage"
New-Item -ItemType Directory -Path $taskMethod -ErrorAction Stop|Out-Null
foreach($taskPath in @('tools/local-profile-session-074/check_boot_ops.ps1','tools/local-profile-session-074/run_boot_ops.py','tools/local-profile-session-074/review_boot_ops.py','tools/local-profile-session-074/test_boot_ops.py','tools/local-profile-session-074/launch_boot_ops.ps1','tools/long-run-supervisor-074/tests/support/boot_operations.rs','crates/xtask/tests/boot_operations_074.rs','crates/xtask/tests/support/boot_operations_guard.rs','docs/testing/performance/0.74/boot-authority-operations-plan-20261021.md')){
    Copy-Item -LiteralPath $taskPath -Destination (Join-Path $taskMethod ([IO.Path]::GetFileName($taskPath)))
}
& wsl.exe -d Ubuntu -- /usr/bin/timeout --signal=TERM --kill-after=10s 15100s /usr/bin/python3 -B /mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/local-profile-session-074/run_boot_ops.py --source $Source --stage $Stage *> (Join-Path $taskPacket "$Stage-session.log")
$taskCode=[int]$LASTEXITCODE
@{source=$Source;stage=$Stage;exit=$taskCode;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$false;product=$false;native_gate_open=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM (Join-Path $taskPacket "$Stage-launch.json")
if($taskCode -ne 0){throw 'Diagnostic stage refused; no retry'}
