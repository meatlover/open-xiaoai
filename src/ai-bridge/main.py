#!/usr/bin/env python3
"""
XiaoAI AI Bridge Service
WebSocket client for bidirectional audio/text streaming with mTLS support
"""

import asyncio
import argparse
import json
import logging
import os
import ssl
import sys
import yaml
from pathlib import Path
from typing import Optional, Dict, Any

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger('ai-bridge')


class MTLSConfig:
    """mTLS configuration manager"""
    
    def __init__(self, config_dir: str = "/data/etc/ssl"):
        self.config_dir = Path(config_dir)
        self.ca_cert = self.config_dir / "ca.crt"
        self.client_cert = self.config_dir / "client.crt"
        self.client_key = self.config_dir / "client.key"
    
    def is_configured(self) -> bool:
        """Check if mTLS certificates are present"""
        return all([
            self.ca_cert.exists(),
            self.client_cert.exists(),
            self.client_key.exists()
        ])
    
    def create_ssl_context(self) -> ssl.SSLContext:
        """Create SSL context with mTLS"""
        if not self.is_configured():
            raise FileNotFoundError("mTLS certificates not found")
        
        context = ssl.create_default_context(ssl.Purpose.SERVER_AUTH)
        context.load_verify_locations(str(self.ca_cert))
        context.load_cert_chain(
            certfile=str(self.client_cert),
            keyfile=str(self.client_key)
        )
        context.check_hostname = True
        context.verify_mode = ssl.CERT_REQUIRED
        
        # TLS 1.2+ only
        context.minimum_version = ssl.TLSVersion.TLSv1_2
        
        return context


class AIBridge:
    """AI Bridge WebSocket Client"""
    
    def __init__(self, config: Dict[str, Any]):
        self.config = config
        self.ws_url = config.get('websocket', {}).get('url')
        self.use_mtls = config.get('mtls', {}).get('enabled', True)
        self.use_proxy = config.get('proxy', {}).get('enabled', False)
        self.proxy_url = config.get('proxy', {}).get('url')
        
        # mTLS configuration
        if self.use_mtls:
            ssl_dir = config.get('mtls', {}).get('cert_dir', '/data/etc/ssl')
            self.mtls = MTLSConfig(ssl_dir)
        else:
            self.mtls = None
        
        self.websocket = None
        self.running = False
    
    async def connect(self):
        """Connect to WebSocket server"""
        import websockets
        
        # Validate URL is configured
        if not self.ws_url:
            raise ValueError("WebSocket URL not configured. Please set websocket.url in config.yaml")
        
        logger.info(f"Connecting to {self.ws_url}")
        
        # Setup SSL context
        ssl_context = None
        if self.use_mtls and self.mtls:
            try:
                ssl_context = self.mtls.create_ssl_context()
                logger.info("mTLS enabled")
            except Exception as e:
                logger.error(f"Failed to setup mTLS: {e}")
                raise
        elif self.ws_url.startswith('wss://'):
            ssl_context = ssl.create_default_context()
            logger.info("Using standard TLS")
        
        # Connect with optional proxy
        extra_args = {}
        if self.use_proxy and self.proxy_url:
            # websockets supports SOCKS5 via proxy parameter
            extra_args['proxy'] = self.proxy_url
            logger.info(f"Using proxy: {self.proxy_url}")
        
        self.websocket = await websockets.connect(
            self.ws_url,
            ssl=ssl_context,
            **extra_args
        )
        
        logger.info("Connected to WebSocket server")
    
    async def send_audio(self, audio_data: bytes):
        """Send audio data to server"""
        if self.websocket:
            message = {
                'type': 'audio',
                'data': audio_data.hex(),
                'format': 'pcm'
            }
            await self.websocket.send(json.dumps(message))
    
    async def send_text(self, text: str):
        """Send text message to server"""
        if self.websocket:
            message = {
                'type': 'text',
                'data': text
            }
            await self.websocket.send(json.dumps(message))
    
    async def receive(self):
        """Receive messages from server"""
        if not self.websocket:
            return None
        
        try:
            message = await self.websocket.recv()
            return json.loads(message)
        except Exception as e:
            logger.error(f"Receive error: {e}")
            return None
    
    async def run(self):
        """Main run loop"""
        self.running = True
        
        try:
            await self.connect()
            
            while self.running:
                # Handle bidirectional communication
                try:
                    message = await asyncio.wait_for(
                        self.receive(),
                        timeout=1.0
                    )
                    
                    if message:
                        await self.handle_message(message)
                    
                except asyncio.TimeoutError:
                    # Send heartbeat or check for local audio
                    pass
                
        except Exception as e:
            logger.error(f"Run error: {e}")
        finally:
            await self.disconnect()
    
    async def handle_message(self, message: Dict[str, Any]):
        """Handle incoming messages"""
        msg_type = message.get('type')
        
        if msg_type == 'text':
            logger.info(f"Received text: {message.get('data', '')[:100]}...")
            # TODO: Handle text response (e.g., TTS)
            
        elif msg_type == 'audio':
            logger.info("Received audio data")
            # TODO: Handle audio response
            
        elif msg_type == 'error':
            logger.error(f"Server error: {message.get('data')}")
            
        else:
            logger.debug(f"Unknown message type: {msg_type}")
    
    async def disconnect(self):
        """Disconnect from server"""
        if self.websocket:
            await self.websocket.close()
            self.websocket = None
            logger.info("Disconnected from server")
    
    def stop(self):
        """Stop the bridge"""
        self.running = False


def load_config(config_path: str) -> Dict[str, Any]:
    """Load configuration from YAML file"""
    with open(config_path, 'r') as f:
        return yaml.safe_load(f)


def main():
    parser = argparse.ArgumentParser(description='XiaoAI AI Bridge')
    parser.add_argument(
        '--config',
        default='/data/etc/ai-bridge/config.yaml',
        help='Configuration file path'
    )
    parser.add_argument(
        '--setup-mtls',
        action='store_true',
        help='Setup mTLS certificates and exit'
    )
    
    args = parser.parse_args()
    
    # Check if config exists
    if not os.path.exists(args.config):
        logger.error(f"Config file not found: {args.config}")
        sys.exit(1)
    
    # Load configuration
    config = load_config(args.config)
    
    # Setup mTLS only
    if args.setup_mtls:
        logger.info("mTLS certificate setup completed by init script")
        return
    
    # Create and run bridge
    bridge = AIBridge(config)
    
    try:
        asyncio.run(bridge.run())
    except KeyboardInterrupt:
        logger.info("Received shutdown signal")
        bridge.stop()
    except Exception as e:
        logger.error(f"Fatal error: {e}")
        sys.exit(1)


if __name__ == '__main__':
    main()
