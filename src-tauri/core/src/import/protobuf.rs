//! A tiny, bounded Protocol Buffers wire-format reader.
//!
//! We deliberately do **not** pull in a general protobuf crate + `protoc`
//! codegen. The Google Authenticator migration message is small and fixed, and
//! a hand-written reader lets us bound every allocation and reject malformed or
//! hostile input crisply (truncated varints, lengths past the buffer, oversized
//! payloads). Only the wire types we actually need are supported.

use crate::error::{CoreError, Result};

/// Wire types (only the ones we consume + skip logic for the rest).
pub const WIRE_VARINT: u64 = 0;
pub const WIRE_I64: u64 = 1;
pub const WIRE_LEN: u64 = 2;
pub const WIRE_I32: u64 = 5;

/// A cursor over a protobuf byte buffer.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.buf.len()
    }

    fn err() -> CoreError {
        CoreError::MigrationDecode("malformed protobuf data")
    }

    /// Read a base-128 varint (max 10 bytes / 64 bits).
    pub fn read_varint(&mut self) -> Result<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        for _ in 0..10 {
            let byte = *self.buf.get(self.pos).ok_or_else(Self::err)?;
            self.pos += 1;
            result |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
        Err(Self::err())
    }

    /// Read a field tag, returning `(field_number, wire_type)`.
    pub fn read_tag(&mut self) -> Result<(u64, u64)> {
        let key = self.read_varint()?;
        let field = key >> 3;
        let wire = key & 0x7;
        if field == 0 {
            return Err(Self::err());
        }
        Ok((field, wire))
    }

    /// Read a length-delimited slice, bounds-checked against the buffer.
    pub fn read_bytes(&mut self) -> Result<&'a [u8]> {
        let len = self.read_varint()? as usize;
        let end = self.pos.checked_add(len).ok_or_else(Self::err)?;
        if end > self.buf.len() {
            return Err(Self::err());
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    /// Skip a field of the given wire type (for forward compatibility).
    pub fn skip(&mut self, wire: u64) -> Result<()> {
        match wire {
            WIRE_VARINT => {
                self.read_varint()?;
            }
            WIRE_I64 => self.advance(8)?,
            WIRE_LEN => {
                let len = self.read_varint()? as usize;
                self.advance(len)?;
            }
            WIRE_I32 => self.advance(4)?,
            _ => return Err(Self::err()),
        }
        Ok(())
    }

    fn advance(&mut self, n: usize) -> Result<()> {
        let end = self.pos.checked_add(n).ok_or_else(Self::err)?;
        if end > self.buf.len() {
            return Err(Self::err());
        }
        self.pos = end;
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod encode {
    //! Minimal encoder used only by tests to build **synthetic** fixtures.
    //! Never used in production code.

    use super::*;

    pub fn varint(out: &mut Vec<u8>, mut v: u64) {
        loop {
            let mut byte = (v & 0x7f) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
            }
            out.push(byte);
            if v == 0 {
                break;
            }
        }
    }

    pub fn tag(out: &mut Vec<u8>, field: u64, wire: u64) {
        varint(out, (field << 3) | wire);
    }

    pub fn field_varint(out: &mut Vec<u8>, field: u64, v: u64) {
        tag(out, field, WIRE_VARINT);
        varint(out, v);
    }

    pub fn field_bytes(out: &mut Vec<u8>, field: u64, bytes: &[u8]) {
        tag(out, field, WIRE_LEN);
        varint(out, bytes.len() as u64);
        out.extend_from_slice(bytes);
    }

    #[test]
    fn varint_round_trip() {
        for v in [0u64, 1, 127, 128, 300, 16384, u32::MAX as u64, u64::MAX] {
            let mut buf = Vec::new();
            varint(&mut buf, v);
            let mut r = Reader::new(&buf);
            assert_eq!(r.read_varint().unwrap(), v);
        }
    }

    #[test]
    fn rejects_truncated_varint() {
        // High bit set with no continuation.
        let buf = [0x80u8, 0x80, 0x80];
        let mut r = Reader::new(&buf);
        assert!(r.read_varint().is_err());
    }

    #[test]
    fn rejects_length_past_buffer() {
        let mut buf = Vec::new();
        tag(&mut buf, 1, WIRE_LEN);
        varint(&mut buf, 100); // claims 100 bytes, buffer has none
        let mut r = Reader::new(&buf);
        let (_f, w) = r.read_tag().unwrap();
        assert_eq!(w, WIRE_LEN);
        assert!(r.read_bytes().is_err());
    }
}
