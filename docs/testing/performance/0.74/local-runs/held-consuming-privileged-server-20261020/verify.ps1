param([switch]$UnsealedReview)
$ErrorActionPreference='Stop'
. "$PSScriptRoot/facts.ps1"
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Json([string]$Name){Get-Content -Raw -LiteralPath "$PSScriptRoot/$Name"|ConvertFrom-Json}
if(@(Get-ChildItem $PSScriptRoot -Force -Recurse|Where-Object{$_.Attributes -band [IO.FileAttributes]::ReparsePoint}).Count){throw 'Reparse member'}
[string[]]$taskActual=@(Get-ChildItem $PSScriptRoot -Force -Recurse -File|Where-Object Name -CNE 'inventory.json'|ForEach-Object{$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/')})
[Array]::Sort($taskActual,[StringComparer]::Ordinal)
if($UnsealedReview){
    if(Test-Path "$PSScriptRoot/inventory.json"){throw 'Do not bypass an existing seal'}
}else{
$taskInventory=Json 'inventory.json'
if($taskInventory.schema -cne 'sha256-local-file-inventory-v1'){throw 'Inventory schema'}
$taskMembers=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskEntry in $taskInventory.files){
    if(-not $taskEntry.path -or $taskEntry.path -match '[:\\\x00-\x1f]' -or $taskEntry.path.StartsWith('/') -or $taskEntry.path.Split('/') -contains '..' -or $taskEntry.path.Split('/') -contains '.' -or $taskEntry.path -ceq 'inventory.json' -or -not $taskMembers.Add($taskEntry.path)){throw 'Invalid member'}
    $taskFile=Get-Item -LiteralPath "$PSScriptRoot/$($taskEntry.path)"
    if($taskFile.PSIsContainer -or $taskFile.Length -ne $taskEntry.bytes -or (Hash $taskFile.FullName) -cne $taskEntry.sha256){throw "Changed member $($taskEntry.path)"}
}
if(($taskActual -join "`n") -cne ($taskInventory.files.path -join "`n")){throw 'Missing, unlisted or unsorted member'}
}
$taskRemote=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
foreach($taskLine in Get-Content "$PSScriptRoot/remote/remote-files.sha256"){
    if($taskLine -cnotmatch '^([0-9a-f]{64})  ([A-Za-z0-9.-]+)$'){throw 'Remote manifest syntax'}
    $taskHash=$Matches[1];$taskName=$Matches[2]
    if(-not $taskRemote.Add($taskName) -or (Hash "$PSScriptRoot/remote/$taskName") -cne $taskHash){throw 'Remote transfer mismatch'}
    if(-not (Get-Content -Raw "$PSScriptRoot/export-check.log").Contains($taskLine)){throw 'Root export transcript differs'}
}
if($taskRemote.Count -ne 17 -or @(Get-ChildItem "$PSScriptRoot/remote" -File).Count -ne 18){throw 'Wrong raw export selection'}
foreach($taskLog in 'preflight.log','postflight.log','194-preflight.log','194-postflight.log','188-preflight.log','188-postflight.log'){
    $taskRaw=Get-Content -Raw "$PSScriptRoot/remote/$taskLog"
    foreach($taskMarker in 'HC074-HELD-ROOT preflight=ok installed_unchanged=true','MainPID=8839','NRestarts=0','ActiveState=active','SubState=running'){
        if(-not $taskRaw.Contains($taskMarker)){throw 'Host preflight missing'}
    }
    if(([regex]::Matches($taskRaw,'(?m)^ABSENT /var/lib/hydracache-performance/')).Count -ne 9){throw 'Marker checks missing'}
}
if((Hash "$PSScriptRoot/remote/mountinfo-before.log") -cne (Hash "$PSScriptRoot/remote/mountinfo-after.log")){throw 'Host mount context changed'}
if((Hash "$PSScriptRoot/run.sh") -cne 'f18122dfa990b0044838db11c6d9291dd54e37fbe0492f675d04cfd4ce3bf109' -or (Hash "$PSScriptRoot/inputs.sha256") -cne 'f68dd8ce74e4cd0d6b31c10ce30b3e429d2ed284233b53cd7a64040f80bdd095'){throw 'Reviewed runner inputs changed'}
if((Get-Content -Raw "$PSScriptRoot/remote/wrapper.exit").Trim() -cne '1' -or (Get-Content -Raw "$PSScriptRoot/remote/postflight.exit").Trim() -cne '0'){throw 'Wrapper result changed'}
$taskCollection=Json 'collection.json'
if($taskCollection.stage_shell_exit -ne 127 -or $taskCollection.wrapper_exit -ne 1 -or $taskCollection.postflight_exit -ne 0 -or $taskCollection.wrapper_started_once -cne $true -or $taskCollection.no_retry -cne $true){throw 'Retain wrapper and transport outcomes separately'}
foreach($taskSelection in 'contract-tests','source-guards','targeted-fmt','doc-check','registry-check','contract-check','lint'){
    if((Json "$taskSelection-exit.json").exit -ne 0){throw "Required local check refused: $taskSelection"}
}
foreach($taskPair in @(@('contract-tests',321),@('source-guards',20))){
    if(-not (Get-Content -Raw "$PSScriptRoot/$($taskPair[0]).log").Contains("test result: ok. $($taskPair[1]) passed; 0 failed; 0 ignored;")){throw 'Wrong local test selection'}
}
if(-not (Get-Content -Raw "$PSScriptRoot/contract-check.log").Contains('performance-contract-check 0.74: OK (local, non-promotable)')){throw 'Local contract incomplete'}
if((Json 'fmt-exit.json').exit -ne 1 -or -not (Get-Content -Raw "$PSScriptRoot/fmt.log").Contains('(os error 206)')){throw 'Retain Windows all-workspace formatting limit'}
$taskSyntax=Json 'syntax-checks.json';$taskPowerShell=Json 'powershell-check.json'
if($taskSyntax.runner_syntax_exit -ne 0 -or $taskSyntax.stage_syntax_exit -ne 0 -or $taskSyntax.export_syntax_exit -ne 0 -or $taskPowerShell.powershell_syntax_passed -cne $true -or $taskPowerShell.parsed_scripts -ne 2){throw 'Recipe syntax not checked'}
$taskExpected=(ControlFacts)|ConvertTo-Json -Depth 20|ConvertFrom-Json
function Summary($Value){
    [string[]]$taskKeys=@($taskExpected.PSObject.Properties.Name)
    [string[]]$taskGiven=@($Value.PSObject.Properties.Name)
    [Array]::Sort($taskKeys,[StringComparer]::Ordinal);[Array]::Sort($taskGiven,[StringComparer]::Ordinal)
    if(($taskKeys -join ',') -cne ($taskGiven -join ',')){throw 'Summary keys'}
    foreach($taskKey in $taskKeys){
        if(($Value.$taskKey|ConvertTo-Json -Depth 20 -Compress) -cne ($taskExpected.$taskKey|ConvertTo-Json -Depth 20 -Compress)){throw "Summary binding $taskKey"}
    }
}
Summary (Json 'summary.json')
$taskNegative=0
foreach($taskKey in $taskExpected.PSObject.Properties.Name){
    foreach($taskMode in 'missing','wrong'){
        $taskMutant=$taskExpected|ConvertTo-Json -Depth 20|ConvertFrom-Json
        if($taskMode -ceq 'missing'){$taskMutant.PSObject.Properties.Remove($taskKey)}else{
            $taskMutant.$taskKey=if($taskMutant.$taskKey -is [bool]){-not $taskMutant.$taskKey}else{'changed'}
        }
        $taskRefused=$false;try{Summary $taskMutant}catch{$taskRefused=$true}
        if(-not $taskRefused){throw 'Metadata mutation accepted'};$taskNegative++
    }
}
$taskMutant=$taskExpected|ConvertTo-Json -Depth 20|ConvertFrom-Json
$taskMutant|Add-Member unexpected $true
$taskRefused=$false;try{Summary $taskMutant}catch{$taskRefused=$true}
if(-not $taskRefused){throw 'Extra key accepted'};$taskNegative++
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
if((Hash "$taskRepo/docs/testing/performance/0.74/qualification-manifest.toml") -cne $taskExpected.qualification_manifest_sha256){throw 'Frozen qualification changed'}
if((Hash "$PSScriptRoot/../held-consuming-live-20261020/inventory.json") -cne '46956c62bddd8def65e8132d905d6235d41f51572f9571573ecf92bbe5aca710'){throw 'Earlier WSL packet changed'}
Push-Location $taskRepo
try{
    & git diff --exit-code 4a01412e6f81fe8b853f511ecf63cde95a732b75 -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
    if($LASTEXITCODE -ne 0){throw 'Original source or fixed qualification changed'}
}finally{Pop-Location}
[ordered]@{evidence_verified=(-not $UnsealedReview);inventory_checked=(-not $UnsealedReview);files=$taskActual.Count;metadata_mutations_refused=$taskNegative;server=$true;native_matrix_passed=$false;full_joined_sigkill=$false;postflight_passed=$true;product=$false}|ConvertTo-Json
