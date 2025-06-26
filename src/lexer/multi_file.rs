use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    error::LexerError,
    lexer::{Lexer, Token},
};

pub struct MultiFileLexer {
    file_cache: HashMap<PathBuf, String>,
    base_dir: PathBuf,
}

impl MultiFileLexer {
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            file_cache: HashMap::new(),
            base_dir: base_dir.as_ref().to_path_buf(),
        }
    }

    pub fn tokenize_file(&mut self, file_path: &Path) -> Result<Vec<Token>, MultiFileError> {
        let resolved_path = self.resolve_path(&file_path);

        self.tokenize_file_recursive(&resolved_path)
    }

    fn resolve_path(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();

        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        }
    }

    fn tokenize_file_recursive(&mut self, file_path: &Path) -> Result<Vec<Token>, MultiFileError> {
        if !self.file_cache.contains_key(file_path) {
            let content =
                fs::read_to_string(file_path).map_err(|_| MultiFileError::FileNotFound {
                    path: file_path.to_path_buf(),
                })?;

            self.file_cache.insert(file_path.to_path_buf(), content);
        }

        let content = &self.file_cache[file_path];
        let mut lexer = Lexer::new(content, file_path);

        Ok(lexer.tokenize())
    }
}

#[derive(Error, Debug)]
pub enum MultiFileError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Circular import detected: {path}")]
    CircularImport { path: PathBuf },

    #[error("Lexer error: {0}")]
    Lexer(#[from] LexerError),
}
