use crate::change::Change;
use crate::natom::NAtom;
use crate::nlist::NList;
use crate::nmap::NMap;
use crate::store::WeakStoreRef;
use crate::{ClockTick, Content, Id, NString, NText, Type};
use std::rc::Rc;

// Transaction represents a transaction with the changes made to the Nitro document
// Change represents the changes made to the Nitro document at local/remote client site
// pub(crate) struct Transaction {
//     store: WeakStoreRef,
//     change: Change,
// }
//
// impl Transaction {
//     pub fn new(store: WeakStoreRef) -> Self {
//         let store_ref = store.upgrade().unwrap();
//         let store_ref = store_ref.borrow();
//
//         let client = store_ref.client;
//         let tick = store_ref.current_tick();
//         let change = Change::from_id(Id::new(client, tick).into());
//
//         Self { store, change }
//     }
//
//     pub fn atom(&mut self, content: impl Into<Content>) -> NAtom {
//         let atom = NAtom::new(self.next_id(), content.into(), self.store.clone());
//         self.insert(atom.clone());
//         self.update_tick();
//
//         atom
//     }
//
//     pub fn list(&mut self) -> NList {
//         let list = NList::new(self.next_id(), self.store.clone());
//         self.insert(list.clone());
//         self.update_tick();
//
//         list
//     }
//
//     pub fn map(&mut self) -> NMap {
//         let map = NMap::new(self.next_id(), self.store.clone());
//         self.insert(map.clone());
//         self.update_tick();
//
//         map
//     }
//
//     pub fn text(&mut self) -> NText {
//         let text = NText::new(self.next_id(), self.store.clone());
//         self.insert(text.clone());
//         self.update_tick();
//
//         text
//     }
//
//     pub fn string(&mut self, value: impl Into<String>) -> NString {
//         let content = value.into();
//         let string = NString::new(self.next_id(), content, self.store.clone());
//         self.insert(string.clone());
//         self.update_tick();
//
//         string
//     }
//
//     fn current_tick(&self) -> ClockTick {
//         self.store.upgrade().unwrap().borrow().current_tick()
//     }
//
//     fn update_tick(&mut self) {
//         self.change.id.end = self.change.id.end.max(self.current_tick() - 1);
//     }
//
//     fn next_id(&mut self) -> Id {
//         self.store.upgrade().unwrap().borrow_mut().next_id()
//     }
//
//     fn insert(&mut self, item: impl Into<Type>) {
//         self.store.upgrade().unwrap().borrow_mut().insert(item);
//     }
//
//     /// Commit the transaction to the store.
//     pub fn commit(&mut self) {
//         let store = self.store.upgrade().unwrap();
//         store.borrow_mut().insert_change(self.change.id);
//     }
// }
//
// pub(crate) struct ChangeTx {
//     pub(crate) store: WeakStoreRef,
//     pub(crate) change: Change,
// }
//
// impl ChangeTx {
//     fn new(store: WeakStoreRef, change: Change) -> Self {
//         Self { store, change }
//     }
//
//     // apply the change items to the document store
//     // the items should be applied in the tick order
//     fn commit(&mut self) {
//         let store_ref = self.store.upgrade().unwrap();
//         let mut store = store_ref.borrow_mut();
//     }
//
//     fn rollback(&mut self) {}
// }
