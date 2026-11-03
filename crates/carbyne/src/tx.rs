use hashbrown::HashMap;
use std::cell::Ref;
use std::collections::BTreeMap;
use std::default::Default;
use std::time::Duration;

use crate::bimapid::ClientId;
use crate::crdt_yata::{integrate_yata, remove_yata};
use crate::delete::DeleteItem;
use crate::diff::Diff;
use crate::id::WithId;
use crate::item::{ItemData, ItemRef, Linked, StartEnd};
use crate::print_yaml;
use crate::queue_store::ClientQueueStore;
use crate::store::{
    ClientStore, DocStore, ItemDataStore, PendingStore, ReadyStore, TypeStore, WeakStoreRef,
};
use crate::types::Type;

#[derive(Debug, Clone, Default)]
pub(crate) struct Tx {
    store: WeakStoreRef,
    // TODO: ready, pending, pending_queue will be removed as
    // the Transactions are already serialized as per dependencies
    ready: ReadyStore,
    pending: PendingStore,
    pending_queue: ClientQueueStore<ItemData>,
    // track which types are integrated before commit failure
    progress: Vec<Type>,

    diff: Diff,
    ops: Vec<TxOp>,

    elapsed: Duration,
    rollback: bool,
}

impl Tx {
    pub(crate) fn new(store: WeakStoreRef, diff: Diff) -> Tx {
        Tx {
            store,
            diff,
            ready: ReadyStore::default(),
            pending: PendingStore::default(),
            ops: Vec::default(),
            pending_queue: ClientQueueStore::default(),
            progress: Vec::default(),
            elapsed: Duration::default(),
            rollback: false,
        }
    }

    pub(crate) fn commit(&mut self) {
        // println!("-----------------------------------------------------");
        // println!("items to integrate: {}", self.diff.items.size());

        let now = std::time::Instant::now();
        self.prepare()
            .and_then(|_| {
                // println!("Time taken to prepare: {:?}", now.elapsed());
                let now = std::time::Instant::now();
                self.merge()?;
                // println!("Time taken to merge: {:?}", now.elapsed());
                Ok(())
            })
            .and_then(|_| {
                let now = std::time::Instant::now();
                self.apply()?;
                // println!("Time taken to apply: {:?}", now.elapsed());
                Ok(())
            })
            .unwrap_or_else(|err| {
                log::error!("Tx commit error: {}", err);
                self.rollback();
            });
    }

    /// Prepare the transaction for integration
    pub(crate) fn prepare(&mut self) -> Result<(), String> {
        let store = self.store.upgrade().unwrap();
        let store = store.borrow();

        for (_, store) in self.diff.items.iter() {
            for (_, data) in store.iter() {
                self.pending.insert(data.clone());
            }
        }

        for (_, store) in self.diff.deletes.iter() {
            for (_, data) in store.iter() {
                self.pending.insert_delete(data.clone());
            }
        }

        // if all item dependencies are satisfied put the item in ready store
        // TODO: check if HashMap is better than BTreeMap
        // we should even be able to use a vector (of clients ids) as ClientId is a u32
        let mut stage: BTreeMap<ClientId, ItemData> = BTreeMap::new();

        let mut progress = false;
        let mut count = 0;

        let clients = self.pending.items.clients();
        // take the first pending item for each client and put it in stage
        for client in &clients {
            if let Some(data) = self.pending.pop_first(&client) {
                stage.insert(*client, data.clone());
            }
        }

        // items that have no dependencies are put in lonely store
        let mut lonely: Vec<ItemData> = Vec::new();

        // let now = std::time::Instant::now();
        loop {
            if stage.is_empty() {
                break;
            }

            for client_id in &clients {
                if let Some(item) = stage.get(&client_id) {
                    let id = item.id;

                    // check the state of the item and take action accordingly
                    if self.is_integrated(item, &store) {
                        progress = true;
                    } else if self.is_ready(item, &store) {
                        self.ready.insert(item.clone());
                        progress = true;
                    } else if self.is_lonely(item) {
                        lonely.push(item.clone());
                        progress = true;
                    }

                    if progress {
                        if let Some(data) = self.pending.pop_first(&client_id) {
                            stage.insert(*client_id, data);
                        } else {
                            stage.remove(client_id);
                        }
                        break;
                    }

                    count += 1;
                    if count > 1000000 {
                        println!("Item: {:?}", id);
                        panic!("Infinite loop while collecting client ready items");
                    }
                }
            }

            // if no progress is made, break the loop
            if !progress {
                break;
            }

            progress = false;
        }

        // println!("Time taken: {:?}", now.elapsed());

        // remaining items has unmet dependencies and are put in pending store
        // they will again try to integrate in the next iteration of prepare
        for (_, data) in stage {
            // self.pending.insert(data.clone());
            self.pending.insert(data.clone());
        }

        for alone in lonely {
            self.pending.insert(alone);
        }

        // now that all ready items are collected, collect the ready delete items
        for (_, store) in self.pending.iter_delete_items() {
            for (_, data) in store.iter() {
                // FIXME: if the the target item is split or merged,
                // the delete item should be split or merged before integration
                let id = data.range().id();
                if self.ready.contains(&id) || store.contains(&id) {
                    self.ready.insert_delete(data.clone());
                }
            }
        }

        Ok(())
    }

