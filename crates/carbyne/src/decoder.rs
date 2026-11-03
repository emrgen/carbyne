use crate::item::ItemData;

pub trait Decoder {
    fn u8(&mut self) -> Result<u8, String>;
    fn u16(&mut self) -> Result<u16, String>;
    fn u32(&mut self) -> Result<u32, String>;
    fn u64(&mut self) -> Result<u64, String>;
    fn uuid(&mut self) -> Result<[u8; 16], String>;
    fn string(&mut self) -> Result<String, String>;
    fn bytes(&mut self) -> Result<Vec<u8>, String>;
    fn slice(&mut self, len: usize) -> Result<&[u8], String>;
    fn item(&mut self, ctx: &DecodeContext) -> Result<ItemData, String>;
}

impl Decoder for Box<dyn Decoder> {
    fn u8(&mut self) -> Result<u8, String> {
        self.as_mut().u8()
    }

    fn u16(&mut self) -> Result<u16, String> {
        self.as_mut().u16()
    }

    fn u32(&mut self) -> Result<u32, String> {
        self.as_mut().u32()
    }

    fn u64(&mut self) -> Result<u64, String> {
        self.as_mut().u64()
    }

    fn uuid(&mut self) -> Result<[u8; 16], String> {
        self.as_mut().uuid()
    }

    fn string(&mut self) -> Result<String, String> {
        self.as_mut().string()
    }

    fn bytes(&mut self) -> Result<Vec<u8>, String> {
        self.as_mut().bytes()
    }

    fn slice(&mut self, len: usize) -> Result<&[u8], String> {
        self.as_mut().slice(len)
    }

    fn item(&mut self, ctx: &DecodeContext) -> Result<ItemData, String> {
        self.as_mut().item(ctx)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DecodeContext {
    pub(crate) version: u8,
}

pub trait Decode {
    fn decode<T: Decoder>(d: &mut T, ctx: &DecodeContext) -> Result<Self, String>
    where
        Self: Sized;
}

impl Decode for u8 {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<u8, String> {
        d.u8()
    }
}

impl Decode for u16 {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<u16, String> {
        d.u16()
    }
}

impl Decode for u32 {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<u32, String> {
        d.u32()
    }
}

impl Decode for u64 {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<u64, String> {
        d.u64()
    }
}

impl Decode for String {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<String, String> {
        d.string()
    }
}

impl Decode for ItemData {
    fn decode<T: Decoder>(d: &mut T, ctx: &DecodeContext) -> Result<ItemData, String> {
        d.item(ctx)
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode<D: Decoder>(d: &mut D, ctx: &DecodeContext) -> Result<Vec<T>, String> {
        let len = d.u32()? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(T::decode(d, ctx)?);
        }
        Ok(vec)
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode<D: Decoder>(d: &mut D, ctx: &DecodeContext) -> Result<Option<T>, String> {
        let has_value = d.u8()? != 0;
        if has_value {
            Ok(Some(T::decode(d, ctx)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: Decode> Decode for Box<T> {
    fn decode<D: Decoder>(d: &mut D, ctx: &DecodeContext) -> Result<Box<T>, String> {
        Ok(Box::new(T::decode(d, ctx)?))
    }
}

impl Decode for bool {
    fn decode<T: Decoder>(d: &mut T, _ctx: &DecodeContext) -> Result<bool, String> {
        Ok(d.u8()? != 0)
    }
}
