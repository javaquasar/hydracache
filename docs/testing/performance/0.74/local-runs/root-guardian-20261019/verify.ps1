param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath($PSScriptRoot)
$taskManifest = Get-Content -LiteralPath (Join-Path $taskRoot 'inventory.json') -Raw | ConvertFrom-Json
function Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Check($Manifest) {
    if ($Manifest.schema_version -ne 1 -or $Manifest.kind -cne 'local-root-guardian-inventory' -or
        $Manifest.source_commit -cne '72f538063aa03d460843ed813a2c249e36223b24' -or
        $Manifest.normalization -cne 'none-raw-bytes') { throw 'Inventory identity refused' }
    foreach ($taskField in @('server','product','original_send','original_armed','creator_sigkill_tested','completion_allowed')) {
        if ($Manifest.$taskField -isnot [bool] -or $Manifest.$taskField) { throw "Authority claim refused: $taskField" }
    }
    $taskExpected = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($taskEntry in $Manifest.files) {
        $taskParts = $taskEntry.path -split '/'
        if ($taskEntry.path -notmatch '^[a-zA-Z0-9._/-]+$' -or [IO.Path]::IsPathRooted($taskEntry.path) -or
            @($taskParts | Where-Object { $_ -eq '.' -or $_ -eq '..' -or $_ -eq '' }).Count -ne 0 -or
            -not $taskExpected.Add($taskEntry.path) -or $taskEntry.path -ceq 'inventory.json') { throw 'Unsafe or duplicate path' }
        $taskPath = [IO.Path]::GetFullPath((Join-Path $taskRoot $taskEntry.path))
        if (-not $taskPath.StartsWith($taskRoot + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw 'Path escape' }
        $taskFile = Get-Item -LiteralPath $taskPath -Force
        if ($taskFile.PSIsContainer -or ($taskFile.Attributes -band [IO.FileAttributes]::ReparsePoint) -or
            $taskFile.Length -ne $taskEntry.bytes -or (Hash $taskPath) -cne $taskEntry.sha256) { throw "Raw bytes differ: $($taskEntry.path)" }
    }
    $taskActual = @(Get-ChildItem -LiteralPath $taskRoot -Recurse -Force | ForEach-Object {
        if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Reparse point refused' }
        if (-not $_.PSIsContainer -and $_.FullName -cne (Join-Path $taskRoot 'inventory.json')) {
            [IO.Path]::GetRelativePath($taskRoot,$_.FullName).Replace('\','/')
        }
    })
    if ($taskActual.Count -ne $taskExpected.Count -or @($taskActual | Where-Object { -not $taskExpected.Contains($_) }).Count) { throw 'Closed file set differs' }
    $taskRepo = (Resolve-Path (Join-Path $taskRoot '../../../../../..')).Path
    if ($Manifest.qualification_sha256 -cne '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc' -or
        (Hash (Join-Path $taskRepo 'docs/testing/performance/0.74/qualification-manifest.toml')) -cne $Manifest.qualification_sha256) { throw 'Frozen qualification differs' }
    foreach ($taskPair in @(@('gnu194-selected','1.94.0'),@('gnu188-confirmed','1.88.0'))) {
        $taskCohort = Join-Path $taskRoot $taskPair[0]
        $taskReceipt = Get-Content -LiteralPath (Join-Path $taskCohort 'receipt.json') -Raw | ConvertFrom-Json
        if ($taskReceipt.source_commit -cne $Manifest.source_commit -or $taskReceipt.compiler -cne $taskPair[1] -or
            $taskReceipt.image_before -cne $taskReceipt.image_after -or $taskReceipt.image_before -notmatch '^[0-9a-f]{64}  /' -or
            $taskReceipt.runner_sha256 -cne (Hash (Join-Path $taskRoot 'run.ps1')) -or
            $taskReceipt.build_uid -cne '1000' -or $taskReceipt.fixture_uid -ne 0 -or
            $taskReceipt.root_pair_cases_per_repeat -ne 16 -or $taskReceipt.transport_cases_per_repeat -ne 12 -or
            $taskReceipt.prior_kernel_cases_per_repeat -ne 11 -or $taskReceipt.repeats -ne 3 -or
            $taskReceipt.fixture_roots_before -ne 0 -or $taskReceipt.fixture_roots_after -ne 0 -or
            $taskReceipt.local_timesyncd_before -cne 'active' -or $taskReceipt.local_timesyncd_after -cne 'active') { throw 'Cohort identity/scope refused' }
        foreach ($taskField in @('server','product','original_send','original_armed','creator_sigkill_tested','completion_allowed')) {
            if ($taskReceipt.$taskField -isnot [bool] -or $taskReceipt.$taskField) { throw 'Cohort authority refused' }
        }
        foreach ($taskRepeat in 1..3) {
            $taskNew = Get-Content -LiteralPath (Join-Path $taskCohort "root-pair-$taskRepeat.log") -Raw
            $taskOld = Get-Content -LiteralPath (Join-Path $taskCohort "kernel-$taskRepeat.log") -Raw
            $taskTransport = Get-Content -LiteralPath (Join-Path $taskCohort "transport-$taskRepeat.log") -Raw
            if ([regex]::Matches($taskNew,'HC074-ROOT-GUARDIAN case=').Count -ne 16 -or
                [regex]::Matches($taskNew,'expected_negative=true').Count -ne 2 -or
                [regex]::Matches($taskNew,'actual_target_sigkill=true').Count -ne 10 -or
                [regex]::Matches($taskNew,'exact_children_reaped=true').Count -ne 4 -or
                -not $taskNew.Contains('completed=true cases=16 creator_sigkill_tested=false original_armed=false completion=false') -or
                -not $taskNew.Contains('test result: ok. 1 passed; 0 failed;') -or
                [regex]::Matches($taskOld,'HC074-CGROUP-ROOT cut=').Count -ne 4 -or
                -not $taskOld.Contains('completed=true cases=11 original_authority=false guardian_armed=false completion=false') -or
                -not $taskOld.Contains('test result: ok. 1 passed; 0 failed;') -or
                -not $taskTransport.Contains('cases=12 root_transport_matrix=pass original_admission=false completion=false') -or
                -not $taskTransport.Contains('test result: ok. 1 passed; 0 failed;')) { throw 'Root matrix incomplete' }
        }
        $taskUnits = Get-Content -LiteralPath (Join-Path $taskCohort 'new-controls.log') -Raw
        $taskGuardians = Get-Content -LiteralPath (Join-Path $taskCohort 'guardian-regression.log') -Raw
        if (-not $taskUnits.Contains('test result: ok. 18 passed; 0 failed; 3 ignored;') -or
            -not $taskGuardians.Contains('completed=true cases=17')) { throw 'Ordinary/regression controls incomplete' }
    }
    $taskSource = Get-Content -LiteralPath (Join-Path $taskRoot 'source-contracts.log') -Raw
    if (-not $taskSource.Contains('test result: ok. 283 passed; 0 failed;')) { throw 'Source gate incomplete' }
    $taskExpected.Count
}
$taskCount = Check $taskManifest
if ($SelfTest) {
    foreach ($taskMutation in @('hash','size','escape','duplicate','missing','source','product','qualification')) {
        $taskCopy = $taskManifest | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        switch ($taskMutation) {
            hash { $taskCopy.files[0].sha256 = '0' * 64 }
            size { $taskCopy.files[0].bytes++ }
            escape { $taskCopy.files[0].path = '../escape' }
            duplicate { $taskCopy.files = @($taskCopy.files) + @($taskCopy.files[0]) }
            missing { $taskCopy.files = @($taskCopy.files | Select-Object -Skip 1) }
            source { $taskCopy.source_commit = '0' * 40 }
            product { $taskCopy.product = $true }
            qualification { $taskCopy.qualification_sha256 = '0' * 64 }
        }
        $taskRejected = $false
        try { $null = Check $taskCopy } catch { $taskRejected = $true }
        if (-not $taskRejected) { throw "Negative control accepted: $taskMutation" }
    }
}
"raw guardian inventory verified: $taskCount files; selftest=$SelfTest; no original Armed or runtime authority"
