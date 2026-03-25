use std::{collections::BTreeSet, fs::read_dir, path::PathBuf};

use anyhow::{Context as _, Error, anyhow, ensure};
use culpa::throws;

use testing::Mode;

use crate::Both;

#[derive(Debug)]
pub(crate) struct TestDirContents {
    pub dir: PathBuf,
    pub extension: String,
    pub stems: BTreeSet<String>,
}

impl TestDirContents {
    #[must_use]
    pub(crate) fn stem_to_path(&self, stem: &str) -> PathBuf {
        let Self {
            dir,
            extension,
            stems: _,
        } = self;
        dir.join(format!("{stem}.{extension}"))
    }

    #[throws]
    pub(crate) fn new(dir: PathBuf) -> Self {
        let mut stems = BTreeSet::new();

        if !dir.exists() {
            return Self {
                dir,
                extension: "*".into(),
                stems,
            };
        }

        let mut expected_extension: Option<String> = None;

        for entry in read_dir(&dir).with_context(|| format!("failed to ls {}", dir.display()))? {
            let entry = entry.with_context(|| format!("Error traversing {}", dir.display()))?;
            let filename = entry.file_name().into_string().map_err(|e| {
                anyhow!("Non-unicode file name? Come on! {}", dir.join(e).display())
            })?;
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
            let inserted = stems.insert(name.to_owned());
            debug_assert!(
                inserted,
                "How do you even get two files with the same name?"
            )
        }

        TestDirContents {
            extension: expected_extension
                .with_context(|| format!("Empty directory: {}", dir.display()))?,
            dir,
            stems,
        }
    }
}

impl crate::Stage {
    #[must_use]
    const fn test_dir_name(self) -> &'static str {
        match self {
            Self::Lexer => "lexer",
            Self::Parser => "parser",
            Self::AST => "ast",
            // Self::Interpreter => "run",
        }
    }

    #[throws]
    fn test_dir_for_mode(self, mode: Mode) -> PathBuf {
        use testing::paths::PathExt as _;
        testing::paths::tests_dir().append(&[self.test_dir_name(), mode.into()])
    }

    #[throws]
    pub(crate) fn tests_for_mode(self, mode: Mode) -> TestDirContents {
        TestDirContents::new(self.test_dir_for_mode(mode)?)?
    }

    #[throws]
    pub(crate) fn tests(self) -> Both<TestDirContents> {
        Both {
            pass: self.tests_for_mode(Mode::Pass)?,
            fail: self.tests_for_mode(Mode::Fail)?,
        }
    }
}

#[cfg(test)]
fn check_tests(actual: &Both<TestDirContents>, expected: &TestDirContents) {
    for dir in actual.iter() {
        for stem in &dir.stems {
            assert!(
                expected.stems.contains(stem),
                "There is {}, but no {}",
                dir.stem_to_path(stem).display(),
                expected.stem_to_path(stem).display(),
            )
        }
    }
    for stem in &expected.stems {
        assert!(
            actual.pass.stems.contains(stem) || actual.fail.stems.contains(stem),
            "There is {}, but no {} or {}",
            expected.stem_to_path(stem).display(),
            actual.pass.stem_to_path(stem).display(),
            actual.fail.stem_to_path(stem).display(),
        )
    }
}

#[test]
#[throws]
fn all_files_are_used() {
    for stage in crate::Stage::all() {
        let expected = match stage.prev() {
            Some(prev) => prev.tests_for_mode(Mode::Pass),
            None => TestDirContents::new(testing::paths::src_dir()),
        }?;
        let tests = stage.tests()?;
        check_tests(&tests, &expected)
    }
}
