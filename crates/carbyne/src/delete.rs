use serde::ser::SerializeStruct;
use serde::Serialize;
use std::ops::{Range, RangeInclusive};

use crate::bimapid::ClientMap;
use crate::decoder::{Decode, DecodeContext, Decoder};
use crate::encoder::{Encode, EncodeContext, Encoder};
use crate::id::{Id, IdRange, WithId};
use crate::ClockTick;

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub(crate) struct DeleteItem {
    id: Id,
    range: IdRange,
}

impl DeleteItem {
    pub(crate) fn new(id: Id, range: impl Into<IdRange>) -> DeleteItem {
        DeleteItem {
            id,
            range: range.into(),
        }
    }

    pub(crate) fn range(&self) -> &IdRange {
        &self.range
    }

    pub(crate) fn target(&self) -> Id {
        self.range.id()
    }

    pub(crate) fn adjust(&self, before: &ClientMap, after: &ClientMap) -> DeleteItem {
        let mut adjust = self.clone();

        adjust.id = self.id.adjust(before, after);
        adjust.range = self.range.adjust(before, after);

        adjust
    }
}

impl Serialize for DeleteItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_struct("DeleteItem", 2)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("range", &self.range)?;
        state.end()
    }
}

impl Encode for DeleteItem {
    fn encode<E: Encoder>(&self, e: &mut E, ctx: &mut EncodeContext) {
        self.id.encode(e, ctx);
        self.range.encode(e, ctx);
    }
}

impl Decode for DeleteItem {
    fn decode<D: Decoder>(d: &mut D, ctx: &DecodeContext) -> Result<DeleteItem, String> {
        let id = Id::decode(d, ctx)?;
        let range = IdRange::decode(d, ctx)?;

        Ok(DeleteItem::new(id, range))
    }
}

impl WithId for DeleteItem {
    #[inline]
    fn id(&self) -> Id {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use crate::codec_v1::EncoderV1;

    use super::*;

    #[test]
    fn test_encode_decode_delete_items() {
        let d1 = DeleteItem::new(Id::new(1, 1), IdRange::new(1, 10, 11));
        let d2 = DeleteItem::new(Id::new(2, 2), IdRange::new(2, 20, 21));
        let d3 = DeleteItem::new(Id::new(3, 3), IdRange::new(3, 30, 31));

        let mut e = EncoderV1::new();
        d1.encode(&mut e, &mut EncodeContext::default());
        d2.encode(&mut e, &mut EncodeContext::default());
        d3.encode(&mut e, &mut EncodeContext::default());

        let mut d = e.decoder();

        let dd1 = DeleteItem::decode(&mut d, &DecodeContext::default()).unwrap();
        let dd2 = DeleteItem::decode(&mut d, &DecodeContext::default()).unwrap();
        let dd3 = DeleteItem::decode(&mut d, &DecodeContext::default()).unwrap();

        assert_eq!(d1, dd1);
        assert_eq!(d2, dd2);
        assert_eq!(d3, dd3);
    }
}
