param([Parameter(Mandatory)][ValidatePattern('^[0-9a-f]{40}$')][string]$Source)
$ErrorActionPreference='Stop'
function Json([string]$Name){Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $Name)|ConvertFrom-Json}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
$taskInventory=Json 'inventory.json'
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
if(@(Get-ChildItem -LiteralPath $PSScriptRoot -Force -Recurse|Where-Object {$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Reparse member'}
$taskMembers=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskEntry in $taskInventory.files){
    if(-not $taskEntry.path -or $taskEntry.path -match '[:\\\x00-\x1f]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskMembers.Add($taskEntry.path)){throw 'Invalid member'}
    $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskEntry.path)
    if($taskFile.PSIsContainer -or $taskFile.Length -ne $taskEntry.bytes -or (Hash $taskFile.FullName) -cne $taskEntry.sha256){throw "Changed member: $($taskEntry.path)"}
}
[string[]]$taskActual=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Force -Recurse|Where-Object Name -CNE 'inventory.json'|ForEach-Object {$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if(($taskActual -join "`n") -cne ($taskInventory.files.path -join "`n")){throw 'Unlisted, missing or unsorted member'}
$taskIdentity=Json 'identity.json'
$taskSummary=Json 'summary.json'
$taskFalse=@('privileged_full_join','product','installation','supervisor_operation','clock_changed','cpu_affinity','full_joined_sigkill','original_send','clock_cause_established','production_enrollment','production_role_receipt_issued','late_original_join')
function Summary($Value){
    [string[]]$taskKeys=@('source','runner_sha256','previous_inventory_sha256','qualification_manifest_sha256','selections','rounds','counts','comparison_passed','supervisor_unchanged')+$taskFalse
    [string[]]$taskGiven=@($Value.PSObject.Properties.Name)
    [Array]::Sort($taskKeys,[StringComparer]::Ordinal);[Array]::Sort($taskGiven,[StringComparer]::Ordinal)
    if(($taskKeys -join ',') -cne ($taskGiven -join ',')){throw 'Unexpected or missing summary field'}
    if($Source -cne 'f07dcca31e2e69796644d70fef695404b22d1519' -or $Value.source -cne $Source -or $Value.runner_sha256 -cne (Hash (Join-Path $PSScriptRoot 'run.sh')) -or $Value.previous_inventory_sha256 -cne '617f703d5b931e3156d4d2a4a5a60db1cfb9a338e5281a3d090e51f677299f1a' -or $Value.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or $Value.selections -ne 16 -or $Value.rounds -ne 2 -or ($Value.counts -join ',') -cne '17,26,5,38'){throw 'Comparison binding'}
    foreach($taskFlag in $taskFalse){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $false){throw "Scope upgrade: $taskFlag"}}
    foreach($taskFlag in 'comparison_passed','supervisor_unchanged'){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $true){throw 'Incomplete comparison'}}
}
Summary $taskSummary
$taskLocal=Json 'local-checks.json'
if($taskLocal.source -cne $Source -or $taskLocal.passed -isnot [bool] -or $taskLocal.passed -cne $true -or ($taskLocal.checks -join ',') -cne 'guards,predecessor,lint,registry,docs,contract,fmt' -or $taskLocal.recipe_sha256 -cne (Hash (Join-Path $PSScriptRoot 'checks.ps1'))){throw 'Local check receipt'}
foreach($taskPair in @(@('guards','test result: ok. 18 passed; 0 failed;'),@('predecessor','test result: ok. 3 passed; 0 failed;'),@('registry','gated-test-check: OK (186 discovered gates, all registered)'),@('docs','doc-check: OK'),@('contract','performance-contract-check 0.74: OK (local, non-promotable)'))){
    if(-not (Get-Content -Raw (Join-Path $PSScriptRoot "local-$($taskPair[0]).log")).Contains($taskPair[1])){throw 'Local check log'}
}
$taskPrior=Json 'preceding-verification.json'
if($taskPrior.evidence_verified -isnot [bool] -or $taskPrior.evidence_verified -cne $true -or $taskPrior.inventory_sha256 -cne $taskSummary.previous_inventory_sha256 -or $taskPrior.local_join_gate_passed -isnot [bool] -or $taskPrior.local_join_gate_passed -cne $false){throw 'Do not erase the preceding failed local gate'}
if($taskIdentity.source -cne $Source -or $taskIdentity.runner_commit -cne 'f13ec7b8620f4cff2be9e72f248665fa05fce26a' -or $taskIdentity.host_alias -cne 'hydracache-ax42' -or $taskIdentity.user -cne 'hydracache-admin' -or $taskIdentity.uid -ne 1000 -or $taskIdentity.gid -ne 1000 -or $taskIdentity.remote_root -cnotmatch '^/tmp/hc074-consuming-server-20261020\.[A-Za-z0-9]{6}$'){throw 'Host identity'}
foreach($taskFlag in @('privileged_full_join','product','installation','supervisor_operation','clock_changed','cpu_affinity')){if($taskIdentity.$taskFlag -isnot [bool] -or $taskIdentity.$taskFlag -cne $false){throw 'Identity scope upgrade'}}
$taskWrapper=Get-Content -Raw (Join-Path $PSScriptRoot 'wrapper.log')
if((Get-Content -Raw (Join-Path $PSScriptRoot 'wrapper.exit')).Trim() -cne '0' -or -not $taskWrapper.Contains('HC074-CONSUMING-SERVER complete=true failed=0 supervisor_unchanged=true') -or ([regex]::Matches($taskWrapper,'HC074-CONSUMING-SERVER preflight=ok supervisor_pid=8839 restarts=0')).Count -ne 34){throw 'Incomplete wrapper/preflight'}
$taskHashes=@{
    'join-194'='ff41048700a4577dcd1efdaf1a2332a9431523086fd84855cb194b3dd98648fd'
    'join-188'='13d4a4b274a172d6e3cc68dbaa352c6a6649ebcea65551cd1925725e4867c135'
    'old-194'='fb9ae321690d0fb4f7eb29aace444e4f4cf06729b5dbb3a77a134ea1cf2bca14'
    'old-188'='64254b3936c76be2dc1fe208ae5c0020d5db713568afd19881c84c129de8d441'
}
$taskTransfer=Get-Content -Raw (Join-Path $PSScriptRoot 'transfer.log')
if(-not $taskTransfer.Contains("$($taskSummary.runner_sha256)  run.sh")){throw 'Transferred runner differs'}
foreach($taskImage in $taskHashes.Keys){
    if(-not $taskTransfer.Contains("$($taskHashes[$taskImage])  $taskImage") -or ([regex]::Matches($taskWrapper,"(?m)^$taskImage`: OK$")).Count -ne 2){throw 'Image binding'}
}
$taskRemoteMembers=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskLane in '194','188'){foreach($taskRound in 1..2){foreach($taskSelection in 'new','trust','fixed','old'){foreach($taskExt in 'log','exit'){[void]$taskRemoteMembers.Add("$taskLane-$taskSelection-$taskRound.$taskExt")}}}}
foreach($taskName in 'wrapper.log','wrapper.exit','run.sh'){[void]$taskRemoteMembers.Add($taskName)}
foreach($taskLine in Get-Content (Join-Path $PSScriptRoot 'remote-hashes.log')){
    if($taskLine -cnotmatch '^([0-9a-f]{64})  ([-A-Za-z0-9.]+)$'){throw 'Invalid remote hash line'}
    $taskDigest=$Matches[1];$taskName=$Matches[2]
    if(-not $taskRemoteMembers.Remove($taskName) -or (Hash (Join-Path $PSScriptRoot $taskName)) -cne $taskDigest){throw 'Raw remote evidence differs'}
}
if($taskRemoteMembers.Count -ne 0){throw 'Missing remote evidence hashes'}
$taskChecks=0
foreach($taskLane in '194','188'){
    foreach($taskRound in 1..2){
        foreach($taskSelection in 'new','trust','fixed','old'){
            $taskCount=@{new=17;trust=26;fixed=5;old=38}[$taskSelection]
            $taskName="$taskLane-$taskSelection-$taskRound"
            $taskRaw=Get-Content -Raw (Join-Path $PSScriptRoot "$taskName.log")
            if((Get-Content -Raw (Join-Path $PSScriptRoot "$taskName.exit")).Trim() -cne '0' -or -not $taskRaw.Contains("test result: ok. $taskCount passed; 0 failed; 0 ignored;") -or -not $taskWrapper.Contains("HC074-CONSUMING-SERVER selection=$taskName expected=$taskCount exit=0")){throw "Failed selection: $taskName"}
            if($taskSelection -ceq 'new'){
                foreach($taskSample in 0..7){if(-not $taskRaw.Contains("HC074-CONSUMING-MISMATCH-TRACE sample=$taskSample assertions_ok=true first=None")){throw 'Missing actual kernel assertion control'}}
            }
            $taskChecks++
        }
    }
}
$taskPost=Get-Content -Raw (Join-Path $PSScriptRoot 'postflight.log')
foreach($taskNeedle in @('MainPID=8839','NRestarts=0','ActiveState=active','SubState=running','e81671d6-bf23-4ebf-b8fe-5dad1f45866f','f797629f65385ebfd4d94577a43a2d1d020f71c1e841028dad7d02a51b47ad62','b314953d4deedaeaade3e24468da4844b7f13d348a5d77b8e8f41c672ff44525','POSTFLIGHT-MARKERS-ABSENT=9')){if(-not $taskPost.Contains($taskNeedle)){throw 'Postflight binding'}}
$taskNegative=0
foreach($taskField in (@('source','runner_sha256','previous_inventory_sha256','qualification_manifest_sha256','selections','rounds','counts','comparison_passed','supervisor_unchanged')+$taskFalse)){
    foreach($taskKind in 'change','missing'){
        $taskBad=$taskSummary|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}
        elseif($taskBad.$taskField -is [bool]){$taskBad.$taskField=-not $taskBad.$taskField}
        else{$taskBad.$taskField='wrong'}
        $taskRefused=$false;try{Summary $taskBad}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted metadata mutation: $taskField/$taskKind"}
        $taskNegative++
    }
}
[ordered]@{evidence_verified=$true;files=$taskMembers.Count;selections=$taskChecks;metadata_mutations_refused=$taskNegative;comparison_passed=$true;privileged_full_join=$false;clock_cause_established=$false;product=$false}|ConvertTo-Json
