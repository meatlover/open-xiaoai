#!/bin/sh
#
# XiaoAI AI Bridge Installer
# Supports both online (fetch from Entware) and offline (bundled packages) modes
# Optional SOCKS5 proxy support
# GitHub: https://github.com/meatlover/open-xiaoai
#

set -e

# Configuration
INSTALL_DIR="/data"
OPT_DIR="${INSTALL_DIR}/opt"
BIN_DIR="${INSTALL_DIR}/bin"
ETC_DIR="${INSTALL_DIR}/etc"
GITHUB_REPO="meatlover/open-xiaoai"
GITHUB_RAW="https://raw.githubusercontent.com/${GITHUB_REPO}/main"
GITHUB_RELEASES="https://github.com/${GITHUB_REPO}/releases/download"
ENTWARE_REPO="https://bin.entware.net/aarch64-k3.10"
VERSION="1.0.0"

# Colors for output (if terminal supports it)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1"
}

log_warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

log_error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
}

# Check if running on XiaoAI device
check_device() {
    log_info "Checking device compatibility..."
    
    # Check for LEDE/OpenWrt
    if [ ! -f /etc/openwrt_release ] && [ ! -f /etc/os-release ]; then
        log_error "This does not appear to be an OpenWrt/LEDE device"
        exit 1
    fi
    
    # Check architecture
    ARCH=$(uname -m)
    if [ "$ARCH" != "aarch64" ]; then
        log_warn "Architecture is $ARCH, expected aarch64"
        log_warn "Installation may not work correctly"
    fi
    
    # Check /data partition exists and is writable
    if [ ! -d "$INSTALL_DIR" ]; then
        log_error "${INSTALL_DIR} directory not found"
        exit 1
    fi
    
    if ! touch "${INSTALL_DIR}/.write_test" 2>/dev/null; then
        log_error "Cannot write to ${INSTALL_DIR}"
        exit 1
    fi
    rm -f "${INSTALL_DIR}/.write_test"
    
    # Check available space (need at least 50MB)
    AVAILABLE=$(df -k "$INSTALL_DIR" | tail -1 | awk '{print $4}')
    if [ "$AVAILABLE" -lt 51200 ]; then
        log_error "Insufficient space in ${INSTALL_DIR}. Need 50MB, have $((AVAILABLE/1024))MB"
        exit 1
    fi
    
    log_info "Device check passed"
}

# Detect SOCKS5 proxy settings
detect_proxy() {
    SOCKS5_PROXY="${SOCKS5_PROXY:-${SOCKS_PROXY:-${ALL_PROXY:-}}}"
    
    if [ -n "$SOCKS5_PROXY" ]; then
        log_info "SOCKS5 proxy detected: $SOCKS5_PROXY"
        export ALL_PROXY="$SOCKS5_PROXY"
    fi
}

# Download function with SOCKS5 support
download() {
    local url="$1"
    local output="$2"
    
    if command -v curl >/dev/null 2>&1; then
        if [ -n "$SOCKS5_PROXY" ]; then
            curl -fsSL --socks5-hostname "$SOCKS5_PROXY" -o "$output" "$url"
        else
            curl -fsSL -o "$output" "$url"
        fi
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "$output" "$url"
    else
        log_error "Neither curl nor wget found"
        exit 1
    fi
}

# Install Entware bootstrap
install_entware() {
    log_info "Installing Entware to ${OPT_DIR}..."
    
    if [ -d "$OPT_DIR" ] && [ -f "$OPT_DIR/bin/opkg" ]; then
        log_info "Entware already installed"
        return 0
    fi
    
    mkdir -p "$OPT_DIR"
    
    # Download and run Entware installer
    local installer_url="${ENTWARE_REPO}/installer/generic.sh"
    local installer_path="/tmp/entware_installer.sh"
    
    log_info "Downloading Entware installer..."
    download "$installer_url" "$installer_path"
    
    log_info "Running Entware installer..."
    chmod +x "$installer_path"
    sh "$installer_path"
    
    rm -f "$installer_path"
    
    # Add to PATH
    if ! grep -q "${OPT_DIR}/bin" /etc/profile 2>/dev/null; then
        log_info "Adding Entware to PATH..."
        echo "export PATH=\"${OPT_DIR}/bin:${OPT_DIR}/sbin:\$PATH\"" >> /etc/profile
    fi
    
    log_info "Entware installed successfully"
}

# Update package list
update_package_list() {
    log_info "Updating package list..."
    "${OPT_DIR}/bin/opkg" update
}

# Install core packages
install_core_packages() {
    log_info "Installing core packages..."
    
    local packages="python3 python3-pip libopenssl3 openssl-util"
    
    for pkg in $packages; do
        log_info "Installing $pkg..."
        "${OPT_DIR}/bin/opkg" install "$pkg" || {
            log_error "Failed to install $pkg"
            exit 1
        }
    done
}

