[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$manifest = Get-Content -LiteralPath (Join-Path $root "manifest.json") -Raw | ConvertFrom-Json

if ($manifest.release -ne "0.71" -or $manifest.result -ne "success") {
    throw "unexpected top-level manifest identity"
}

$expected = @{}
foreach ($line in Get-Content -LiteralPath (Join-Path $root "SHA256SUMS")) {
    if ($line -notmatch '^([0-9a-f]{64})  (.+)$') {
        throw "malformed SHA256SUMS line: $line"
    }
    $expected[$Matches[2]] = $Matches[1]
}

$evidenceFiles = Get-ChildItem -LiteralPath $root -Recurse -File | Where-Object {
    $_.Name -notin @("README.md", "manifest.json", "SHA256SUMS", "verify.ps1")
}
if ($evidenceFiles.Count -ne $expected.Count) {
    throw "evidence file count mismatch: found $($evidenceFiles.Count), expected $($expected.Count)"
}

foreach ($file in $evidenceFiles) {
    $relative = [IO.Path]::GetRelativePath($root, $file.FullName).Replace('\', '/')
    if (-not $expected.ContainsKey($relative)) {
        throw "unexpected evidence file: $relative"
    }
    $actual = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected[$relative]) {
        throw "SHA-256 mismatch: $relative"
    }
}

$totalJobs = 0
foreach ($campaign in $manifest.campaigns) {
    $campaignRoot = Join-Path $root $campaign.directory
    if (-not (Test-Path -LiteralPath $campaignRoot -PathType Container)) {
        throw "missing campaign directory: $($campaign.directory)"
    }

    $campaignArchiveName = if ($campaign.campaign_archive.published_file) {
        $campaign.campaign_archive.published_file
    } else {
        "$($campaign.campaign_id).campaign.tar.gz"
    }
    $campaignArchive = Join-Path $campaignRoot "raw/$campaignArchiveName"
    $mirrorArchive = Join-Path $campaignRoot "raw/$($campaign.campaign_id).mirror.tar.gz"
    foreach ($pair in @(
        @($campaignArchive, $campaign.campaign_archive),
        @($mirrorArchive, $campaign.mirror_archive)
    )) {
        $path = $pair[0]
        $record = $pair[1]
        if ((Get-Item -LiteralPath $path).Length -ne $record.bytes) {
            throw "archive size mismatch: $path"
        }
        $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($hash -ne $record.sha256) {
            throw "archive manifest hash mismatch: $path"
        }
        $members = & tar -tzf $path
        if ($LASTEXITCODE -ne 0) {
            throw "unreadable tar.gz archive: $path"
        }
        if ($members -match '/hc2-pki/(?:client|server)\.key$') {
            throw "published archive contains generated mTLS private keys: $path"
        }
    }

    if ($campaign.campaign_archive.redaction_receipt) {
        $redactionPath = Join-Path $campaignRoot "raw/$($campaign.campaign_archive.redaction_receipt)"
        $redaction = Get-Content -LiteralPath $redactionPath -Raw | ConvertFrom-Json
        if ($redaction.campaign_id -ne $campaign.campaign_id -or
            $redaction.original_archive.bytes -ne $campaign.campaign_archive.original_bytes -or
            $redaction.original_archive.sha256 -ne $campaign.campaign_archive.original_sha256 -or
            $redaction.published_archive.bytes -ne $campaign.campaign_archive.bytes -or
            $redaction.published_archive.sha256 -ne $campaign.campaign_archive.sha256 -or
            $redaction.removed_members.Count -ne $campaign.campaign_archive.removed_generated_private_keys) {
            throw "redaction receipt mismatch: $($campaign.campaign_id)"
        }
    }

    $artifactRoot = Join-Path $campaignRoot "github"
    $identityFile = Get-ChildItem -LiteralPath $artifactRoot -Recurse -File -Filter "campaign-identity.json" -ErrorAction Stop
    $receiptFile = Get-ChildItem -LiteralPath $artifactRoot -Recurse -File -Filter "campaign-receipt.json" -ErrorAction Stop
    if ($identityFile.Count -ne 1 -or $receiptFile.Count -ne 1) {
        throw "expected one identity and one receipt for $($campaign.campaign_id)"
    }
    $identity = Get-Content -LiteralPath $identityFile.FullName -Raw | ConvertFrom-Json
    $receipt = Get-Content -LiteralPath $receiptFile.FullName -Raw | ConvertFrom-Json
    if ($identity.campaign_id -ne $campaign.campaign_id -or
        $identity.source_sha -ne $manifest.source_sha -or
        $identity.workflow_sha -ne $manifest.workflow_sha) {
        throw "campaign identity mismatch: $($campaign.campaign_id)"
    }
    if ($receipt.result -ne "success" -or
        $receipt.campaign_id -ne $campaign.campaign_id -or
        $receipt.source_sha -ne $manifest.source_sha -or
        $receipt.workflow_sha -ne $manifest.workflow_sha -or
        $receipt.job_count -ne $campaign.job_count -or
        $receipt.completed_jobs -ne $campaign.job_count) {
        throw "campaign receipt mismatch: $($campaign.campaign_id)"
    }

    foreach ($json in Get-ChildItem -LiteralPath $artifactRoot -Recurse -File -Filter "*.json") {
        Get-Content -LiteralPath $json.FullName -Raw | ConvertFrom-Json > $null
    }
    foreach ($jsonl in Get-ChildItem -LiteralPath $artifactRoot -Recurse -File -Filter "*.jsonl") {
        $lineNumber = 0
        foreach ($line in Get-Content -LiteralPath $jsonl.FullName) {
            $lineNumber++
            if ($line.Trim().Length -gt 0) {
                try { $line | ConvertFrom-Json > $null }
                catch { throw "malformed JSONL: $($jsonl.FullName):$lineNumber" }
            }
        }
    }
    $totalJobs += $campaign.job_count
}

if ($totalJobs -ne $manifest.job_count -or $manifest.completed_jobs -ne $manifest.job_count) {
    throw "top-level job total mismatch"
}

Write-Host "M0-M7 archive verification passed: $($manifest.campaigns.Count) campaigns, $totalJobs jobs"
