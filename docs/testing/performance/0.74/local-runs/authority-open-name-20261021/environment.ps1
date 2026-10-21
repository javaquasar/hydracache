$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskFile=Join-Path $PSScriptRoot 'environment.json'
if(Test-Path $taskFile){throw 'Never replace environment observations'}
$taskCommands=@(
    @{name='kernel';exe='/usr/bin/uname';args=@('-srm')},
    @{name='cpu';exe='/usr/bin/lscpu';args=@('--json')},
    @{name='online';exe='/usr/bin/getconf';args=@('_NPROCESSORS_ONLN')},
    @{name='distribution';exe='/usr/bin/cat';args=@('/etc/os-release')},
    @{name='rust194';exe='/home/artur/.cargo/bin/rustc';args=@('+1.94.0','--version','--verbose')},
    @{name='rust188';exe='/home/artur/.cargo/bin/rustc';args=@('+1.88.0','--version','--verbose')}
)
$taskRows=@()
foreach($taskCommand in $taskCommands){
    $taskExecutable=$taskCommand.exe
    $taskArguments=$taskCommand.args
    $taskOutput=(& wsl.exe -d Ubuntu -- $taskExecutable @taskArguments 2>&1) -join "`n"
    $taskCode=[int]$LASTEXITCODE
    $taskRows+=@{name=$taskCommand.name;exe=$taskExecutable;args=$taskArguments;exit=$taskCode;output=$taskOutput}
}
[ordered]@{source=(& git rev-parse HEAD).Trim();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();commands=$taskRows;affinity_pinned=$false;services_changed=$false;clocks_changed=$false;server=$false;product=$false}|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8NoBOM $taskFile
if(@($taskRows|Where-Object exit -ne 0).Count){throw 'Read-only environment capture failed'}
