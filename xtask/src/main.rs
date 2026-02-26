use core::fmt;
use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Error, anyhow, ensure};
use culpa::throws;

#[derive(Debug)]
struct TestDirContents {
    dir: PathBuf,
    extension: String,
    names: HashSet<String>,
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
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .context("failed to find workspace root")?
}

#[throws]
fn tests_dir() -> PathBuf {
    workspace_root()?.join("tests")
}

#[throws]
fn lexer_tests() -> TestDirContents {
    list_tests(tests_dir()?.join("lexer")).context("Failed to get a list of lexer tests")?
}

#[throws]
fn lexer_tests_file() -> PathBuf {
    workspace_root()?.join("compiler/src/lexer/tests.rs")
}

#[cfg(test)]
mod tests;

struct TestCase<'a> {
    ident: String,
    name: &'a str,
}

impl fmt::Display for TestCase<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { ident, name } = self;
        write!(f, "{ident} => {name:?}")
    }
}

#[throws]
fn main() {
    let tests = lexer_tests()?;
    debug_assert_eq!(tests.extension, "txt");
    println!(
        "Adding following test cases (found in {}):",
        tests.dir.display()
    );

    let mut test_cases = tests
        .names
        .iter()
        .map(|name| TestCase {
            ident: name.replace(' ', "_"),
            name,
        })
        .collect::<Vec<_>>();
    test_cases.sort_by(|a, b| a.ident.cmp(&b.ident));

    for case in &test_cases {
        println!("\tfrom {}:\n\t\t{case},", tests.name_to_path(case.name).display())
    }

    let path = lexer_tests_file()?;
    let s =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;

    let (prefix, remainder) = s
        .split_once("tests!")
        .with_context(|| format!("Cannot find \"tests!\" in {}", path.display()))?;
    let (_, remainder) = remainder.split_once("\n];").context("TODO")?;

    let s = &mut prefix.to_owned();
    s.push_str("tests! [\n");
    for case in test_cases {
        use core::fmt::Write;
        writeln!(s, "    {case},").expect("Writing to String won't fail")
    }
    s.push_str("];");
    s.push_str(remainder);

    fs::write(&path, s).with_context(|| format!("Failed to write back to {}", path.display()))?
}
