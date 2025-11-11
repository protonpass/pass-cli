#!/bin/bash
set -e

# Proton Pass CLI Installation Script
# Usage: curl -fsSL https://proton.me/download/pass-cli/install.sh | bash
# Or with custom install dir: PROTON_PASS_CLI_INSTALL_DIR=/custom/path bash install.sh
# Or with custom channel: PROTON_PASS_CLI_INSTALL_CHANNEL=beta bash install.sh

MANIFEST_BASE_URL="https://proton.me/download/pass-cli/"
BINARY_NAME="pass-cli"

# Get manifest URL based on channel
get_manifest_url() {
    channel="${PROTON_PASS_CLI_INSTALL_CHANNEL:-}"
    channel=$(echo "$channel" | tr -d ' ')
    
    if [ -z "$channel" ] || [ "$channel" = "stable" ]; then
        echo "${MANIFEST_BASE_URL}versions.json"
    else
        echo "${MANIFEST_BASE_URL}versions.${channel}.json"
    fi
}

MANIFEST_URL=$(get_manifest_url)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1" >&2
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Detect OS
detect_os() {
    os_name=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$os_name" in
        linux*)
            echo "linux"
            ;;
        darwin*)
            echo "macos"
            ;;
        *)
            log_error "Unsupported OS: $os_name"
            exit 1
            ;;
    esac
}

# Detect architecture
detect_arch() {
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            log_error "Unsupported architecture: $arch"
            exit 1
            ;;
    esac
}

# Check if required commands exist
check_dependencies() {
    missing_deps=()
    
    if ! command -v curl &> /dev/null; then
        missing_deps+=("curl")
    fi
    
    if ! command -v jq &> /dev/null; then
        missing_deps+=("jq")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_error "Missing required dependencies: ${missing_deps[*]}"
        echo ""
        echo "Please install the missing dependencies:"
        echo ""
        
        for dep in "${missing_deps[@]}"; do
            case "$dep" in
                curl)
                    echo "  Ubuntu/Debian: sudo apt-get install curl"
                    echo "  RHEL/Fedora:   sudo dnf install curl"
                    echo "  macOS:         brew install curl (or use built-in curl)"
                    ;;
                jq)
                    echo "  Ubuntu/Debian: sudo apt-get install jq"
                    echo "  RHEL/Fedora:   sudo dnf install jq"
                    echo "  macOS:         brew install jq"
                    ;;
            esac
            echo ""
        done
        
        exit 1
    fi
}

# Fetch and parse manifest
fetch_binary_info() {
    os=$1
    arch=$2

    log_info "Fetching manifest from $MANIFEST_URL..."
    
    manifest=$(curl -fsSL "$MANIFEST_URL") || {
        log_error "Failed to download manifest from $MANIFEST_URL"
        exit 1
    }
    
    # Parse format version
    format_version=$(echo "$manifest" | jq -r '.formatVersion')
    
    if [ "$format_version" != "1" ]; then
        log_error "Unsupported manifest format version: $format_version"
        log_error "Please upgrade your installation script or install manually."
        exit 1
    fi
    
    # Extract version
    version=$(echo "$manifest" | jq -r '.passCliVersions.version')
    
    if [ -z "$version" ] || [ "$version" = "null" ]; then
        log_error "Could not determine latest version from manifest"
        exit 1
    fi
    
    log_info "Latest version: $version"
    
    # Extract URL and hash for platform
    url=$(echo "$manifest" | jq -r ".passCliVersions.urls.\"$os\".\"$arch\".url")
    hash=$(echo "$manifest" | jq -r ".passCliVersions.urls.\"$os\".\"$arch\".hash")

    if [ -z "$url" ] || [ "$url" = "null" ] || [ -z "$hash" ] || [ "$hash" = "null" ]; then
        log_error "No binary available for platform: $os/$arch"
        exit 1
    fi
    
    echo "$version|$url|$hash"
}

# Download and verify binary
download_binary() {
    url=$1
    expected_hash=$2
    temp_file=$3
    
    log_info "Downloading binary from $url..."
    
    if ! curl -fsSL -o "$temp_file" "$url"; then
        log_error "Failed to download binary from $url"
        exit 1
    fi
    
    log_info "Verifying SHA256 hash..."
    
    local computed_hash
    if command -v sha256sum &> /dev/null; then
        # Linux
        computed_hash=$(sha256sum "$temp_file" | awk '{print $1}')
    elif command -v shasum &> /dev/null; then
        # macOS
        computed_hash=$(shasum -a 256 "$temp_file" | awk '{print $1}')
    else
        log_error "No SHA256 verification tool found (sha256sum or shasum)"
        exit 1
    fi
    
    if [ "$computed_hash" != "$expected_hash" ]; then
        log_error "Hash verification failed!"
        log_error "Expected: $expected_hash"
        log_error "Got:      $computed_hash"
        rm -f "$temp_file"
        exit 1
    fi
    
    log_info "Hash verification successful"
    
    # Make executable
    chmod +x "$temp_file"
}

# Detect Linux package manager
detect_package_manager() {
    if command -v apt-get &> /dev/null; then
        echo "apt"
    elif command -v dnf &> /dev/null; then
        echo "dnf"
    elif command -v yum &> /dev/null; then
        echo "yum"
    else
        echo "unknown"
    fi
}

