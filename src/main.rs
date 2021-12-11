use std::collections::hash_map::OccupiedEntry;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use glob::glob;
use image;
use img_hash::{HasherConfig, Image};

const VALID_EXTENSIONS: [&str; 4] = ["png", "jpeg", "jpg", "gif"];

#[derive(Debug)]
struct ImageProperties {
    path: String,
    file_size: usize,
}

fn find_duplicates(directory: &str) {
    let mut new_dir = directory.to_owned();
    new_dir.push_str(r"\*.*");

    let images: Vec<PathBuf> = glob(&new_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|pb| VALID_EXTENSIONS.contains(&pb.extension().unwrap().to_str().unwrap()))
        .collect();

    let mut hashes: HashMap<String, Vec<ImageProperties>> = HashMap::new();
    let hasher = HasherConfig::new().to_hasher();
    for image in &images {
        let image_obj = image::open(image).unwrap();
        let hash = hasher.hash_image(&image_obj);
        let hash_b64 = hash.to_base64();

        // Get the vector (or make a new one if not added) and add a new ImageProperties to it
        let img_prop = ImageProperties {
            path: image.to_str().unwrap().to_owned(),
            file_size: fs::metadata(image).unwrap().len() as usize,

        };
        // https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html#method.or_default
        hashes.entry(hash_b64).or_default().push(img_prop);
    }

    println!("{:?}", hashes);
}

fn main() {
    find_duplicates(r"C:\Users\Chris\Pictures\Sample");
}
