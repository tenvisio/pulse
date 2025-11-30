# Pulse Wire Protocol Specification

**Version**: 1.0  
**Status**: Draft  
**Last Updated**: 2024

## Overview

The Pulse protocol is a binary protocol designed for high-performance realtime communication. It uses MessagePack for serialization and supports multiple transport layers (WebSocket, WebTransport).

## Design Goals

1. **Performance**: Minimize serialization overhead and memory allocations
2. **Simplicity**: Easy to implement in any language
3. **Extensibility**: Support for future frame types and features
4. **Reliability**: Built-in acknowledgment and error handling

## Frame Format

All frames follow a length-prefixed format:

```
┌─────────────────┬────────────────────────────────────┐
│  Length (4B)    │  MessagePack Payload (variable)    │
│  Big Endian     │                                    │
└─────────────────┴────────────────────────────────────┘
```

- **Length**: 4-byte big-endian unsigned integer representing the payload length
- **Payload**: MessagePack-encoded frame data

Maximum frame size: 16 MiB (16,777,216 bytes)

## Frame Types

Each frame is a MessagePack map with a required `type` field:

| Type ID | Name        | Direction      | Description                    |
|---------|-------------|----------------|--------------------------------|
| 0x01    | Subscribe   | Client → Server| Subscribe to a channel         |
| 0x02    | Unsubscribe | Client → Server| Unsubscribe from a channel     |
| 0x03    | Publish     | Bidirectional  | Send message to channel        |
| 0x04    | Presence    | Bidirectional  | Presence update                |
| 0x05    | Ack         | Server → Client| Acknowledgment                 |
| 0x06    | Error       | Server → Client| Error response                 |
| 0x07    | Ping        | Bidirectional  | Keepalive ping                 |
| 0x08    | Pong        | Bidirectional  | Keepalive pong                 |
| 0x09    | Connect     | Client → Server| Initial connection handshake   |
| 0x0A    | Connected   | Server → Client| Connection established         |

### Subscribe (0x01)

Subscribe to receive messages on a channel.

```javascript
{
  "type": 0x01,
  "id": <uint64>,        // Request ID for acknowledgment
  "channel": <string>    // Channel name (max 256 bytes)
}
```

### Unsubscribe (0x02)

Stop receiving messages on a channel.

```javascript
{
  "type": 0x02,
  "id": <uint64>,        // Request ID for acknowledgment
  "channel": <string>    // Channel name
}
```

### Publish (0x03)

Send a message to a channel.

```javascript
{
  "type": 0x03,
  "id": <uint64>,        // Request ID (optional, for ack)
  "channel": <string>,   // Target channel
  "event": <string>,     // Event name (optional)
  "payload": <binary>    // Message payload (MessagePack or raw bytes)
}
```

### Presence (0x04)

Announce or query presence state.

```javascript
{
  "type": 0x04,
  "id": <uint64>,
  "channel": <string>,
  "action": <uint8>,     // 0=join, 1=leave, 2=update, 3=sync
  "data": <map>          // Presence metadata (optional)
}
```

Presence Actions:
- `0` (Join): Client joined the channel
- `1` (Leave): Client left the channel  
- `2` (Update): Client updated their presence data
- `3` (Sync): Server sending full presence state

### Ack (0x05)

Server acknowledgment of a client request.

```javascript
{
  "type": 0x05,
  "id": <uint64>         // ID of the acknowledged request
}
```

### Error (0x06)

Error response from server.

```javascript
{
  "type": 0x06,
  "id": <uint64>,        // ID of the failed request (0 if N/A)
  "code": <uint16>,      // Error code
  "message": <string>    // Human-readable error message
}
```

### Ping (0x07)

Keepalive ping (either direction).

```javascript
{
  "type": 0x07,
  "timestamp": <uint64>  // Unix timestamp in milliseconds (optional)
}
```

### Pong (0x08)

Keepalive pong response.

```javascript
{
  "type": 0x08,
  "timestamp": <uint64>  // Echoed timestamp from ping (optional)
}
```

### Connect (0x09)

Initial connection handshake from client.

```javascript
{
  "type": 0x09,
  "version": <uint8>,    // Protocol version (currently 1)
  "token": <string>      // Authentication token (optional)
}
```

### Connected (0x0A)

Server response to successful connection.

