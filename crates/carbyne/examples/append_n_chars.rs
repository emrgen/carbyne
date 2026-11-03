use miniz_oxide::deflate::compress_to_vec;
use carbyne::codec_v1::EncoderV1;
use carbyne::encoder::{Encode, Encoder};
use carbyne::Doc;
use rand::prelude::SliceRandom;

fn main() {
    let doc = Doc::default();
    let text = doc.text();
    doc.set("text", text.clone());

    let chars = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r",
        "s", "t", "u", "v", "w", "x", "y", "z",
    ];

    let now = std::time::Instant::now();
    let mut vec = Vec::new();
    for i in 0..6000 {
        vec.push(chars[i % 26]);
    }
    vec.shuffle(&mut rand::thread_rng());

    for c in vec {
        text.prepend(doc.string(c));
    }

    println!("elapsed: {:?}", now.elapsed());

    let mut encoder = EncoderV1::new();
    doc.encode(&mut encoder, &mut Default::default());

    let comp = compress_to_vec(&encoder.buffer(), 1);

    println!("Doc size: {}", encoder.buffer().len());
    println!("Compressed size: {}", comp.len());
}
