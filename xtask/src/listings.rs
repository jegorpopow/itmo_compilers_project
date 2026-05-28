use core::{fmt, mem};
use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Error};
use culpa::throws;
use derive_where::derive_where;
use testing::paths::{PathExt as _, workspace_root};

use crate::{Both, Stage, folders::TestDirContents};

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
            src_path: srcs.path(&stem),
            stem,
        }
    }

    fn for_dir(mut list: TestDirContents) -> Vec<Self> {
        let mut result: Vec<_> = mem::take(&mut list.stems)
            .into_iter()
            .map(|name| Self::new(&list, name))
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
            Self::Interpreter => "interpreter",
            Self::Codegen => "codegen",
        }
    }

    fn tests_file(self) -> PathBuf {
        workspace_root().append(&["compiler", self.crate_name(), "src", "tests.rs"])
    }

    #[throws]
    fn test_cases(self) -> Both<Vec<TestCase>> {
        self.tests()?.map(TestCase::for_dir)
    }
}

#[test]
#[throws]
fn all_are_up_to_date() {
    for stage in Stage::all() {
        let test_file = stage.tests_file();
        let actual = fs::read_to_string(&test_file).with_context(|| {
            format!(
                "Failed to read the file with {stage:?} tests ({})",
                test_file.display()
            )
        })?;

        for (_, cases) in stage.test_cases()?.iter() {
            for case in cases {
                let expected = case.to_string();
                assert!(
                    actual.contains(&expected),
                    "Test case `{expected}` is missing in {} (expected because {:?} exists).\n\
                    !!!! Consider running `cargo x update-listings`. !!!!",
                    test_file.display(),
                    case.src_path.display(),
                )
            }
        }
    }
}

#[throws]
fn update(path: &Path, test_cases: &Both<Vec<TestCase>>) {
    const LISTING_START: &str = "\n    pass = [\n";
    const LISTING_END: &str = "\n}\n";

    println!("Adding following test cases to {}:", path.display());
    for (mode, cases) in test_cases.iter() {
        println!("\t{mode}:");
        for case in cases {
            println!("\t\t{} => {}", case.ident, case.src_path.display())
        }
    }

    let s =
        fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let (prefix, tail) = s
        .split_once(LISTING_START)
        .with_context(|| format!("Can't find {LISTING_START:?}"))?;
    let (_, tail) = tail
        .split_once(LISTING_END)
        .with_context(|| format!("Can't find {LISTING_END:?}"))?;

    let mut s = prefix.to_owned();

    for (mode, cases) in test_cases.iter() {
        use core::fmt::Write;
        writeln!(s, "\n    {mode} = [")?;
        for case in cases {
            writeln!(s, "        {case}")?
        }
        write!(s, "    ]")?
    }

    s.push_str(LISTING_END);
    s.push_str(tail);

    fs::write(path, s).with_context(|| format!("Failed to write back to {}", path.display()))?
}

#[throws]
pub(crate) fn update_all() {
    for stage in Stage::all() {
        update(&stage.tests_file(), &stage.test_cases()?)
            .with_context(|| format!("Failed to update test cases for {stage:?}"))?
    }
}
