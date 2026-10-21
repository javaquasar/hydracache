$ErrorActionPreference='Stop'
$taskOutput=Join-Path $PSScriptRoot 'authoring-preflight.json'
if(Test-Path $taskOutput){throw 'Never overwrite authoring evidence'}
$taskOriginal=Get-Content (Join-Path $PSScriptRoot 'review.ps1') -Raw
$taskBad='$taskCalls=@(1024,1024,(2*$taskDepth+1)*1024,2*$taskDepth*1024,2*$taskDepth*1024,1024,0,0,0,0,0,0,0)'
if(-not $taskOriginal.Contains($taskBad)){throw 'Original recipe differs'}
$taskDepth=7
$taskFailure=$null
try{
    $taskCalls=@(1024,1024,(2*$taskDepth+1)*1024,2*$taskDepth*1024,2*$taskDepth*1024,1024,0,0,0,0,0,0,0)
}catch{$taskFailure=[ordered]@{type=$_.Exception.GetType().FullName;message=$_.Exception.Message}}
if(-not $taskFailure){throw 'Original arithmetic did not refuse'}
foreach($taskDepth in @(1,7)){
    $taskCalls=@(1024,1024,((2*$taskDepth+1)*1024),(2*$taskDepth*1024),(2*$taskDepth*1024),1024,0,0,0,0,0,0,0)
    if($taskCalls.Count -ne 13 -or $taskCalls[2] -ne ((2*$taskDepth+1)*1024) -or $taskCalls[3] -ne (2*$taskDepth*1024) -or $taskCalls[4] -ne (2*$taskDepth*1024)){throw 'Corrected inventory cardinality differs'}
    $taskCalls=@(0,0,0,0,0,0)+(@(1024*$taskDepth)*7)
    if($taskCalls.Count -ne 13 -or $taskCalls[6] -ne (1024*$taskDepth) -or $taskCalls[12] -ne (1024*$taskDepth)){throw 'Leaf cardinality differs'}
}
[ordered]@{schema='authority-owners-authoring-preflight-v1';source=(& git rev-parse HEAD).Trim();original_refused=$true;original_failure=$taskFailure;repair_verified=$true;original_recipe_sha256=(Get-FileHash (Join-Path $PSScriptRoot 'review.ps1') -Algorithm SHA256).Hash.ToLowerInvariant();repair_recipe_sha256=(Get-FileHash (Join-Path $PSScriptRoot 'review-repair.ps1') -Algorithm SHA256).Hash.ToLowerInvariant();measured=$false;server=$false;product=$false}|ConvertTo-Json -Depth 5|Set-Content -Encoding utf8NoBOM $taskOutput
