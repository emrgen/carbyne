use hashbrown::HashMap;

use crate::{ClientState, Diff, DocId};

/// DiffStore stores the doc diff
pub trait DiffStore {
    type Error;

    fn state(&self, doc_id: impl Into<DocId>) -> Result<ClientState, Self::Error>;
    fn get(&self, doc_id: impl Into<DocId>, state: &ClientState) -> Result<Diff, Self::Error>;
    fn put(&mut self, diff: Diff) -> Result<(), Self::Error>;
}

pub(crate) struct InMemoryDiffStore {
    diffs: HashMap<DocId, Diff>,
}

impl InMemoryDiffStore {
    pub fn new() -> Self {
        InMemoryDiffStore {
            diffs: HashMap::new(),
        }
    }
}

// in memory implementation of the DiffStore trait
impl DiffStore for InMemoryDiffStore {
    type Error = ();

    fn state(&self, doc_id: impl Into<DocId>) -> Result<ClientState, Self::Error> {
        let diff = self.diffs.get(&doc_id.into()).unwrap();
        Ok(diff.state.clone())
    }

    fn get(&self, doc_id: impl Into<DocId>, state: &ClientState) -> Result<Diff, Self::Error> {
        let diff = self.diffs.get(&doc_id.into()).unwrap().clone();

        Ok(diff)
    }

    fn put(&mut self, diff: Diff) -> Result<(), Self::Error> {
        let doc_id = diff.doc_id.clone();

        let old_diff = self.diffs.get_mut(&doc_id);
        match old_diff {
            Some(old_diff) => {
                let adjusted = diff.adjust_diff(&old_diff);
                old_diff.merge(&adjusted);
            }
            None => {
                self.diffs.insert(doc_id, diff);
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{print_yaml, Doc};

    use super::*;

    #[test]
    fn test_in_memory_diff_store() {
        let mut store = InMemoryDiffStore::new();

        let doc = Doc::default();
        let text = doc.atom("hello world");
        doc.set("world", text);

        let diff = doc.diff(ClientState::default());
        store.put(diff.clone()).unwrap();
        let diff = store
            .get(&diff.doc_id, &ClientState::default())
            .unwrap()
            .clone();

        let state = doc.state();
        doc.update_client();
        let text = doc.atom("hello earth!");
        doc.set("earth", text);

        let diff2 = doc.diff(&state);
        store.put(diff2).unwrap();

        let diff = store.get(&diff.doc_id, &ClientState::default()).unwrap();

        let doc3 = Doc::from(&diff).unwrap();

        assert_eq!(doc, doc3);
    }

    #[test]
    fn test_save_docs() {
        let mut store = InMemoryDiffStore::new();

        let doc = Doc::default();
        let text = doc.atom("hello world");
        doc.set("world", text);

        let diff = doc.diff(ClientState::default());
        store.put(diff.clone()).unwrap();

        let ds = store.get(&diff.doc_id, &ClientState::default()).unwrap();

        print_yaml(&ds);

        let doc2 = Doc::from(&ds).unwrap();

        doc2.set("day", doc2.atom("good day"));

        print_yaml(doc2);
    }
}