# Check Linux dependencies
check_linux_dependencies() {
    pkg_manager=$(detect_package_manager)
    missing_deps=()
    
    case "$pkg_manager" in
        apt)
            # Check for libdbus-1-3
            if ! dpkg -s libdbus-1-3 &> /dev/null; then
                missing_deps+=("libdbus-1-3")
            fi
            ;;
        dnf|yum)
            # Check for dbus-libs
            if ! rpm -q dbus-libs &> /dev/null; then
                missing_deps+=("dbus-libs")
            fi
            ;;
        unknown)
            log_warn "Unknown package manager. Skipping dependency check."
            log_warn "Please ensure libdbus is installed for proper operation."
            return
            ;;
    esac
    
    if [ ${#missing_deps[@]} -eq 0 ]; then
        log_info "All required dependencies are installed"
        return
    fi
    
    log_warn "Missing runtime dependencies: ${missing_deps[*]}"
    echo ""
    read -p "Do you want to install missing dependencies? [Y/n] " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Nn]$ ]]; then
        log_warn "Skipping dependency installation. The CLI may not work correctly."
        return
    fi
    
    log_info "Installing dependencies..."
    
    case "$pkg_manager" in
        apt)
            sudo apt-get update
            sudo apt-get install -y "${missing_deps[@]}"
            ;;
        dnf)
            sudo dnf install -y "${missing_deps[@]}"
            ;;
        yum)
            sudo yum install -y "${missing_deps[@]}"
            ;;
    esac
    
    log_info "Dependencies installed successfully"
}

# Determine install directory
get_install_dir() {
    if [ -n "$PROTON_PASS_CLI_INSTALL_DIR" ]; then
        echo "$PROTON_PASS_CLI_INSTALL_DIR"
        return
    fi
    
    # Try user directory first
    local user_bin="$HOME/.local/bin"
    if [ -d "$user_bin" ] || mkdir -p "$user_bin" 2>/dev/null; then
        echo "$user_bin"
        return
    fi
    
    # Fall back to system directory
    echo "/usr/local/bin"
}

# Install binary
install_binary() {
    temp_file=$1
    install_dir=$(get_install_dir)
    target_path="$install_dir/$BINARY_NAME"
    needs_sudo=false
    
    log_info "Installing to $target_path..."
    
    # Create install directory if needed
    if [ ! -d "$install_dir" ]; then
        if ! mkdir -p "$install_dir" 2>/dev/null; then
            needs_sudo=true
        fi
    fi
    
    # Check if we need sudo for installation
    if [ ! -w "$install_dir" ]; then
        needs_sudo=true
    fi
    
    # Copy binary
    if [ "$needs_sudo" = true ]; then
        log_info "Installing to system directory requires sudo..."
        if ! sudo cp "$temp_file" "$target_path"; then
            log_error "Failed to install binary to $target_path"
            exit 1
        fi
        if ! sudo chmod 755 "$target_path"; then
            log_error "Failed to set permissions on $target_path"
            exit 1
        fi
    else
        if ! cp "$temp_file" "$target_path"; then
            log_error "Failed to install binary to $target_path"
            exit 1
        fi
    fi
    
    rm -f "$temp_file"

    log_info "Installation complete!"
    echo ""

    # Set release track if custom channel was used during installation
    channel="${PROTON_PASS_CLI_INSTALL_CHANNEL:-}"
    channel=$(echo "$channel" | tr -d ' ')

    if [ -n "$channel" ] && [ "$channel" != "stable" ]; then
        log_info "Setting release track to $channel..."
        if "$target_path" update --set-track "$channel" 2>/dev/null; then
            log_info "Release track set successfully"
        else
            log_warn "Could not set release track automatically. You can set it manually later with: $BINARY_NAME update --set-track $channel"
        fi
        echo ""
    fi

    # Check if install dir is in PATH
    if [[ ":$PATH:" != *":$install_dir:"* ]]; then
        log_warn "Installation directory is not in your PATH"
        echo ""
        echo "To use $BINARY_NAME, add the following to your shell configuration:"
        echo ""
        echo "  export PATH=\"$install_dir:\$PATH\""
        echo ""
        echo "For bash, add to ~/.bashrc or ~/.bash_profile"
        echo "For zsh, add to ~/.zshrc"
        echo ""
    else
        echo "You can now run: $BINARY_NAME --help"
    fi
}

# Main installation flow
main() {
    log_info "Starting Proton Pass CLI installation..."
    echo ""
    
    # Check dependencies
    check_dependencies
    
    # Detect platform
    os=$(detect_os)
    arch=$(detect_arch)
    log_info "Detected platform: $os/$arch"
    
    # Fetch binary info from manifest
    binary_info=$(fetch_binary_info "$os" "$arch")
    version=$(echo "$binary_info" | cut -d'|' -f1)
    url=$(echo "$binary_info" | cut -d'|' -f2)
    hash=$(echo "$binary_info" | cut -d'|' -f3)

    log_info "Download URL: $url"
    log_info "Binary hash: $hash"

    # Download binary
    temp_file=$(mktemp)

    # Set up trap so we remove the temp_file in case of exit
    trap "rm -f $temp_file" EXIT
    
    download_binary "$url" "$hash" "$temp_file"
    
    # Check Linux dependencies if on Linux
    if [ "$os" = "linux" ]; then
        check_linux_dependencies
    fi
    
    # Install binary
    install_binary "$temp_file"
}

main


