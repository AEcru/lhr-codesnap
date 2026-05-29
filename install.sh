#!/usr/bin/env bash
set -euo pipefail

# CodeSnap installer for macOS and Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.sh | sh

REPO="AEcru/lhr-codesnap"
BIN_NAME="codesnap"
INSTALL_DIR="${HOME}/.local/bin"
VERSION="${CODESNAP_VERSION:-latest}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { printf "${GREEN}[CodeSnap]${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW}[CodeSnap]${NC} %s\n" "$*"; }
error() { printf "${RED}[CodeSnap]${NC} %s\n" "$*"; exit 1; }

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case "$ARCH" in
        x86_64|amd64)  ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        *) error "Unsupported architecture: $ARCH" ;;
    esac

    case "$OS" in
        linux)  PLATFORM="unknown-linux-musl" ;;
        darwin) PLATFORM="apple-darwin" ;;
        *) error "Unsupported OS: $OS" ;;
    esac

    TARGET="${ARCH}-${PLATFORM}"
    info "Detected platform: $TARGET"
}

# Check for required tools
check_deps() {
    if ! command -v curl &> /dev/null && ! command -v wget &> /dev/null; then
        error "curl or wget is required to download CodeSnap"
    fi
}

# Install from source via cargo
install_from_source() {
    info "Building from source with Cargo..."
    if ! command -v cargo &> /dev/null; then
        warn "Cargo not found. Install Rust first: https://rustup.rs"
        warn "Then run: cargo install codesnap"
        exit 1
    fi
    cargo install codesnap
    info "Installed codesnap via cargo"
}

# Download pre-built binary
install_binary() {
    local url="https://github.com/${REPO}/releases/download/v${VERSION}/${BIN_NAME}-${TARGET}.tar.gz"
    local tmpdir
    tmpdir=$(mktemp -d)
    local archive="${tmpdir}/codesnap.tar.gz"

    info "Downloading CodeSnap ${VERSION}..."
    if command -v curl &> /dev/null; then
        curl -fsSL "$url" -o "$archive" || {
            warn "Pre-built binary not available, falling back to source build"
            rm -rf "$tmpdir"
            install_from_source
            return
        }
    else
        wget -q "$url" -O "$archive" || {
            warn "Pre-built binary not available, falling back to source build"
            rm -rf "$tmpdir"
            install_from_source
            return
        }
    fi

    mkdir -p "$INSTALL_DIR"
    tar -xzf "$archive" -C "$tmpdir"
    cp "${tmpdir}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
    rm -rf "$tmpdir"

    info "Installed codesnap to ${INSTALL_DIR}/${BIN_NAME}"
}

# Add to PATH if needed
setup_path() {
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        warn "Add ${INSTALL_DIR} to your PATH:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
        echo "Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
    fi
}

main() {
    info "CodeSnap installer"
    echo ""

    detect_platform
    check_deps

    # If cargo is available, prefer source build (always latest)
    if command -v cargo &> /dev/null; then
        install_from_source
    else
        install_binary
    fi

    setup_path

    echo ""
    info "Installation complete! Run 'codesnap init' in any project to get started."
}

main
