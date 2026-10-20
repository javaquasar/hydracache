param([switch]$Review)
$ErrorActionPreference='Stop'
. "$PSScriptRoot/facts.ps1"
$taskRepo=(Resolve-Path "$PSScriptRoot/../../../../../..").Path
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function ReadJson([string]$Name){Get-Content -Raw "$PSScriptRoot/$Name"|ConvertFrom-Json}
function Json($Value){ConvertTo-Json -Depth 30 -Compress -InputObject $Value}
function CheckSummary($Summary,$Expected){Require ((Json $Summary) -ceq (Json $Expected)) 'Summary does not match original logs'}
function Inventory([string]$Directory,[string]$Expected){
    if($Expected){Require ((Hash "$Directory/inventory.json") -ceq $Expected) 'Prior seal changed'}
    Require (@(Get-ChildItem $Directory -Recurse -Force|Where-Object{($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0}).Count -eq 0) 'No reparse members'
    $taskInventory=Get-Content -Raw "$Directory/inventory.json"|ConvertFrom-Json
    Require ($taskInventory.schema -ceq 'sha256-local-file-inventory-v1') 'Inventory schema'
    $taskActual=[string[]]@(Get-ChildItem $Directory -File -Recurse -Force|Where-Object{$_.FullName -cne "$Directory\inventory.json"}|ForEach-Object{$_.FullName.Substring($Directory.Length+1).Replace('\','/')})
    [Array]::Sort($taskActual,[StringComparer]::Ordinal)
    $taskPaths=[string[]]@($taskInventory.files.path)
    Require ((Json $taskActual) -ceq (Json $taskPaths) -and @($taskPaths|Select-Object -Unique).Count -eq $taskPaths.Count) 'Missing extra duplicate or unsorted member'
    foreach($taskEntry in $taskInventory.files){
        Require ($taskEntry.path -cmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -and $taskEntry.path -cnotmatch '(^|/)\.\.(/|$)' -and $taskEntry.path -cne 'inventory.json') 'Unsafe member'
        $taskPath=Join-Path $Directory $taskEntry.path
        Require ((Get-Item -LiteralPath $taskPath).Length -eq $taskEntry.bytes -and (Hash $taskPath) -ceq $taskEntry.sha256) "Changed member: $($taskEntry.path)"
    }
    $taskPaths.Count
}
Push-Location $taskRepo
try {
    foreach($taskPrior in @(
        @('held-consuming-privileged-server-20261020','4d592e07dcc6a8dc461827801b724bea1ca803d22d8a61f50915988e28fec079',51),
        @('pid-path-cost-20261020','982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05',116),
        @('held-composition-recheck-20261020','0bfa7cef4d3ebd69c9e751fe6e4304a8bade3133122dcc48249c9d1396bfe1c0',25)
    )){Require ((Inventory (Join-Path $taskRepo "docs/testing/performance/0.74/local-runs/$($taskPrior[0])") $taskPrior[1]) -eq $taskPrior[2]) 'Prior packet count'}
    $taskSelected=@('tools/long-run-supervisor-074','crates/xtask','crates/hydracache','Cargo.lock','docs/COMPAT.md','docs/testing/gated-test-registry.toml','docs/testing/performance/0.74/qualification-manifest.toml')
    foreach($taskSource in '1ecaa895fdf84114f441f95f04c6411e9389096a','b5f1b8fce6b3046239087873cb85ff1bfc557f0f'){
        & git diff --quiet $taskSource HEAD -- @taskSelected
        Require ($LASTEXITCODE -eq 0) 'Source changed'
    }
    Require (-not (& git status --porcelain -- @taskSelected)) 'Dirty selected source'
    Require ((Hash 'docs/testing/performance/0.74/qualification-manifest.toml') -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'Qualification changed'
    $taskOld=Get-Content -Raw 'docs/testing/performance/0.74/local-runs/held-consuming-privileged-server-20261020/run.sh'
    Require ((Get-Content -Raw "$PSScriptRoot/run.sh") -ceq $taskOld.Replace('/tmp/hc074-held-root-20261020.??????','/tmp/hc074-native-recheck-root-20261020.??????')) 'Old runner predicates changed'
    Require ((Hash "$PSScriptRoot/run.sh") -ceq '5d52bde41aedce9ac1a7d5b9943d32fd3145bee8f789bfb20419cf17c8a10534' -and (Hash "$PSScriptRoot/inputs.sha256") -ceq '9435b7c519e0368b459aee9cb765c11a6ca2c0373c1ac27ec5a8e937026c9da0') 'Frozen runner/manifest drift'
    $taskManifest=Get-Content -Raw "$PSScriptRoot/inputs.sha256"
    foreach($taskLane in '194','188'){
        $taskImage=Get-Content -Raw "docs/testing/performance/0.74/local-runs/pid-path-cost-20261020/Candidate-$taskLane-plain-image.json"|ConvertFrom-Json
        Require ($taskImage.source -ceq 'b5f1b8fce6b3046239087873cb85ff1bfc557f0f' -and $taskImage.flavor -ceq 'plain' -and $taskManifest.Contains("$($taskImage.sha256)  join-$taskLane")) 'Prior image differs'
    }
    foreach($taskPhase in 'preflight','stage','collect','postflight'){
        $taskReceipt=ReadJson "$taskPhase-exit.json";$taskStart=ReadJson "$taskPhase-start.json"
        $taskScript=if($taskPhase -ceq 'postflight'){'preflight'}else{$taskPhase}
        Require ($taskReceipt.phase -ceq $taskPhase -and $taskReceipt.exit -eq 0 -and $taskReceipt.server -and -not $taskReceipt.product) 'SSH phase refused'
        Require ($taskStart.phase -ceq $taskPhase -and $taskStart.server -and -not $taskStart.product -and $taskStart.script_sha256 -ceq (Hash "$PSScriptRoot/$taskScript.sh") -and $taskStart.recipe_sha256 -ceq (Hash "$PSScriptRoot/invoke.ps1")) 'SSH recipe binding'
        Require ((Get-Item "$PSScriptRoot/$taskPhase-stderr.log").Length -eq 0) 'Unexpected SSH stderr'
    }
    $taskTransfer=ReadJson 'transfer.json';$taskExits=ReadJson 'transfer-exits.json'
    Require ($taskTransfer.source -ceq '1ecaa895fdf84114f441f95f04c6411e9389096a' -and $taskTransfer.image_source -ceq 'b5f1b8fce6b3046239087873cb85ff1bfc557f0f' -and $taskTransfer.recipe_sha256 -ceq (Hash "$PSScriptRoot/transfer.ps1") -and $taskTransfer.upload -ceq '/tmp/hc074-native-recheck-upload-20261020.OiTtzO' -and $taskTransfer.server -and -not $taskTransfer.product) 'Transfer binding'
    Require ($taskExits.inputs_exit -eq 0 -and $taskExits.bundle_exit -eq 0 -and $taskExits.server -and -not $taskExits.product) 'SCP refused'
    $taskStage=Get-Content -Raw "$PSScriptRoot/stage-stdout.log"
    Require ($taskStage.Contains('ROOT=/tmp/hc074-native-recheck-root-20261020.hBFgSO') -and ([regex]::Matches($taskStage,'WRAPPER_PID=266408')).Count -eq 1) 'Protected attempt identity'
    foreach($taskFile in @('run.sh','inputs.sha256')+@(Get-Content "$PSScriptRoot/inputs.sha256"|ForEach-Object{($_ -split '  ',2)[1]})){
        Require ($taskStage.Contains("${taskFile}: OK")) 'Protected input not checked'
    }
    $taskNames=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $taskExport=Get-Content -Raw "$PSScriptRoot/collect-stdout.log"
    foreach($taskLine in Get-Content "$PSScriptRoot/remote/remote-files.sha256"){
        Require ($taskLine -cmatch '^([0-9a-f]{64})  ([A-Za-z0-9.-]+)$') 'Remote manifest syntax'
        $taskHash=$Matches[1];$taskName=$Matches[2]
        Require ($taskNames.Add($taskName) -and (Hash "$PSScriptRoot/remote/$taskName") -ceq $taskHash -and $taskExport.Contains($taskLine)) 'Raw transfer mismatch'
    }
    Require ($taskNames.Count -eq 17 -and @(Get-ChildItem "$PSScriptRoot/remote" -File).Count -eq 18 -and $taskExport.Contains('EXPORT=/tmp/hc074-native-recheck-export-20261020.nI8uQm')) 'Unexpected completed export'
    foreach($taskName in 'preflight.log','postflight.log','194-preflight.log','194-postflight.log','188-preflight.log','188-postflight.log'){
        $taskRaw=Get-Content -Raw "$PSScriptRoot/remote/$taskName"
        foreach($taskMarker in 'HC074-HELD-ROOT preflight=ok installed_unchanged=true','MainPID=8839','NRestarts=0','ActiveState=active','SubState=running'){Require ($taskRaw.Contains($taskMarker)) 'Host gate absent'}
        Require (([regex]::Matches($taskRaw,'(?m)^ABSENT /var/lib/hydracache-performance/')).Count -eq 9) 'Campaign gate missing'
    }
    foreach($taskPhase in 'preflight','postflight'){
        $taskRaw=Get-Content -Raw "$PSScriptRoot/$taskPhase-stdout.log"
        foreach($taskMarker in 'e81671d6-bf23-4ebf-b8fe-5dad1f45866f','MainPID=8839','NRestarts=0','ActiveState=active','SubState=running','HC074-NATIVE-RECHECK preflight=ok server=true product=false'){Require ($taskRaw.Contains($taskMarker)) 'Independent host check absent'}
        Require (([regex]::Matches($taskRaw,'(?m)^ABSENT /var/lib/hydracache-performance/')).Count -eq 9 -and ([regex]::Matches($taskRaw,': OK')).Count -eq 2) 'Independent marker/artifact check missing'
    }
    Require ((Hash "$PSScriptRoot/remote/mountinfo-before.log") -ceq (Hash "$PSScriptRoot/remote/mountinfo-after.log")) 'Host mounts changed'
    Require ((Get-Content -Raw "$PSScriptRoot/remote/wrapper.exit").Trim() -ceq '1' -and (Get-Content -Raw "$PSScriptRoot/remote/postflight.exit").Trim() -ceq '0') 'Wrapper/postflight changed'
    $taskFacts=NativeFacts
    if($Review){Require (-not (Test-Path "$PSScriptRoot/inventory.json")) 'Cannot bypass seal';$taskFacts|ConvertTo-Json -Depth 30;exit}
    $taskCount=Inventory $PSScriptRoot ''
    CheckSummary (ReadJson 'summary.json') $taskFacts
    foreach($taskCheck in 'format','doc-check','registry','contract-check'){
        $taskReceipt=ReadJson "$taskCheck-exit.json"
        Require ($taskReceipt.exit -eq 0 -and -not $taskReceipt.server -and -not $taskReceipt.product -and $taskReceipt.recipe_sha256 -ceq (Hash "$PSScriptRoot/checks.ps1")) 'Local check refused'
    }
    $taskSyntax=ReadJson 'syntax.json'
    Require ($taskSyntax.powershell_scripts -eq 6 -and $taskSyntax.powershell_syntax_passed -and $taskSyntax.shell_syntax_passed -and (Json @($taskSyntax.shell_scripts)) -ceq (Json @('run','stage','collect','preflight')) -and -not $taskSyntax.server -and -not $taskSyntax.product) 'Recipe syntax gate missing'
    $taskMutations=0
    foreach($taskKey in $taskFacts.Keys){
        foreach($taskMode in 'missing','wrong'){
            $taskBad=Json $taskFacts|ConvertFrom-Json
            if($taskMode -ceq 'missing'){$taskBad.PSObject.Properties.Remove($taskKey)}elseif($taskBad.$taskKey -is [bool]){$taskBad.$taskKey=-not $taskBad.$taskKey}else{$taskBad.$taskKey='changed'}
            $taskRefused=$false
            try{CheckSummary $taskBad $taskFacts}catch{if($_.Exception.Message -cne 'Summary does not match original logs'){throw};$taskRefused=$true}
            Require $taskRefused 'Summary mutation accepted';$taskMutations++
        }
    }
    [ordered]@{evidence_verified=$true;inventory_checked=$true;files=$taskCount;summary_mutations_refused=$taskMutations;native_matrix_passed=$false;postflight_passed=$true;server=$true;product=$false}|ConvertTo-Json
} finally {Pop-Location}
