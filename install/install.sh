#!/bin/sh
#
# XiaoAI AI Bridge - Complete Installer
# Installs: Python 3.11, OpenSSL 3.x, websockets, redsocks
# GitHub: https://github.com/meatlover/open-xiaoai
#

set -e

INSTALL_DIR="/data"
OPT_DIR="${INSTALL_DIR}/opt"
REPO="https://bin.entware.net/aarch64-k3.10"
WEBSOCKETS_URL="https://files.pythonhosted.org/packages/6f/28/258ebab549c2bf3e64d2b0217b973467394a9cea8c42f70418ca2c5d0d2e/websockets-16.0-py3-none-any.whl"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    printf "${GREEN}[INFO]${NC} %s\n" "$1"
}

log_warn() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

log_error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
}

check_device() {
    log_info "Checking device..."
    
    ARCH=$(uname -m)
    if [ "$ARCH" != "aarch64" ]; then
        log_warn "Architecture is $ARCH, expected aarch64"
    fi
    
    if [ ! -d "$INSTALL_DIR" ]; then
        log_error "$INSTALL_DIR not found"
        exit 1
    fi
    
    AVAILABLE=$(df -k "$INSTALL_DIR" | tail -1 | awk '{print $4}')
    if [ "$AVAILABLE" -lt 51200 ]; then
        log_error "Need at least 50MB free, have $((AVAILABLE/1024))MB"
        exit 1
    fi
    
    log_info "Device OK, $((AVAILABLE/1024))MB available"
}

install_pkg() {
    pkg_file=$1
    url="${REPO}/${pkg_file}"
    
    log_info "Installing ${pkg_file}..."
    
    cd /tmp
    wget -q "$url" -O "${pkg_file}.tmp" || {
        log_error "Failed to download ${pkg_file}"
        return 1
    }
    
    tar -xzf "${pkg_file}.tmp" 2>/dev/null || {
        log_error "Failed to extract ${pkg_file}"
        rm -f "${pkg_file}.tmp"
        return 1
    }
    
    if [ -f data.tar.gz ]; then
        tar -xzf data.tar.gz -C "$OPT_DIR"
        rm -f data.tar.gz control.tar.gz debian-binary
    fi
    
    rm -f "${pkg_file}.tmp"
}

fix_paths() {
    if [ -d "$OPT_DIR/opt" ]; then
        cp -r "$OPT_DIR/opt/"* "$OPT_DIR/" 2>/dev/null || true
        rm -rf "$OPT_DIR/opt"
    fi
}

install_base_libs() {
    log_info "Installing base libraries..."
    
    install_pkg "libgcc_8.4.0-11_aarch64-3.10.ipk"
    install_pkg "libc_2.27-11_aarch64-3.10.ipk"
    install_pkg "libssp_8.4.0-11_aarch64-3.10.ipk"
    install_pkg "librt_2.27-11_aarch64-3.10.ipk"
    install_pkg "libpthread_2.27-11_aarch64-3.10.ipk"
    install_pkg "zlib_1.3.1-1_aarch64-3.10.ipk"
    
    fix_paths
}

install_openssl() {
    log_info "Installing OpenSSL 3.x..."
    
    install_pkg "libopenssl_3.5.0-1_aarch64-3.10.ipk"
    install_pkg "openssl-util_3.5.0-1_aarch64-3.10.ipk"
    
    fix_paths
}

install_python() {
    log_info "Installing Python 3.11..."
    
    install_pkg "libpython3_3.11.10-1_aarch64-3.10.ipk"
    install_pkg "python3-base_3.11.10-1_aarch64-3.10.ipk"
    install_pkg "python3-light_3.11.10-1_aarch64-3.10.ipk"
    install_pkg "python3_3.11.10-1_aarch64-3.10.ipk"
    install_pkg "python3-pip_23.3.1-1_aarch64-3.10.ipk"
    install_pkg "python3-openssl_3.11.10-1_aarch64-3.10.ipk"
    
    # Additional Python modules
    for pkg in urllib ctypes email logging html; do
        install_pkg "python3-${pkg}_3.11.10-1_aarch64-3.10.ipk" || true
    done
    
    install_pkg "ca-bundle_20241223-1_all.ipk"
    
    fix_paths
}

install_websockets() {
    log_info "Installing websockets library..."
    
    cd /tmp
    wget -q "$WEBSOCKETS_URL" -O websockets.whl || {
        log_error "Failed to download websockets"
        return 1
    }
    
    export LD_LIBRARY_PATH="$OPT_DIR/lib"
    "$OPT_DIR/lib/ld-2.27.so" "$OPT_DIR/bin/python3" -c \
        "import zipfile; zipfile.ZipFile('websockets.whl').extractall('$OPT_DIR/lib/python3.11/site-packages/')"
    
    rm -f websockets.whl
}

install_redsocks() {
    log_info "Installing redsocks (SOCKS5 proxy)..."
    
    install_pkg "libevent2-core_2.1.12-2_aarch64-3.10.ipk"
    install_pkg "redsocks_0.5-2_aarch64-3.10.ipk"
    
    fix_paths
    
    # Create config
    mkdir -p "$INSTALL_DIR/etc"
    cat > "$INSTALL_DIR/etc/redsocks.conf" << 'EOF'
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
}

