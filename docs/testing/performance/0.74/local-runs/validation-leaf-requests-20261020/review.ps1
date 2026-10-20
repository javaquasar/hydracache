param([ValidateSet('WriteSummary','Seal','Review')][string]$Mode='Review')
$ErrorActionPreference='Stop'
$taskRepo=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
function Hash([string]$Path){(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()}
function Require([bool]$Condition,[string]$Message){if(-not $Condition){throw $Message}}
function Read-Json([string]$Name){Get-Content -LiteralPath (Join-Path $PSScriptRoot $Name) -Raw|ConvertFrom-Json}
function Facts{
    $taskCases=@('boot','namespace-user','namespace-mnt','namespace-pid','namespace-time','origin','authority-1','authority-7')
    $taskLanes=@()
    foreach($taskLane in @('194','188')){
        $taskReceipt=Read-Json "profile-$taskLane-exit.json"
        Require ($taskReceipt.exit -eq 0 -and $taskReceipt.image_sha256 -ceq $taskReceipt.image_after_sha256) 'Failed profile or changed image'
        Require (-not $taskReceipt.server -and -not $taskReceipt.product -and -not $taskReceipt.native_gate_open -and -not $taskReceipt.runtime_candidate) 'Scope differs'
        foreach($taskInput in $taskReceipt.inputs){Require ((Hash (Join-Path $taskRepo $taskInput.path)) -ceq $taskInput.sha256) "Input changed: $($taskInput.path)"}
        Require ($taskReceipt.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'profile.ps1'))) 'Profile recipe drift'
        $taskAlloc=@();$taskIo=@();$taskOrigins=@()
        foreach($taskLine in Get-Content (Join-Path $PSScriptRoot "profile-$taskLane.log")){
            if($taskLine.StartsWith('HC074-VALIDATION-LEAF-ALLOC ')){$taskAlloc+=($taskLine.Substring(28)|ConvertFrom-Json)}
            if($taskLine.StartsWith('HC074-VALIDATION-LEAF-IO ')){$taskIo+=($taskLine.Substring(25)|ConvertFrom-Json)}
            if($taskLine.StartsWith('HC074-VALIDATION-LEAF-ORIGIN ')){$taskOrigins+=($taskLine.Substring(29)|ConvertFrom-Json)}
        }
        Require ($taskAlloc.Count -eq 24 -and $taskIo.Count -eq 24 -and $taskOrigins.Count -eq 1) 'Incomplete cohort'
        $taskOrigin=$taskOrigins[0]
        Require ($taskOrigin.uid -gt 0 -and $taskOrigin.samples -eq 3 -and $taskOrigin.iterations -eq 1024 -and $taskOrigin.warmups -eq 32) 'Origin differs'
        Require ((Get-Content (Join-Path $PSScriptRoot "profile-$taskLane.log") -Raw).Contains('1 passed; 0 failed')) 'Native test did not complete'
        $taskRows=@()
        foreach($taskCase in $taskCases){
            $taskA=@($taskAlloc|Where-Object case -CEQ $taskCase|Sort-Object sample)
            $taskI=@($taskIo|Where-Object case -CEQ $taskCase|Sort-Object sample)
            Require ($taskA.Count -eq 3 -and $taskI.Count -eq 3) 'Case incomplete'
            $taskNet=@()
            for($taskSample=0;$taskSample -lt 3;$taskSample++){
                $taskOne=$taskA[$taskSample];$taskTwo=$taskI[$taskSample]
                Require ($taskOne.sample -eq $taskSample -and $taskTwo.sample -eq $taskSample -and $taskOne.iterations -eq 1024 -and $taskTwo.iterations -eq 1024) 'Sample identity differs'
                Require ($taskOne.rust_requests_only -and -not $taskTwo.allocator_active -and -not $taskTwo.all_syscalls_observed -and -not $taskOne.authority_fixed_ancestry -and -not $taskTwo.authority_fixed_ancestry -and -not $taskOne.server -and -not $taskTwo.server -and -not $taskOne.product -and -not $taskTwo.product) 'Measurement scope differs'
                Require ($taskTwo.empty.syscr -eq 1 -and $taskTwo.empty.syscw -eq 0 -and $taskTwo.observed.syscw -eq 0 -and $taskTwo.observed.wchar -eq 0) 'Unexpected I/O writes or reader overhead'
                $taskNet+=($taskTwo.observed.syscr-$taskTwo.empty.syscr)
            }
            $taskRows+=[ordered]@{case=$taskCase;iterations=1024;allocations=@($taskA|ForEach-Object {$_.counts.allocations});reallocations=@($taskA|ForEach-Object {$_.counts.reallocations});requested_bytes=@($taskA|ForEach-Object {$_.counts.requested_bytes});read_requests_minus_empty=$taskNet;write_requests=@($taskI|ForEach-Object {$_.observed.syscw})}
        }
        $taskLanes+=[ordered]@{lane=$taskLane;source=$taskReceipt.source;image_sha256=$taskReceipt.image_sha256;origin=$taskOrigin;rows=$taskRows}
    }
    Require ($taskLanes[0].source -ceq $taskLanes[1].source) 'Different sources'
    foreach($taskLane in @('194','188','xtask','contracts','repository')){
        $taskCheck=Read-Json "check-$taskLane-exit.json"
        Require (-not $taskCheck.failure -and @($taskCheck.steps|Where-Object exit -ne 0).Count -eq 0) 'Checks failed'
        Require ($taskCheck.recipe_sha256 -ceq (Hash (Join-Path $PSScriptRoot 'check.ps1'))) 'Check recipe drift'
        foreach($taskStep in $taskCheck.steps){Require ((Hash (Join-Path $PSScriptRoot $taskStep.log)) -ceq $taskStep.sha256) 'Check log drift'}
    }
    [ordered]@{schema='validation-leaf-request-control-074-v1';complete_request_cohort=$true;runtime_candidate=$false;latency_gain_claimed=$false;all_syscalls_observed=$false;full_active_authority_measured=$false;existing_live_refusals_retained=$true;native_gate_open=$false;server=$false;product=$false;lanes=$taskLanes}
}
function Verify-Inventory([string]$Directory,[string]$ExpectedHash){
    $taskInventory=Join-Path $Directory 'inventory.json'
    if($ExpectedHash){Require ((Hash $taskInventory) -ceq $ExpectedHash) 'Prior inventory changed'}
    $taskParsed=Get-Content -LiteralPath $taskInventory -Raw|ConvertFrom-Json
    $taskNames=@($taskParsed.files|ForEach-Object path|Sort-Object)
    $taskActual=@(Get-ChildItem -LiteralPath $Directory -File -Recurse|Where-Object Name -ne 'inventory.json'|ForEach-Object {$_.FullName.Substring($Directory.Length+1).Replace('\','/')}|Sort-Object)
    Require (($taskNames -join "`n") -ceq ($taskActual -join "`n")) 'Inventory members differ'
    foreach($taskMember in $taskParsed.files){
        $taskFull=[IO.Path]::GetFullPath((Join-Path $Directory $taskMember.path))
        Require ($taskFull.StartsWith($Directory+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) 'Member escapes packet'
        Require ((Get-Item -LiteralPath $taskFull).Length -eq $taskMember.bytes -and (Hash $taskFull) -ceq $taskMember.sha256) "Member changed: $($taskMember.path)"
    }
    $taskParsed.files.Count
}
$taskFacts=Facts
$taskSummary=Join-Path $PSScriptRoot 'summary.json'
if($Mode -ceq 'WriteSummary'){
    Require (-not (Test-Path $taskSummary)) 'Never replace summary'
    $taskFacts|ConvertTo-Json -Depth 12|Set-Content -Encoding utf8NoBOM $taskSummary
    exit
}
$taskStored=Read-Json 'summary.json'
Require (($taskStored|ConvertTo-Json -Depth 12 -Compress) -ceq ($taskFacts|ConvertTo-Json -Depth 12 -Compress)) 'Summary differs from raw observations'
$taskMutations=0
foreach($taskKey in @('complete_request_cohort','runtime_candidate','latency_gain_claimed','all_syscalls_observed','full_active_authority_measured','existing_live_refusals_retained','native_gate_open','server','product')){
    $taskChanged=($taskStored|ConvertTo-Json -Depth 12)|ConvertFrom-Json
    $taskChanged.$taskKey=-not $taskChanged.$taskKey
    Require (($taskChanged|ConvertTo-Json -Depth 12 -Compress) -cne ($taskFacts|ConvertTo-Json -Depth 12 -Compress)) 'Mutation accepted'
    $taskMutations++
}
$taskPrior=@(
    @('validation-round-cost-20261020','b089573e0dc544df769c07b64be41562f107cdc1add64b1cd0c9512df2d882d6'),
    @('starting-effects-cost-20261020','69c7057e4994abea20e148a9179fe8229b6f6725ec7d1002e6d80484931a2d3c'),
    @('remaining-preparation-cost-20261020','2839ea78850f17f8070807b8853c0726133200311ccfd37f4f811b95ff796fbc'),
    @('custody-preparation-cost-20261020','3194244cb4e45c6f4f611b946afaa6a6a8b3aaf83179457c260d0a9e25f93864'),
    @('held-native-recheck-20261020','2a7be06fe6f93e3e8761b29afebaf89ac6266d7c405b457ef8ea26d60259bd63'),
    @('held-composition-recheck-20261020','0bfa7cef4d3ebd69c9e751fe6e4304a8bade3133122dcc48249c9d1396bfe1c0'),
    @('pid-path-cost-20261020','982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05')
)
foreach($taskOld in $taskPrior){$null=Verify-Inventory (Join-Path (Split-Path $PSScriptRoot) $taskOld[0]) $taskOld[1]}
if($Mode -ceq 'Seal'){
    $taskInventory=Join-Path $PSScriptRoot 'inventory.json'
    Require (-not (Test-Path $taskInventory)) 'Never reseal'
    [ordered]@{schema='sha256-local-file-inventory-v1';files=@(Get-ChildItem -LiteralPath $PSScriptRoot -File -Recurse|Sort-Object FullName|ForEach-Object {[ordered]@{path=$_.FullName.Substring($PSScriptRoot.Length+1).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}})}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM $taskInventory
}
$taskMembers=Verify-Inventory $PSScriptRoot ''
[ordered]@{integrity_verified=$true;members=$taskMembers;prior_packets=7;summary_mutations_rejected=$taskMutations;complete_request_cohort=$true;all_syscalls_observed=$false;full_active_authority_measured=$false;native_gate_open=$false;server=$false;product=$false}|ConvertTo-Json
