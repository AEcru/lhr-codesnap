# CodeSnap installer for Windows
# Usage: irm https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.ps1 | iex

param(
    [string]$Version = $env:CODESNAP_VERSION
)

$ErrorActionPreference = "Stop"
$Repo = "AEcru/lhr-codesnap"
$BinName = "codesnap"
$InstallDir = "$env:USERPROFILE\.codesnap\bin"

function Write-Info  { Write-Host "[CodeSnap] $args" -ForegroundColor Green }
function Write-Warn  { Write-Host "[CodeSnap] $args" -ForegroundColor Yellow }
function Write-Error { Write-Host "[CodeSnap] $args" -ForegroundColor Red; exit 1 }

# Detect architecture
function Get-Platform {
    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        "AMD64" { "x86_64" }
        "ARM64" { "aarch64" }
        default { Write-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
    }
    return "$arch-pc-windows-msvc"
}

# Install from source via cargo
function Install-FromSource {
    Write-Info "Building from source with Cargo..."
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargo) {
        Write-Warn "Cargo not found. Install Rust first: https://rustup.rs"
        Write-Warn "Then run: cargo install codesnap"
        exit 1
    }
    cargo install codesnap
    Write-Info "Installed codesnap via cargo"
}

# Download pre-built binary
function Install-Binary {
    $platform = Get-Platform
    $archiveUrl = "https://github.com/$Repo/releases/download/v$Version/$BinName-$platform.zip"

    $tmpDir = Join-Path $env:TEMP "codesnap_install"
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
    $zipPath = Join-Path $tmpDir "codesnap.zip"

    Write-Info "Downloading CodeSnap..."
    try {
        Invoke-WebRequest -Uri $archiveUrl -OutFile $zipPath -ErrorAction Stop
    } catch {
        Write-Warn "Pre-built binary not available, falling back to source build"
        Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
        Install-FromSource
        return
    }

    Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Path "$tmpDir\$BinName.exe" -Destination "$InstallDir\$BinName.exe" -Force
    Remove-Item -Recurse -Force $tmpDir

    Write-Info "Installed codesnap to $InstallDir\$BinName.exe"
}

# Add to PATH
function Add-ToPath {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        Write-Warn "Adding $InstallDir to your PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Info "Added to PATH. Restart your terminal for changes to take effect."
    }
}

# Main
Write-Info "CodeSnap installer"
Write-Host ""

# If cargo is available, prefer source build
$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($cargo) {
    Install-FromSource
} else {
    Install-Binary
}

Add-ToPath

Write-Host ""
Write-Info "Installation complete! Run 'codesnap init' in any project to get started."
