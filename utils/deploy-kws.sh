#!/bin/bash

# Open-XiaoAI KWS Complete Setup and Management Tool
# Deploys customized wake-up word feature to Mi speakers

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

DEVICE_IP="${DEVICE_IP:-192.168.143.211}"
DEVICE_USER="${DEVICE_USER:-root}"
DEVICE_PASS="${DEVICE_PASS:-open-xiaoai}"

echo "🎯 Open-XiaoAI KWS Complete Setup Tool"
echo "======================================"
echo "📱 Device: $DEVICE_USER@$DEVICE_IP"
echo "📁 Project: $PROJECT_ROOT"
echo ""

# Source color codes for better output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check dependencies
check_dependencies() {
    print_status "Checking dependencies..."
    
    local missing_deps=()
    
    if ! command -v sshpass &> /dev/null; then
        missing_deps+=("sshpass")
    fi
    
    if ! command -v ssh &> /dev/null; then
        missing_deps+=("ssh")
    fi
    
    if ! command -v scp &> /dev/null; then
        missing_deps+=("scp")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_error "Missing dependencies: ${missing_deps[*]}"
        echo ""
        echo "Install instructions:"
        echo "  macOS: brew install sshpass openssh"
        echo "  Ubuntu/Debian: sudo apt-get install sshpass openssh-client"
        echo "  CentOS/RHEL: sudo yum install sshpass openssh-clients"
        exit 1
    fi
    
    print_success "All dependencies satisfied"
}

# Function to test device connectivity
test_connectivity() {
    print_status "Testing device connectivity..."
    
    if ! sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa -o ConnectTimeout=10 "$DEVICE_USER@$DEVICE_IP" "echo 'Connected'" &>/dev/null; then
        print_error "Cannot connect to device $DEVICE_USER@$DEVICE_IP"
        echo ""
        echo "Please check:"
        echo "  - Device IP address"
        echo "  - Network connectivity"
        echo "  - SSH credentials"
        echo "  - Device SSH service status"
        exit 1
    fi
    
    print_success "Device connectivity verified"
}

# Function to deploy KWS setup
deploy_kws() {
    print_status "Deploying KWS system..."
    
    # Run the setup-kws.sh script
    "$SCRIPT_DIR/setup-kws.sh" "$DEVICE_IP" "$DEVICE_USER" "$DEVICE_PASS" "setup"
    
    print_success "KWS system deployed"
}

# Function to deploy custom configuration
deploy_config() {
    print_status "Deploying custom configuration..."
    
    local workspace_dir="$PROJECT_ROOT/workspace"
    
    # Check if custom config exists
    if [ -f "$workspace_dir/keywords.txt" ]; then
        print_status "Uploading custom keywords..."
        "$SCRIPT_DIR/manage-wake-words.sh" "upload-keywords"
    else
        print_warning "No custom keywords found, using defaults"
    fi
    
    if [ -f "$workspace_dir/replies.txt" ]; then
        print_status "Uploading custom replies..."
        "$SCRIPT_DIR/manage-wake-words.sh" "upload-replies"
    else
        print_warning "No custom replies found, using defaults"
    fi
    
    print_success "Configuration deployed"
}

# Function to verify deployment
verify_deployment() {
    print_status "Verifying deployment..."
    
    # Check if KWS files exist on device
    if sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "test -f /data/open-xiaoai/kws/keywords.txt"; then
        print_success "Keywords file deployed"
    else
        print_error "Keywords file missing"
        return 1
    fi
    
    if sshpass -p "$DEVICE_PASS" ssh -o HostKeyAlgorithms=+ssh-rsa "$DEVICE_USER@$DEVICE_IP" "test -f /data/init.sh"; then
        print_success "Auto-start configured"
    else
        print_warning "Auto-start not configured"
    fi
    
    print_success "Deployment verified"
}

# Function to show post-deployment info
show_post_deployment_info() {
    echo ""
    echo "🎉 KWS Deployment Completed Successfully!"
    echo "========================================"
    echo ""
    echo "📋 What was deployed:"
    echo "  ✅ KWS directory structure"
    echo "  ✅ Default wake words (天猫精灵, 小度小度, 豆包豆包, 你好小智)"
    echo "  ✅ Default welcome messages"
    echo "  ✅ Sherpa-ONNX KWS service"
    echo "  ✅ Auto-start configuration"
    echo ""
    echo "🎮 Next Steps:"
    echo "  1. Reboot device: ssh $DEVICE_USER@$DEVICE_IP 'reboot'"
    echo "  2. Test wake words: $SCRIPT_DIR/manage-wake-words.sh debug"
    echo "  3. Customize words: $SCRIPT_DIR/manage-wake-words.sh edit-keywords"
    echo "  4. Check status: $SCRIPT_DIR/manage-wake-words.sh status"
    echo ""
    echo "🛠️  Management Tools:"
    echo "  • Setup: $SCRIPT_DIR/setup-kws.sh"
    echo "  • Manage: $SCRIPT_DIR/manage-wake-words.sh"
    echo "  • Generate: $SCRIPT_DIR/keywords.py"
    echo ""
    echo "📚 Documentation:"
    echo "  • KWS Guide: $PROJECT_ROOT/docs/kws-setup.md"
    echo "  • Templates: $PROJECT_ROOT/workspace/"
    echo ""
    echo "🐛 Troubleshooting:"
    echo "  • Debug mode: $SCRIPT_DIR/manage-wake-words.sh debug"
    echo "  • Check logs: ssh $DEVICE_USER@$DEVICE_IP 'tail -f /tmp/open-xiaoai/kws.log'"
    echo "  • Restart service: ssh $DEVICE_USER@$DEVICE_IP 'pkill -f sherpa && /data/init.sh'"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  deploy    - Complete KWS deployment (default)"
    echo "  check     - Check dependencies and connectivity"
    echo "  config    - Deploy custom configuration only"
    echo "  verify    - Verify existing deployment"
    echo "  status    - Show current status"
    echo "  debug     - Run debug mode"
    echo ""
    echo "Environment Variables:"
    echo "  DEVICE_IP       - Device IP address (default: 192.168.143.211)"
    echo "  DEVICE_USER     - Device username (default: root)"
    echo "  DEVICE_PASS     - Device password (default: open-xiaoai)"
    echo ""
    echo "Examples:"
    echo "  $0                    # Complete deployment"
    echo "  $0 check              # Check dependencies and connectivity"
    echo "  $0 config             # Deploy custom config only"
    echo "  DEVICE_IP=192.168.1.100 $0 deploy  # Deploy to different device"
}

# Parse command
COMMAND="${1:-deploy}"

case "$COMMAND" in
    "deploy")
        check_dependencies
        test_connectivity
        deploy_kws
        deploy_config
        verify_deployment
        show_post_deployment_info
        ;;
    "check")
        check_dependencies
        test_connectivity
        print_success "All checks passed"
        ;;
    "config")
        check_dependencies
        test_connectivity
        deploy_config
        print_success "Configuration deployed"
        ;;
    "verify")
        check_dependencies
        test_connectivity
        verify_deployment
        print_success "Verification completed"
        ;;
    "status")
        "$SCRIPT_DIR/manage-wake-words.sh" "status"
        ;;
    "debug")
        "$SCRIPT_DIR/manage-wake-words.sh" "debug"
        ;;
    "help"|"-h"|"--help")
        show_usage
        ;;
    *)
        print_error "Unknown command: $COMMAND"
        echo ""
        show_usage
        exit 1
        ;;
esac
