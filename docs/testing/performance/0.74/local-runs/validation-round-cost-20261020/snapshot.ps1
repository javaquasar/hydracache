param([Parameter(Mandatory)][ValidateSet('before','after')][string]$Stage)
$ErrorActionPreference='Stop'
if(Test-Path "$PSScriptRoot/inputs-$Stage.json"){throw 'Never replace input observations'}
$taskAnchors=@(
    @('starting-effects-cost-20261020','69c7057e4994abea20e148a9179fe8229b6f6725ec7d1002e6d80484931a2d3c'),
    @('remaining-preparation-cost-20261020','2839ea78850f17f8070807b8853c0726133200311ccfd37f4f811b95ff796fbc'),
    @('custody-preparation-cost-20261020','3194244cb4e45c6f4f611b946afaa6a6a8b3aaf83179457c260d0a9e25f93864'),
    @('held-native-recheck-20261020','2a7be06fe6f93e3e8761b29afebaf89ac6266d7c405b457ef8ea26d60259bd63'),
    @('held-composition-recheck-20261020','0bfa7cef4d3ebd69c9e751fe6e4304a8bade3133122dcc48249c9d1396bfe1c0'),
    @('pid-path-cost-20261020','982d9598426830f6b3e44f9a701220b5778ceb3e6b568922d3b2ee405a0e6b05')
)
foreach($taskAnchor in $taskAnchors){
    $taskDirectory=Join-Path $PSScriptRoot "../$($taskAnchor[0])"
    if((Get-FileHash "$taskDirectory/inventory.json" -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskAnchor[1]){throw 'Prior seal changed'}
    foreach($taskFile in (Get-Content -Raw "$taskDirectory/inventory.json"|ConvertFrom-Json).files){
        if((Get-FileHash "$taskDirectory/$($taskFile.path)" -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskFile.sha256){throw 'Prior member changed'}
    }
}
$taskBundle='target/diagnostic-start-material-bundle-20261017'
$taskRoot=(Resolve-Path $taskBundle).Path
$taskFiles=@(Get-ChildItem $taskBundle -File|Sort-Object Name|ForEach-Object{[ordered]@{name=$_.Name;bytes=$_.Length;sha256=(Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}})
$taskOld=Get-Content -Raw "$PSScriptRoot/../held-preparation-cost-20261020/bundle-before.json"|ConvertFrom-Json
if(($taskFiles|ConvertTo-Json -Depth 5 -Compress) -cne ($taskOld.files|ConvertTo-Json -Depth 5 -Compress)){throw 'Original bundle changed'}
[ordered]@{stage=$Stage;bundle=$taskFiles;qualification_sha256=(Get-FileHash docs/testing/performance/0.74/qualification-manifest.toml -Algorithm SHA256).Hash.ToLowerInvariant();prior_inventories=$taskAnchors;server=$false;product=$false}|ConvertTo-Json -Depth 6|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/inputs-$Stage.json"
