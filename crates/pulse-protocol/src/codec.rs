//! Codec for encoding and decoding Pulse frames.
//!
//! This module provides MessagePack-based serialization with length-prefixed framing.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

use crate::frames::Frame;

/// Maximum frame size (16 MiB).
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Length prefix size in bytes.
pub const LENGTH_PREFIX_SIZE: usize = 4;

/// Protocol errors that can occur during encoding/decoding.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// Frame exceeds maximum size.
    #[error("Frame size {0} exceeds maximum {MAX_FRAME_SIZE}")]
    FrameTooLarge(usize),

    /// Not enough data to decode frame.
    #[error("Incomplete frame: need {0} more bytes")]
    Incomplete(usize),

    /// MessagePack encoding error.
    #[error("Encoding error: {0}")]
    Encode(#[from] rmp_serde::encode::Error),

    /// MessagePack decoding error.
    #[error("Decoding error: {0}")]
    Decode(#[from] rmp_serde::decode::Error),

    /// Invalid frame data.
    #[error("Invalid frame: {0}")]
    Invalid(String),
}

/// Encode a frame to bytes.
///
/// The encoded format is:
/// - 4 bytes: Big-endian length prefix
/// - N bytes: MessagePack-encoded frame
///
/// # Errors
///
/// Returns an error if the frame is too large or encoding fails.
pub fn encode(frame: &Frame) -> Result<Bytes, ProtocolError> {
    let payload = rmp_serde::to_vec_named(frame)?;

    if payload.len() > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(payload.len()));
    }

    let mut buf = BytesMut::with_capacity(LENGTH_PREFIX_SIZE + payload.len());
    buf.put_u32(payload.len() as u32);
    buf.extend_from_slice(&payload);

    Ok(buf.freeze())
}

/// Encode a frame into an existing buffer.
///
/// # Errors
///
/// Returns an error if the frame is too large or encoding fails.
pub fn encode_into(frame: &Frame, buf: &mut BytesMut) -> Result<(), ProtocolError> {
    let payload = rmp_serde::to_vec_named(frame)?;

    if payload.len() > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(payload.len()));
    }

    buf.reserve(LENGTH_PREFIX_SIZE + payload.len());
    buf.put_u32(payload.len() as u32);
    buf.extend_from_slice(&payload);

    Ok(())
}

/// Decode a frame from bytes.
///
/// # Errors
///
/// Returns an error if the data is incomplete, too large, or invalid.
pub fn decode(data: &[u8]) -> Result<Frame, ProtocolError> {
    if data.len() < LENGTH_PREFIX_SIZE {
        return Err(ProtocolError::Incomplete(LENGTH_PREFIX_SIZE - data.len()));
    }

    let length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;

    if length > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(length));
    }

    let total_size = LENGTH_PREFIX_SIZE + length;
    if data.len() < total_size {
        return Err(ProtocolError::Incomplete(total_size - data.len()));
    }

    let frame = rmp_serde::from_slice(&data[LENGTH_PREFIX_SIZE..total_size])?;
    Ok(frame)
}

/// Try to decode a frame from a buffer, advancing it if successful.
///
/// Returns `Ok(Some(frame))` if a complete frame was decoded,
/// `Ok(None)` if more data is needed, or `Err` on protocol error.
///
/// # Errors
///
/// Returns an error if the frame is too large or invalid.
pub fn decode_from(buf: &mut BytesMut) -> Result<Option<Frame>, ProtocolError> {
    if buf.len() < LENGTH_PREFIX_SIZE {
        return Ok(None);
    }

    let length = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;

    if length > MAX_FRAME_SIZE {
        return Err(ProtocolError::FrameTooLarge(length));
    }

    let total_size = LENGTH_PREFIX_SIZE + length;
    if buf.len() < total_size {
        return Ok(None);
    }

    buf.advance(LENGTH_PREFIX_SIZE);
    let payload = buf.split_to(length);
    let frame = rmp_serde::from_slice(&payload)?;

    Ok(Some(frame))
}

/// Codec for streaming frame encoding/decoding.
#[derive(Debug, Default)]
pub struct FrameCodec {
    // Reserved for future state (e.g., compression context)
}

impl FrameCodec {
    /// Create a new codec instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Encode a frame to bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn encode(&self, frame: &Frame) -> Result<Bytes, ProtocolError> {
        encode(frame)
    }

    /// Decode a frame from bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if decoding fails.
    pub fn decode(&self, data: &[u8]) -> Result<Frame, ProtocolError> {
        decode(data)
    }

    /// Try to decode a frame from a buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame is invalid.
    pub fn decode_from(&self, buf: &mut BytesMut) -> Result<Option<Frame>, ProtocolError> {
        decode_from(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let frames = vec![
            Frame::subscribe(1, "test-channel"),
            Frame::publish("chat:room", b"Hello, world!".to_vec()),
            Frame::ack(42),
            Frame::error(1, 1001, "Invalid frame"),
            Frame::ping(),
            Frame::connect(1, Some("token123".to_string())),
            Frame::connected("conn-123", 1, 30000),
        ];

        for frame in frames {
            let encoded = encode(&frame).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(frame, decoded);
        }
    }

    #[test]
    fn test_decode_incomplete() {
        let frame = Frame::subscribe(1, "test");
        let encoded = encode(&frame).unwrap();

        // Test with partial data
        let partial = &encoded[..5];
        match decode(partial) {
            Err(ProtocolError::Incomplete(_)) => {}
            other => panic!("Expected Incomplete error, got {:?}", other),
        }
    }

    #[test]
    fn test_frame_too_large() {
        // Create a frame that's too large
        let large_payload = vec![0u8; MAX_FRAME_SIZE + 1];
        let frame = Frame::publish("test", large_payload);

        match encode(&frame) {
            Err(ProtocolError::FrameTooLarge(_)) => {}
            other => panic!("Expected FrameTooLarge error, got {:?}", other),
        }
    }

    #[test]
    fn test_streaming_decode() {
        let frame1 = Frame::subscribe(1, "test1");
        let frame2 = Frame::subscribe(2, "test2");

        let mut buf = BytesMut::new();
        encode_into(&frame1, &mut buf).unwrap();
        encode_into(&frame2, &mut buf).unwrap();

        let decoded1 = decode_from(&mut buf).unwrap().unwrap();
        let decoded2 = decode_from(&mut buf).unwrap().unwrap();

        assert_eq!(frame1, decoded1);
        assert_eq!(frame2, decoded2);
        assert!(buf.is_empty());
    }
}
