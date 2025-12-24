#!/bin/sh
# odd-dashboard installation script for Linux/macOS
# Downloads the appropriate binary from GitHub Releases
# Verifies checksum before installation

set -eu

# Configuration
REPO="oddessentials/odd-demonstration"
INSTALL_DIR="${ODD_DASHBOARD_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${ODD_DASHBOARD_VERSION:-latest}"

# Colors (if terminal supports it)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

# Detect platform
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    
    case "$OS" in
        Linux*)
            case "$ARCH" in
                x86_64) PLATFORM="linux-x64" ;;
                aarch64|arm64) PLATFORM="linux-arm64" ;;
                *) echo "${RED}Unsupported architecture: $ARCH${NC}"; exit 1 ;;
            esac
            ;;
        Darwin*)
            case "$ARCH" in
                x86_64) PLATFORM="macos-x64" ;;
                arm64) PLATFORM="macos-arm64" ;;
                *) echo "${RED}Unsupported architecture: $ARCH${NC}"; exit 1 ;;
            esac
            ;;
        *)
            echo "${RED}Unsupported OS: $OS${NC}"
            echo "For Windows, use install.ps1 instead."
            exit 1
            ;;
    esac
    
    ARTIFACT="odd-dashboard-${PLATFORM}"
    echo "Detected platform: ${PLATFORM}"
}

# Resolve "latest" to actual version
resolve_version() {
    if [ "$VERSION" = "latest" ]; then
        echo "Fetching latest version..."
        VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | 
                  grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
        if [ -z "$VERSION" ]; then
            echo "${RED}Failed to determine latest version${NC}"
            exit 1
        fi
    fi
    echo "Version: v${VERSION}"
    
    BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
}

# Download and verify
download_and_verify() {
    TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "$TEMP_DIR"' EXIT
    
    echo "Downloading ${ARTIFACT}..."
    curl -fsSL -o "${TEMP_DIR}/${ARTIFACT}" "${BASE_URL}/${ARTIFACT}"
    
    echo "Downloading checksums..."
    curl -fsSL -o "${TEMP_DIR}/SHA256SUMS" "${BASE_URL}/SHA256SUMS"
    
    echo "Verifying checksum..."
    cd "$TEMP_DIR"
    
    # Extract expected checksum using awk (anchored match)
    EXPECTED=$(awk -v artifact="$ARTIFACT" '$2 == artifact {print $1}' SHA256SUMS)
    
    if [ -z "$EXPECTED" ]; then
        echo "${RED}Artifact '$ARTIFACT' not found in SHA256SUMS${NC}"
        exit 1
    fi
    
    # Calculate actual checksum (works on both GNU and BSD)
    if command -v sha256sum >/dev/null 2>&1; then
        ACTUAL=$(sha256sum "$ARTIFACT" | awk '{print $1}')
    elif command -v shasum >/dev/null 2>&1; then
        ACTUAL=$(shasum -a 256 "$ARTIFACT" | awk '{print $1}')
    else
        echo "${RED}No sha256sum or shasum found${NC}"
        exit 1
    fi
    
    # Compare (case-insensitive)
    if [ "$(echo "$EXPECTED" | tr '[:upper:]' '[:lower:]')" != "$(echo "$ACTUAL" | tr '[:upper:]' '[:lower:]')" ]; then
        echo "${RED}Checksum mismatch!${NC}"
        echo "  Expected: $EXPECTED"
        echo "  Actual:   $ACTUAL"
        exit 1
    fi
    
    echo "${GREEN}Checksum verified${NC}"
    
    # Install
    echo "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    mv "$ARTIFACT" "${INSTALL_DIR}/odd-dashboard"
    chmod +x "${INSTALL_DIR}/odd-dashboard"
}

# Verify installation
verify_install() {
    if [ -x "${INSTALL_DIR}/odd-dashboard" ]; then
        echo ""
        echo "${GREEN}Successfully installed odd-dashboard!${NC}"
        echo ""
        "${INSTALL_DIR}/odd-dashboard" --version
        echo ""
        
        # Check if install dir is in PATH
        case ":$PATH:" in
            *":${INSTALL_DIR}:"*) ;;
            *)
                echo "${YELLOW}Note: ${INSTALL_DIR} is not in your PATH${NC}"
                echo "Add this to your shell profile:"
                echo ""
                echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
                echo ""
                ;;
        esac
        
        echo "Run 'odd-dashboard doctor' to check prerequisites."
    else
        echo "${RED}Installation failed${NC}"
        exit 1
    fi
}

# Main
main() {
    echo "odd-dashboard installer"
    echo "======================="
    echo ""
    
    detect_platform
    resolve_version
    download_and_verify
    verify_install
}

main "$@"