    /// Apply the transaction to the store
    pub(crate) fn apply(&mut self) -> Result<(), String> {
        // println!("[items ready to integrate: {}]", self.ready.queue.len());

        // let fields = self.store.upgrade().unwrap().borrow().fields.clone();

        // self.store.upgrade().unwrap().borrow_mut().fields = fields.as_per(&self.diff.fields);

        let now = std::time::Instant::now();
        let mut times: Vec<Duration> = Vec::new();
        let client_map = self.store.upgrade().unwrap().borrow().state.clients.clone();
        let store = self.store.upgrade().unwrap();
        let mut store = store.borrow_mut();

        while let Some(data) = self.ready.queue.pop_front() {
            let parent = {
                if let Some(parent_id) = &data.parent_id {
                    store.find(parent_id)
                } else if let Some(left_id) = &data.left_id {
                    store.find(left_id).and_then(|item| item.parent())
                } else if let Some(right_id) = &data.right_id {
                    store.find(right_id).and_then(|item| item.parent())
                } else {
                    None
                }
            };

            let now = std::time::Instant::now();
            if let Some(parent) = parent {
                let mut left = data.left_id.as_ref().map(|id| store.find(id)).flatten();
                let right = data.right_id.as_ref().map(|id| store.find(id)).flatten();

                // println!("integrating: {:?}", data.id);

                let item: Type = ItemRef::new(data.into(), self.store.clone()).into();

                let count = integrate_yata(
                    &client_map,
                    &item,
                    &parent,
                    parent.start(),
                    &mut left,
                    right,
                    |start| parent.set_start(start),
                    |end| parent.set_end(end),
                )?;

                parent.on_insert(&item);
                store.insert(item.clone());

                // track integration progress
                self.progress.push(item);

                // println!("integrated with count: {}", count);
            }

            times.push(now.elapsed());
        }

        // println!("Time taken to integrate: {:?}", now.elapsed());
        if times.is_empty() {
            return Ok(());
        }

        // println!(
        //     "Average time taken to integrate: {:?}",
        //     times.iter().sum::<Duration>() / times.len() as u32
        // );
        // println!(
        //     "Average count of items integrated: {}",
        //     counters.iter().sum::<i32>() / counters.len() as i32
        // );

        Ok(())
    }

    pub(crate) fn merge(&self) -> Result<(), String> {
        if let Some(store) = self.store.upgrade() {
            let mut store = store.borrow_mut();

            // store.fields.extend(&self.diff.fields);
            // store.state.clients.extend(&self.diff.state.clients);
            store.pending.extend(&self.pending);
            // self.pending_queue.items.iter().for_each(|(client, queue)| {
            //     for item in queue.iter() {
            //         store.pending.insert(item.clone());
            //     }
            // });
        }
        Ok(())
    }

    pub(crate) fn rollback(&mut self) {
        println!("-----------------------------------------------------");
        println!("|            Rolling back transaction               |");
        println!("-----------------------------------------------------");
        let store = self.store.upgrade().unwrap();
        let mut store = store.borrow_mut();
        for item in self.progress.iter().rev() {
            remove_yata(item);
            store.remove(&item.id());
        }

        // keep the items in progress for debugging
    }

    pub(crate) fn is_ready(&self, data: &ItemData, store: &Ref<DocStore>) -> bool {
        if data.is_root() {
            return true;
        }

        if let Some(parent_id) = data.parent_id {
            if !(self.ready.contains(&parent_id) || store.contains(&parent_id)) {
                return false;
            }
        }

        if let Some(left_id) = data.left_id {
            // println!("left");
            if !(self.ready.contains(&left_id) || store.contains(&left_id)) {
                return false;
            }
        }

        if let Some(right_id) = data.right_id {
            // println!("right");
            if !(self.ready.contains(&right_id) || store.contains(&right_id)) {
                return false;
            }
        }

        true
    }

    // check if the item is an orphan, i.e. it has no parent, left or right siblings
    pub(crate) fn is_lonely(&self, data: &ItemData) -> bool {
        if data.is_root() {
            return false;
        }

        data.parent_id.is_none() && data.left_id.is_none() && data.right_id.is_none()
    }

    pub(crate) fn is_integrated(&self, data: &ItemData, store: &Ref<DocStore>) -> bool {
        store.contains(&data.id)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) enum TxOp {
    Insert(ItemData),
    Delete(DeleteItem),
    Split(ItemData, (ItemData, ItemData)),
    #[default]
    None,
}
