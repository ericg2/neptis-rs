use inquire::{CustomType, Select};
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

use crate::apis::{
    NeptisError,
    api::WebApi,
    dtos::{NodeDto, PostForFileApi},
};

pub struct TreeBrowser<'a> {
    api: &'a WebApi,
    rt: &'a Runtime,
    current_path: PathBuf,
}

impl<'a> TreeBrowser<'a> {
    pub fn new(api: &'a WebApi, rt: &'a Runtime) -> Self {
        Self {
            api,
            rt,
            current_path: PathBuf::from("/"),
        }
    }

    pub fn run(&mut self) -> Result<(), NeptisError> {
        loop {
            clearscreen::clear().expect("Failed to clear screen!");
            let entries = self.load_current_dir()?;

            let options = self.build_menu_options(&entries);
            let selection = Select::new("Select file/directory:", options)
                .with_page_size(30)
                .prompt()
                .map_err(|_| NeptisError::Str("Selection cancelled".into()))?;

            match selection {
                MenuAction::Navigate(path) => {
                    self.current_path = path;
                }
                MenuAction::Action(action) => {
                    self.handle_action(action)?;
                }
                MenuAction::GoUp => {
                    self.current_path.pop();
                }
                MenuAction::Quit => break,
            }
        }
        Ok(())
    }

    fn load_current_dir(&mut self) -> Result<Vec<NodeDto>, NeptisError> {
        self.rt.block_on(async {
            self.api
                .browse_file(self.current_path.to_str().unwrap_or("/"))
                .await
        })
    }

    fn build_menu_options(&self, entries: &[NodeDto]) -> Vec<MenuAction> {
        let mut options = Vec::new();

        // Add parent directory option if not at root
        if self.current_path != Path::new("/") {
            options.push(MenuAction::GoUp);
        }
        options.push(MenuAction::Quit);

        // Add directory entries
        for entry in entries.iter().filter(|e| e.is_dir) {
            options.push(MenuAction::Navigate(self.current_path.join(&entry.path)));
        }

        // Add file entries
        for entry in entries.iter().filter(|e| !e.is_dir) {
            options.push(MenuAction::Action(FileAction::Select(entry.clone())));
        }

        // Add actions
        options.push(MenuAction::Action(FileAction::CreateFile));
        options.push(MenuAction::Action(FileAction::CreateDir));
        options
    }

    fn handle_action(&mut self, action: FileAction) -> Result<(), NeptisError> {
        match action {
            FileAction::Select(file) => {
                // Handle file selection
                let content = self
                    .rt
                    .block_on(async { self.api.dump_file(&file.path, None, None).await })?;
                println!("File content:\n{}", content);
            }
            FileAction::CreateFile => {
                let name = CustomType::<String>::new("File name:")
                    .prompt()
                    .map_err(|_| NeptisError::Str("Invalid file name".into()))?;

                let path = self.current_path.join(name);
                self.rt.block_on(async {
                    self.api
                        .post_file(PostForFileApi {
                            path: path.to_str().unwrap().to_string(),
                            is_dir: false,
                            base64: None,
                            offset: None,
                        })
                        .await
                })?;
            }
            FileAction::CreateDir => {
                let name = CustomType::<String>::new("Directory name:")
                    .prompt()
                    .map_err(|_| NeptisError::Str("Invalid directory name".into()))?;

                let path = self.current_path.join(name);
                self.rt.block_on(async {
                    self.api
                        .post_file(PostForFileApi {
                            path: path.to_str().unwrap().to_string(),
                            is_dir: true,
                            base64: None,
                            offset: None,
                        })
                        .await
                })?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
enum MenuAction {
    Navigate(PathBuf),
    Action(FileAction),
    GoUp,
    Quit,
}

#[derive(Debug)]
enum FileAction {
    Select(NodeDto),
    CreateFile,
    CreateDir,
}

impl std::fmt::Display for MenuAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MenuAction::Navigate(path) => write!(f, "📁 {}", path.display()),
            MenuAction::Action(action) => match action {
                FileAction::Select(file) => write!(
                    f,
                    "📄 {}",
                    Path::new(&file.path).file_name().unwrap().to_str().unwrap()
                ),
                FileAction::CreateFile => write!(f, "➕ Create new file"),
                FileAction::CreateDir => write!(f, "📂 Create new directory"),
            },
            MenuAction::GoUp => write!(f, "⬆️ Go up"),
            MenuAction::Quit => write!(f, "🚪 Quit"),
        }
    }
}