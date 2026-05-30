#!/usr/bin/env bash
set -euo pipefail

# CodeSnap installer for macOS and Linux
# Primary: download pre-built binary from GitHub Releases (~30 seconds)
# Fallback: cargo install from source (~10-25 minutes)
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

# --- Detect OS and architecture, map to Rust target triple ---
detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"
    case "${os}-${arch}" in
        Darwin-x86_64)  echo "x86_64-apple-darwin" ;;
        Darwin-arm64)   echo "aarch64-apple-darwin" ;;
        Linux-x86_64)   echo "x86_64-unknown-linux-musl" ;;
        Linux-aarch64)  echo "aarch64-unknown-linux-musl" ;;
        Linux-arm64)    echo "aarch64-unknown-linux-musl" ;;
        *) error "Unsupported platform: ${os}-${arch}" ;;
    esac
}

# --- Get latest release tag from GitHub (no API, uses redirect header) ---
get_latest_tag() {
    local tag
    tag=$(curl -sI "https://github.com/${REPO}/releases/latest" 2>/dev/null \
        | grep -i "^location:" \
        | sed 's|.*/||' \
        | tr -d '\r\n')
    if [ -z "$tag" ]; then
        # Fallback: try GitHub API
        tag=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
            | grep '"tag_name"' \
            | head -1 \
            | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
    fi
    if [ -z "$tag" ]; then
        warn "Could not determine latest release. Check your internet connection."
        return 1
    fi
    echo "$tag"
}

# --- Primary: download pre-built binary from GitHub Releases ---
install_binary() {
    local target tag url archive_name tmpdir
    target="$(detect_target)" || return 1
    tag="$(get_latest_tag)" || return 1

    archive_name="codesnap-${tag}-${target}"
    url="https://github.com/${REPO}/releases/download/${tag}/${archive_name}.tar.gz"

    info "Downloading CodeSnap ${tag} for ${target}..."

    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    if ! curl -fsSL "$url" -o "${tmpdir}/${archive_name}.tar.gz"; then
        warn "Binary download failed."
        return 1
    fi

    # Verify SHA256 checksum
    local checksum_url
    checksum_url="https://github.com/${REPO}/releases/download/${tag}/${archive_name}.tar.gz.sha256"
    if curl -fsSL "$checksum_url" -o "${tmpdir}/${archive_name}.tar.gz.sha256" 2>/dev/null; then
        info "Verifying checksum..."
        (cd "$tmpdir" && sha256sum -c "${archive_name}.tar.gz.sha256" --quiet) || {
            warn "Checksum verification failed. The download may be corrupted."
            return 1
        }
    else
        warn "Checksum file not available, skipping verification."
    fi

    # Extract
    info "Extracting..."
    tar xzf "${tmpdir}/${archive_name}.tar.gz" -C "${tmpdir}" > /dev/null

    # Install binary
    mkdir -p "$INSTALL_DIR"
    cp "${tmpdir}/${archive_name}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
    chmod +x "${INSTALL_DIR}/${BIN_NAME}"

    # Remove quarantine attribute on macOS
    if [[ "$(uname -s)" == "Darwin" ]]; then
        xattr -d com.apple.quarantine "${INSTALL_DIR}/${BIN_NAME}" 2>/dev/null || true
    fi

    # Install skill files if running in a project directory
    if [ -d ".claude" ] || [ -d ".git" ]; then
        local skill_src="${tmpdir}/${archive_name}/skill"
        if [ -d "$skill_src" ]; then
            local skill_dest=".claude/skills/lhr-codesnap"
            mkdir -p "$skill_dest/references"
            cp "${skill_src}/SKILL.md" "$skill_dest/" 2>/dev/null || true
            cp "${skill_src}/references/"*.md "$skill_dest/references/" 2>/dev/null || true
            info "Skill files installed to ${skill_dest}"
        fi
    fi

    info "Installed to ${INSTALL_DIR}/${BIN_NAME}"
    return 0
}

# --- Fallback: install via cargo from GitHub source ---
install_via_cargo() {
    if ! command -v cargo &> /dev/null; then
        error "No pre-built binary available and Rust not found.
Install Rust: https://rustup.rs
Then run: cargo install --git https://github.com/${REPO}.git codesnap"
    fi
    warn "Falling back to cargo install (this may take 10-25 minutes)..."
    cargo install --git "https://github.com/${REPO}.git" codesnap
}

# --- Remind user to add INSTALL_DIR to PATH ---
setup_path() {
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        warn "Add ${INSTALL_DIR} to your PATH:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
        echo "Add this line to your shell profile (~/.bashrc, ~/.zshrc, etc.)"
    fi
}

# --- Main ---
main() {
    info "CodeSnap installer"
    echo ""

    if install_binary; then
        setup_path
        echo ""
        info "Done! Run 'codesnap skill' in your project to install skill files."
        info "Then run 'codesnap init' to build the index."
        return 0
    fi

    warn "Pre-built binary not available for your platform."
    install_via_cargo
    setup_path
    echo ""
    info "Installation complete!"
}

main
