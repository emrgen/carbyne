use crate::bimapid::ClientId;
use crate::{Content, Id, ItemData};
use serde::{Deserialize, Serialize};
use serde_columnar::columnar;
use std::default::Default;

#[columnar(vec, ser, de)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Data {
    #[columnar(strategy = "Rle")]
    id_client: ClientId,
    #[columnar(strategy = "DeltaRle")]
    id_clock: u32,
    #[columnar(strategy = "Rle")]
    parent_id_client: ClientId,
    #[columnar(strategy = "DeltaRle")]
    parent_id_clock: u32,
    #[columnar(strategy = "Rle")]
    left_id_client: ClientId,
    #[columnar(strategy = "DeltaRle")]
    left_id_clock: u32,
    #[columnar(strategy = "Rle")]
    right_id_client: ClientId,
    #[columnar(strategy = "DeltaRle")]
    right_id_clock: u32,
    #[columnar(strategy = "Rle")]
    target_id: ClientId,
    #[columnar(strategy = "DeltaRle")]
    target_id_clock: u32,
    #[columnar(strategy = "Rle")]
    mover_id: ClientId,
    #[columnar(strategy = "DeltaRle")]
    mover_id_clock: u32,
    #[columnar(strategy = "Rle")]
    content: String,
    #[columnar(strategy = "Rle")]
    flags: u8,
    #[columnar(strategy = "Rle")]
    kind_flag: u8,
}

#[columnar(ser, de)]
#[derive(Default, Debug, Clone)]
pub(crate) struct Table {
    #[columnar(class = "vec")]
    pub(crate) data: Vec<Data>,
    // #[columnar(strategy = "Rle")]
    // pub(crate) content: Vec<String>,
}

impl Table {
    pub(crate) fn add(&mut self, item: &ItemData, kind_flag: u8, flag: u8) {
        let mut data = Data {
            id_client: item.id.client,
            id_clock: item.id.clock,
            kind_flag,
            flags: flag,
            ..Default::default()
        };

        if !matches!(item.content, Content::Null) {
            let content = serde_json::to_string(&item.content).unwrap();
            // println!("content: {}", content);
            data.content = content;
            // self.content.push(content);
        }

        if let Some(left_id) = item.left_id {
            data.left_id_client = left_id.client;
            data.left_id_clock = left_id.clock;
        } else if let Some(parent_id) = item.parent_id {
            data.parent_id_client = parent_id.client;
            data.parent_id_clock = parent_id.clock;
        }

        if let Some(right_id) = item.right_id {
            data.right_id_client = right_id.client;
            data.right_id_clock = right_id.clock;
        }

        self.data.push(data);
    }

    pub(crate) fn buffer(&self) -> Vec<u8> {
        let bytes = serde_columnar::to_vec(&self).unwrap();

        println!("size: {}", bytes.len());

        bytes
    }
}

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};
    use serde_columnar::columnar;

    #[columnar(vec, ser, de)]
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    struct Row {
        #[columnar(strategy = "Rle")]
        id: String,
    }

    #[columnar(ser, de)]
    #[derive(Default, Debug, Clone)]
    struct Table {
        #[columnar(class = "vec")]
        data: Vec<Row>,
    }

    #[test]
    fn test_table() {
        let mut table = Table::default();

        for i in (0..6000) {
            let random_char = std::char::from_u32(i).unwrap();
            table.data.push(Row {
                id: random_char.to_string(),
            });
        }

        let bytes = serde_columnar::to_vec(&table).unwrap();
        println!("size: {}", bytes.len());
    }
}
