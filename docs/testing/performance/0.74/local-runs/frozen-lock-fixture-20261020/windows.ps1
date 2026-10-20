param([Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    $taskSource=(& git rev-parse HEAD).Trim()
    function Stable {
        if((& git status --porcelain -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests Cargo.lock) -or (& git rev-parse HEAD).Trim() -cne $taskSource){throw 'Source drift'}
    }
    function Run([string]$Name,[string[]]$Arguments){
        & cargo @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
    }
    Stable
    [ordered]@{source=$taskSource;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();ordinary_package_only=$true;ignored_selected=$false;runtime_changed=$false;production_pin_changed=$false;original_send=$false;server=$false;supervisor=$false;product=$false;qualification=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'identity.json') -Encoding utf8NoBOM
    Run 'build-json' @('test','--locked','-p','hydracache-long-run-supervisor-074','--no-run','--message-format=json')
    $taskRows=@(Get-Content -LiteralPath (Join-Path $taskOut 'build-json.log') | Where-Object { $_.StartsWith('{',[StringComparison]::Ordinal) } | ForEach-Object {$_ | ConvertFrom-Json})
    $taskTargets=@($taskRows | Where-Object { $_.reason -ceq 'compiler-artifact' -and $_.package_id -cmatch '#hydracache-long-run-supervisor-074@' -and $_.profile.test -ceq $true -and $_.executable })
    if($taskTargets.Count -eq 0){throw 'No package test images'}
    $taskProofs=@($taskTargets | ForEach-Object {[ordered]@{target=$_.target.name;kind=$_.target.kind;path=$_.executable;sha256=(Get-FileHash -LiteralPath $_.executable -Algorithm SHA256).Hash.ToLowerInvariant()}})
    $taskProofs | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $taskOut 'images.json') -Encoding utf8NoBOM
    Run 'package' @('test','--locked','-p','hydracache-long-run-supervisor-074')
    $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut 'package.log')
    $taskSummaries=@([regex]::Matches($taskRaw,'test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored;') | ForEach-Object {[ordered]@{result=$_.Groups[1].Value;passed=[int]$_.Groups[2].Value;failed=[int]$_.Groups[3].Value;ignored=[int]$_.Groups[4].Value}})
    if($taskSummaries.Count -eq 0 -or @($taskSummaries | Where-Object result -CNE 'ok').Count -ne 0 -or -not $taskRaw.Contains('test result: ok. 17 passed; 0 failed;') -or -not $taskRaw.Contains('test result: ok. 8 passed; 0 failed;') -or -not $taskRaw.Contains('test result: ok. 20 passed; 0 failed;')){throw 'Package incomplete'}
    Stable
    foreach($taskImage in $taskProofs){if((Get-FileHash -LiteralPath $taskImage.path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskImage.sha256){throw 'Image drift'}}
    [ordered]@{source=$taskSource;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();package_passed=$true;passed=($taskSummaries.passed | Measure-Object -Sum).Sum;ignored=($taskSummaries.ignored | Measure-Object -Sum).Sum;summaries=$taskSummaries;images=$taskProofs.Count;runtime_changed=$false;production_pin_changed=$false;new_live_matrix=$false;clock_changed=$false;original_send=$false;server=$false;supervisor=$false;product=$false;qualification=$false} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
