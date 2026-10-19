param([Parameter(Mandatory)][ValidateSet('cost','remaining')][string]$Stage,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label)
$ErrorActionPreference='Stop'
$taskOut=Join-Path $PSScriptRoot "$Label-analysis.json"
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing analysis'}
$taskReceiptPath=Join-Path $PSScriptRoot "$Label/receipt.json"
if(-not (Test-Path -LiteralPath $taskReceiptPath)){$taskReceiptPath=Join-Path $PSScriptRoot "$Label/outcome.json"}
$taskReceipt=Get-Content -Raw -LiteralPath $taskReceiptPath | ConvertFrom-Json
if($taskReceipt.stage -cne $Stage){throw 'Wrong cohort stage'}
$taskRows=@()
foreach($taskRow in $taskReceipt.rows){
    $taskDir=Join-Path $PSScriptRoot "$Label/$($taskRow.label)/$($taskRow.case)"
    $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskDir 'creator.log')
    $taskMatch=[regex]::Matches($taskRaw,'HC074-CLOSING (\{[^\r\n]+\})')
    if($taskMatch.Count -eq 0 -and $taskRow.exit_code -ne 0){
        $taskEnrollment=[regex]::Match($taskRaw,'HC074-OWNED-ENROLL success=false facts=(\{[^\r\n]+\})')
        if(-not $taskEnrollment.Success){throw 'Missing preclosing refusal observations'}
        $taskRows+=[ordered]@{label=$taskRow.label;case=$taskRow.case;exit_code=$taskRow.exit_code;actual_creator_sigkill=$false;closing_not_reached=$true;report=$null;enrollment_report=($taskEnrollment.Groups[1].Value | ConvertFrom-Json)}
        continue
    }
    if($taskMatch.Count -ne 1){throw 'Closing call did not produce exactly one report'}
    $taskReport=$taskMatch[0].Groups[1].Value | ConvertFrom-Json
    if($taskReport.mode -cne $taskRow.diagnostic_mode -or $taskReport.schedule_changed -cne $false -or $taskReport.deadline_changed -cne $false -or $taskReport.product -cne $false){throw 'Diagnostic scope'}
    $taskCustody=$taskReport.age.sites[0]
    foreach($taskFact in @($taskCustody.first,$taskCustody.last,$taskCustody.first_refusal) | Where-Object {$null -ne $_}){
        if(-not $taskFact.file.EndsWith('diagnostic_original_custody_join.rs') -or $taskFact.line -ne 121 -or $taskFact.age_ns -ne ($taskFact.now_ns-$taskFact.issued_ns)){throw 'Not the original custody comparison'}
    }
    if($taskRow.exit_code -eq 0){
        if($taskReport.success -cne $true -or $taskCustody.calls -le 0 -or $null -ne $taskCustody.first_refusal -or $taskCustody.last.reason -cne 'within-window'){throw 'Successful attempt has refused/missing custody sample'}
        $taskReady=Get-Content -Raw -LiteralPath (Join-Path $taskDir 'ready.json') | ConvertFrom-Json
        if($taskCustody.last.issued_ns -ne $taskReady.origins[2]){throw 'Original challenge changed'}
    }
    if($Stage -ceq 'cost'){
        if($taskReport.cost.wall_not_cpu -cne $true -or $taskReport.cost.overlapping_spans -cne $true -or $taskReport.cost.live.Count -ne 6 -or $taskReport.cost.kernel.Count -ne 10){throw 'Attribution units'}
    }elseif($null -ne $taskReport.cost){throw 'Facts cohort unexpectedly profiled'}
    $taskRows+=[ordered]@{label=$taskRow.label;case=$taskRow.case;exit_code=$taskRow.exit_code;actual_creator_sigkill=$taskRow.actual_creator_sigkill;report=$taskReport}
}
[ordered]@{schema=1;source=$taskReceipt.source;stage=$Stage;rows=$taskRows;historical_failing_tuple_recovered=$false;product_speedup_claimed=$false;production_admission=$false} | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $taskOut -Encoding utf8NoBOM
"Closing analysis retained; stage=$Stage; rows=$($taskRows.Count); historical_tuple_recovered=false"