# Install Python websockets
install_websockets() {
    log_info "Installing Python websockets library..."
    
    "${OPT_DIR}/bin/pip3" install --no-cache-dir websockets || {
        log_error "Failed to install websockets"
        exit 1
    }
}

# Install optional packages
install_optional_packages() {
    log_info "Installing optional packages..."
    
    # redsocks for SOCKS5 proxy support
    if [ -n "$INSTALL_REDSOCKS" ]; then
        log_info "Installing redsocks for SOCKS5 support..."
        "${OPT_DIR}/bin/opkg" install redsocks || log_warn "Failed to install redsocks"
    fi
    
    # Additional useful tools
    "${OPT_DIR}/bin/opkg" install ca-bundle || true
}

# Setup configuration directories
setup_directories() {
    log_info "Setting up directories..."
    
    mkdir -p "${BIN_DIR}"
    mkdir -p "${ETC_DIR}/ai-bridge"
    mkdir -p "${ETC_DIR}/ssl"
    mkdir -p "${INSTALL_DIR}/log"
}

# Download and install AI Bridge service
download_service() {
    log_info "Downloading AI Bridge service..."
    
    local service_url="${GITHUB_RAW}/src/ai-bridge/main.py"
    local config_url="${GITHUB_RAW}/src/ai-bridge/config.yaml"
    
    download "$service_url" "${BIN_DIR}/ai-bridge.py"
    chmod +x "${BIN_DIR}/ai-bridge.py"
    
    if [ ! -f "${ETC_DIR}/ai-bridge/config.yaml" ]; then
        download "$config_url" "${ETC_DIR}/ai-bridge/config.yaml"
    else
        log_warn "Config exists, skipping download"
    fi
}

# Setup init script
setup_init() {
    log_info "Setting up init script..."
    
    local init_url="${GITHUB_RAW}/config/init.d/S99ai-bridge"
    local init_path="${INSTALL_DIR}/init.d/S99ai-bridge"
    
    mkdir -p "${INSTALL_DIR}/init.d"
    
    if [ -f "$init_path" ]; then
        log_warn "Init script exists, backing up..."
        mv "$init_path" "${init_path}.backup.$(date +%s)"
    fi
    
    download "$init_url" "$init_path"
    chmod +x "$init_path"
    
    # Add to startup if not already present
    if [ ! -f /etc/rc.d/S99ai-bridge ]; then
        ln -sf "$init_path" /etc/rc.d/S99ai-bridge
    fi
}

# Create environment file
create_env_file() {
    log_info "Creating environment file..."
    
    cat > "${ETC_DIR}/ai-bridge/env" << EOF
# AI Bridge Environment Configuration
INSTALL_DIR=${INSTALL_DIR}
OPT_DIR=${OPT_DIR}
PYTHON=${OPT_DIR}/bin/python3
PIP=${OPT_DIR}/bin/pip3

# WebSocket Configuration
WS_HOST=localhost
WS_PORT=8765
WS_USE_TLS=true

# mTLS Configuration (will be generated on first run)
MTLS_CERT_DIR=${ETC_DIR}/ssl
MTLS_CA_CERT=
MTLS_CLIENT_CERT=
MTLS_CLIENT_KEY=

# Proxy Configuration (optional)
SOCKS5_PROXY=${SOCKS5_PROXY:-}
REDSOCKS_ENABLED=${INSTALL_REDSOCKS:-false}
EOF
}

# Setup redsocks configuration
setup_redsocks() {
    if [ -z "$INSTALL_REDSOCKS" ]; then
        return 0
    fi
    
    log_info "Setting up redsocks configuration..."
    
    if [ -f "${ETC_DIR}/redsocks.conf" ]; then
        log_warn "Redsocks config exists, skipping"
        return 0
    fi
    
    cat > "${ETC_DIR}/redsocks.conf" << 'EOF'
base {
    log_debug = off;
    log_info = on;
    log = stderr;
    daemon = off;
    redirector = iptables;
}

redsocks {
    local_ip = 127.0.0.1;
    local_port = 12345;
    ip = 127.0.0.1;
    port = 1080;
    type = socks5;
}
EOF
    
    log_info "Redsocks config created at ${ETC_DIR}/redsocks.conf"
    log_warn "Please edit the config to set your SOCKS5 proxy IP and port"
}

