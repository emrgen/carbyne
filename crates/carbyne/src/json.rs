use crate::{Doc, Type};
use fake::Opt;

/// JsonDoc that can be converted to a Doc.
/// It may not be optimum for many use cases as it might be
/// unnecessary to convert all fields of a json to a CRDT type
pub(crate) struct JsonDoc {
    value: Option<serde_json::Value>,
}

impl JsonDoc {
    pub(crate) fn new(value: serde_json::Value) -> Self {
        JsonDoc { value: Some(value) }
    }

    pub(crate) fn to_doc(mut self) -> Doc {
        let mut doc = Doc::default();
        // take the value out of the option
        let value = self.value.take().unwrap_or_default();
        self.build(Type::from(doc.root.clone()), &value);

        doc
    }

    fn build(&mut self, parent: Type, value: &serde_json::Value) {
        match value {
            serde_json::Value::Bool(b) => {
                // parent.set(key, Type::from(b));
            }
            serde_json::Value::Null => {
                // parent.set(key, Type::from(()));
            }
            serde_json::Value::Number(n) => {
                // parent.set(key, Type::from(n));
            }
            serde_json::Value::String(s) => {
                // parent.set(key, Type::from(s));
            }
            serde_json::Value::Array(arr) => {
                for value in arr.iter() {
                    // self.build(value);
                }
            }
            serde_json::Value::Object(obj) => {
                for (key, value) in obj.iter() {
                    // self.build(value);
                }
            }
        }
    }
}
