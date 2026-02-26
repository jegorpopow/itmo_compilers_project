use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Error, anyhow, ensure};
use culpa::throws;

#[derive(Debug)]
pub struct TestDirContents {
    pub dir: PathBuf,
    pub extension: String,
    pub names: HashSet<String>,
}

impl TestDirContents {
    #[must_use]
    pub fn name_to_path(&self, name: &str) -> PathBuf {
        let Self {
            dir,
            extension,
            names: _,
        } = self;
        dir.join(format!("{name}.{extension}"))
    }
}

#[throws]
pub fn list_tests(dir: PathBuf) -> TestDirContents {
    let mut expected_extension: Option<String> = None;

    let mut names = HashSet::new();

    for entry in fs::read_dir(&dir).with_context(|| format!("failed to ls {}", dir.display()))? {
        let entry = entry.with_context(|| format!("Error traversing {}", dir.display()))?;
        let filename = entry
            .file_name()
            .into_string()
            .map_err(|e| anyhow!("Non-unicode file name? Come on! {}", dir.join(e).display()))?;
        let (name, extension) = filename.rsplit_once('.').with_context(|| {
            format!(
                "{} does not have an extension",
                dir.join(&filename).display()
            )
        })?;
        let expected = expected_extension.get_or_insert_with(|| extension.to_owned());
        ensure!(
            expected == extension,
            "Expected {} to have extension .{expected}",
            dir.join(filename).display()
        );
        let inserted = names.insert(name.to_owned());
        debug_assert!(
            inserted,
            "How do you even get two files with the same name?"
        )
    }

    TestDirContents {
        extension: expected_extension
            .with_context(|| format!("Empty directory: {}", dir.display()))?,
        dir,
        names,
    }
}

#[throws]
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .context("failed to find workspace root")?
}

#[throws]
pub fn tests_dir() -> PathBuf {
    workspace_root()?.join("tests")
}

#[throws]
pub fn test_sources() -> TestDirContents {
    list_tests(tests_dir()?.join("src")).context("Failed to get a list test sources")?
}

#[throws]
pub fn lexer_tests() -> TestDirContents {
    list_tests(tests_dir()?.join("lexer")).context("Failed to get a list of lexer tests")?
}

#[cfg(test)]
mod tests;
