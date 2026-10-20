param([Parameter(Mandatory)][ValidateSet('before','after')][string]$Stage)
$ErrorActionPreference='Stop'
if(Test-Path "$PSScriptRoot/bundle-$Stage.json"){throw 'Retain recorded input observation'}
$taskRoot=(Resolve-Path "$PSScriptRoot/../../../../../../target/diagnostic-start-material-bundle-20261017").Path
$taskExpected=Get-Content "$PSScriptRoot/../held-consuming-privileged-server-20261020/inputs.sha256"
$taskFiles=@(foreach($taskLine in $taskExpected){
    if($taskLine -cnotmatch '^([0-9a-f]{64})  bundle/([A-Za-z0-9._-]+)$'){continue}
    $taskHash=$Matches[1];$taskName=$Matches[2]
    $taskFile=Get-Item -LiteralPath "$taskRoot/$taskName"
    $taskObserved=(Get-FileHash -LiteralPath $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if($taskObserved -cne $taskHash){throw 'Frozen material input changed'}
    [ordered]@{name=$taskName;bytes=$taskFile.Length;sha256=$taskObserved}
})
if($taskFiles.Count -ne 9){throw 'Wrong bundle input set'}
[ordered]@{stage=$Stage;files=$taskFiles;server=$false;product=$false}|ConvertTo-Json -Depth 5|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/bundle-$Stage.json"
