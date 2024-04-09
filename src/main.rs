use clap::{App, Arg};
use colored::*;
use crossterm::style::Stylize;
use std::collections::HashSet;
use std::env;
use std::fs::read_to_string;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{self};
use std::path::PathBuf;
use std::process::Command;
use walkdir::{DirEntry, WalkDir};

fn main() {
    let matches = App::new("addpath")
        .version("1.0")
        .author("Tao119")
        .about("Automatically adds paths to your shell configuration")
        .arg(
            Arg::with_name("pkgname")
                .help("The package name to search for")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("adddir")
                .help("Additional directory to include in the search path")
                .long("adddir")
                .takes_value(true)
                .multiple(true),
        )
        .get_matches();

    let pkgname = matches.value_of("pkgname").unwrap();

    if let Ok(output) = Command::new("which").arg(pkgname).output() {
        if !output.stdout.is_empty() {
            println!("{} is already in the PATH.", pkgname);
            return;
        }
    }

    let mut search_dirs = vec![String::from("/usr"), String::from("/opt")];

    if let Some(add_dirs) = matches.values_of("adddir") {
        search_dirs.extend(add_dirs.map(String::from));
    }

    println!("Searching in directories: {:?}", search_dirs);

    let mut candidates = Vec::new();

    for dir in &search_dirs {
        println!("{}", format!("Checking directory: {}", dir).dark_green());
        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_entry(is_not_skippable)
            .filter_map(Result::ok)
        {
            if entry.file_type().is_dir() && entry.file_name() == "bin" {
                for sub_entry in WalkDir::new(entry.path())
                    .max_depth(1)
                    .into_iter()
                    .filter_map(Result::ok)
                {
                    if sub_entry.file_name().to_string_lossy().contains(pkgname) {
                        candidates.push(entry.clone().into_path());
                    }
                }
            }
        }
    }

    remove_duplicates(&mut candidates);

    let home_dir = dirs::home_dir().expect("Failed to find home directory");
    let shell_path = env::var("SHELL").unwrap_or_default();
    let config_file = if shell_path.ends_with("/bash") {
        "bashrc"
    } else if shell_path.ends_with("/zsh") {
        "zshrc"
    } else {
        eprintln!("Unsupported shell");
        return;
    };

    let mut config_path = home_dir;
    config_path.push(format!(".{}", config_file));
    let source_config_path = config_path.clone();

    let existing_contents = read_to_string(&config_path).unwrap_or_default();

    if candidates.is_empty() {
        println!("No paths found. Consider broadening your search.");
    } else {
        for (index, path) in candidates.iter().enumerate() {
            let path_str = format!("{}", path.display());
            if existing_contents.contains(&path.display().to_string()) {
                println!(
                    "{}: {} {}",
                    index,
                    path_str.bright_black(),
                    "(already exists)".to_string().red()
                );
            } else {
                println!("{}: {}", index, path_str.bright_yellow());
            }
        }
        println!("Select the path to add by number: ");
        let index: usize;

        let mut selection = String::new();
        loop {
            io::stdin().read_line(&mut selection).unwrap();
            let trimmed = selection.trim();
            if !trimmed.is_empty() {
                match trimmed.parse::<usize>() {
                    Ok(parsed_index) => {
                        index = parsed_index;
                        break;
                    }
                    Err(_) => {
                        println!("Please enter a valid number.");
                    }
                }
            } else {
                println!("{}", "Please enter a valid number.".to_string().red());
            }
            selection.clear();
        }

        if let Some(selected_path) = candidates.get(index) {
            let path_str = format!("\nexport PATH=\"$PATH:{}\"", selected_path.display());
            if !existing_contents.contains(&path_str) {
                append_to_file(config_path, &path_str);
                println!();
                println!("Added the following line to your {} file:", config_file);
                println!(
                    "export PATH=\"$PATH:\"{}\"",
                    selected_path.display().to_string().bright_yellow()
                );

                let source_command = format!("source {}", source_config_path.to_string_lossy());

                println!();
                println!(
                    "{}\n{}\n{}",
                    "finished setting the path!"
                        .to_string()
                        .bright_blue()
                        .bold(),
                    "Please run the following command to update your shell environment:"
                        .to_string()
                        .bright_blue()
                        .bold(),
                    source_command.yellow().bold()
                )
            } else {
                println!("Path already exists in the {} file.", config_file);
            }
        }
    }
}

fn is_not_skippable(entry: &DirEntry) -> bool {
    let skip_dirs = ["dev", "proc", "sys"];
    !entry
        .path()
        .components()
        .any(|c| skip_dirs.contains(&c.as_os_str().to_str().unwrap()))
}

fn append_to_file(file_path: PathBuf, content: &str) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(file_path)
        .expect("Failed to open file");
    if let Err(e) = writeln!(file, "{}", content) {
        eprintln!("Failed to write to file: {}", e);
    }
}

fn remove_duplicates(vec: &mut Vec<PathBuf>) {
    let mut seen = HashSet::new();
    vec.retain(|e| seen.insert(e.clone()));
}
