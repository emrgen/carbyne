use miniz_oxide::deflate::compress_to_vec;
use carbyne::codec_v1::EncoderV1;
use carbyne::encoder::{Encode, Encoder};
use carbyne::{Doc, Type};

fn main() {
    let doc = Doc::default();
    let mut l1: Type = doc.list().into();
    doc.set("list1", l1.clone());

    // 26 letters
    let chars = vec![
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
        "s", "t", "u", "v", "w", "x", "y", "z",
    ];

    let mut size = 1;

    for _ in 0..230 {
        for c in &chars {
            let item = doc.atom(size as u32);
            // random index
            let index = rand::random::<usize>() % size;

            l1.insert(index as u32, item);
            // l1.append(item);
            // l1.prepend(item);
            size += 1;
        }
    }

    let now = std::time::Instant::now();

    let mut encoder = EncoderV1::new();
    doc.encode(&mut encoder, &mut Default::default());

    let comp = compress_to_vec(&encoder.buffer(), 1);
    println!("first doc: {:?}", now.elapsed());

    println!("Doc size: {}", encoder.buffer().len());
    println!("Compressed size: {}", comp.len());

    println!("{:?}", l1.size());

    // print_yaml(l1.content());
}
