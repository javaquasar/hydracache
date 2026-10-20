param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
function Json([string]$Name){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Name)|ConvertFrom-Json}
function Raw([string]$Name){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Name)}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$taskInventory=Json 'inventory.json'
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
if(@(Get-ChildItem $PSScriptRoot -Force -Recurse|Where-Object{$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Reparse member'}
$taskMembers=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskEntry in $taskInventory.files){
    if(-not $taskEntry.path -or $taskEntry.path -match '[:\\\x00-\x1f]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskMembers.Add($taskEntry.path)){throw 'Invalid member'}
    $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskEntry.path)
    if($taskFile.PSIsContainer -or $taskFile.Length -ne $taskEntry.bytes -or (Hash $taskFile.FullName) -cne $taskEntry.sha256){throw "Changed member: $($taskEntry.path)"}
}
[string[]]$taskActual=@(Get-ChildItem $PSScriptRoot -Force -Recurse -File|Where-Object Name -CNE 'inventory.json'|ForEach-Object{$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if(($taskActual -join "`n") -cne ($taskInventory.files.path -join "`n")){throw 'Unlisted, missing or unsorted member'}
if($Source -cne '1eac3bb37814d5aaf44f84370606218719c01c2f'){throw 'Wrong final code source'}
$taskPriorLocal=Json 'preceding-local-verification.json';$taskPriorServer=Json 'preceding-server-verification.json'
if($taskPriorLocal.inventory_sha256 -cne '617f703d5b931e3156d4d2a4a5a60db1cfb9a338e5281a3d090e51f677299f1a' -or $taskPriorLocal.local_join_gate_passed -isnot [bool] -or $taskPriorLocal.local_join_gate_passed -cne $false -or $taskPriorServer.comparison_passed -isnot [bool] -or $taskPriorServer.comparison_passed -cne $true -or $taskPriorServer.privileged_full_join -isnot [bool] -or $taskPriorServer.privileged_full_join -cne $false){throw 'Earlier comparison scopes changed'}
function TestLog([string]$Name,[int]$Count,[int]$Ignored=0){if(-not (Raw $Name).Contains("test result: ok. $Count passed; 0 failed; $Ignored ignored;")){throw "Wrong test selection: $Name"}}
function Exit([string]$Prefix,[string]$Selection,[string]$ExpectedSource){
    $taskExit=Json "$Prefix-$Selection-exit.json"
    if($taskExit.source -cne $ExpectedSource -or $taskExit.selection -cne $Selection -or ($taskExit.exit -isnot [long] -and $taskExit.exit -isnot [int])){throw 'Exit binding'}
    return $taskExit.exit
}
if((Json 'baseline-result.json').exit -ne 0){throw 'Missing old baseline pass'}
TestLog 'baseline-live.log' 1
foreach($taskLane in '194','188'){
    $taskFirst=Json "$taskLane-result.json"
    if($taskFirst.source -cne 'fcac7eddade33b7173c9d8602033d64afc39309b' -or $taskFirst.results.live -ne 101 -or $taskFirst.results.attribution -ne 101 -or $taskFirst.live_matrix_passed -isnot [bool] -or $taskFirst.live_matrix_passed -cne $false){throw 'Retain initial fixture refusal'}
    TestLog "$taskLane-models.log" 18 1;TestLog "$taskLane-trust.log" 26
    if(-not (Raw "$taskLane-live.log").Contains('consuming_held_live_documents.rs:127:10:')){throw 'Initial policy refusal site missing'}
}
$taskBound=Json 'bound-194-result.json'
if($taskBound.source -cne '5172d69b916dcf8aa97c04a602ebc419e229b3c6' -or $taskBound.results.live -ne 101 -or $taskBound.results.attribution -ne 101){throw 'Retain overlapping corrected-input refusal'}
TestLog 'bound-194-models.log' 19 1;TestLog 'bound-194-trust.log' 26
if(-not (Raw 'bound-194-live.log').Contains('Original(Challenge(Time))')){throw 'Original Time refusal missing'}
$taskSerial=Json 'serial-194-result.json'
if($taskSerial.image_source -cne $taskBound.source -or $taskSerial.results.live -ne 101 -or $taskSerial.results.attribution -ne 0 -or $taskSerial.preceding_cohort_accepted -isnot [bool] -or $taskSerial.preceding_cohort_accepted -cne $false){throw 'Do not promote partial serial control'}
foreach($taskCase in 'healthy','revalidate','role-before'){if(-not (Raw 'serial-194-live.log').Contains("HC074-HELD-LIVE case=$taskCase seed=0x074c051e actual_fixed=true actual_consuming=true")){throw 'Actual partial holder control missing'}}
TestLog 'serial-194-attribution.log' 1
$taskSerialIdentity=Json 'serial-identity.json'
if($taskSerialIdentity.image_sha256 -cne (Json 'bound-194-image.json').sha256 -or $taskSerialIdentity.recipe_sha256 -cne (Hash (Join-Path $PSScriptRoot 'serial-control.ps1'))){throw 'Serial image/recipe differs'}
$taskDerived=$true
$taskReports=@()
foreach($taskLane in '194','188'){
    $taskImage=Json "traced-$taskLane-image.json"
    if($taskImage.source -cne $Source -or $taskImage.mode -cne $taskLane -or $taskImage.sha256 -cnotmatch '^[0-9a-f]{64}$' -or $taskImage.recipe_sha256 -cne (Hash (Join-Path $PSScriptRoot 'traced-checks.ps1'))){throw 'Final image binding'}
    $taskResult=Json "traced-$taskLane-result.json"
    if($taskResult.source -cne $Source -or $taskResult.mode -cne $taskLane){throw 'Final result binding'}
    foreach($taskSelection in 'models','trust','live','attribution','lint'){
        if((Exit "traced-$taskLane" $taskSelection $Source) -ne $taskResult.results.$taskSelection){throw 'Result/exit mismatch'}
    }
    if($taskResult.results.models -ne 0 -or $taskResult.results.trust -ne 0 -or $taskResult.results.lint -ne 0){throw 'Required targeted checks refused'}
    TestLog "traced-$taskLane-models.log" 20 1;TestLog "traced-$taskLane-trust.log" 26
    $taskPassed=$taskResult.results.live -eq 0
    if($taskResult.live_matrix_passed -isnot [bool] -or $taskResult.live_matrix_passed -cne $taskPassed){throw 'Wrong matrix status'}
    if($taskPassed){foreach($taskCase in 'healthy','revalidate','role-before','role-after','trust-after'){if(-not (Raw "traced-$taskLane-live.log").Contains("HC074-HELD-LIVE case=$taskCase seed=0x074c051e actual_fixed=true actual_consuming=true")){throw 'Incomplete matrix'}};TestLog "traced-$taskLane-live.log" 1}
    $taskDerived=$taskDerived -and $taskPassed
    foreach($taskSelection in 'live','attribution'){
        foreach($taskLine in Get-Content (Join-Path $PSScriptRoot "traced-$taskLane-$taskSelection.log")){
            $taskMarker='HC074-HELD-LIVE-ORIGINAL ';$taskIndex=$taskLine.IndexOf($taskMarker,[StringComparison]::Ordinal)
            if($taskIndex -ge 0){$taskReports+=($taskLine.Substring($taskIndex+$taskMarker.Length)|ConvertFrom-Json)}
        }
    }
    foreach($taskFlag in 'server','product','original_send','full_joined_sigkill','real_role_image_cost'){if($taskResult.$taskFlag -isnot [bool] -or $taskResult.$taskFlag -cne $false){throw 'Final scope upgrade'}}
}
if($taskReports.Count -lt 4){throw 'Missing original observation reports'}
foreach($taskReport in $taskReports){foreach($taskFlag in 'guard_clock_added','deadline_changed','server','product'){if($taskReport.$taskFlag -isnot [bool] -or $taskReport.$taskFlag -cne $false){throw 'Original observation scope upgrade'}}}
TestLog 'full-contract.log' 321;TestLog 'windows-held.log' 28;TestLog 'guards.log' 20
TestLog 'final-guards.log' 20
if(-not (Raw 'windows-lint.log').Contains('duplicate_mod') -or -not (Raw 'windows-lint-fixed.log').Contains('Finished')){throw 'Retain lint refusal and fix'}
if(-not (Raw 'final-windows-lint.log').Contains('Finished') -or -not (Raw 'final-contract.log').Contains('performance-contract-check 0.74: OK (local, non-promotable)')){throw 'Final repository checks incomplete'}
$taskReporting=Json 'reporting-checks.json'
if($taskReporting.source -cne '4a01412e6f81fe8b853f511ecf63cde95a732b75' -or $taskReporting.linux_lint_exit -ne 0 -or $taskReporting.doc_check_exit -ne 0 -or $taskReporting.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'){throw 'Reporting checks changed'}
$taskFormat=Json 'final-fmt.json'
if($taskFormat.source -cne '4a01412e6f81fe8b853f511ecf63cde95a732b75' -or $taskFormat.fmt_exit -ne 0){throw 'Final format receipt'}
# The last source change corrects only a report label. No assertion or callback changes.
Push-Location (Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
try {
    $taskPath='tools/long-run-supervisor-074/tests/support/consuming_held_live.rs'
    $taskOld=(@(& git show "$Source`:$taskPath") -join "`n")+"`n"
    $taskNew=(@(& git show "4a01412e6f81fe8b853f511ecf63cde95a732b75`:$taskPath") -join "`n")+"`n"
    $taskLabel=' fence_retained=true guardians_terminal=true synthetic_images=true '
    if(([regex]::Matches($taskOld,[regex]::Escape($taskLabel))).Count -ne 1 -or $taskNew -cne $taskOld.Replace($taskLabel,' fence_retained=true custodians_terminal=true synthetic_images=true ')){throw 'Not a report-only correction'}
}finally{Pop-Location}
$taskSummary=Json 'summary.json'
$taskFalse=@('server','product','original_send','full_joined_sigkill','real_role_image_cost','clock_actor_established','production_enrollment','qualification_changed','preceding_cohorts_accepted','root_guardians_terminal_claim')
function Summary($Value){
    [string[]]$taskExpected=@('source','reporting_source','frozen_old_sources','qualification_manifest_sha256','actual_holder_observed','final_matrix_passed')+$taskFalse
    [string[]]$taskGiven=@($Value.PSObject.Properties.Name)
    [Array]::Sort($taskExpected,[StringComparer]::Ordinal);[Array]::Sort($taskGiven,[StringComparer]::Ordinal)
    if(($taskExpected -join ',') -cne ($taskGiven -join ',')){throw 'Unexpected or missing summary fields'}
    if($Value.source -cne $Source -or $Value.reporting_source -cne '4a01412e6f81fe8b853f511ecf63cde95a732b75' -or $Value.frozen_old_sources -ne 446 -or $Value.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or $Value.actual_holder_observed -isnot [bool] -or $Value.actual_holder_observed -cne $true -or $Value.final_matrix_passed -isnot [bool] -or $Value.final_matrix_passed -cne $taskDerived){throw 'Summary binding'}
    foreach($taskFlag in $taskFalse){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $false){throw "Scope upgrade: $taskFlag"}}
}
Summary $taskSummary
$taskNegative=0
foreach($taskField in (@('source','reporting_source','frozen_old_sources','qualification_manifest_sha256','actual_holder_observed','final_matrix_passed')+$taskFalse)){
    foreach($taskKind in 'change','missing'){
        $taskBad=$taskSummary|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}elseif($taskBad.$taskField -is [bool]){$taskBad.$taskField=-not $taskBad.$taskField}else{$taskBad.$taskField='wrong'}
        $taskRefused=$false;try{Summary $taskBad}catch{$taskRefused=$true}
        if(-not $taskRefused){throw 'Accepted metadata mutation'};$taskNegative++
    }
}
[ordered]@{evidence_verified=$true;files=$taskMembers.Count;metadata_mutations_refused=$taskNegative;actual_holder_observed=$true;final_matrix_passed=$taskDerived;full_joined_sigkill=$false;real_role_image_cost=$false;clock_actor_established=$false;server=$false;product=$false}|ConvertTo-Json
