use std::io::{Read, Write};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use serde::{de::DeserializeOwned, Serialize};

pub fn write_compressed<D: Serialize>(d: &D, f: &str) {
    let mut compressor = ZlibEncoder::new(Vec::new(), Compression::default());
    let data = &bincode::serialize(d).expect("Failed to serialize");
    compressor
        .write_all(data)
        .expect("Failed to write data to compressor");
    let compressed = compressor.finish().expect("Failed to compress");
    std::fs::write(f, compressed).expect("Failed to save");
}

pub fn read_compressed<D: DeserializeOwned>(f: &str) -> Option<D> {
    if let Ok(d) = std::fs::read(f) {
        let mut decompressor = ZlibDecoder::new(&*d);
        let mut data = vec![];
        decompressor
            .read_to_end(&mut data)
            .expect("Could not decoompress");
        let out: D = bincode::deserialize(&data).unwrap();
        Some(out)
    } else {
        None
    }
}
