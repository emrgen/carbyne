#![feature(slice_take)]
#![feature(bstr)]
// allow warnings for now to make it easier to work on this
#![allow(warnings)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_assignments)]
#![allow(unused_must_use)]
#![allow(clippy::derived_hash_with_manual_eq)]

pub use crate::change::*;
pub use crate::diff::*;
pub use crate::diffstore::*;
pub use crate::doc::*;
pub use crate::id::*;
pub use crate::item::*;
pub use crate::nstring::*;
pub use crate::ntext::*;
pub use crate::richtext::*;
pub use crate::state::*;
pub use crate::sync::*;
pub use crate::types::*;
pub use crate::utils::*;

use crate::index::*;

mod bimap;
mod bimapid;
mod change;
mod change_btree;
mod change_list;
mod change_store;
pub mod codec_v1;
mod crdt_fugue;
mod crdt_yata;
mod cycle;
mod dag;
pub mod decoder;
mod delete;
mod diff;
pub mod diffstore;
mod doc;
pub mod encoder;
mod frontier;
mod hash;
mod id;
mod id_store;
mod index;
mod index_map;
mod index_tree;
mod inner_type;
mod item;
mod json;
mod mark;
mod natom;
mod nlist;
mod nmap;
mod nmark;
mod nmove;
mod nstring;
mod ntext;
mod ntree;
mod persist;
mod queue_store;
mod richtext;
mod state;
mod store;
mod sync;
mod table;
mod transaction;
mod tx;
mod types;
mod undo_redo;
mod utils;
mod version;
