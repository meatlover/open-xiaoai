# SSH Public Key Authentication Setup

This guide explains how to set up SSH public key authentication on your Xiaomi Smart Speaker Pro (OH2P) to enable password-less SSH access.

## Prerequisites

- Xiaomi 智能音箱 Pro (OH2P) with patched firmware already installed
- SSH password access working (default password: `open-xiaoai`)
- SSH key pair generated on your local machine

## Generate SSH Key Pair (if needed)

If you don't have an SSH key pair yet, generate one on your local machine:

```bash
ssh-keygen -t rsa -b 4096 -f ~/.ssh/xiaoai_key
# Press Enter for no passphrase or enter a secure passphrase
```

This creates:
- `~/.ssh/xiaoai_key` (private key - keep secure!)
- `~/.ssh/xiaoai_key.pub` (public key - safe to share)

## Setup Overview

The OH2P device runs Dropbear SSH daemon and has limited writable storage:
- `/data` is writable and persistent (use for SSH keys)
- `/tmp` is writable but cleared on reboot
- Most system directories are read-only

We'll store SSH keys in `/data/.ssh/` and use the startup script `/data/init.sh` (supported by the patched firmware) to configure Dropbear on boot.

## Deployment Script

Use the provided deployment script to automate the setup:

```bash
# From the repository root
cd scripts
./deploy-ssh-key.sh <device-ip> ~/.ssh/xiaoai_key.pub
```

For example:
```bash
./deploy-ssh-key.sh 192.168.31.140 ~/.ssh/xiaoai_key.pub
```

The script will:
1. Detect the SSH daemon type (Dropbear or OpenSSH)
2. Upload your public key to `/data/.ssh/authorized_keys`
3. Set correct permissions (700 for `.ssh`, 600 for `authorized_keys`)
4. Configure Dropbear to use the key file
5. Create/update `/data/init.sh` to persist the configuration across reboots

## Manual Setup

If you prefer manual setup:

### 1. Create SSH directory on device

```bash
ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>
mkdir -p /data/.ssh
chmod 700 /data/.ssh
```

### 2. Upload your public key

From your local machine:
```bash
# Using password authentication (will prompt for password)
cat ~/.ssh/xiaoai_key.pub | ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip> \
  "cat > /data/.ssh/authorized_keys && chmod 600 /data/.ssh/authorized_keys"
```

### 3. Configure Dropbear

Create or update `/data/init.sh` on the device:

```bash
ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>

cat > /data/init.sh << 'EOF'
#!/bin/sh

# Setup SSH public key authentication
if [ -f /data/.ssh/authorized_keys ]; then
    # CRITICAL: Dropbear reads authorized_keys from the user's home directory
    # as defined in /etc/passwd (which is /root for the root user).
    # Since /root is on a read-only SquashFS filesystem, we must use a bind mount.
    
    # Create temporary overlay directory for /root
    mkdir -p /tmp/root_overlay/.ssh
    
    # Copy authorized_keys from persistent storage
    cp /data/.ssh/authorized_keys /tmp/root_overlay/.ssh/authorized_keys
    
    # Set correct permissions (critical for SSH security)
    chmod 700 /tmp/root_overlay/.ssh
    chmod 600 /tmp/root_overlay/.ssh/authorized_keys
    
    # Bind mount the overlay to /root so Dropbear can find it
    mount --bind /tmp/root_overlay /root
fi
EOF

chmod +x /data/init.sh
```

### 4. Restart the device

```bash
reboot
```

Wait for the device to reboot (about 30 seconds).

## Test SSH Key Authentication

After reboot, test key-based authentication:

```bash
ssh -i ~/.ssh/xiaoai_key -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>
```

You should connect without entering a password!

## Configure SSH Client

Add to your `~/.ssh/config` for easier access:

```
Host xiaoai
    HostName 192.168.31.140
    User root
    IdentityFile ~/.ssh/xiaoai_key
    HostKeyAlgorithms +ssh-rsa
```

Now you can simply run:
```bash
ssh xiaoai
```

## Troubleshooting

### Connection still asks for password

1. Check if bind mount is active:
```bash
ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>
mount | grep /root
ls -la /root/.ssh/
```

