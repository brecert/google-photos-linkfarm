#![feature(fs_try_exists)]
#![feature(absolute_path)]

use chrono::NaiveDateTime;
use filetime::FileTime;
use glob::glob;
use rexif::{self, ExifTag};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::os::windows::fs::symlink_file;
use std::path::{absolute, Path, PathBuf};

mod counter;
mod metadata;
use counter::Counter;
use metadata::Metadata;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut counter: HashMap<String, u64> = HashMap::new();
    let mut jsons: HashSet<PathBuf> = HashSet::new();

    fs::create_dir_all(".links")?;

    println!("Indexing metadata");
    for file in glob(".Takeout/**/*.json")? {
        let file = absolute(file?)?;
        jsons.insert(file);
    }

    println!("Linking files");
    for file in glob(".Takeout/**/*").expect("Failed to read glob pattern") {
        let file = file?;
        let file = absolute(file)?;
        match file.extension() {
            Some(extension) if extension != "json" => {
                if let Ok(exif) = rexif::parse_file(&file) {
                    let date_entry = exif
                        .entries
                        .into_iter()
                        .filter(|exif| {
                            exif.tag == ExifTag::DateTime || exif.tag == ExifTag::DateTimeOriginal
                        })
                        .min_by_key(|f| f.value_more_readable.clone());

                    if let Some(entry) = date_entry {
                        let date = NaiveDateTime::parse_from_str(
                            &entry.value_more_readable,
                            "%Y:%m:%d %H:%M:%S",
                        )
                        .ok();

                        // TODO: actually error handle
                        let json_path =
                            file.with_extension(extension.to_string_lossy().to_string() + ".json");

                        
                        let meta: Option<Metadata> = match jsons.contains(&json_path) {
                            true => {
                                serde_json::from_str(&fs::read_to_string(json_path)?)?
                            }
                            false => None
                        };
                        
                        let people: Option<_> = meta
                            .and_then(|m| m.people)
                            .map(|people| people.into_iter().map(|person| person.name))
                            .map(|names| names.collect())
                            .map(|names: Vec<String>| names.join(", "));

                        match date {
                            Some(date) => {
                                let mut name =
                                    entry.value_more_readable.to_string().replace(":", "-");
                                let name_count = counter.add(name.clone());
                                if let Some(people) = people {
                                    name += &format!(" ({})", people)
                                }
                                if name_count > 0 {
                                    name += &format!(" ({})", name_count)
                                };

                                let link_path: PathBuf = [Path::new(".links"), Path::new(&name)]
                                    .into_iter()
                                    .collect();

                                let link_path = absolute(&link_path)?.with_extension(extension);

                                symlink_file(&file, link_path).expect("Could not create symlink");

                                let time = FileTime::from_unix_time(date.timestamp(), 0);
                                filetime::set_symlink_file_times(&file, time, time).unwrap();
                            }
                            None => {
                                let dir = Path::new(".links/No Date");
                                let mut name = file
                                    .file_name()
                                    .ok_or("File ends in ..")?
                                    .to_string_lossy()
                                    .to_string();

                                let name_count = counter.add(name.clone());
                                if let Some(people) = people {
                                    name += &format!(" ({})", people)
                                }
                                if name_count > 0 {
                                    name += &format!(" ({})", name_count)
                                };

                                fs::create_dir_all(dir)?;
                                let link_path: PathBuf =
                                    [dir, Path::new(&name)].into_iter().collect();

                                let link_path = absolute(&link_path)?.with_extension(extension);

                                symlink_file(&file, link_path).expect("Could not create symlink");

                                let metadata = &file.metadata()?;
                                if let Some(time) = FileTime::from_creation_time(&metadata) {
                                    filetime::set_symlink_file_times(&file, time, time).unwrap();
                                };
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }

    Ok(())
}
