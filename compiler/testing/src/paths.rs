use std::{
    env::current_dir,
    fs,
    path::{Path, PathBuf},
};

use pathdiff::diff_paths;

use crate::Mode;

pub trait PathExt {
    #[must_use]
    fn append(self, components: &[&str]) -> Self;
    #[must_use]
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
            && let Some(relative) = diff_paths(dbg!(path), dbg!(cwd))
        {
            relative
        } else {
            self
        }
    }
}

#[must_use]
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .to_owned()
        .append(&["..", ".."])
}

#[must_use]
pub fn tests_dir() -> PathBuf {
    workspace_root().append(&["tests"])
}

#[must_use]
pub fn src_dir() -> PathBuf {
    tests_dir().append(&["src"])
}

#[must_use]
pub(crate) fn src_for(stem: &str) -> PathBuf {
    let mut result = tests_dir().append(&["src", stem]);
    let set = result.add_extension("i");
    assert!(set, "Failed to set extension");
    result
}

#[must_use]
pub fn output_for(folder: &str, stem: &str, extension: &str, mode: Mode) -> PathBuf {
    let mut result = tests_dir().append(&[folder, mode.into(), stem]);
    let set = result.set_extension(match mode {
        Mode::Pass => extension,
        Mode::Fail => "txt",
    });
    assert!(set, "Failed to set extension");
    result
}
