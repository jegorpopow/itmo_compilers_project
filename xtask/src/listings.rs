use core::{fmt, mem};
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Error};
use culpa::throws;
use derive_where::derive_where;
use testing::paths::{PathExt as _, workspace_root};

use crate::{Stage, folders::TestDirContents};

#[derive_where(Ord, PartialOrd, Eq, PartialEq)]
struct TestCase {
    ident: String,

    #[derive_where(skip)]
    stem: String,

    #[derive_where(skip)]
    src_path: PathBuf,
}

impl fmt::Display for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            ident,
            stem,
            src_path: _,
        } = self;
        write!(f, "{ident} => {stem:?}")
    }
}

impl TestCase {
    #[must_use]
    fn new(srcs: &TestDirContents, stem: String) -> Self {
        Self {
            ident: stem.to_lowercase(),
            src_path: srcs.stem_to_path(&stem),
            stem,
        }
    }

    #[throws]
    fn all() -> Vec<Self> {
        let mut srcs = TestDirContents::srcs()?;
        let mut result: Vec<_> = mem::take(&mut srcs.stems)
            .into_iter()
            .map(|name| Self::new(&srcs, name))
            .collect();
        result.sort_unstable();
        result
    }
}

impl Stage {
    fn crate_name(self) -> &'static str {
        match self {
            Self::Lexer => "lexer",
            Self::Parser => "parser",
            Self::AST => "ast",
        }
    }

    fn tests_file(self) -> PathBuf {
        workspace_root().append(&["compiler", self.crate_name(), "src", "tests.rs"])
    }
}

#[test]
#[throws]
fn all_are_up_to_date() {
    let test_cases = TestCase::all()?;
    for stage in Stage::all() {
        let test_file = stage.tests_file();
        let actual = fs::read_to_string(&test_file).with_context(|| {
            format!(
                "Failed to read the file with {stage:?} tests ({})",
                test_file.display()
            )
        })?;
        for case in &test_cases {
            let expected = case.to_string();
            assert!(
                actual.contains(&expected),
                "Test case `{expected}` is missing in {stage:?} (expected because {:?} exists).\n\
                !!!! Consider running `cargo x update-listings`. !!!!",
                case.src_path.display()
            )
        }
    }
}

#[throws]
fn update(path: &Path, test_cases: &[TestCase]) {
    println!("Adding following test cases to {}:", path.display());

    for case in test_cases {
        println!("\tfrom {}:\n\t\t{case},", case.src_path.display())
    }

    let s =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let (prefix, remainder) = s
        .split_once("tests!")
        .with_context(|| format!("Cannot find \"tests!\" in {}", path.display()))?;
    let (_, remainder) = remainder
        .split_once("\n];")
        .context("Couldn't find the closing bracket")?;

    let s = &mut prefix.to_owned();
    s.push_str("tests! [\n");
    for case in test_cases {
        use core::fmt::Write;
        writeln!(s, "    {case},").expect("Writing to String won't fail")
    }
    s.push_str("];");
    s.push_str(remainder);

    fs::write(path, s).with_context(|| format!("Failed to write back to {}", path.display()))?
}

#[throws]
pub(crate) fn update_all() {
    let test_cases: Vec<TestCase> = TestCase::all()?;

    for stage in Stage::all() {
        update(&stage.tests_file(), &test_cases)
            .with_context(|| format!("Failed to update test cases for {stage:?}"))?
    }
}
