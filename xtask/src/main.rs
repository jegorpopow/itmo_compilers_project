use core::{fmt, mem};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Error, anyhow, ensure};
use culpa::throws;
use derive_where::derive_where;
use strum::{EnumIter, IntoEnumIterator};

mod cli;

#[cfg(test)]
mod tests;

#[derive(Debug)]
struct TestDirContents {
    dir: PathBuf,
    extension: String,
    names: BTreeSet<String>,
}

impl TestDirContents {
    #[must_use]
    fn name_to_path(&self, name: &str) -> PathBuf {
        let Self {
            dir,
            extension,
            names: _,
        } = self;
        dir.join(format!("{name}.{extension}"))
    }
}

#[throws]
fn list_tests(dir: PathBuf) -> TestDirContents {
    let mut expected_extension: Option<String> = None;

    let mut names = BTreeSet::new();

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

fn path_append(mut path: PathBuf, components: &[&str]) -> PathBuf {
    for component in components {
        path.push(component);
    }
    path
}

#[throws]
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .context("failed to find workspace root")?
}

#[throws]
fn tests_dir() -> PathBuf {
    path_append(workspace_root()?, &["tests"])
}

#[throws]
fn srcs_dir() -> PathBuf {
    path_append(tests_dir()?, &["src"])
}

#[throws]
fn test_sources() -> TestDirContents {
    list_tests(srcs_dir()?).context("Failed to get a list test sources")?
}

#[derive(Clone, Copy, EnumIter)]
enum TestedCrate {
    Lexer,
    Parser,
    AST,
}

impl fmt::Display for TestedCrate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl TestedCrate {
    #[must_use]
    const fn name(self) -> &'static str {
        match self {
            Self::Lexer => "lexer",
            Self::Parser => "parser",
            Self::AST => "ast",
        }
    }

    #[throws]
    fn tests_file(self) -> PathBuf {
        path_append(
            workspace_root()?,
            &["compiler", self.name(), "src", "tests.rs"],
        )
    }
}

#[derive_where(Ord, PartialOrd, Eq, PartialEq)]
struct TestCase {
    ident: String,

    #[derive_where(skip)]
    filename: String,
    #[derive_where(skip)]
    src_path: PathBuf,
}

impl fmt::Display for TestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            ident,
            filename,
            src_path: _,
        } = self;
        write!(f, "{ident} => {filename:?}")
    }
}

impl TestCase {
    #[must_use]
    fn new(srcs: &TestDirContents, filename: String) -> Self {
        Self {
            ident: filename.to_lowercase(),
            src_path: srcs.name_to_path(&filename),
            filename,
        }
    }

    #[throws]
    fn all() -> Vec<Self> {
        let mut srcs = test_sources()?;
        let mut result: Vec<Self> = mem::take(&mut srcs.names)
            .into_iter()
            .map(|name| Self::new(&srcs, name))
            .collect();
        result.sort_unstable();
        result
    }
}

#[throws]
fn update_listing(path: &Path, test_cases: &[TestCase]) {
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
fn update_listings() {
    let test_cases = TestCase::all()?;

    for target in TestedCrate::iter() {
        update_listing(&target.tests_file()?, &test_cases)
            .with_context(|| format!("Failed to update test cases for {target}"))?
    }
}

#[throws]
fn main() {
    match cli::Task::parse() {
        cli::Task::UpdateListings => update_listings()?,
    }
}
