param(
    [string]$Release = $(if ($env:DCODE_RELEASE) { $env:DCODE_RELEASE } else { "latest" }),
    [string]$InstallDir = $(if ($env:DCODE_INSTALL_DIR) { $env:DCODE_INSTALL_DIR } else { Join-Path $HOME ".local\bin" })
)

$ErrorActionPreference = "Stop"
$repository = if ($env:DCODE_GITHUB_REPOSITORY) { $env:DCODE_GITHUB_REPOSITORY } else { "dopejs/dcode" }
$downloadBase = if ($env:DCODE_RELEASE_BASE_URL) { $env:DCODE_RELEASE_BASE_URL.TrimEnd('/') } else { "https://github.com/$repository/releases/download" }
$codexHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HOME ".codex" }
$releasesDir = Join-Path $codexHome "packages\dcode-standalone\releases"
$tempRoot = [System.IO.Path]::GetTempPath()
$tempDir = Join-Path $tempRoot ("dcode-install-" + [guid]::NewGuid().ToString("N"))
$stageDir = $null

function Normalize-Version([string]$Value) {
    if ($Value.StartsWith("dcode-v")) { return $Value.Substring(7) }
    if ($Value.StartsWith("v")) { return $Value.Substring(1) }
    return $Value
}

function Resolve-Version {
    if ($Release -ne "latest") {
        return Normalize-Version $Release
    }
    if ($env:DCODE_RELEASE_BASE_URL) {
        throw "DCODE_RELEASE must be explicit when DCODE_RELEASE_BASE_URL is overridden"
    }
    $metadata = Invoke-RestMethod -Uri "https://api.github.com/repos/$repository/releases/latest"
    if (-not $metadata.tag_name.StartsWith("dcode-v")) {
        throw "Latest GitHub release is not tagged dcode-v*: $($metadata.tag_name)"
    }
    return Normalize-Version $metadata.tag_name
}

function Assert-ReleasePackage([string]$PackageDir, [string]$ExpectedVersion) {
    $packageBinary = Join-Path $PackageDir "bin\dcode.exe"
    if (-not (Test-Path -LiteralPath $packageBinary -PathType Leaf)) { throw "Package does not contain bin\dcode.exe" }
    if (-not (Test-Path -LiteralPath (Join-Path $PackageDir "bin\codex-code-mode-host.exe") -PathType Leaf)) { throw "Package is missing code-mode host" }
    if (-not (Test-Path -LiteralPath (Join-Path $PackageDir "codex-path\rg.exe") -PathType Leaf)) { throw "Package is missing ripgrep" }
    $binaryVersion = & $packageBinary --version
    if ($binaryVersion -notmatch [regex]::Escape($ExpectedVersion)) { throw "Downloaded binary version does not match $ExpectedVersion" }
}

try {
    $version = Resolve-Version
    if ($version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$') {
        throw "Invalid dcode release version: $version"
    }
    if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
        throw "The GitHub installer currently supports Windows x86_64 only"
    }
    if (-not (Get-Command tar.exe -ErrorAction SilentlyContinue)) {
        throw "tar.exe is required"
    }

    $target = "x86_64-pc-windows-msvc"
    $tag = "dcode-v$version"
    $asset = "dcode-package-$target.tar.gz"
    $releaseUrl = "$downloadBase/$tag"
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    $archive = Join-Path $tempDir $asset
    $checksums = Join-Path $tempDir "dcode_SHA256SUMS"
    Invoke-WebRequest -Uri "$releaseUrl/$asset" -OutFile $archive
    Invoke-WebRequest -Uri "$releaseUrl/dcode_SHA256SUMS" -OutFile $checksums

    $checksumLine = Get-Content -LiteralPath $checksums | Where-Object { $_ -match "^[0-9a-fA-F]{64}\s+$([regex]::Escape($asset))$" } | Select-Object -First 1
    if (-not $checksumLine) { throw "Checksum manifest does not contain $asset" }
    $expected = ($checksumLine -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw "SHA-256 mismatch for $asset" }

    New-Item -ItemType Directory -Force -Path $releasesDir, $InstallDir | Out-Null
    $releaseDir = Join-Path $releasesDir "$version-$target"
    if (-not (Test-Path -LiteralPath $releaseDir)) {
        $stageDir = Join-Path $releasesDir (".staging.$version-$target." + [guid]::NewGuid().ToString("N"))
        New-Item -ItemType Directory -Path $stageDir | Out-Null
        tar.exe -xzf $archive -C $stageDir
        if ($LASTEXITCODE -ne 0) { throw "Failed to extract $asset" }
        Assert-ReleasePackage $stageDir $version
        Move-Item -LiteralPath $stageDir -Destination $releaseDir
        $stageDir = $null
    }
    else {
        Assert-ReleasePackage $releaseDir $version
    }

    $binary = Join-Path $releaseDir "bin\dcode.exe"
    $launcher = Join-Path $InstallDir "dcode.cmd"
    if (Test-Path -LiteralPath $launcher) {
        $existing = Get-Content -Raw -LiteralPath $launcher
        if (-not $existing.StartsWith("@rem managed-by-dcode-installer")) {
            throw "Refusing to overwrite unmanaged launcher: $launcher"
        }
    }
    $launcherBody = "@rem managed-by-dcode-installer`r`n@echo off`r`n`"$binary`" %*`r`n"
    Set-Content -LiteralPath $launcher -Value $launcherBody -Encoding Ascii -NoNewline
    & $binary --version | Out-Null
    Write-Host "Installed dcode $version to $launcher"
    if (($env:PATH -split ';') -notcontains $InstallDir) {
        Write-Host "Add $InstallDir to PATH to run dcode."
    }
}
finally {
    if ($stageDir -and (Test-Path -LiteralPath $stageDir)) {
        $resolvedStage = [System.IO.Path]::GetFullPath($stageDir)
        $resolvedReleases = [System.IO.Path]::GetFullPath($releasesDir) + [System.IO.Path]::DirectorySeparatorChar
        if ($resolvedStage.StartsWith($resolvedReleases) -and (Split-Path $resolvedStage -Leaf).StartsWith(".staging.")) {
            Remove-Item -LiteralPath $resolvedStage -Recurse -Force
        }
    }
    if (Test-Path -LiteralPath $tempDir) {
        $resolvedTemp = [System.IO.Path]::GetFullPath($tempDir)
        $resolvedRoot = [System.IO.Path]::GetFullPath($tempRoot)
        if ($resolvedTemp.StartsWith($resolvedRoot) -and (Split-Path $resolvedTemp -Leaf).StartsWith("dcode-install-")) {
            Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
        }
    }
}
