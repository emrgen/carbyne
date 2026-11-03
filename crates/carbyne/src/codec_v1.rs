use std::ops::Deref;

use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::Id;
use crate::item::{Content, ItemData, ItemKind, ItemKindFlags, ItemSide, ItemSideFlags};

const VERSION: u8 = 1;
const BUF_STEP: usize = 1024;
const INIT_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct EncoderV1 {
    buf: Vec<u8>,
    pos: usize,
}

impl Default for EncoderV1 {
    fn default() -> Self {
        Self::with_capacity(INIT_SIZE)
    }
}

impl EncoderV1 {
    pub fn new() -> Self {
        Self::with_capacity(INIT_SIZE)
    }

    pub(crate) fn with_capacity(size: usize) -> Self {
        Self {
            buf: Vec::with_capacity(size),
            pos: 0,
        }
        .write_header()
    }

    fn ensure_capacity(&mut self, size: usize) {
        // println!("size: {}, pos: {}, len: {}", size, self.pos, self.buf.len());
        if self.buf.len() + size > self.buf.capacity() {
            self.buf.reserve(BUF_STEP);
        }
    }

    fn write_header(mut self) -> Self {
        self.u8(VERSION);

        self
    }
}

impl Encoder for EncoderV1 {
    fn u8(&mut self, value: u8) {
        self.ensure_capacity(1);
        self.buf.push(value);
        self.pos += 1;
    }

    fn u16(&mut self, value: u16) {
        self.ensure_capacity(2);
        self.buf.extend_from_slice(&value.to_be_bytes());
        self.pos += 2;
    }

    fn u32(&mut self, value: u32) {
        self.ensure_capacity(4);
        self.buf.extend_from_slice(&value.to_be_bytes());
        self.pos += 4;
    }

    fn u64(&mut self, value: u64) {
        self.ensure_capacity(8);
        self.buf.extend_from_slice(&value.to_be_bytes());
        self.pos += 8;
    }

    fn uuid(&mut self, value: &[u8]) {
        let mut array = [0; 16];
        array.copy_from_slice(value);

        self.ensure_capacity(16);
        self.buf.extend_from_slice(&array);
        self.pos += 16;
    }

    fn string(&mut self, value: &str) {
        self.u32(value.len() as u32);
        self.ensure_capacity(value.len());
        self.buf.extend_from_slice(value.as_bytes());
        self.pos += value.len();
    }

    fn bytes(&mut self, value: &[u8]) {
        self.ensure_capacity(value.len() + 4);
        self.u32(value.len() as u32);
        self.buf.extend_from_slice(value);
        self.pos += value.len();
    }

    fn slice(&mut self, value: &[u8]) {
        self.buf.extend_from_slice(value);
        self.pos += value.len();
    }

    fn item(&mut self, cx: &mut EncodeContext, value: &ItemData) {
        encode_item(self, cx, value);
    }

    fn finish(&mut self) {
        self.buf.shrink_to_fit();
    }

    fn decoder(&mut self) -> Box<dyn Decoder> {
        self.finish();
        Box::new(DecoderV1::new(self.buf.clone()))
    }

    fn buffer(&self) -> Vec<u8> {
        self.buf.clone()
    }

    fn size(&self) -> usize {
        self.buf.len()
    }
}

impl Deref for EncoderV1 {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

pub struct DecoderV1 {
    buf: Vec<u8>,
    pos: usize,
}

impl DecoderV1 {
    pub fn new(buf: Vec<u8>) -> Self {
        let mut d = Self { buf, pos: 0 };

        if d.u8().unwrap() != VERSION {
            panic!("decoder: invalid version");
        }

        // println!("buffer: {:?}", d.buf);

        d
    }

    fn ensure_capacity(&mut self, size: usize) {
        // println!("size: {}, pos: {}, len: {}", size, self.pos, self.buf.len());
        if self.pos + size > self.buf.len() {
            panic!("decoder: out of bounds");
        }
    }
}

impl Decoder for DecoderV1 {
    fn u8(&mut self) -> Result<u8, String> {
        self.ensure_capacity(1);
        let value = self.buf[self.pos];
        self.pos += 1;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, String> {
        self.ensure_capacity(2);
        let value = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
        self.pos += 2;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, String> {
        self.ensure_capacity(4);
        let value = u32::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(value)
    }

    fn u64(&mut self) -> Result<u64, String> {
        self.ensure_capacity(8);
        let value = u64::from_be_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
            self.buf[self.pos + 4],
            self.buf[self.pos + 5],
            self.buf[self.pos + 6],
            self.buf[self.pos + 7],
        ]);
        self.pos += 8;
        Ok(value)
    }

    fn uuid(&mut self) -> Result<[u8; 16], String> {
        self.ensure_capacity(16);
        let mut value = [0; 16];
        value.copy_from_slice(&self.buf[self.pos..self.pos + 16]);
        self.pos += 16;
        Ok(value)
    }

