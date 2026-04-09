use core::{cmp::Ordering, fmt};
use std::{
    collections::BTreeSet,
    fs::{File, read_dir},
    path::PathBuf,
};

use anyhow::{Context, Error, anyhow, ensure};
use culpa::throws;

use testing::Mode;

use crate::{Both, Stage};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Extension {
    Hola,
    Boring(String),
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hola => f.write_str("¡…!"),
            Self::Boring(e) => write!(f, ".{e}"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct TestDirContents {
    pub dir: PathBuf,
    pub extension: Extension,
    pub stems: BTreeSet<String>,
}

impl TestDirContents {
    #[must_use]
    pub(crate) fn path(&self, stem: &str) -> PathBuf {
        let Self {
            dir,
            extension,
            stems: _,
        } = self;

        dir.join(match extension {
            Extension::Hola => format!("¡{stem}!"),
            Extension::Boring(extension) => format!("{stem}.{extension}"),
        })
    }

    #[throws]
    pub(crate) fn new(dir: PathBuf) -> Self {
        let mut stems = BTreeSet::new();

        if !dir.exists() {
            return Self {
                dir,
                extension: Extension::Hola,
                stems,
            };
        }

        let mut expected_extension: Option<Extension> = None;

        for entry in read_dir(&dir).with_context(|| format!("failed to ls {}", dir.display()))? {
            let entry = entry.with_context(|| format!("Error traversing {}", dir.display()))?;
            let filename = entry.file_name().into_string().map_err(|e| {
                anyhow!("Non-unicode file name? Come on! {}", dir.join(e).display())
            })?;
            let (name, extension) = match filename.rsplit_once('.') {
                Some((name, extension)) => (name, Extension::Boring(extension.to_owned())),
                None => (
                    filename
                        .strip_suffix("!")
                        .and_then(|name| name.strip_prefix("¡"))
                        .with_context(|| {
                            format!(
                                "{} does not have an extension",
                                dir.join(&filename).display()
                            )
                        })?,
                    Extension::Hola,
                ),
            };
            match &expected_extension {
                Some(expected) => ensure!(
                    expected == &extension,
                    "Expected {} to have extension {expected}",
                    dir.join(filename).display()
                ),
                None => expected_extension = Some(extension),
            }

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

    #[throws]
    fn create_new(&self, name: &str) -> PathBuf {
        let path = self.path(name);
        let file: File = File::create_new(&path).with_context(|| {
            format!(
                "Could not create {} (maybe it already exists?)",
                path.display()
            )
        })?;
        drop(file);
        path
    }
}

impl Stage {
    #[must_use]
    const fn test_dir_name(self) -> &'static str {
        match self {
            Self::Lexer => "lexer",
            Self::Parser => "parser",
            Self::AST => "ast",
            Self::Run => "run",
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
    for (_, dir) in actual.iter() {
        for stem in &dir.stems {
            assert!(
                expected.stems.contains(stem),
                "There is {}, but no {}",
                dir.path(stem).display(),
                expected.path(stem).display(),
            )
        }
    }
    for stem in &expected.stems {
        assert!(
            actual.pass.stems.contains(stem) || actual.fail.stems.contains(stem),
            "There is {}, but no {} or {}",
            expected.path(stem).display(),
            actual.pass.path(stem).display(),
            actual.fail.path(stem).display(),
        )
    }
}

#[test]
#[throws]
fn all_files_are_used() {
    for stage in Stage::all() {
        let expected = match stage.prev() {
            Some(prev) => prev.tests_for_mode(Mode::Pass),
            None => TestDirContents::srcs(),
        }?;
        let tests = stage.tests()?;
        check_tests(&tests, &expected)
    }
}

#[throws]
pub(super) fn add_test(name: &str, fail_stage: Option<Stage>) -> PathBuf {
    let result = TestDirContents::srcs()?
        .create_new(name)
        .with_context(|| format!("Could not create a source file for {name:?}"))?;
    println!("Created {}", result.display());
    for stage in Stage::all() {
        let mode = match fail_stage.and_then(|s| s.partial_cmp(stage)) {
            Some(Ordering::Less) => continue,
            Some(Ordering::Equal) => Mode::Fail,
            Some(Ordering::Greater) | None => Mode::Pass,
        };
        let path = stage
            .tests_for_mode(mode)?
            .create_new(name)
            .with_context(|| format!("Error adding test {name:?} to {stage:?}"))?;
        println!("Created {}", path.display())
    }
    result
}

#[test]
#[throws]
fn no_trailing_whitespace_in_src() {
    use std::io::{BufRead, BufReader};

    let srcs = TestDirContents::srcs()?;
    for stem in &srcs.stems {
        let path = srcs.path(stem);
        for (i, line) in BufReader::new(
            File::open(&path).with_context(|| format!("Cannot open {}", path.display()))?,
        )
        .lines()
        .enumerate()
        {
            let i = i + 1;
            let line =
                line.with_context(|| format!("Cannot read line {i} from {}", path.display()))?;
            assert_eq!(
                line,
                line.trim_end(),
                "Trailing whitespace at {}:{i}",
                path.display()
            )
        }
    }
}
