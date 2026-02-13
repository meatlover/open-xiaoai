# Installing Additional Linux Utilities on OH2P

This guide explains how to install additional Linux utilities on your Xiaomi Smart Speaker Pro (OH2P) device despite its limited disk space and restricted filesystem.

## Built-in Utilities

**Good news!** The OH2P firmware (v1.58.1) already includes several essential utilities:

- **wget** (GNU Wget 1.19.2) - Download files from web
- **curl** (curl 7.55.1) - Transfer data with URLs
- **busybox** - Multi-call binary with common Unix utilities

Check what's available:
```bash
ssh root@<device-ip>
which wget curl busybox
wget --version
curl --version
```

## Prerequisites

- OH2P device with patched firmware and SSH access
- SSH key authentication configured (recommended, see [docs/ssh-keys.md](ssh-keys.md))

## Device Limitations

Understanding the device constraints:

1. **Limited disk space**: Only ~20-30MB typically available in `/data`
2. **Read-only filesystem**: Most system directories (`/`, `/etc`, `/usr`, `/bin`) are read-only
3. **Architecture**: aarch64 (ARM 64-bit)
4. **Writable locations**:
   - `/data` - persistent, writable ✓
   - `/tmp` - writable but cleared on reboot, may have noexec restrictions ✗

## Installing Additional Utilities

If you need utilities not included in the firmware, you can install static binaries to `/data/bin`:

### Step 1: Create binary directory

```bash
ssh root@<device-ip>
mkdir -p /data/bin
export PATH="/data/bin:$PATH"
```

The `/data/init.sh` script (created during SSH key setup) automatically adds `/data/bin` to PATH on boot.
### Step 2: Download Additional Binaries (if needed)

The firmware already includes wget and curl, but if you need additional tools, download ARM64 (aarch64) static binaries:

```bash
# Example: Download a specific tool (replace with actual aarch64 binary URL)
# On your local machine:
curl -Lo mytool https://example.com/path/to/mytool-aarch64
chmod +x mytool

# Upload to device
scp -o HostKeyAlgorithms=+ssh-rsa mytool root@<device-ip>:/data/bin/

# Or download directly on device using built-in wget:
ssh root@<device-ip>
cd /data/bin
wget https://example.com/path/to/mytool-aarch64 -O mytool
chmod +x mytool
```

**Important**: The device architecture is **aarch64** (ARM 64-bit), not armv7l. Use binaries compiled for:
- `aarch64`
- `arm64`
- `aarch64-linux-gnu`

### Step 3: Test Utilities

```bash
# Test built-in wget
wget --version

# Test built-in curl  
curl --version

# Test any additional binaries you installed
/data/bin/mytool --version
```

### Step 4: Add to PATH (Persistent)

The `/data/init.sh` script should already include PATH setup. Verify:

```bash
ssh root@<device-ip>
cat /data/init.sh | grep PATH
```

Should see:
```sh
export PATH="/data/bin:$PATH"
```

## Available Utilities After Installation

### BusyBox Utilities (300+ commands)

```bash
# List all available commands
busybox --list

# Common utilities included:
# - File operations: cp, mv, rm, mkdir, rmdir, touch, ls, find
# - Text processing: cat, grep, sed, awk, head, tail, wc, sort, uniq
# - Archiving: tar, gzip, gunzip, unzip, bzip2
# - Networking: wget, ping, nc, telnet, traceroute
# - System: ps, top, kill, df, du, free, uptime
# - Editors: vi
```

### Curl

```bash
curl --version
curl -I https://www.google.com      # Test HTTP request
curl -o /tmp/test.txt https://...   # Download file
```

### Wget

```bash
wget --version
wget -O /tmp/test.txt https://...   # Download file
```

## Disk Space Management

Check available space:

```bash
df -h /data
du -sh /data/bin
```

Clean up if needed:

```bash
# Remove unused binaries
rm /data/bin/wget  # Keep only curl, or vice versa

# Strip binaries (reduce size)
# Note: Do this on your local machine before uploading
arm-linux-gnueabihf-strip busybox  # Requires cross toolchain
```

## Installing Additional Tools

### Python (if space allows)

```bash
# Download Python for ARM (requires ~15-20MB)
# Warning: This will consume significant disk space
cd /data
wget https://www.python.org/ftp/python/3.9.7/Python-3.9.7.tgz
tar xzf Python-3.9.7.tgz
# ... follow standard build/install process
```

**Not recommended** due to space constraints. Consider running Python scripts on a server and connecting via SSH/HTTP instead.

### Other Utilities

Download from:
- https://busybox.net/downloads/binaries/
- https://github.com/ernw/static-toolbox/releases
- https://github.com/andrew-d/static-binaries

Ensure you download **ARM v7 (armv7l/armhf)** static binaries.

## Persistence Across Reboots

All files in `/data/bin` persist across reboots. The `/data/init.sh` script ensures `/data/bin` is added to PATH on every boot.

## Verification Script

Save as `/data/bin/check-utils.sh`:

```bash
#!/bin/sh
echo "=== Checking installed utilities ==="
echo ""
echo "BusyBox:"
[ -x /data/bin/busybox ] && /data/bin/busybox --help | head -n1 || echo "  Not installed"
echo ""
echo "Curl:"
[ -x /data/bin/curl ] && /data/bin/curl --version | head -n1 || echo "  Not installed"
echo ""
echo "Wget:"
[ -x /data/bin/wget ] && /data/bin/wget --version | head -n1 || echo "  Not installed"
echo ""
echo "Disk usage:"
df -h /data
echo ""
echo "Installed utilities:"
ls -lh /data/bin/
```

Run it:

```bash
chmod +x /data/bin/check-utils.sh
/data/bin/check-utils.sh
```

## Troubleshooting

### Binary won't execute: "not found"

This usually means wrong architecture. Verify device architecture:

```bash
ssh root@<device-ip> uname -m
# Should show: armv7l
```

Ensure downloaded binary is for `armv7l` or `armhf`.

### Binary won't execute: "Permission denied"

Set execute permission:

```bash
ssh root@<device-ip> chmod +x /data/bin/*
```

### "No space left on device"

Free up space:

```bash
# Check disk usage
df -h /data
du -sh /data/*

# Remove logs (if any)
rm -rf /data/log/*

# Remove unnecessary binaries
rm /data/bin/wget  # Keep only curl, or vice versa
```

### PATH not working after reboot

Verify `/data/init.sh` is executable and contains PATH setup:

```bash
ssh root@<device-ip>
ls -la /data/init.sh      # Should be -rwxr-xr-x
cat /data/init.sh | grep PATH
```

If missing, re-run the SSH key deployment script or manually add:

```bash
cat >> /data/init.sh << 'EOF'
# Setup PATH for utilities in /data/bin
if [ -d /data/bin ]; then
    export PATH="/data/bin:$PATH"
    echo "[init.sh] Added /data/bin to PATH"
fi
EOF
```

## Security Considerations

- Only download binaries from trusted sources
- Verify checksums when available
- Keep binaries up-to-date for security patches
- Consider using signed releases

## Next Steps

With utilities installed, you can:
1. Download and extract archives directly on device
2. Use advanced text processing (sed, awk, grep)
3. Write more complex shell scripts
4. Fetch data from APIs using curl/wget
5. Monitor system resources (top, ps, df)

See [examples/](../examples/) for use cases leveraging these utilities.