# Create upgrade script
create_upgrade_script() {
    log_info "Creating upgrade script..."
    
    cat > "${BIN_DIR}/ai-bridge-upgrade" << 'EOF'
#!/bin/sh
# AI Bridge Upgrade Script

INSTALL_DIR="/data"
GITHUB_RAW="https://raw.githubusercontent.com/meatlover/open-xiaoai/main"

echo "Upgrading AI Bridge..."

# Backup current config
if [ -f "${INSTALL_DIR}/etc/ai-bridge/config.yaml" ]; then
    cp "${INSTALL_DIR}/etc/ai-bridge/config.yaml" "${INSTALL_DIR}/etc/ai-bridge/config.yaml.backup"
fi

# Download latest service
curl -fsSL "${GITHUB_RAW}/src/ai-bridge/main.py" -o "${INSTALL_DIR}/bin/ai-bridge.py"
chmod +x "${INSTALL_DIR}/bin/ai-bridge.py"

# Update init script
curl -fsSL "${GITHUB_RAW}/config/init.d/S99ai-bridge" -o "${INSTALL_DIR}/init.d/S99ai-bridge"
chmod +x "${INSTALL_DIR}/init.d/S99ai-bridge"

# Update packages
echo "Updating packages..."
"${INSTALL_DIR}/opt/bin/opkg" update
"${INSTALL_DIR}/opt/bin/opkg" upgrade

# Upgrade Python packages
"${INSTALL_DIR}/opt/bin/pip3" install --upgrade websockets

echo "Upgrade complete!"
echo "Restarting service..."
"${INSTALL_DIR}/init.d/S99ai-bridge" restart
EOF
    
    chmod +x "${BIN_DIR}/ai-bridge-upgrade"
}

# Print installation summary
print_summary() {
    echo ""
    echo "========================================"
    log_info "Installation Complete!"
    echo "========================================"
    echo ""
    echo "Installed Components:"
    echo "  - Python 3: ${OPT_DIR}/bin/python3"
    echo "  - pip3: ${OPT_DIR}/bin/pip3"
    echo "  - OpenSSL 3.x: ${OPT_DIR}/bin/openssl"
    echo "  - websockets: Python library"
    [ -n "$INSTALL_REDSOCKS" ] && echo "  - redsocks: ${OPT_DIR}/bin/redsocks"
    echo ""
    echo "AI Bridge:"
    echo "  - Service: ${BIN_DIR}/ai-bridge.py"
    echo "  - Config: ${ETC_DIR}/ai-bridge/config.yaml"
    echo "  - Init: ${INSTALL_DIR}/init.d/S99ai-bridge"
    echo "  - Upgrade: ${BIN_DIR}/ai-bridge-upgrade"
    echo ""
    echo "Usage:"
    echo "  Start:   ${INSTALL_DIR}/init.d/S99ai-bridge start"
    echo "  Stop:    ${INSTALL_DIR}/init.d/S99ai-bridge stop"
    echo "  Restart: ${INSTALL_DIR}/init.d/S99ai-bridge restart"
    echo "  Status:  ${INSTALL_DIR}/init.d/S99ai-bridge status"
    [ -n "$INSTALL_REDSOCKS" ] && echo "  Proxy:   ${INSTALL_DIR}/init.d/S99ai-bridge proxy-start"
    echo "  Upgrade: ${BIN_DIR}/ai-bridge-upgrade"
    echo ""
    echo "Next Steps:"
    echo "  1. Edit configuration: vi ${ETC_DIR}/ai-bridge/config.yaml"
    [ -n "$INSTALL_REDSOCKS" ] && echo "  2. Configure proxy: vi ${ETC_DIR}/redsocks.conf"
    echo "  2. Start service: ${INSTALL_DIR}/init.d/S99ai-bridge start"
    echo "  3. Check logs: tail -f ${INSTALL_DIR}/log/ai-bridge.log"
    echo ""
    log_info "To re-install or upgrade, simply run this installer again"
    echo "========================================"
}

# Main installation function
main() {
    echo "========================================"
    echo "XiaoAI AI Bridge Installer v${VERSION}"
    echo "========================================"
    echo ""
    
    # Parse arguments
    while [ $# -gt 0 ]; do
        case "$1" in
            --offline)
                OFFLINE_MODE=1
                shift
                ;;
            --socks5)
                INSTALL_REDSOCKS=1
                shift
                ;;
            --proxy)
                SOCKS5_PROXY="$2"
                export ALL_PROXY="$2"
                shift 2
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --offline     Use bundled packages (offline mode)"
                echo "  --socks5      Install redsocks for SOCKS5 support"
                echo "  --proxy URL   Use SOCKS5 proxy for downloads (e.g., socks5://host:1080)"
                echo "  --help        Show this help message"
                echo ""
                echo "Environment Variables:"
                echo "  SOCKS5_PROXY  SOCKS5 proxy URL"
                echo "  SOCKS_PROXY   Alternative proxy variable"
                echo "  ALL_PROXY     Generic proxy variable"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Detect proxy settings
    detect_proxy
    
    # Check device compatibility
    check_device
    
    # Stop existing service if running
    if [ -f "${INSTALL_DIR}/init.d/S99ai-bridge" ]; then
        log_info "Stopping existing service..."
        "${INSTALL_DIR}/init.d/S99ai-bridge" stop 2>/dev/null || true
    fi
    
    # Install Entware
    install_entware
    
    # Update package list
    update_package_list
    
    # Install packages
    install_core_packages
    install_websockets
    install_optional_packages
    
    # Setup directories
    setup_directories
    
    # Download service files
    download_service
    
    # Setup init
    setup_init
    
    # Create configuration
    create_env_file
    setup_redsocks
    create_upgrade_script
    
    # Print summary
    print_summary
}

# Run main function
main "$@"
