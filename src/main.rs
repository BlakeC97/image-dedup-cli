use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use glob::glob;
use image::imageops::FilterType;
use img_hash::{HashAlg, HasherConfig};

const VALID_EXTENSIONS: [&str; 4] = ["png", "jpeg", "jpg", "gif"];

#[derive(Debug)]
struct ImageProperties {
    path: PathBuf,
    file_size: u64,
}

fn find_duplicates(directory: &str) {
    let owned_dir = directory.to_owned();

    let images: Vec<PathBuf> = glob(&(owned_dir.clone() + r"/*.*"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|pb| VALID_EXTENSIONS.contains(&pb.extension().unwrap().to_str().unwrap()))
        .collect();

    let mut hashes: HashMap<u64, Vec<ImageProperties>> = HashMap::new();
    let hasher = HasherConfig::with_bytes_type::<[u8; 8]>()
        .hash_alg(HashAlg::Gradient)
        .resize_filter(FilterType::Triangle)
        .to_hasher();
    for image in &images {
        let image_obj = image::open(image).unwrap();
        let hash = hasher.hash_image(&image_obj);
        let mut hash_num: u64 = 0;
        for (idx, byte) in hash.as_bytes().iter().enumerate() {
            hash_num |= (*byte as u64) << (8 * idx);
        }

        // Get the vector (or make a new one if not added) and add a new ImageProperties to it
        let img_prop = ImageProperties {
            path: image.clone(),
            file_size: fs::metadata(image).unwrap().len(),
        };

        // Gets an Entry from the hash map, which is an enum that can be: OccupiedEntry, VacantEntry.
        // Adds a new vector if it's a VacantEntry.
        // https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html#method.or_default
        hashes.entry(hash_num).or_default().push(img_prop);
    }

    for (hash, entries) in &hashes {
        if entries.len() > 1 {
            println!("{}: {:?}", hash, entries);
        }
    }

    let duplicate_path = owned_dir + "/duplicates";
    // `duplicate_path` *will* be modified within the for_each loops below, beware!
    let mut duplicate_path = PathBuf::from(&duplicate_path);
    fs::create_dir_all(&duplicate_path).unwrap();
    hashes
        .values()
        .filter(|entries| entries.len() > 1)
        .for_each(|entries| {
            entries.iter().for_each(|e| {
                // Modify `duplicate_path` to move the file into the `duplicates` directory
                let name = &e.path.file_name().unwrap();

                duplicate_path.push(name);
                match fs::rename(&e.path, &duplicate_path) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Failed moving file: {:?}\nReason: {}", name, e),
                };
                duplicate_path.pop();
            })
        });

    // If we didn't find any duplicates, don't leave the empty directory behind
    if fs::read_dir(&duplicate_path).unwrap().count() == 0 {
        println!("No duplicates found in {:?}", duplicate_path);
        match fs::remove_dir(&duplicate_path) {
            Ok(_) => {}
            Err(e) => eprintln!(
                "Failed removing duplicate_path: {:?}\nReason: {}",
                &duplicate_path, e
            ),
        }
    }
}

fn main() {
    find_duplicates(r"C:\Users\Chris\Pictures\Sample");
}
