use miniz_oxide::deflate::compress_to_vec;
use carbyne::codec_v1::EncoderV1;
use carbyne::encoder::{Encode, Encoder};
use carbyne::{print_yaml, sync_docs, CloneDeep, Doc, Type};

fn main() {
    let doc = Doc::default();
    let mut l1: Type = doc.list().into();
    doc.set("list", l1.clone());

    l1.as_list().unwrap().append(doc.atom("a"));

    let doc2 = doc.clone_deep();
    doc2.update_client();
    doc2.get("list").unwrap().as_list().unwrap().prepend(doc2.atom("b"));

    sync_docs(&doc, &doc2, Default::default());

    assert_eq!(doc, doc2);
    print_yaml(&doc);
    print_yaml(&doc2);
}
