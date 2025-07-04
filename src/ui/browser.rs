use std::{
    cmp::Ordering,
    collections::HashMap,
    fs::{self, File},
    io::{BufWriter, Write},
    ops::Div,
    path::{Component, Path, PathBuf},
    thread,
    time::Duration,
};

use chrono::{DateTime, Local};
use indexmap::IndexMap;
use inquire::{Confirm, Editor, Select, Text, required, validator::Validation};
use itertools::Itertools;
use crate::file_size::FileSize;
use crate::filesystem::{FsNode, NeptisFS};
use crate::prelude::GenericFileType;
use crate::to_dto_time;

pub struct FileBrowser {
    fs: NeptisFS,
}

pub enum FileBrowserMode {
    Normal,
    SelectFile,
    SelectFolder,
    SelectFileRW,
    SelectFolderRW,
}

const BUFFER_BYTES: u64 = 16_000_000;

impl FileBrowser {
    pub fn new(fs: impl Into<NeptisFS>) -> Self {
        FileBrowser { fs: fs.into() }
    }

    pub fn is_read_only(&self, path: &Path) -> bool {
        let mut parts = path.components().filter_map(|c| match c {
            Component::Normal(p) => p.to_str(),
            _ => None,
        });

        match (parts.next(), parts.next()) {
            (Some(_), Some("data")) => false, // Matches /anything/data or deeper
            _ => true,                        // All others are read-only
        }
    }

