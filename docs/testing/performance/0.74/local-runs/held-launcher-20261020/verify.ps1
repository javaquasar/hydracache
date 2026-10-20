$ErrorActionPreference='Stop'
$taskSource='68b71834b7f9ec13a48ccbea93cd244935e9cf0b'
$taskRecipe=(Get-FileHash (Join-Path $PSScriptRoot 'final/checks.ps1') -Algorithm SHA256).Hash.ToLowerInvariant()
$taskFlags=@('original_join','watchdogs_armed','all_writer_revocation','trusted_role_build_production','original_send','server','supervisor','product','full_joined_sigkill','clock_changed')
function ReadJson([string]$Name){$taskPath=if($Name -in @('inventory.json','summary.json')){$Name}else{"final/$Name"};Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot $taskPath)|ConvertFrom-Json}
function Scope($Value){foreach($taskFlag in $taskFlags){if($Value.$taskFlag -isnot [bool] -or $Value.$taskFlag -cne $false){throw "Scope upgrade: $taskFlag"}}}
function Receipt($Value,[string]$Mode){
    $taskRounds=if($Mode -in @('194','188')){2}else{0}
    $taskCases=if($Mode -in @('194','188')){27}else{0}
    if($Value.source -cne $taskSource -or $Value.mode -cne $Mode -or $Value.recipe -cne $taskRecipe -or $Value.passed -isnot [bool] -or $Value.passed -cne $true -or $Value.matrix_rounds -ne $taskRounds -or $Value.cases_per_round -ne $taskCases){throw 'Receipt binding'}
    Scope $Value
}
function TestLog([string]$Name,[int]$Count){if(-not (Get-Content -Raw (Join-Path $PSScriptRoot "final/$Name")).Contains("test result: ok. $Count passed; 0 failed; 0 ignored;")){throw "Test count: $Name"}}
$taskInventory=ReadJson 'inventory.json'
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
$taskMembers=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskEntry in $taskInventory.files){
    if(-not $taskEntry.path -or $taskEntry.path -match '[:\\\x00-\x1f]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskMembers.Add($taskEntry.path)){throw 'Invalid member'}
    $taskFile=Get-Item -LiteralPath (Join-Path $PSScriptRoot $taskEntry.path)
    if($taskFile.PSIsContainer -or $taskFile.Attributes -band [IO.FileAttributes]::ReparsePoint -or $taskFile.Length -ne $taskEntry.bytes -or (Get-FileHash $taskFile.FullName -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskEntry.sha256){throw "Changed member: $($taskEntry.path)"}
}
$taskInventoryPath=Join-Path $PSScriptRoot 'inventory.json'
[string[]]$taskExpected=@($taskInventory.files.path)
[string[]]$taskActual=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Where-Object FullName -CNE $taskInventoryPath|ForEach-Object {$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if(($taskActual -join "`n") -cne ($taskExpected -join "`n")){throw 'Unlisted, missing or unsorted member'}
$taskCaseNames=@('healthy','owner-loss','foreign-sender','sigkill-held','sigkill-after-exec','lost-confirmation','guardian-one-loss','guardian-two-loss','expiry','guard-wins-over-release','socket-loss','short','long','wrong-token','wrong-digest','rights','duplicate','bad-hash','bad-stream','bad-pidfd','unsealed-config','exec-refused','changed-alarm','bad-config-hash','unsealed-worker','root-file-wrong-hash','opath-stdin')
[Array]::Sort($taskCaseNames,[StringComparer]::Ordinal)
foreach($taskMode in @('windows','194','188','repository')){
    Receipt (ReadJson "$taskMode-receipt.json") $taskMode
    $taskIdentity=ReadJson "$taskMode-identity.json"
    if($taskIdentity.source -cne $taskSource -or $taskIdentity.mode -cne $taskMode -or $taskIdentity.recipe -cne $taskRecipe -or $taskIdentity.helper_binary_built -isnot [bool] -or $taskIdentity.helper_binary_built -cne ($taskMode -cne 'repository')){throw 'Identity binding'}
    Scope $taskIdentity
    if($taskMode -ceq 'repository'){continue}
    $taskImages=ReadJson "$taskMode-images.json"
    if($taskImages.source -cne $taskSource -or $taskImages.mode -cne $taskMode -or $taskImages.recipe -cne $taskRecipe -or $taskImages.role_receipt_issued -isnot [bool] -or $taskImages.role_receipt_issued -cne $false){throw 'Image source binding'}
    $taskRoles=@('helper','unit','build-authority-tests');if($taskMode -cne 'windows'){$taskRoles+=@('process-matrix')}
    if($taskImages.images.Count -ne $taskRoles.Count){throw 'Image count'}
    foreach($taskRole in $taskRoles){
        $taskImage=@($taskImages.images|Where-Object role -CEQ $taskRole)
        if($taskImage.Count -ne 1 -or $taskImage[0].sha256 -cnotmatch '^[0-9a-f]{64}$' -or -not $taskImage[0].path){throw 'Image identity'}
        if($taskMode -ceq 'windows'){if($taskImage[0].path -notmatch '^[A-Za-z]:\\.+\.exe$'){throw 'Windows image path'}}
        elseif(-not $taskImage[0].path.StartsWith("/home/artur/.cache/hydracache-074-$taskMode-check/debug/",[StringComparison]::Ordinal)){throw 'Linux compiler path'}
    }
    TestLog "$taskMode-held-build.log" 16
    TestLog "$taskMode-docs.log" 2
    if($taskMode -ceq 'windows'){
        if(-not (Get-Content -Raw (Join-Path $PSScriptRoot 'final/windows-fallback.log')).Contains('no command/path fallback')){throw 'Fallback refusal'}
    }else{
        TestLog "$taskMode-unit.log" 3
        foreach($taskRound in 1..2){
            $taskName="$taskMode-matrix-$taskRound.log";TestLog $taskName 1
            $taskRaw=Get-Content -Raw (Join-Path $PSScriptRoot "final/$taskName")
            [string[]]$taskSeen=@([regex]::Matches($taskRaw,'held_launcher seed=0x74e7ec case=([a-z-]+) ')|ForEach-Object {$_.Groups[1].Value})
            [Array]::Sort($taskSeen,[StringComparer]::Ordinal)
            if(($taskSeen -join "`n") -cne ($taskCaseNames -join "`n")){throw 'Wrong case names/seed or duplicate/missing cases'}
            if([regex]::Matches($taskRaw,'held_exec_observed pid=[0-9]+ start_ticks=[0-9]+ helper_image=[0-9]+:[0-9]+ worker_image=[0-9]+:[0-9]+ retained_fds=0,1,2,5 argv_verified=true environment_empty=true').Count -ne 3){throw 'Missing kernel exec observations'}
            foreach($taskLine in ($taskRaw -split "`n"|Where-Object {$_ -match 'held_launcher seed='})){
                if(-not $taskLine.Contains('original_join=false product=false all_writer_revocation=false')){throw 'Case scope upgrade'}
            }
        }
    }
}
TestLog 'repository-source-guards.log' 10
TestLog 'repository-old-bridge.log' 3
$taskSummary=ReadJson 'summary.json'
if($taskSummary.source -cne $taskSource -or $taskSummary.recipe -cne $taskRecipe -or $taskSummary.old_sources_reconstructed -ne 438 -or $taskSummary.named_runtime_additions -ne 2 -or $taskSummary.qualification_manifest_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or $taskSummary.full_321_rerun -isnot [bool] -or $taskSummary.full_321_rerun -cne $false){throw 'Summary binding'}
Scope $taskSummary
# In-memory receipt controls, never mutate sealed files.
$taskNegative=0;$taskBase=ReadJson 'windows-receipt.json'
foreach($taskField in (@('source','mode','recipe','passed','matrix_rounds','cases_per_round')+$taskFlags)){
    foreach($taskKind in @('change','missing')){
        $taskBad=$taskBase|ConvertTo-Json|ConvertFrom-Json
        if($taskKind -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskField)}
        elseif($taskField -ceq 'passed'){$taskBad.$taskField=$false}
        elseif($taskField -in @('matrix_rounds','cases_per_round')){$taskBad.$taskField=1}
        elseif($taskBad.$taskField -is [bool]){$taskBad.$taskField=$true}
        else{$taskBad.$taskField='wrong'}
        $taskRefused=$false;try{Receipt $taskBad 'windows'}catch{$taskRefused=$true}
        if(-not $taskRefused){throw "Accepted metadata mutation: $taskField/$taskKind"};$taskNegative++
    }
}
[ordered]@{verified=$true;files=$taskMembers.Count;receipt_metadata_mutations_refused=$taskNegative;inventory_sha256=(Get-FileHash $taskInventoryPath -Algorithm SHA256).Hash.ToLowerInvariant();process_cases_per_round=27;rounds_per_linux_compiler=2;original_join=$false;all_writer_revocation=$false;server=$false;product=$false}|ConvertTo-Json
