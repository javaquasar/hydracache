param([Parameter(Mandatory)][ValidateSet('preflight','stage','collect','postflight')][string]$Phase)
$ErrorActionPreference='Stop'
$taskScript=if($Phase -ceq 'postflight'){'preflight'}else{$Phase}
$taskPrefix="$PSScriptRoot/$Phase"
if(Test-Path "$taskPrefix-exit.json"){throw 'Never replay an invocation'}
if(Test-Path "$taskPrefix-start.json"){throw 'Invocation already recorded; inspect, do not replay'}
if(Test-Path "$taskPrefix-stdout.log"){throw 'Invocation may already have started; inspect, do not replay'}
$taskText=[IO.File]::ReadAllText("$PSScriptRoot/$taskScript.sh").Replace("`r`n","`n")
$taskInfo=[Diagnostics.ProcessStartInfo]::new()
$taskInfo.FileName='ssh.exe'
foreach($taskArg in @('-T','-o','BatchMode=yes','-o','ConnectTimeout=10','hydracache-ax42','sudo -n /bin/sh -s')){$taskInfo.ArgumentList.Add($taskArg)}
$taskInfo.UseShellExecute=$false;$taskInfo.CreateNoWindow=$true
$taskInfo.RedirectStandardInput=$true;$taskInfo.RedirectStandardOutput=$true;$taskInfo.RedirectStandardError=$true
# Record before dispatch: even interrupted local collection forbids a rerun.
[ordered]@{phase=$Phase;script_sha256=(Get-FileHash "$PSScriptRoot/$taskScript.sh" -Algorithm SHA256).Hash.ToLowerInvariant();recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$true;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-start.json"
$taskProcess=[Diagnostics.Process]::Start($taskInfo)
$taskOut=$taskProcess.StandardOutput.ReadToEndAsync();$taskErr=$taskProcess.StandardError.ReadToEndAsync()
$taskProcess.StandardInput.Write($taskText);$taskProcess.StandardInput.Close()
$taskProcess.WaitForExit()
$taskOut.Result|Set-Content -NoNewline -Encoding utf8NoBOM "$taskPrefix-stdout.log"
$taskErr.Result|Set-Content -NoNewline -Encoding utf8NoBOM "$taskPrefix-stderr.log"
[ordered]@{phase=$Phase;exit=$taskProcess.ExitCode;server=$true;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$taskPrefix-exit.json"
Write-Output $taskOut.Result
if($taskProcess.ExitCode -ne 0){throw "Remote $Phase refused: $($taskProcess.ExitCode). Inspect existing state; never replay staging."}
