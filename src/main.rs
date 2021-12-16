use std::collections::HashMap;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::{fs, io};

use argh::FromArgs;
use glob::glob;
use image::imageops::FilterType;
use img_hash::{HashAlg, HasherConfig};
use rayon::prelude::*;

const VALID_EXTENSIONS: [&str; 4] = ["png", "jpeg", "jpg", "gif"];

#[derive(FromArgs, Debug)]
/// Command-line arguments for image deduplication.
struct Args {
    /// one or more directories to check for duplicates in, each directory is checked independently
    #[argh(positional)]
    directories: Vec<String>,
}

#[derive(Debug)]
/// Various properties to describe an image.
struct ImageProperties {
    path: PathBuf,
    file_size: u64,
}

fn find_duplicates(directory: &str) -> Result<(), io::Error> {
    let owned_dir = directory.to_owned();

    let dup_glob = match glob(&(owned_dir.clone() + r"/*.*")) {
        Ok(g) => g,
        Err(e) => return Err(io::Error::new(ErrorKind::Other, e.msg)),
    };
    let images: Vec<PathBuf> = dup_glob
        .filter_map(Result::ok)
        .filter(|pb| {
            VALID_EXTENSIONS.contains(
                &pb.extension()
                    .unwrap_or(&OsString::new())
                    .to_str()
                    .unwrap_or(""),
            )
        })
        .collect();

    let mut hashes: HashMap<u64, Vec<ImageProperties>> = HashMap::new();
    let hasher = HasherConfig::with_bytes_type::<[u8; 8]>()
        .hash_alg(HashAlg::Gradient)
        .resize_filter(FilterType::Triangle)
        .to_hasher();

    for image in &images {
        let image_obj = match image::open(image) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("Skipping opening {:?}, original error: {}", image, e);
                continue;
            }
        };
        let hash = hasher.hash_image(&image_obj);
        let mut hash_num: u64 = 0;
        for (idx, byte) in hash.as_bytes().iter().enumerate() {
            hash_num |= (*byte as u64) << (8 * idx);
        }

        // Get the vector (or make a new one if not added) and add a new ImageProperties to it
        let img_prop = ImageProperties {
            path: image.clone(),
            file_size: match fs::metadata(image) {
                Ok(md) => md.len(),
                Err(_) => 0,
            },
        };

        // Gets an Entry from the hash map, which is an enum that can be: OccupiedEntry, VacantEntry.
        // Adds a new vector if it's a VacantEntry.
        // https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html#method.or_default
        hashes.entry(hash_num).or_default().push(img_prop);
    }

    let duplicate_path = owned_dir + "/duplicates";
    // `duplicate_path` *will* be modified within the for_each loops below, beware!
    let mut duplicate_path = PathBuf::from(&duplicate_path);
    fs::create_dir_all(&duplicate_path)?;
    hashes
        .values()
        .filter(|entries| entries.len() > 1)
        .for_each(|entries| {
            entries.iter().for_each(|e| {
                // Modify `duplicate_path` to move the file into the `duplicates` directory
                if let Some(name) = &e.path.file_name() {
                    duplicate_path.push(name);
                    match fs::rename(&e.path, &duplicate_path) {
                        Ok(_) => {}
                        Err(e) => eprintln!("Failed moving file: {:?}\nReason: {}", name, e),
                    };
                    duplicate_path.pop();
                }
            })
        });

    // If we didn't find any duplicates, don't leave the empty directory behind
    if let Ok(d) = fs::read_dir(&duplicate_path) {
        if d.count() == 0 {
            match fs::remove_dir(&duplicate_path) {
                Ok(_) => println!(
                    "No duplicates found in {:?}",
                    duplicate_path.parent().unwrap_or_else(|| Path::new(""))
                ),
                Err(e) => eprintln!(
                    "Failed removing duplicate_path: {:?}\nReason: {}",
                    &duplicate_path, e
                ),
            }
        }
    }

    // Clippy complains if I use an explicit return, but I hate implicit returns >:(
    Ok(())
}

fn main() {
    let args: Args = argh::from_env();

    if args.directories.is_empty() {
        eprintln!("Missing argument: directory. Try '--help' for more info.")
    } else {
        args.directories
            .par_iter()
            .map(|dir| find_duplicates(dir))
            .filter_map(Result::err)
            .for_each(|err| eprintln!("{:?}", err));
    }
}
