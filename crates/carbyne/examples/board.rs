use uuid::Error;

use carbyne::{CloneDeep, Doc, equal_docs, print_yaml, sync_docs};
use carbyne::encoder::Encoder;

fn main() -> Result<(), Error> {
    let doc = Doc::default();

    let todo = doc.list();
    doc.set("todo", todo.clone());

    let progress = doc.list();
    doc.set("progress", progress.clone());

    let done = doc.list();
    doc.set("done", done.clone());

    let pair = doc.clone_deep();
    pair.update_client();

    assert!(equal_docs(&doc, &pair));

    todo.append(doc.string("a"));
    pair.get("todo").unwrap().append(pair.string("b"));

    progress.append(doc.string("c"));
    pair.get("progress").unwrap().append(pair.string("d"));

    done.append(doc.string("e"));
    pair.get("done").unwrap().append(pair.string("f"));

    sync_docs(&doc, &pair, Default::default());

    assert_eq!(doc, pair);

    print_yaml(&doc);
    print_yaml(&pair);

    Ok(())
}
