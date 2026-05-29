# CodeSnap installer for Windows
# Primary: cargo install from GitHub source
# Usage: irm https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.ps1 | iex

$ErrorActionPreference = "Stop"
$Repo = "AEcru/lhr-codesnap"
$BinName = "codesnap"
$InstallDir = "$env:USERPROFILE\.codesnap\bin"

function Write-Info  { Write-Host "[CodeSnap] $args" -ForegroundColor Green }
function Write-Warn  { Write-Host "[CodeSnap] $args" -ForegroundColor Yellow }
function Write-Error { Write-Host "[CodeSnap] $args" -ForegroundColor Red; exit 1 }

function Add-ToPath {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$InstallDir*") {
        Write-Warn "Adding $InstallDir to your PATH..."
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Info "Added to PATH. Restart your terminal for changes to take effect."
    }
}

function Install-ViaCargo {
    Write-Info "Installing via cargo from GitHub source..."
    cargo install --git "https://github.com/$Repo.git" codesnap
    if ($LASTEXITCODE -eq 0) {
        Write-Info "Done! Run 'codesnap init' in any project to get started."
        return
    }
    throw "cargo install failed"
}

function Install-ViaClone {
    Write-Info "Cloning and building from source..."
    $tmpDir = Join-Path $env:TEMP "codesnap_build"
    git clone "https://github.com/$Repo.git" $tmpDir
    Push-Location $tmpDir
    cargo build --release
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Path "target\release\$BinName.exe" -Destination "$InstallDir\$BinName.exe" -Force
    Pop-Location
    Remove-Item -Recurse -Force $tmpDir
    Write-Info "Installed to $InstallDir\$BinName.exe"
}

# Main
Write-Info "CodeSnap installer"
Write-Host ""

$cargo = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargo) {
    Write-Error "Rust is required. Install it first: https://rustup.rs"
}

Write-Info "Rust toolchain detected. Installing from source..."
try {
    Install-ViaCargo
} catch {
    Write-Warn "cargo install --git failed, trying manual build..."
    Install-ViaClone
}

Add-ToPath
Write-Host ""
Write-Info "Installation complete! Run 'codesnap init' in any project to get started."
