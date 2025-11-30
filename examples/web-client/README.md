# Pulse Web Client

A browser-based demo client for testing Pulse server.

## Usage

1. Start the Pulse server:
   ```bash
   cargo run --release -p pulse-server
   ```

2. Open `index.html` in your browser

3. Click **Connect** to establish a WebSocket connection

4. Subscribe to channels and send messages!

## Features

- Connect/disconnect to Pulse server
- Subscribe/unsubscribe to channels
- Publish messages to channels
- View real-time message feed
- Uses MessagePack encoding with length-prefixed framing

## Default Configuration

- Server URL: `ws://127.0.0.1:8080/ws`
- Default channel: `chat:lobby`

