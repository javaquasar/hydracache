$ErrorActionPreference='Stop'
Set-Location (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskRows=@()
foreach($taskName in @('docs','diff')){
    $taskLog=Join-Path $PSScriptRoot "final-$taskName.log"
    if(Test-Path -LiteralPath $taskLog){throw 'Never overwrite final evidence'}
    if($taskName -ceq 'docs'){ & cargo xtask doc-check *> $taskLog }
    else{ & git diff --check *> $taskLog }
    $taskCode=$LASTEXITCODE
    $taskRows+=@{name=$taskName;exit=$taskCode;log=[IO.Path]::GetFileName($taskLog);sha256=(Get-FileHash $taskLog -Algorithm SHA256).Hash.ToLowerInvariant()}
    if($taskCode -ne 0){throw "Final check refused $taskName"}
}
$taskPath=Join-Path $PSScriptRoot 'final-checks.json'
$taskRaw=[Text.UTF8Encoding]::new($false).GetBytes((ConvertTo-Json -InputObject @{steps=$taskRows} -Depth 8)+"`n")
$taskFile=[IO.File]::Open($taskPath,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::Read)
try{$taskFile.Write($taskRaw);$taskFile.Flush($true)}finally{$taskFile.Dispose()}
