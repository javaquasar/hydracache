param([switch]$SelfTest,[switch]$Preflight)
$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath($PSScriptRoot)
function Hash([string]$Path) { (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
if ($Preflight) {
    if (Test-Path -LiteralPath (Join-Path $taskRoot 'inventory.json')) { throw 'Closed packet: use normal verification' }
    $taskFiles = @(Get-ChildItem -LiteralPath $taskRoot -Recurse -Force -File | Sort-Object FullName | ForEach-Object {
        [pscustomobject]@{path=[IO.Path]::GetRelativePath($taskRoot,$_.FullName).Replace('\','/');bytes=$_.Length;sha256=(Hash $_.FullName)}
    })
    $taskManifest = [pscustomobject]@{schema_version=1;kind='local-root-creator-frame-inventory';source_commit='f48fd4bf1de242d4130cd61459a3daed8437f33e';normalization='none-raw-bytes';qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc';creator_sigkill_tested=$true;server=$false;product=$false;original_send=$false;original_armed=$false;end_to_end_original=$false;completion_allowed=$false;files=$taskFiles}
} else {
    $taskManifest = Get-Content -LiteralPath (Join-Path $taskRoot 'inventory.json') -Raw | ConvertFrom-Json
}
function Log([string]$Path) { Get-Content -LiteralPath (Join-Path $taskRoot $Path) -Raw }
function Check($Manifest) {
    if ($Manifest.schema_version -ne 1 -or $Manifest.kind -cne 'local-root-creator-frame-inventory' -or
        $Manifest.source_commit -cne 'f48fd4bf1de242d4130cd61459a3daed8437f33e' -or
        $Manifest.normalization -cne 'none-raw-bytes' -or $Manifest.creator_sigkill_tested -isnot [bool] -or
        -not $Manifest.creator_sigkill_tested) { throw 'Inventory identity refused' }
    foreach ($taskField in @('server','product','original_send','original_armed','end_to_end_original','completion_allowed')) {
        if ($Manifest.$taskField -isnot [bool] -or $Manifest.$taskField) { throw "Authority refused: $taskField" }
    }
    $taskExpected = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    foreach ($taskEntry in $Manifest.files) {
        $taskParts = $taskEntry.path -split '/'
        if ($taskEntry.path -notmatch '^[a-zA-Z0-9._/-]+$' -or [IO.Path]::IsPathRooted($taskEntry.path) -or
            @($taskParts | Where-Object { $_ -eq '.' -or $_ -eq '..' -or $_ -eq '' }).Count -ne 0 -or
            -not $taskExpected.Add($taskEntry.path) -or $taskEntry.path -ceq 'inventory.json') { throw 'Unsafe/duplicate path' }
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
        (Hash (Join-Path $taskRepo 'docs/testing/performance/0.74/qualification-manifest.toml')) -cne $Manifest.qualification_sha256) { throw 'Frozen qualification drift' }
    foreach ($taskPair in @(@('gnu194-confirmed','1.94.0'),@('gnu188-confirmed','1.88.0'))) {
        $taskCohort = $taskPair[0]
        $taskBuild = Log "$taskCohort/build.json" | ConvertFrom-Json
        $taskFrame = Log "$taskCohort/receipt.json" | ConvertFrom-Json
        if ($taskBuild.source_commit -cne $Manifest.source_commit -or $taskFrame.source_commit -cne $Manifest.source_commit -or
            $taskBuild.compiler -cne $taskPair[1] -or $taskFrame.compiler -cne $taskPair[1] -or
            $taskBuild.image_before -cne $taskBuild.image_after -or $taskBuild.image_after -cne $taskFrame.image -or
            $taskBuild.image_after -notmatch '^[0-9a-f]{64}  /' -or
            $taskBuild.runner_sha256 -cne (Hash (Join-Path $taskRoot 'run.ps1')) -or $taskFrame.runner_sha256 -cne $taskBuild.runner_sha256 -or
            $taskBuild.build_uid -cne '1000' -or $taskBuild.fixture_uid -ne 0 -or
            $taskBuild.creator_cases -ne 12 -or $taskBuild.join_kernel_cases -ne 4 -or $taskBuild.root_pair_cases -ne 16 -or
            $taskBuild.transport_cases -ne 12 -or $taskBuild.prior_kernel_cases -ne 11 -or $taskBuild.repeats -ne 3 -or
            $taskBuild.fixture_roots_before -ne 0 -or $taskBuild.fixture_roots_after -ne 0 -or
            $taskBuild.creator_sigkill_tested -isnot [bool] -or -not $taskBuild.creator_sigkill_tested -or
            $taskFrame.frame_cases -ne 8 -or $taskFrame.frame_repeats -ne 3 -or $taskFrame.prior_pending_cases -ne 10 -or
            $taskFrame.clock_service_before -cne 'active' -or $taskFrame.clock_service_after -cne 'active' -or
            $taskFrame.kernel_simulated -isnot [bool] -or -not $taskFrame.kernel_simulated) { throw 'Cohort identity/scope refused' }
        foreach ($taskRecord in @($taskBuild,$taskFrame)) {
            foreach ($taskField in @('server','product','original_send','completion_allowed')) {
                if ($taskRecord.$taskField -isnot [bool] -or $taskRecord.$taskField) { throw 'Cohort authority refused' }
            }
        }
        if ($taskFrame.end_to_end_original -isnot [bool] -or $taskFrame.end_to_end_original) { throw 'False end-to-end claim' }
        foreach ($taskRepeat in 1..3) {
            $taskCreator = Log "$taskCohort/creator-$taskRepeat.log"
            if ([regex]::Matches($taskCreator,'HC074-ROOT-CREATOR case=').Count -ne 12 -or
                -not $taskCreator.Contains('HC074-ROOT-CREATOR image_sha256=' + ($taskBuild.image_after -split '  ')[0]) -or
                [regex]::Matches($taskCreator,'creator_sigkill=true').Count -ne 12 -or
                [regex]::Matches($taskCreator,'expected_negative=True').Count -ne 2 -or
                -not $taskCreator.Contains('completed=true cases=12 server=false product=false')) { throw 'Creator matrix incomplete' }
            $taskKernel = Log "$taskCohort/join-kernel-$taskRepeat.log"
            $taskOriginal = Log "$taskCohort/original-frame-$taskRepeat.log"
            if ([regex]::Matches($taskKernel,'HC074-ROOT-JOIN-KERNEL case=').Count -ne 4 -or
                -not $taskKernel.Contains('test result: ok. 1 passed; 0 failed;') -or
                [regex]::Matches($taskOriginal,'HC074-ORIGINAL-ROOT-FRAME case=').Count -ne 8 -or
                -not $taskOriginal.Contains('test result: ok. 1 passed; 0 failed;')) { throw 'Join controls incomplete' }
            if (-not (Log "$taskCohort/root-pair-$taskRepeat.log").Contains('completed=true cases=16 creator_sigkill_tested=false original_armed=false completion=false') -or
                -not (Log "$taskCohort/transport-$taskRepeat.log").Contains('cases=12 root_transport_matrix=pass original_admission=false completion=false') -or
                -not (Log "$taskCohort/prior-kernel-$taskRepeat.log").Contains('completed=true cases=11 original_authority=false guardian_armed=false completion=false')) { throw 'Prior root regressions incomplete' }
        }
        foreach ($taskGate in @(@('root-units',18),@('join-units',3),@('two-phase',40),@('exchange',30),@('prior-pending-frame',1))) {
            if (-not (Log "$taskCohort/$($taskGate[0]).log").Contains("test result: ok. $($taskGate[1]) passed; 0 failed;")) { throw 'Wrong/zero libtest selection' }
        }
        if (-not (Log "$taskCohort/old-guardians.log").Contains('completed=true cases=17') -or
            -not (Log "$taskCohort/clock-probe.log").Contains('clock_probe_complete=true backward_samples=0') -or
            -not (Log "$taskCohort/clock-settle.log").Contains('settled=true stable_seconds=65') -or
            (Log "$taskCohort/bundle-before.log") -cne (Log "$taskCohort/bundle-after.log")) { throw 'Regression/input/environment check missing' }
    }
    if (-not (Log 'source-suite-final.log').Contains('test result: ok. 289 passed; 0 failed;') -or
        -not (Log 'windows-unit.log').Contains('test result: ok. 54 passed; 0 failed;') -or
        -not (Log 'windows-doc.log').Contains('test result: ok. 18 passed; 0 failed;') -or
        -not (Log 'carrier-controls.log').Contains('completed=true cases=9')) { throw 'Local/source controls incomplete' }
    $taskExpected.Count
}
$taskCount = Check $taskManifest
if ($SelfTest) {
    foreach ($taskMutation in @('hash','size','escape','duplicate','missing','source','product','qualification','creator')) {
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
            creator { $taskCopy.creator_sigkill_tested = $false }
        }
        $taskRejected = $false
        try { $null = Check $taskCopy } catch { $taskRejected = $true }
        if (-not $taskRejected) { throw "Negative accepted: $taskMutation" }
    }
}
"raw creator/frame inventory verified: $taskCount files; selftest=$SelfTest; original execution remains closed"