    pub fn do_download(&self, node: &FsNode, path: &Path) {
        clearscreen::clear().expect("Failed to clear screen!");
        match Text::new("Please enter a base directory")
            .with_default(
                &dirs_next::download_dir()
                    .map(|x| x.join("Neptis Downloads").to_str().unwrap().to_string())
                    .unwrap_or("".into()),
            )
            .with_validator(required!())
            .with_validator(|s: &str| {
                let path = Path::new(s);
                if path.exists() && !path.is_dir() {
                    Ok(Validation::Invalid("The path must be a directory!".into()))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt_skippable()
            .expect("Failed to show prompt!")
            .map(|x| PathBuf::from(x))
        {
            Some(base_sp) => {
                let save_path = base_sp.join(format!(
                    "{}-{}",
                    path.file_name()
                        .map(|x| x.to_str().unwrap())
                        .unwrap_or("unknown"),
                    Local::now().to_string()
                ));
                if !fs::exists(&base_sp).unwrap_or(false) {
                    let _ = fs::create_dir_all(&base_sp);
                }
                let mut file = match File::create(&save_path) {
                    Ok(f) => BufWriter::new(f),
                    Err(e) => {
                        eprintln!("Failed to create file: {}", e);
                        return;
                    }
                };
                match self.fs.do_dump(path, 0, usize::MAX) {
                    Some(data) => {
                        if let Err(e) = file.write_all(&data) {
                            println!("> Failed to write to file: {}", e);
                        } else {
                            let _ = file.flush();
                            println!(
                                "> Downloaded: {} / {}",
                                FileSize::prettify(data.len() as u64),
                                FileSize::prettify(node.attr.size),
                            );
                        }
                    }
                    None => {
                        println!("> Error reading from source");
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
            None => return,
        }
    }

    pub fn do_stats(&self, node: &FsNode, path: &Path) {
        loop {
            clearscreen::clear().expect("Failed to clear screen!");
            println!("===== Stats for '{}' =====\n", path.to_str().unwrap());
            println!(
                "Type -> {}",
                match node.attr.kind {
                    GenericFileType::BlockDevice => "Block Device",
                    GenericFileType::CharDevice => "Char Device",
                    GenericFileType::Directory => "Directory",
                    GenericFileType::NamedPipe => "Pipe",
                    GenericFileType::RegularFile => "File",
                    GenericFileType::Socket => "Socket",
                    GenericFileType::Symlink => "Symlink",
                }
            );
            println!("Size -> {}", FileSize::prettify(node.attr.size));
            println!(
                "Date Created -> {}",
                to_dto_time!(node.attr.ctime)
                    .and_utc()
                    .with_timezone(&Local)
                    .to_string()
            );
            println!(
                "Date Accessed -> {}",
                to_dto_time!(node.attr.atime)
                    .and_utc()
                    .with_timezone(&Local)
                    .to_string()
            );
            println!(
                "Date Modified -> {}\n",
                to_dto_time!(node.attr.mtime)
                    .and_utc()
                    .with_timezone(&Local)
                    .to_string()
            );
            if Confirm::new("Do you want to go back")
                .with_default(true)
                .prompt_skippable()
                .expect("Failed to show prompt!")
                .map(|x| if x { None } else { Some(x) })
                .flatten()
                .is_none()
            {
                break;
            }
        }
    }

    pub fn do_delete(&self, path: &Path) {
        clearscreen::clear().expect("Failed to clear screen!");
        if Confirm::new(&format!(
            "Are you sure you want to delete '{}'",
            path.to_str().unwrap()
        ))
        .with_default(true)
        .prompt_skippable()
        .expect("Failed to show prompt!")
        .map(|x| if !x { None } else { Some(x) })
        .flatten()
            == Some(true)
        {
            match self.fs.do_delete(path) {
                Some(()) => println!("> Successfully deleted."),
                _ => {
                    println!("> Failed to delete...");
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    }

    pub fn do_rename(&self, path: &Path) {
        clearscreen::clear().expect("Failed to clear screen!");
        match Text::new("Please enter a new file name")
            .with_validator(required!())
            .prompt_skippable()
            .expect("Failed to show prompt!")
            .map(|x| {
                PathBuf::from(path.parent().map(|x| x.to_str().unwrap()).unwrap_or("/")).join(x)
            }) {
            Some(new_path) => {
                match self
                    .fs
                    .do_write(&path, Some(&new_path), None, None, None, None, None)
                {
                    Some(_) => println!("> Rename successful."),
                    _ => {
                        println!("> Failed to rename file!");
                        thread::sleep(Duration::from_secs(2));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn do_create(&self, parent: &Path, is_dir: bool) {
        clearscreen::clear().expect("Failed to clear screen!");
        if let Some(path) = Text::new(if is_dir {
            "Please enter a directory name"
        } else {
            "Please enter a file name"
        })
        .with_validator(required!())
        .prompt_skippable()
        .expect("Failed to show prompt!")
        .map(|x| parent.join(x))
        {
            match self.fs.do_create(&path, is_dir) {
                Some(_) => println!("> Create successful."),
                _ => {
                    println!("> Failed to create file!");
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
    }

    pub fn do_edit(&self, node: &FsNode, path: &Path) {
        let items = if node.attr.size < BUFFER_BYTES {
            self.fs
                .do_dump(path, 0, BUFFER_BYTES as usize)
                .map(|x| String::from_utf8(x.to_vec()).ok())
                .flatten()
        } else {
            None
        };
        if items.is_none() &&
            Confirm::new("This item is too large (or failed) and a preview will not be displayed. Do you want to continue")
                .with_default(false)
                .prompt_skippable()
                .expect("Failed to show prompt!")
                .map(|x|if !x { None } else { Some(x) })
                .is_none()
        {
            return;
        }
        match Editor::new(&format!("Modifying {}", path.to_str().unwrap()))
            .with_predefined_text(&items.unwrap_or("".into()))
            .prompt_skippable()
            .expect("Failed to show prompt!")
        {
            Some(content) => {
                match self
                    .fs
                    .do_write(path, None, None, Some(content.as_bytes()), None, None, None)
                {
                    Some(_) => println!("> Successfully wrote the data."),
                    None => println!("> Failed to write the data"),
                }
                thread::sleep(Duration::from_secs(1));
            }
            _ => {}
        }
    }

    pub fn show_browser(&self, mode: FileBrowserMode) -> Option<PathBuf> {
        // Start at the root directory and show everything
        let mut sel_path = PathBuf::from("/");
        const STR_RO_SELECT: &'static str = "Select";
        const STR_RO_FINAL_FOLDER: &'static str = "Use This Folder";
        const STR_RO_FINAL_FILE: &'static str = "Use This File";
        const STR_RO_STAT: &'static str = "Stats";
        const STR_RO_SAVE: &'static str = "Download";
        const STR_RW_EDIT: &'static str = "Edit";
        const STR_RW_RENAME: &'static str = "Rename";
        const STR_RW_DELETE: &'static str = "Delete";
        const STR_RW_MKDIR: &'static str = "Create Directory";
        const STR_RW_MKNOD: &'static str = "Create File";
        const STR_BACK: &'static str = "Go Back";
        const STR_UP: &'static str = "Go Up";
        const PLAINTEXT_EXTENSIONS: &[&str] = &[
            "txt",
            "text",
            "log",
            "out",
            "nfo",
            "readme",
            "c",
            "h",
            "cpp",
            "cc",
            "cxx",
            "hpp",
            "hxx",
            "py",
            "pyw",
            "ipynb",
            "rs",
            "toml",
            "go",
            "js",
            "ts",
            "jsx",
            "tsx",
            "java",
            "kt",
            "kts",
            "rb",
            "php",
            "lua",
            "sh",
            "bash",
            "zsh",
            "fish",
            "bat",
            "cmd",
            "ps1",
            "swift",
            "scala",
            "cs",
            "vb",
            "pl",
            "pm",
            "r",
            "asm",
            "s",
            "v",
            "sv",
            "vhdl",
            "clj",
            "cljs",
            "dart",
            "conf",
            "cfg",
            "ini",
            "json",
            "yaml",
            "yml",
            "toml",
            "env",
            "properties",
            "prefs",
            "editorconfig",
            "md",
            "markdown",
            "rst",
            "asciidoc",
            "adoc",
            "tex",
            "textile",
            "pod",
            "csv",
            "tsv",
            "psv",
            "jsonl",
            "ndjson",
            "xml",
            "html",
            "htm",
            "xhtml",
            "sql",
            "db",
            "dump",
            "makefile",
            "mk",
            "mkfile",
            "dockerfile",
            "gradle",
            "bazel",
            "bzl",
            "buck",
            "gitattributes",
            "gitignore",
            "gitkeep",
            "gitmodules",
            "editorconfig",
            "npmrc",
            "yarnrc",
            "eslintignore",
            "prettierrc",
            "manifest",
            "license",
            "copying",
            "todo",
            "changelog",
            "credits",
            "authors",
        ];

        loop {
            clearscreen::clear().expect("Failed to clear screen!");
            match self.fs.do_readdir(&sel_path).map(|x| {
                x.into_iter()
                    .filter(|x| {
                        x.path != PathBuf::from("../..")
                            && x.path != PathBuf::from("../../..")
                            && x.path.to_str().unwrap().trim() != ""
                    })
                    .sorted_by(|a, b| {
                        match (
                            a.attr.kind == GenericFileType::Directory,
                            b.attr.kind == GenericFileType::Directory,
                        ) {
                            (true, false) => Ordering::Less, // directories come first
                            (false, true) => Ordering::Greater,
                            _ => b.attr.atime.cmp(&a.attr.atime), // if same type, sort by atime desc
                        }
                    })
                    .map(|node| {
                        (
                            format!(
                                "{:<2} {:<50} ({:<22}) {:<5}",
                                if node.attr.kind == GenericFileType::Directory {
                                    "📁"
                                } else {
                                    "📄"
                                },
                                node.path.to_str().unwrap(),
                                chrono::DateTime::<chrono::Local>::from(node.attr.atime)
                                    .format("%Y-%m-%d %I:%M:%S %p"),
                                FileSize::prettify(node.attr.size)
                            ),
                            node,
                        )
                    })
                    .collect::<IndexMap<_, _>>()
            }) {
                Some(ret) => {
                    let mut title = format!("Current Path: {}", sel_path.to_str().unwrap().replace("\\", "/"));
                    title += match mode {
                        FileBrowserMode::Normal => "",
                        FileBrowserMode::SelectFile => "\nPlease select any file.",
                        FileBrowserMode::SelectFolder => "\nPlease select any folder.",
                        FileBrowserMode::SelectFileRW => "\nPlease select an R/W file.",
                        FileBrowserMode::SelectFolderRW => "\nPlease select an R/W folder.",
                    };
                    match Select::new(&title, {
                        let is_rw = !self.is_read_only(&sel_path);
                        let mut keys = ret
                            .keys()
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<_>>();
                        keys.push(STR_UP.to_string());
                        if is_rw {
                            keys.push(STR_RW_MKDIR.to_string());
                            keys.push(STR_RW_MKNOD.to_string());
                        }
                        keys
                    })
                    .with_page_size(30)
                    .prompt_skippable()
                    .expect("Expected prompt to show!")
                    .map(|x| if x == STR_UP { None } else { Some(x) })
                    .flatten()
                    {
                        Some(f_name) => {
                            if f_name == STR_RW_MKDIR.to_string() {
                                self.do_create(&sel_path, true);
                            } else if f_name == STR_RW_MKNOD.to_string() {
                                self.do_create(&sel_path, false);
                            } else {
                                let f_node = ret.get(&f_name).expect("Expected file to match!");
                                let full_path = sel_path.join(f_node.path.clone());
                                let is_rw = !self.is_read_only(&full_path);
                                match Select::new(
                                    &format!("Select action for {}", full_path.to_str().unwrap().replace("\\", "/")),
                                    {
                                        let mut actions = vec![];
                                        if f_node.attr.kind == GenericFileType::Directory {
                                            actions.push(STR_RO_SELECT);
                                        }

                                        match mode {
                                            FileBrowserMode::SelectFile => {
                                                if f_node.attr.kind == GenericFileType::RegularFile
                                                {
                                                    actions.push(STR_RO_FINAL_FILE);
                                                }
                                            }
                                            FileBrowserMode::SelectFolder => {
                                                if f_node.attr.kind == GenericFileType::Directory {
                                                    actions.push(STR_RO_FINAL_FOLDER);
                                                }
                                            }
                                            FileBrowserMode::SelectFileRW => {
                                                if is_rw
                                                    && f_node.attr.kind
                                                        == GenericFileType::RegularFile
                                                {
                                                    actions.push(STR_RO_FINAL_FILE);
                                                }
                                            }
                                            FileBrowserMode::SelectFolderRW => {
                                                if is_rw
                                                    && f_node.attr.kind
                                                        == GenericFileType::Directory
                                                {
                                                    actions.push(STR_RO_FINAL_FOLDER);
                                                }
                                            }
                                            _ => {}
                                        }

                                        actions.push(STR_RO_STAT);
                                        if f_node.attr.kind == GenericFileType::RegularFile {
                                            actions.push(STR_RO_SAVE);
                                            if is_rw
                                                && f_node.path.extension().is_none_or(|x| {
                                                    PLAINTEXT_EXTENSIONS.contains(
                                                        &x.to_str()
                                                            .unwrap()
                                                            .to_lowercase()
                                                            .as_str(),
                                                    )
                                                })
                                            {
                                                actions.push(STR_RW_EDIT);
                                            }
                                        }
                                        if is_rw {
                                            if f_node.attr.kind == GenericFileType::Directory {
                                                actions.push(STR_RW_MKDIR);
                                                actions.push(STR_RW_MKNOD);
                                            }
                                            actions.push(STR_RW_RENAME);
                                            actions.push(STR_RW_DELETE);
                                        }
                                        actions.push(STR_BACK);
                                        actions
                                    },
                                )
                                .with_page_size(20)
                                .prompt_skippable()
                                .expect("Failed to show prompt!")
                                .map(|x| if x == STR_BACK { None } else { Some(x) })
                                .flatten()
                                {
                                    Some(STR_RO_FINAL_FOLDER) => return Some(full_path),
                                    Some(STR_RO_FINAL_FILE) => return Some(full_path),
                                    Some(STR_RO_SAVE) => self.do_download(f_node, &full_path),
                                    Some(STR_RO_STAT) => self.do_stats(f_node, &full_path),
                                    Some(STR_RW_DELETE) => self.do_delete(&full_path),
                                    Some(STR_RW_EDIT) => self.do_edit(f_node, &full_path),
                                    Some(STR_RW_MKDIR) => self.do_create(&full_path, true),
                                    Some(STR_RW_MKNOD) => self.do_create(&full_path, false),
                                    Some(STR_RW_RENAME) => self.do_rename(&full_path),
                                    Some(STR_RO_SELECT) => {
                                        sel_path = full_path; // go to next level
                                    }
                                    _ => {}
                                }
                            }
                            continue;
                        }
                        None => {
                            // The user is requesting to go up one level.
                            match sel_path.parent().map(|x| x.to_path_buf()) {
                                Some(x) => {
                                    sel_path = x;
                                    continue;
                                }
                                None => {
                                    if Confirm::new("Are you sure you want to leave the browser? This will cancel all pending actions!")
                                        .with_default(false)
                                        .prompt_skippable()
                                        .expect("Failed to show prompt!")
                                        .map(|x|if !x { None } else { Some(x) })
                                        .is_none() {
                                        break;
                                    } else {
                                        continue;
                                    }
                                },
                            }
                        }
                    }
                }
                None => {
                    match Confirm::new("An unexpected error has occurred. Do you want to try again")
                        .with_default(true)
                        .prompt_skippable()
                        .expect("Expected prompt to show!")
                    {
                        Some(true) => continue,
                        _ => break,
                    };
                }
            }
        }
        None
    }
}
