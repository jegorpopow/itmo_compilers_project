use std::{env::current_dir, fs, path::PathBuf};

use pathdiff::diff_paths;

pub(crate) trait PathExt {
    fn append(self, components: &[&str]) -> Self;
    fn make_relative_if_possible(self) -> Self;
}

impl PathExt for PathBuf {
    fn append(mut self, components: &[&str]) -> Self {
        for component in components {
            self.push(component);
        }
        self
    }

    fn make_relative_if_possible(self) -> Self {
        if let Ok(cwd) = current_dir()
            && let path = fs::canonicalize(&self).as_ref().unwrap_or(&self)
            && let Some(relative) = diff_paths(path, cwd)
        {
            relative
        } else {
            self
        }
    }
}
