# CodeSnap installer for Windows
# Primary: download pre-built binary from GitHub Releases (~30 seconds)
# Fallback: cargo install from source (~10-25 minutes)
# Usage: irm https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
$Repo = "AEcru/lhr-codesnap"
$BinName = "codesnap"
$InstallDir = "$env:USERPROFILE\.codesnap\bin"

function Write-Info  { Write-Host "[CodeSnap] $args" -ForegroundColor Green }
function Write-Warn  { Write-Host "[CodeSnap] $args" -ForegroundColor Yellow }
function Write-Err   { Write-Host "[CodeSnap] $args" -ForegroundColor Red; exit 1 }

function Add-ToPath {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        Write-Warn "Adding $InstallDir to your PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Info "Added to PATH. Restart your terminal for changes to take effect."
    }
}

function Get-LatestTag {
    # Try redirect header first (no API rate limit)
    try {
        $response = Invoke-WebRequest -Uri "https://github.com/$Repo/releases/latest" `
            -Method Head -MaximumRedirection 0 -ErrorAction SilentlyContinue
        if ($response.Headers.Location -match '([^/]+)$') {
            return $Matches[1]
        }
    } catch {}
    # Fallback: GitHub API
    try {
        $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
        $release = Invoke-RestMethod -Uri $apiUrl
        return $release.tag_name
    } catch {
        Write-Warn "Could not determine latest release. Check your internet connection."
        return $null
    }
}

function Install-Binary {
    $target = "x86_64-pc-windows-msvc"
    $tag = Get-LatestTag
    if (-not $tag) { return $false }

    $archiveName = "codesnap-$tag-$target"
    $url = "https://github.com/$Repo/releases/download/$tag/$archiveName.zip"

    Write-Info "Downloading CodeSnap $tag for $target..."

    $tmpDir = Join-Path $env:TEMP "codesnap_install"
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

    try {
        Invoke-WebRequest -Uri $url -OutFile "$tmpDir\$archiveName.zip"
    } catch {
        Write-Warn "Binary download failed."
        return $false
    }

    # Verify SHA256
    $checksumUrl = "https://github.com/$Repo/releases/download/$tag/$archiveName.zip.sha256"
    try {
        $checksumFile = "$tmpDir\$archiveName.zip.sha256"
        Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumFile
        $expectedHash = (Get-Content $checksumFile -Raw).Split()[0]
        $actualHash = (Get-FileHash "$tmpDir\$archiveName.zip" -Algorithm SHA256).Hash.ToLower()
        if ($expectedHash.ToLower() -ne $actualHash) {
            Write-Warn "Checksum mismatch. Download may be corrupted."
            return $false
        }
        Write-Info "Checksum verified."
    } catch {
        Write-Warn "Checksum file not available, skipping verification."
    }

    # Extract
    Write-Info "Extracting..."
    Expand-Archive -Path "$tmpDir\$archiveName.zip" -DestinationPath "$tmpDir\extracted" -Force

    # Install binary
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Path "$tmpDir\extracted\$archiveName\$BinName.exe" -Destination "$InstallDir\$BinName.exe" -Force

    # Install skill files if running in a project directory
    $skillSrc = "$tmpDir\extracted\$archiveName\skill"
    if ((Test-Path ".claude") -or (Test-Path ".git")) {
        if (Test-Path $skillSrc) {
            $skillDest = ".claude\skills\lhr-codesnap"
            New-Item -ItemType Directory -Force -Path "$skillDest\references" | Out-Null
            Copy-Item -Path "$skillSrc\SKILL.md" -Destination "$skillDest\" -ErrorAction SilentlyContinue
            Copy-Item -Path "$skillSrc\references\*.md" -Destination "$skillDest\references\" -ErrorAction SilentlyContinue
            Write-Info "Skill files installed to $skillDest"
        }
    }

    # Cleanup
    Remove-Item -Recurse -Force $tmpDir
    Write-Info "Installed to $InstallDir\$BinName.exe"
    return $true
}

function Install-ViaCargo {
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        Write-Err "No pre-built binary available and Rust not found.`nInstall Rust: https://rustup.rs`nThen run: cargo install --git https://github.com/$Repo.git codesnap"
    }
    Write-Warn "Falling back to cargo install (this may take 10-25 minutes)..."
    cargo install --git "https://github.com/$Repo.git" codesnap
}

# Main
Write-Info "CodeSnap installer"
Write-Host ""

if (Install-Binary) {
    Add-ToPath
    Write-Host ""
    Write-Info "Done! Run 'codesnap skill' in your project to install skill files."
    Write-Info "Then run 'codesnap init' to build the index."
} else {
    Write-Warn "Pre-built binary not available."
    Install-ViaCargo
    Add-ToPath
    Write-Host ""
    Write-Info "Installation complete!"
}
