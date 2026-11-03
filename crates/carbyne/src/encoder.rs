use crate::decoder::Decoder;
use crate::item::ItemData;
use crate::store::WeakStoreRef;
use crate::table::Table;

//
pub trait Encoder: Clone {
    fn u8(&mut self, value: u8);
    fn u16(&mut self, value: u16);
    fn u32(&mut self, value: u32);
    fn u64(&mut self, value: u64);
    fn uuid(&mut self, value: &[u8]);
    fn string(&mut self, value: &str);
    fn bytes(&mut self, value: &[u8]);
    fn slice(&mut self, value: &[u8]);
    fn item(&mut self, ctx: &mut EncodeContext, value: &ItemData);
    fn finish(&mut self);
    fn decoder(&mut self) -> Box<dyn Decoder>;
    fn buffer(&self) -> Vec<u8>;
    fn size(&self) -> usize;
}

#[derive(Default, Clone)]
pub struct EncodeContext {
    pub(crate) version: u8,
    pub(crate) store: WeakStoreRef,
    pub(crate) table: Table,
}

impl EncodeContext {
    pub(crate) fn new(version: u8, store: WeakStoreRef) -> EncodeContext {
        EncodeContext {
            version,
            store,
            table: Table::default(),
        }
    }
}

pub trait Encode {
    fn encode<T: Encoder>(&self, e: &mut T, cx: &mut EncodeContext);
}

impl Encode for u8 {
    fn encode<T: Encoder>(&self, e: &mut T, _cx: &mut EncodeContext) {
        e.u8(*self);
    }
}

impl Encode for u16 {
    fn encode<T: Encoder>(&self, e: &mut T, _cx: &mut EncodeContext) {
        e.u16(*self);
    }
}

impl Encode for u32 {
    fn encode<T: Encoder>(&self, e: &mut T, _cx: &mut EncodeContext) {
        e.u32(*self);
    }
}

impl Encode for u64 {
    fn encode<T: Encoder>(&self, e: &mut T, _cx: &mut EncodeContext) {
        e.u64(*self);
    }
}

impl Encode for String {
    fn encode<T: Encoder>(&self, e: &mut T, _cx: &mut EncodeContext) {
        e.string(self);
    }
}

impl Encode for ItemData {
    fn encode<T: Encoder>(&self, e: &mut T, cx: &mut EncodeContext) {
        e.item(cx, self);
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode<E: Encoder>(&self, e: &mut E, ctx: &mut EncodeContext) {
        match self {
            Some(value) => value.encode(e, ctx),
            None => {}
        }
    }
}

impl<T: Encode> Encode for &T {
    fn encode<E: Encoder>(&self, e: &mut E, ctx: &mut EncodeContext) {
        (*self).encode(e, ctx);
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode<E: Encoder>(&self, e: &mut E, ctx: &mut EncodeContext) {
        e.u32(self.len() as u32);
        for value in self {
            value.encode(e, ctx);
        }
    }
}

impl<T: Encode> Encode for [T] {
    fn encode<E: Encoder>(&self, e: &mut E, ctx: &mut EncodeContext) {
        e.u32(self.len() as u32);
        for value in self {
            value.encode(e, ctx);
        }
    }
}

impl<T: Encode> Encode for &mut T {
    fn encode<E: Encoder>(&self, e: &mut E, ctx: &mut EncodeContext) {
        (**self).encode(e, ctx);
    }
}

impl<T: Encode> Encode for Box<T> {
    fn encode<E: Encoder>(&self, e: &mut E, cx: &mut EncodeContext) {
        (**self).encode(e, cx);
    }
}
