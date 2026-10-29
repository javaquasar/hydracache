$ErrorActionPreference = 'Stop'
$taskRoot = 'C:\Workspace\prj\jq\cashe\hydracache-074'
$taskBase = Join-Path $taskRoot 'target\chitchat-compat-20261029'
$taskOut = Join-Path $taskBase 'diagnosis'
if (Test-Path -LiteralPath $taskOut) { throw 'Exclusive first standby diagnosis already exists' }
New-Item -ItemType Directory -Path $taskOut | Out-Null
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $taskOut 'producer.ps1')
$taskBegin = (Get-Item -LiteralPath (Join-Path $taskBase 'windows-188\intent.json')).LastWriteTime.AddMinutes(-3)
$taskEnd = (Get-Item -LiteralPath (Join-Path $taskBase 'windows-188\result.json')).LastWriteTime.AddMinutes(3)
$taskEvents = @(Get-WinEvent -FilterHashtable @{LogName='System';StartTime=$taskBegin;EndTime=$taskEnd;Id=@(1,42,107,506,507)} -ErrorAction Stop |
    Where-Object { $_.ProviderName -in @('Microsoft-Windows-Kernel-Power','Microsoft-Windows-Power-Troubleshooter','Microsoft-Windows-Kernel-General') } |
    ForEach-Object { [ordered]@{utc=$_.TimeCreated.ToUniversalTime().ToString('o');id=$_.Id;provider=$_.ProviderName;message=$_.Message;xml=$_.ToXml()} })
$taskValue = [ordered]@{diagnosis_complete=$true;settings_changed=$false;clocks_changed=$false;services_changed=$false;old_outcomes_promoted=$false;events=$taskEvents}
$taskUtf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $taskOut 'standby-events.json'),($taskValue | ConvertTo-Json -Depth 6),$taskUtf8)
$taskValue | ConvertTo-Json -Depth 6