create_wrappers() {
    log_info "Creating convenience wrappers..."
    
    mkdir -p "$INSTALL_DIR/bin"
    
    # Python wrapper
    cat > "$INSTALL_DIR/bin/ai-python3" << 'EOF'
#!/bin/sh
export LD_LIBRARY_PATH=/data/opt/lib
exec /data/opt/lib/ld-2.27.so /data/opt/bin/python3 "$@"
EOF
    chmod +x "$INSTALL_DIR/bin/ai-python3"
    
    # OpenSSL wrapper
    cat > "$INSTALL_DIR/bin/ai-openssl" << 'EOF'
#!/bin/sh
export LD_LIBRARY_PATH=/data/opt/lib
exec /data/opt/lib/ld-2.27.so /data/opt/bin/openssl "$@"
EOF
    chmod +x "$INSTALL_DIR/bin/ai-openssl"
    
    # Redsocks wrapper
    cat > "$INSTALL_DIR/bin/ai-redsocks" << 'EOF'
#!/bin/sh
export LD_LIBRARY_PATH=/data/opt/lib
exec /data/opt/lib/ld-2.27.so /data/opt/sbin/redsocks "$@"
EOF
    chmod +x "$INSTALL_DIR/bin/ai-redsocks"
}

create_dirs() {
    mkdir -p "$INSTALL_DIR/etc/ai-bridge"
    mkdir -p "$INSTALL_DIR/etc/ssl"
    mkdir -p "$INSTALL_DIR/log"
    mkdir -p "$INSTALL_DIR/init.d"
}

test_installation() {
    log_info "Testing installation..."
    
    export LD_LIBRARY_PATH="$OPT_DIR/lib"
    
    echo ""
    echo "OpenSSL:"
    "$OPT_DIR/lib/ld-2.27.so" "$OPT_DIR/bin/openssl" version 2>&1 | head -1 || echo "  ✗ Failed"
    
    echo ""
    echo "Python:"
    "$OPT_DIR/lib/ld-2.27.so" "$OPT_DIR/bin/python3" --version 2>&1 || echo "  ✗ Failed"
    
    echo ""
    echo "Websockets:"
    "$OPT_DIR/lib/ld-2.27.so" "$OPT_DIR/bin/python3" -c "import websockets; print('websockets:', websockets.__version__)" 2>&1 || echo "  ✗ Failed"
    
    echo ""
    echo "Redsocks:"
    "$OPT_DIR/lib/ld-2.27.so" "$OPT_DIR/sbin/redsocks" -h 2>&1 | head -1 || echo "  ✗ Failed"
}

print_summary() {
    echo ""
    echo "========================================"
    log_info "Installation Complete!"
    echo "========================================"
    echo ""
    echo "Installed:"
    echo "  ✓ Python 3.11.10"
    echo "  ✓ OpenSSL 3.5.0"
    echo "  ✓ websockets 16.0"
    echo "  ✓ redsocks 0.5"
    echo ""
    echo "Location: $OPT_DIR"
    echo "Disk Usage:"
    du -sh "$OPT_DIR"
    echo ""
    echo "Usage:"
    echo "  Python:   /data/bin/ai-python3 <script.py>"
    echo "  OpenSSL:  /data/bin/ai-openssl <args>"
    echo "  Redsocks: /data/bin/ai-redsocks -c /data/etc/redsocks.conf"
    echo ""
    echo "Config Files:"
    echo "  AI Bridge:  $INSTALL_DIR/etc/ai-bridge/"
    echo "  SSL Certs:  $INSTALL_DIR/etc/ssl/"
    echo "  Redsocks:   $INSTALL_DIR/etc/redsocks.conf"
    echo ""
    echo "Logs:"
    echo "  $INSTALL_DIR/log/"
    echo ""
    echo "To use Python directly:"
    echo "  export LD_LIBRARY_PATH=/data/opt/lib"
    echo "  /data/opt/lib/ld-2.27.so /data/opt/bin/python3 <script>"
    echo ""
    echo "Or use the wrapper:"
    echo "  /data/bin/ai-python3 <script>"
    echo ""
}

main() {
    echo "========================================"
    echo "XiaoAI AI Bridge - Complete Installer"
    echo "========================================"
    echo ""
    
    # Parse args
    SKIP_REDSOCKS=0
    while [ $# -gt 0 ]; do
        case "$1" in
            --skip-redsocks)
                SKIP_REDSOCKS=1
                shift
                ;;
            --help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --skip-redsocks  Don't install redsocks"
                echo "  --help           Show this help"
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    check_device
    
    # Create directories
    mkdir -p "$OPT_DIR"/{bin,lib,sbin,share,include,tmp}
    create_dirs
    
    # Install packages
    install_base_libs
    install_openssl
    install_python
    install_websockets
    
    if [ "$SKIP_REDSOCKS" -eq 0 ]; then
        install_redsocks
    fi
    
    create_wrappers
    
    # Test
    test_installation
    
    # Summary
    print_summary
}

main "$@"
