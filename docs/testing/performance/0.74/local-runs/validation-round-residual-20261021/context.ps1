param([Parameter(Mandatory)][ValidateSet('Before','After')][string]$Stage)
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
Set-Location $taskRepo
$taskPath=Join-Path $PSScriptRoot "context-$Stage.json"
if(Test-Path $taskPath){throw 'Never replace context'}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$taskState=(& wsl.exe --list --verbose 2>&1)-join "`n"
if($LASTEXITCODE -ne 0){throw 'WSL inventory unavailable'}
$taskCommands=@(
    @{name='boot';exe='/usr/bin/cat';args=@('/proc/sys/kernel/random/boot_id')},
    @{name='config';exe='/usr/bin/sha256sum';args=@('/etc/wsl.conf','/etc/systemd/timesyncd.conf')},
    @{name='kernel';exe='/usr/bin/uname';args=@('-srm')},
    @{name='fixtures';exe='/usr/bin/find';args=@('/tmp','/sys/fs/cgroup','-maxdepth','1','-name','hc074-root-fixture-*','-print')},
    @{name='compiler194';exe='/home/artur/.cargo/bin/rustc';args=@('+1.94.0','--version','--verbose')},
    @{name='compiler188';exe='/home/artur/.cargo/bin/rustc';args=@('+1.88.0','--version','--verbose')}
)
$taskRows=@(foreach($taskCommand in $taskCommands){
    $taskExe=$taskCommand.exe;$taskArgs=$taskCommand.args
    $taskOutput=(& wsl.exe -d Ubuntu -u root -- $taskExe @taskArgs 2>&1)-join "`n"
    @{name=$taskCommand.name;exe=$taskExe;args=$taskArgs;exit=[int]$LASTEXITCODE;output=$taskOutput}
})
$taskBundle=@(Get-ChildItem -LiteralPath (Join-Path $taskRepo 'target/diagnostic-start-material-bundle-20261017') -File|Sort-Object Name|ForEach-Object{@{name=$_.Name;bytes=$_.Length;sha256=(Hash $_.FullName)}})
$taskWindowsConfig=Join-Path $env:USERPROFILE '.wslconfig'
$taskWindows=if(Test-Path -LiteralPath $taskWindowsConfig){@{exists=$true;sha256=(Hash $taskWindowsConfig)}}else{@{exists=$false;sha256=$null}}
@{stage=$Stage;source=(& git rev-parse HEAD).Trim();recipe_sha256=(Hash $PSCommandPath);wsl_state_before_entry=$taskState;commands=$taskRows;bundle=$taskBundle;windows_config=$taskWindows;qualification_sha256=(Hash 'docs/testing/performance/0.74/qualification-manifest.toml');clocks_changed=$false;services_changed=$false;server=$false;product=$false}|ConvertTo-Json -Depth 12|Set-Content -Encoding utf8NoBOM $taskPath
if(@($taskRows|Where-Object exit -ne 0).Count -or $taskRows[3].output.Trim()){throw 'Context refused or pre-existing owned fixture residue; do not delete/retry'}