```javascript
{
  "type": 0x0A,
  "connection_id": <string>,  // Unique connection identifier
  "version": <uint8>,         // Negotiated protocol version
  "heartbeat": <uint32>       // Recommended heartbeat interval (ms)
}
```

## Error Codes

| Code   | Name                  | Description                              |
|--------|-----------------------|------------------------------------------|
| 1000   | UnknownError          | An unknown error occurred                |
| 1001   | InvalidFrame          | Malformed or invalid frame               |
| 1002   | InvalidChannel        | Invalid channel name                     |
| 1003   | Unauthorized          | Authentication required or failed        |
| 1004   | Forbidden             | Permission denied for operation          |
| 1005   | ChannelNotFound       | Channel does not exist                   |
| 1006   | RateLimited           | Too many requests                        |
| 1007   | PayloadTooLarge       | Message exceeds size limit               |
| 1008   | NotSubscribed         | Not subscribed to channel                |
| 1009   | AlreadySubscribed     | Already subscribed to channel            |
| 1010   | ConnectionClosed      | Connection is closing                    |
| 1011   | ServerError           | Internal server error                    |
| 1012   | ProtocolMismatch      | Protocol version not supported           |

## Connection Lifecycle

### 1. Connection Establishment

```
Client                          Server
   |                               |
   |-------- [Connect] ----------->|
   |                               |
   |<------- [Connected] ----------|
   |                               |
```

### 2. Subscribe to Channels

```
Client                          Server
   |                               |
   |-------- [Subscribe] --------->|
   |          id: 1                |
   |          channel: "chat:room" |
   |                               |
   |<---------- [Ack] -------------|
   |            id: 1              |
   |                               |
```

### 3. Publish Messages

```
Client A                        Server                        Client B
   |                               |                               |
   |-------- [Publish] ----------->|                               |
   |          channel: "chat:room" |                               |
   |          payload: "Hello"     |                               |
   |                               |-------- [Publish] ----------->|
   |                               |          channel: "chat:room" |
   |                               |          payload: "Hello"     |
   |                               |                               |
```

### 4. Heartbeat

Both client and server should send Ping frames periodically:

```
Client                          Server
   |                               |
   |-------- [Ping] -------------->|
   |          timestamp: 123456    |
   |                               |
   |<--------- [Pong] -------------|
   |           timestamp: 123456   |
   |                               |
```

Recommended heartbeat interval: 30 seconds  
Connection timeout: 60 seconds without activity

### 5. Graceful Disconnect

Close the transport connection. No explicit disconnect frame is needed.

## Channel Names

Channel names must:
- Be 1-256 bytes in length
- Contain only ASCII printable characters (0x20-0x7E)
- Not start with `$` (reserved for system channels)

Recommended conventions:
- Use `:` as namespace separator (e.g., `chat:room:123`)
- Use `private:` prefix for authenticated channels
- Use `presence:` prefix for presence-enabled channels

## Security Considerations

### Authentication

1. Clients should provide an authentication token in the Connect frame
2. Servers should validate tokens before allowing channel subscriptions
3. Tokens should be short-lived and signed (e.g., JWT)

### Channel Authorization

1. Implement server-side authorization for channel access
2. Use channel name prefixes to enforce policies (e.g., `private:*`)
3. Reject unauthorized Subscribe requests with error code 1004

### Transport Security

1. Always use TLS in production (WSS, HTTPS)
2. Validate server certificates
3. Consider certificate pinning for mobile clients

## Implementation Notes

### MessagePack

This protocol uses MessagePack for serialization. Key points:

- Use binary format (not JSON compatibility mode)
- Strings are UTF-8 encoded
- Binary data uses the bin format family
- Maps use string keys

### Flow Control

1. Clients should implement backpressure when receiving messages
2. Servers should buffer messages for slow clients (with limits)
3. Consider implementing per-channel and per-connection rate limits

### Ordering

1. Messages within a single channel are ordered
2. Messages across different channels may be interleaved
3. Acknowledgments may arrive out of order

## Version History

| Version | Date       | Changes                           |
|---------|------------|-----------------------------------|
| 1.0     | 2024       | Initial specification             |

## References

- [MessagePack Specification](https://msgpack.org/index.html)
- [WebSocket Protocol (RFC 6455)](https://tools.ietf.org/html/rfc6455)
- [WebTransport](https://www.w3.org/TR/webtransport/)