You should see `/tmp/root_overlay on /root type none (rw,bind)` in the mount output.

2. Check permissions on device:
```bash
ssh -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>
ls -la /root/.ssh/
ls -la /data/.ssh/
```

Expected permissions:
- `/root/.ssh/` → 700 (drwx------)
- `/root/.ssh/authorized_keys` → 600 (-rw-------)
- `/data/.ssh/` → 700 (drwx------)
- `/data/.ssh/authorized_keys` → 600 (-rw-------)

3. Check if key is properly formatted:
```bash
cat /root/.ssh/authorized_keys
```

Should show your public key in format: `ssh-rsa AAAAB3...` or `ssh-ed25519 AAAAC3...`

4. Test with verbose SSH logging:
```bash
ssh -vvv -i ~/.ssh/xiaoai_key -o HostKeyAlgorithms=+ssh-rsa root@<device-ip>
```

Look for "Offering public key" and "Server accepts key" messages.

### Key authentication fails after reboot

Check if `/data/init.sh` is executable and runs on boot:
```bash
ls -la /data/init.sh
cat /data/init.sh
```

The init script should be executable (`-rwxr-xr-x`) and contain the SSH setup code with the bind mount.

## Technical Details

### Why Bind Mount is Required

The OH2P firmware has a unique filesystem layout that requires a special approach for SSH key authentication:

1. **Read-only root filesystem**: The root filesystem (`/`) is mounted from a SquashFS image on `/dev/mtdblock4`, which is read-only and compressed.

2. **Dropbear's authorized_keys lookup**: Dropbear (the SSH daemon) determines where to look for `authorized_keys` by reading the user's home directory from `/etc/passwd`:
   ```
   root:x:0:0:root:/root:/bin/ash
   ```
   This specifies `/root` as the home directory, so Dropbear looks for `/root/.ssh/authorized_keys`.

3. **The HOME environment variable doesn't help**: Even though the patched firmware's init script sets `HOME=/tmp` for the Dropbear process, Dropbear's authentication code uses `getpwnam()` to look up the user's home directory from `/etc/passwd`, NOT the `HOME` environment variable.

4. **The bind mount solution**: Since we cannot write to `/root` directly (read-only filesystem) and cannot change `/etc/passwd` (also read-only), we use a bind mount:
   - Create `/tmp/root_overlay/.ssh/` with our authorized_keys
   - Mount it with `mount --bind /tmp/root_overlay /root`
   - Now `/root/.ssh/authorized_keys` exists and is readable by Dropbear

This approach was discovered through `strace` analysis, which revealed:
```
stat64("/root/.ssh", ...) = -1 ENOENT (No such file or directory)
```

### Alternative Approaches That Don't Work

- **Copying to `/root/.ssh/`**: Fails because `/root` is read-only
- **Symlinking `/root/.ssh` to `/data/.ssh/`**: Can't create symlink in read-only `/root`  
- **Using `/tmp/.ssh/` with HOME=/tmp**: Dropbear ignores the `HOME` variable
- **Bind mounting just `.ssh` directory**: Requires `/root/.ssh` to exist first (can't create it)
- **Modifying `/etc/passwd`**: File is read-only on SquashFS

## Security Notes

- **Keep password authentication enabled** until you verify key authentication works
- **Backup your private key** securely
- **Use a passphrase** on your private key for added security
- The device keeps both password and key authentication enabled (this is safer for recovery)

## Disable Password Authentication (Optional)

⚠️ **Warning**: Only do this after confirming key authentication works! If something goes wrong, you'll lose SSH access.

To disable password authentication and use only keys:

1. Create `/data/dropbear.conf`:
```bash
ssh xiaoai
cat > /data/dropbear.conf << 'EOF'
# Disable password authentication (keys only)
PasswordAuth no
EOF
```

2. Update `/data/init.sh` to apply the config:
```bash
# Add to /data/init.sh
if [ -f /data/dropbear.conf ]; then
    # Note: Dropbear doesn't support config files directly
    # You need to restart dropbear with -s flag to disable password auth
    killall dropbear
    dropbear -s -r /data/etc/dropbear/dropbear_rsa_host_key
fi
```

⚠️ This is advanced - make sure you have a recovery method (serial console or reflashing) before disabling password auth!
