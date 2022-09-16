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
use counter::Counter;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut counter: HashMap<String, u64> = HashMap::new();

    fs::create_dir_all(".links")?;

    for file in glob(".Takeout/**/*").expect("Failed to read glob pattern") {
        let file = file.expect("Glob Error lol");
        let file = absolute(file)?;
        match file.extension() {
            Some(extension) if extension != "json" => {
                if let Ok(exif) = rexif::parse_file(&file) {
                    let date_entry = exif
                        .entries
                        .into_iter()
                        .filter(|exif| {
                            exif.tag == ExifTag::DateTime
                                || exif.tag == ExifTag::DateTimeOriginal
                        })
                        .min_by_key(|f| f.value_more_readable.clone());

                    if let Some(entry) = date_entry {
                        let date = NaiveDateTime::parse_from_str(
                            &entry.value_more_readable,
                            "%Y:%m:%d %H:%M:%S",
                        )
                        .ok();

                        match date {
                            Some(date) => {
                                let name = entry.value_more_readable.to_string().replace(":", "-");
                                let name_count = counter.add(name.clone());
                                let name = if name_count > 0 {
                                    name + &format!(" ({})", name_count)
                                } else {
                                    name
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
                                let name = file
                                    .file_name()
                                    .ok_or("File ends in ..")?
                                    .to_string_lossy()
                                    .to_string();

                                let name_count = counter.add(name.clone());
                                let name = if name_count > 0 {
                                    name + &format!(" ({})", name_count)
                                } else {
                                    name
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
