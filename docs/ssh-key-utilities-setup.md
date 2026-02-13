# SSH Key Authentication & Utilities Setup

This branch adds SSH public key authentication and utility installation support for the Xiaomi Smart Speaker Pro (OH2P) device.

## New Features

### 1. SSH Public Key Authentication

Enable password-less SSH access using SSH keys for improved security and convenience.

**Documentation**: [docs/ssh-keys.md](docs/ssh-keys.md)

**Quick Start**:
```bash
# Generate SSH key (if needed)
ssh-keygen -t rsa -b 4096 -f ~/.ssh/xiaoai_key

# Deploy public key to device
cd scripts
./deploy-ssh-key.sh 192.168.31.140 ~/.ssh/xiaoai_key.pub

# Test key authentication
ssh -i ~/.ssh/xiaoai_key -o HostKeyAlgorithms=+ssh-rsa root@192.168.31.140
```

**Features**:
- Automatic SSH daemon detection (Dropbear/OpenSSH)
- Persistent configuration via `/data/init.sh`
- Keeps password authentication enabled for recovery
- Works across device reboots

### 2. Additional Linux Utilities

Install essential Linux utilities (busybox, wget, curl) to overcome the device's limited built-in toolset.

**Documentation**: [docs/utilities.md](docs/utilities.md)

**Quick Start**:
```bash
# Install utilities
cd scripts
./install-utilities.sh 192.168.31.140

# Or install only specific utilities
./install-utilities.sh 192.168.31.140 --busybox-only
```

**Installed Tools**:
- **BusyBox** (~1MB): 300+ Unix utilities (tar, gzip, grep, sed, awk, find, etc.)
- **wget** (~500KB): HTTP file downloader
- **curl** (optional): HTTP client with advanced features

**Storage**:
- All utilities stored in `/data/bin` (persistent across reboots)
- Minimal disk space usage (~2-5MB total)
- PATH automatically configured via `/data/init.sh`

## Scripts

### SSH Key Deployment

**`scripts/deploy-ssh-key.sh`**
- Uploads your SSH public key to the device
- Configures Dropbear to accept key authentication
- Creates persistent boot script (`/data/init.sh`)
- Verifies configuration

Usage:
```bash
./deploy-ssh-key.sh <device-ip> <public-key-file>
```

### Utility Installation

**`scripts/install-utilities.sh`**
- Downloads ARM v7 static binaries
- Uploads to `/data/bin` on device
- Creates symlinks for common commands
- Verifies installation

Usage:
```bash
./install-utilities.sh <device-ip> [options]

Options:
  --busybox-only   Install only BusyBox
  --skip-busybox   Skip BusyBox installation
  --wget-only      Install only wget
```

### Init Script Template

**`scripts/init-template.sh`**
- Template for `/data/init.sh` boot script
- Configures SSH keys and PATH on device startup
- Customizable for additional init tasks

## Device Constraints

The OH2P device has several limitations:

1. **Limited disk space**: ~20-30MB available in `/data`
2. **Read-only filesystem**: Most directories immutable
3. **Writable locations**:
   - `/data` ✓ (persistent, use this)
   - `/tmp` ✗ (cleared on reboot, may have noexec)
4. **Limited utilities**: Only basic BusyBox commands by default

## Architecture

- **Device**: ARMv7-A (Cortex-A35)
- **OS**: Custom Linux firmware based on Amlogic platform
- **SSH**: Dropbear SSH daemon
- **Firmware version**: v1.58.1 (OH2P)

## Workflow

1. **Flash device** with patched firmware (see [docs/flash.md](docs/flash.md))
2. **Deploy SSH keys** for password-less access
3. **Install utilities** for enhanced functionality
4. **Build and deploy** your custom applications

## Benefits

### Security
- SSH key authentication is more secure than passwords
- Keys can be rotated without reflashing
- Password auth remains as fallback

### Convenience
- No password prompts for SSH/SCP operations
- Easier automation and scripting
- Better CI/CD integration

### Capability
- Enhanced Linux utilities (tar, gzip, grep, etc.)
- Download files directly on device
- Advanced text processing and scripting
- Reduced dependency on host machine

## Troubleshooting

See detailed troubleshooting sections in:
- [docs/ssh-keys.md#troubleshooting](docs/ssh-keys.md#troubleshooting)
- [docs/utilities.md#troubleshooting](docs/utilities.md#troubleshooting)

Common issues:

**SSH key authentication not working**:
1. Check permissions: `/root/.ssh` (700), `/root/.ssh/authorized_keys` (600)
2. Verify `/data/init.sh` is executable and runs on boot
3. Check Dropbear logs: `logread | grep dropbear`

**Utilities not in PATH**:
1. Verify `/data/init.sh` contains: `export PATH="/data/bin:$PATH"`
2. Current session: `export PATH="/data/bin:$PATH"` manually
3. Make init script executable: `chmod +x /data/init.sh`

**No disk space**:
1. Check usage: `df -h /data && du -sh /data/*`
2. Remove unnecessary files or binaries
3. Use `--busybox-only` to save space

## Testing

To test the implementation:

1. **Generate test key**:
   ```bash
   ssh-keygen -t rsa -b 2048 -f /tmp/test_key -N ""
   ```

2. **Deploy to device**:
   ```bash
   cd scripts
   ./deploy-ssh-key.sh 192.168.31.140 /tmp/test_key.pub
   ```

3. **Test key auth**:
   ```bash
   ssh -i /tmp/test_key -o HostKeyAlgorithms=+ssh-rsa root@192.168.31.140
   ```

4. **Install utilities**:
   ```bash
   ./install-utilities.sh 192.168.31.140
   ```

5. **Test utilities**:
   ```bash
   ssh root@192.168.31.140 "busybox --help"
   ssh root@192.168.31.140 "wget --version"
   ```

6. **Reboot and verify persistence**:
   ```bash
   ssh root@192.168.31.140 "reboot"
   # Wait 30 seconds
   ssh -i /tmp/test_key -o HostKeyAlgorithms=+ssh-rsa root@192.168.31.140
   ssh root@192.168.31.140 "busybox --list"
   ```

## Contributing

Improvements welcome! Areas for contribution:

- Support for additional device models (LX06)
- Pre-built static binary hosting
- Additional utility installation options
- Automated testing suite
- Device management scripts

## Related Documentation

- [Flash Device](docs/flash.md) - Initial firmware flashing
- [SSH Keys](docs/ssh-keys.md) - SSH key authentication setup
- [Utilities](docs/utilities.md) - Linux utility installation

## License

MIT License - see [LICENSE](LICENSE)
