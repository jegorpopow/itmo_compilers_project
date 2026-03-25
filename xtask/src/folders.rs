use std::{collections::BTreeSet, fs::read_dir, path::PathBuf};

use anyhow::{Context as _, Error, anyhow, ensure};
use culpa::throws;

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
        let mut expected_extension: Option<String> = None;

        let mut stems = BTreeSet::new();

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

    #[throws]
    pub(crate) fn srcs() -> Self {
        Self::new(testing::paths::src_dir())?
    }
}

#[cfg(test)]
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
    fn test_dir(self) -> PathBuf {
        use testing::paths::PathExt as _;
        testing::paths::src_dir().append(&[self.test_dir_name()])
    }

    #[throws]
    fn tests(self) -> TestDirContents {
        TestDirContents::new(self.test_dir()?)?
    }
}

#[cfg(test)]
fn diff<'a>(
    lhs: &'a TestDirContents,
    rhs: &'a TestDirContents,
) -> impl Iterator<Item = [PathBuf; 2]> + use<'a> {
    lhs.stems
        .difference(&rhs.stems)
        .map(|stem| [lhs.stem_to_path(stem), rhs.stem_to_path(stem)])
}

#[cfg(test)]
fn check_has_all_tests(actual: &TestDirContents, expected: &TestDirContents) -> anyhow::Result<()> {
    let missing: Vec<_> = diff(actual, expected)
        .chain(diff(expected, actual))
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        let mut message = "Some files are missing:\n".to_string();
        for [present, missing] in missing {
            use core::fmt::Write;
            writeln!(
                &mut message,
                "{missing:?} (expected because {present:?} exists)",
            )
            .expect("Writing to String can't fail")
        }
        Err(Error::msg(message))
    }
}

#[test]
#[throws]
fn all_files_are_used() {
    let srcs = TestDirContents::srcs()?;
    for stage in crate::Stage::all() {
        let tests = stage.tests()?;
        check_has_all_tests(&tests, &srcs)
            .with_context(|| format!("Some tests are missing for {stage:?}"))?;
        check_has_all_tests(&srcs, &tests)
            .with_context(|| format!("Unexpected tests for {stage:?}"))?;
    }
}
