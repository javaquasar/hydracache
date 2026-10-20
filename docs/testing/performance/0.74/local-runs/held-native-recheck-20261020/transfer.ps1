$ErrorActionPreference='Stop'
Push-Location (Resolve-Path "$PSScriptRoot/../../../../../..")
try {
    if(Test-Path "$PSScriptRoot/transfer.json"){throw 'Never create duplicate staging'}
    $taskPreflight=Get-Content -Raw "$PSScriptRoot/preflight-exit.json"|ConvertFrom-Json
    if($taskPreflight.exit -ne 0){throw 'Preflight refused'}
    foreach($taskSource in '1ecaa895fdf84114f441f95f04c6411e9389096a','b5f1b8fce6b3046239087873cb85ff1bfc557f0f'){
        & git diff --quiet $taskSource HEAD -- tools/long-run-supervisor-074 crates/xtask Cargo.lock docs/COMPAT.md docs/testing/gated-test-registry.toml docs/testing/performance/0.74/qualification-manifest.toml
        if($LASTEXITCODE -ne 0){throw 'Selected source changed'}
    }
    & ./docs/testing/performance/0.74/local-runs/held-composition-recheck-20261020/verify.ps1
    $taskBinding=Get-Content -Raw docs/testing/performance/0.74/local-runs/held-composition-recheck-20261020/bindings.json|ConvertFrom-Json
    $taskLocal=Join-Path (Get-Location).Path ('target/native-recheck-transfer-'+[guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path "$taskLocal/bundle" -ErrorAction Stop|Out-Null
    foreach($taskImage in $taskBinding.images){
        $taskUnc='\\wsl.localhost\Ubuntu'+$taskImage.path.Replace('/','\')
        $taskDest="$taskLocal/join-$($taskImage.lane)"
        Copy-Item -LiteralPath $taskUnc -Destination $taskDest
        if((Get-FileHash $taskDest -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskImage.sha256){throw 'Image changed'}
    }
    foreach($taskFile in $taskBinding.bundle_before){
        if($taskFile.path -cnotmatch '^[A-Za-z0-9._-]+$'){throw 'Unexpected bundle name'}
        $taskDest="$taskLocal/bundle/$($taskFile.path)"
        Copy-Item -LiteralPath "target/diagnostic-start-material-bundle-20261017/$($taskFile.path)" -Destination $taskDest
        if((Get-FileHash $taskDest -Algorithm SHA256).Hash.ToLowerInvariant() -cne $taskFile.sha256){throw 'Bundle changed'}
    }
    Copy-Item -LiteralPath "$PSScriptRoot/run.sh","$PSScriptRoot/inputs.sha256" -Destination $taskLocal
    $taskUpload=(& ssh -o BatchMode=yes -o ConnectTimeout=10 hydracache-ax42 'umask 077; mktemp -d /tmp/hc074-native-recheck-upload-20261020.XXXXXX').Trim()
    if($LASTEXITCODE -ne 0 -or $taskUpload -cnotmatch '^/tmp/hc074-native-recheck-upload-20261020\.[A-Za-z0-9]{6}$'){throw 'Upload directory unknown; inspect, never repeat'}
    [ordered]@{source='1ecaa895fdf84114f441f95f04c6411e9389096a';image_source=$taskBinding.image_source;local_copy=$taskLocal;upload=$taskUpload;recipe_sha256=(Get-FileHash $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();server=$true;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/transfer.json"
    & ssh -o BatchMode=yes hydracache-ax42 "mkdir -m 700 $taskUpload/bundle"
    if($LASTEXITCODE -ne 0){throw 'Bundle staging refused'}
    & scp -p "$taskLocal/run.sh" "$taskLocal/inputs.sha256" "$taskLocal/join-194" "$taskLocal/join-188" "hydracache-ax42:$taskUpload/" 2>&1|Tee-Object "$PSScriptRoot/transfer-inputs.log"
    $taskImagesExit=$LASTEXITCODE
    & scp -p "$taskLocal/bundle/*" "hydracache-ax42:$taskUpload/bundle/" 2>&1|Tee-Object "$PSScriptRoot/transfer-bundle.log"
    $taskBundleExit=$LASTEXITCODE
    [ordered]@{inputs_exit=$taskImagesExit;bundle_exit=$taskBundleExit;server=$true;product=$false}|ConvertTo-Json|Set-Content -Encoding utf8NoBOM "$PSScriptRoot/transfer-exits.json"
    if($taskImagesExit -ne 0 -or $taskBundleExit -ne 0){throw 'Transfer refused; inspect existing upload'}
    Write-Output "UPLOAD=$taskUpload"
} finally {Pop-Location}