    fn string(&mut self) -> Result<String, String> {
        let len = self.u32()? as usize;
        self.ensure_capacity(len);
        let value = String::from_utf8(self.buf[self.pos..self.pos + len].to_vec())
            .map_err(|_| "decoder: invalid utf8 string".to_string())?;
        self.pos += len;
        Ok(value)
    }

    fn bytes(&mut self) -> Result<Vec<u8>, String> {
        let len = self.u32()? as usize;
        self.ensure_capacity(len);
        let value = self.buf[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(value)
    }

    fn slice(&mut self, len: usize) -> Result<&[u8], String> {
        self.ensure_capacity(len);
        let value = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok(value)
    }

    fn item(&mut self, ctx: &DecodeContext) -> Result<ItemData, String> {
        decode_item(self, ctx)
    }
}

fn encode_item(e: &mut EncoderV1, cx: &mut EncodeContext, value: &ItemData) {
    // | kind, content, field, parent | left, right | ...
    // println!("encode_item: {}, {:?}", value.kind, value.id);
    let kind_flags = ItemKindFlags::from(&value.kind).bits();
    e.u8(kind_flags);

    let mut flags = ItemSideFlags::from(&value.side).bits() << 4;

    // let is_root = matches!(value.content, Content::Doc(_));
    if !matches!(value.content, Content::Null) {
        flags |= 1 << 3; // has content
    }

    if value.field.is_some() {
        flags |= 1 << 2; // has field
    }

    // if left_id is not None then we can get the parent_id from left item during integration,
    // so we don't need to store parent_id in the item
    if value.left_id.is_some() {
        flags |= 1 << 1;
    }

    if value.right_id.is_some() {
        flags |= 1;
    }

    e.u8(flags);

    // println!("flags: {:b}", flags);

    if !matches!(value.content, Content::Null) {
        value.content.encode(e, cx);
    }

    if let Some(field) = value.field {
        e.u32(field);
    }

    value.id.encode(e, cx);

    if let Some(left_id) = value.left_id {
        left_id.encode(e, cx);
    }

    if let Some(right_id) = value.right_id {
        right_id.encode(e, cx);
    }

    // if side is none, that means we are using yata
    // else we are using fugue
    if value.side.is_none() && value.left_id.is_none() {
        if let Some(parent_id) = value.parent_id {
            parent_id.encode(e, cx);
        }
    }

    // cx.table.add(value, kind_flags, flags);
}

fn decode_item(d: &mut DecoderV1, ctx: &DecodeContext) -> Result<ItemData, String> {
    let kind_flag = d.u8()?;
    // println!("flags: {:b}", flags);

    let kind: ItemKind = ItemKindFlags::from_bits(kind_flag).unwrap().into();

    let flags = d.u8()?;

    let side: ItemSide = ItemSideFlags::from_bits(flags >> 4).unwrap().into();

    let content = if flags & 0b1000 != 0 {
        Content::decode(d, ctx)?
    } else {
        Content::Null
    };

    let field = if flags & 0b100 != 0 {
        Some(d.u32()?)
    } else {
        None
    };

    let id = Id::decode(d, ctx)?;

    let is_root = matches!(content, Content::Doc(_));
    let mut left_id = None;
    let mut right_id = None;
    let mut parent_id = None;
    let mut target_id = None;
    let mut mover_id = None;

    if !is_root && flags & 0b10 != 0 {
        left_id = Some(Id::decode(d, ctx)?)
    }

    if flags & 0b1 != 0 {
        right_id = Some(Id::decode(d, ctx)?)
    }

    if side.is_none() && !is_root && flags & 0b10 == 0 {
        parent_id = Some(Id::decode(d, ctx)?)
    }

    if flags & 0b1 != 0 {
        target_id = Some(Id::decode(d, ctx)?)
    }

    if flags & 0b1 != 0 {
        mover_id = Some(Id::decode(d, ctx)?)
    }

    // println!("id: {:?}, field: {:?}", id, field);

    Ok(ItemData {
        id,
        kind,
        content,
        field,
        side,
        left_id,
        parent_id,
        right_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_v1() {
        let mut encoder = EncoderV1::with_capacity(0);
        encoder.u8(1);
        encoder.u32(2);
        encoder.u64(3);
        encoder.string("hello");
        encoder.bytes(&[1, 2, 3, 4]);

        let buf = encoder.buffer();
        let mut decoder = DecoderV1::new(buf);
        assert_eq!(decoder.u8().unwrap(), 1);
        assert_eq!(decoder.u32().unwrap(), 2);
        assert_eq!(decoder.u64().unwrap(), 3);
        assert_eq!(decoder.string().unwrap(), "hello");
        assert_eq!(decoder.bytes().unwrap(), vec![1, 2, 3, 4]);
    }
}
