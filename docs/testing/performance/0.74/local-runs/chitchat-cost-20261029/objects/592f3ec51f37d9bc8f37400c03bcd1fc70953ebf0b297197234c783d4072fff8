$ErrorActionPreference = 'Stop'
$taskRoot = 'C:\Workspace\prj\jq\cashe\hydracache-074\target\chitchat-compat-20261029-r1'
$taskOut = Join-Path $taskRoot 'power-postflight'
if (Test-Path -LiteralPath $taskOut) { throw 'Exclusive recovery postflight already exists' }
New-Item -ItemType Directory -Path $taskOut | Out-Null
Copy-Item -LiteralPath $PSCommandPath -Destination (Join-Path $taskOut 'producer.ps1')
$taskBegin = @('gnu-188','windows-188') | ForEach-Object { (Get-Item -LiteralPath (Join-Path $taskRoot "$_\intent.json")).LastWriteTime } | Sort-Object | Select-Object -First 1
$taskEnd = @('gnu-188','windows-188') | ForEach-Object { (Get-Item -LiteralPath (Join-Path $taskRoot "$_\result.json")).LastWriteTime } | Sort-Object | Select-Object -Last 1
$taskReadErrors = @()
$taskEvents = @(Get-WinEvent -FilterHashtable @{LogName='System';ProviderName='Microsoft-Windows-Kernel-Power';StartTime=$taskBegin;EndTime=$taskEnd;Id=@(506,507)} -ErrorAction SilentlyContinue -ErrorVariable taskReadErrors |
    ForEach-Object { [ordered]@{utc=$_.TimeCreated.ToUniversalTime().ToString('o');id=$_.Id;message=$_.Message;xml=$_.ToXml()} })
foreach ($taskReadError in $taskReadErrors) {
    if ($taskReadError.FullyQualifiedErrorId -notlike 'NoMatchingEventsFound*') { throw $taskReadError }
}
$taskValue = [ordered]@{read_only=$true;start_utc=$taskBegin.ToUniversalTime().ToString('o');end_utc=$taskEnd.ToUniversalTime().ToString('o');standby_events=$taskEvents;settings_changed=$false;clocks_changed=$false;services_changed=$false}
[IO.File]::WriteAllText((Join-Path $taskOut 'result.json'),($taskValue|ConvertTo-Json -Depth 6),[Text.UTF8Encoding]::new($false))
$taskValue|ConvertTo-Json -Depth 6
