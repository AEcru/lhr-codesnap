#!/usr/bin/env bash
set -euo pipefail

# CodeSnap installer for macOS and Linux
# Primary: cargo install from GitHub source
# Fallback: pre-built binary download (when GitHub Releases are available)
# Usage: curl -fsSL https://raw.githubusercontent.com/AEcru/lhr-codesnap/main/install.sh | sh

REPO="AEcru/lhr-codesnap"
BIN_NAME="codesnap"
INSTALL_DIR="${HOME}/.local/bin"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { printf "${GREEN}[CodeSnap]${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW}[CodeSnap]${NC} %s\n" "$*"; }
error() { printf "${RED}[CodeSnap]${NC} %s\n" "$*"; exit 1; }

# Check prerequisites
check_rust() {
    if command -v cargo &> /dev/null; then
        return 0
    fi
    return 1
}

# Primary: install via cargo from GitHub source
install_via_cargo() {
    info "Installing via cargo from GitHub source..."
    cargo install --git "https://github.com/${REPO}.git" codesnap
}

# Fallback: clone and build manually
install_via_clone() {
    info "Cloning and building from source..."
    local tmpdir
    tmpdir=$(mktemp -d)
    git clone "https://github.com/${REPO}.git" "$tmpdir"
    cd "$tmpdir"
    cargo build --release
    mkdir -p "$INSTALL_DIR"
    cp "target/release/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"
    cd - > /dev/null
    rm -rf "$tmpdir"
    info "Installed to ${INSTALL_DIR}/${BIN_NAME}"
}

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

    if check_rust; then
        info "Rust toolchain detected. Installing from source..."
        if install_via_cargo; then
            info "Done! Run 'codesnap init' in any project to get started."
            return
        fi
        warn "cargo install --git failed, trying manual build..."
        install_via_clone
    else
        error "Rust is required. Install it first: https://rustup.rs"
    fi

    setup_path
    echo ""
    info "Installation complete! Run 'codesnap init' in any project to get started."
}

main
