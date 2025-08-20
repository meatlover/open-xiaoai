#!/bin/bash

# Quick SSH access to your Mi device
MI_DEVICE_IP=${1:-"192.168.143.211"}
MI_PASSWORD="open-xiaoai"

echo "🔌 Connecting to Mi device at $MI_DEVICE_IP..."
sshpass -p "$MI_PASSWORD" ssh -o HostKeyAlgorithms=+ssh-rsa root@$MI_DEVICE_IP
