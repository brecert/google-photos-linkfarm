#![feature(fs_try_exists)]
#![feature(absolute_path)]

use chrono::NaiveDateTime;
use filetime::FileTime;
use glob::glob;
use rexif::{self, ExifTag};
use std::collections::HashMap;
use std::fs;
use std::os::windows::fs::symlink_file;
use std::path::{absolute, Path, PathBuf};

mod counter;
mod metadata;
use counter::Counter;
use metadata::Metadata;

#[fncmd::fncmd]
fn main(
    /// Directory to link from
    #[opt(short, long)]
    input: PathBuf,
    /// Directory to create links to
    #[opt(short, long)]
    output: PathBuf,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    fs::metadata(&output).map_err(|e| format!("Unable to read output directory: {e}"))?;

    let mut counter: HashMap<String, u64> = HashMap::new();
    let mut metadata: HashMap<PathBuf, Metadata> = HashMap::new();

    let json_pattern = input
        .join("**")
        .join("*.json")
        .to_string_lossy()
        .to_string();

    let file_pattern = input.join("**").join("*").to_string_lossy().to_string();

    println!("Indexing Metadata");
    for file in glob(&json_pattern)? {
        let file = absolute(file?)?;
        let meta: Metadata = serde_json::from_str(&fs::read_to_string(&file)?)?;
        metadata.insert(file, meta);
    }

    println!("Linking Files");
    for file in glob(&file_pattern)? {
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

                        let meta: Option<&Metadata> = metadata.get(&json_path);

                        let people: Option<_> = meta
                            .and_then(|m| m.people.as_ref())
                            .map(|people| people.into_iter().map(|person| person.name.as_ref()))
                            .map(|names| names.collect())
                            .map(|names: Vec<&str>| names.join(", "));

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

                                let link_path: PathBuf = output.join(&name);

                                let link_path = absolute(&link_path)?.with_extension(extension);

                                symlink_file(&file, link_path).expect("Could not create symlink");

                                let time = FileTime::from_unix_time(date.timestamp(), 0);
                                filetime::set_symlink_file_times(&file, time, time).unwrap();
                            }
                            None => {
                                let link_path = output.join("No Date");

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

                                fs::create_dir_all(&link_path)?;
                                let link_path: PathBuf =
                                    [&link_path, Path::new(&name)].into_iter().collect();

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
