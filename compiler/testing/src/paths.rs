use std::path::{Path, PathBuf};

use crate::Mode;

pub trait PathExt {
    #[must_use]
    fn append(self, components: &[&str]) -> Self;
}

impl PathExt for PathBuf {
    fn append(mut self, components: &[&str]) -> Self {
        for component in components {
            self.push(component);
        }
        self
    }
}

#[must_use]
pub fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(parent) = manifest_dir.parent()
        && let Some(root) = parent.parent()
    {
        root.to_owned()
    } else {
        manifest_dir.to_owned().append(&["..", ".."])
    }
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
#[cfg(feature = "testing")]
pub(crate) fn src_for(name: &str) -> PathBuf {
    tests_dir().append(&["src", &format!("¡{name}!")])
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
