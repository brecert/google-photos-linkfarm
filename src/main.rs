#![feature(fs_try_exists)]
#![feature(absolute_path)]
#![feature(backtrace)]
#![feature(path_file_prefix)]

use anyhow::{anyhow, Context, Result};
use chrono::NaiveDateTime;
use dashmap::{DashMap, DashSet};
use filetime::FileTime;
use glob::glob;
use rayon::prelude::*;
use rexif::{self, ExifTag};
use std::fs;
use std::fs::hard_link;
use std::path::{absolute, PathBuf};

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
) -> Result<()> {
    fs::metadata(&output).map_err(|e| anyhow!("Unable to read output directory: {e}"))?;

    let counter: DashMap<String, u64> = DashMap::new();
    let used_named: DashSet<String> = DashSet::new();
    let metadata: DashMap<PathBuf, Metadata> = DashMap::new();

    let json_pattern = input
        .join("**")
        .join("*.json")
        .to_string_lossy()
        .to_string();

    let file_pattern = input.join("**").join("*").to_string_lossy().to_string();

    println!("Indexing Metadata");
    glob(&json_pattern)?
        .par_bridge()
        .try_for_each(|file| -> Result<()> {
            let file = absolute(file?)?;
            let meta: Metadata = serde_json::from_str(&fs::read_to_string(&file)?)?;
            metadata.insert(file, meta);
            Ok(())
        })?;

    println!("Linking Files");
    glob(&file_pattern)?
        .into_iter()
        .try_for_each(|file| -> Result<()> {
            let file = file?;
            let file = absolute(file)?;
            try_link_file(file, &metadata, &counter, &used_named, &output)?;
            Ok(())
        })?;

    Ok(())
}

fn try_link_file(
    file: PathBuf,
    metadata: &DashMap<PathBuf, Metadata>,
    counter: &DashMap<String, u64>,
    used_named: &DashSet<String>,
    output: &PathBuf,
) -> Result<(), anyhow::Error> {
    Ok(match file.extension() {
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

                    let people: Option<_> = metadata.get(&json_path).and_then(|m| {
                        if let Some(people) = &m.people {
                            let names: Vec<&str> = people
                                .into_iter()
                                .map(|person| person.name.as_ref())
                                .collect();
                            Some(names.join(", "))
                        } else {
                            None
                        }
                    });

                    match date {
                        Some(date) => {
                            let mut name = entry.value_more_readable.to_string().replace(":", "-");
                            let name_count = counter.add(name.clone());
                            if let Some(people) = people {
                                name += &format!(" ({})", people)
                            }
                            if name_count > 0 {
                                name += &format!(" ({})", name_count)
                            };

                            let link_path: PathBuf = output.join(&name);

                            let link_path = absolute(&link_path)?.with_extension(extension);

                            hard_link(&file, &link_path).with_context(|| {
                                format!("Failed to link file {:?} to {:?}", &link_path, &file)
                            })?;

                            let time = FileTime::from_unix_time(date.timestamp(), 0);
                            filetime::set_symlink_file_times(&file, time, time)?;
                        }
                        None => {
                            let link_path = output.join("No Date");

                            let name = file
                                .file_prefix()
                                .expect("File ends in ..")
                                .to_string_lossy()
                                .to_string();

                            let name = if let Some(people) = people {
                                name + &format!(" ({})", people)
                            } else {
                                name
                            };

                            // ew
                            let mut try_name = name.clone();
                            let mut num = 0;
                            while used_named.contains(&try_name) {
                                num += 1;
                                try_name = name.clone() + &format!(" ({})", num);
                                dbg!("Try", &try_name);
                            }
                            used_named.insert(try_name.clone());
                            let name = try_name;


                            fs::create_dir_all(&link_path).with_context(|| {
                                anyhow!("Failed to create directory at {:?}", &link_path)
                            })?;
                            let link_path = link_path.join(&name);

                            let link_path = absolute(&link_path)?.with_extension(extension);

                            hard_link(&file, &link_path).with_context(|| {
                                format!("Failed to link file {:?} to {:?}", &file, &link_path)
                            })?;

                            let metadata = &file.metadata()?;
                            if let Some(time) = FileTime::from_creation_time(&metadata) {
                                filetime::set_symlink_file_times(&file, time, time)?;
                            };
                        }
                    }
                }
            }
        }
        _ => (),
    })
}
