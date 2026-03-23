use super::*;

impl TestedCrate {
    #[throws]
    fn test_dir(self) -> PathBuf {
        path_append(tests_dir()?, &[self.name()])
    }

    #[throws]
    fn tests(self) -> TestDirContents {
        list_tests(self.test_dir()?)?
    }
}

fn diff<'a>(
    lhs: &'a TestDirContents,
    rhs: &'a TestDirContents,
) -> impl Iterator<Item = [PathBuf; 2]> + use<'a> {
    lhs.names
        .difference(&rhs.names)
        .map(|name| [lhs.name_to_path(name), rhs.name_to_path(name)])
}

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
    let srcs = test_sources()?;
    for target in TestedCrate::iter() {
        let tests = target.tests()?;
        check_has_all_tests(&tests, &srcs)
            .with_context(|| format!("Some tests are missing for {target}"))?;
        check_has_all_tests(&srcs, &tests)
            .with_context(|| format!("Unexpected tests for {target}"))?;
    }
}

#[test]
#[throws]
fn listings_are_up_to_date() {
    let test_cases = TestCase::all()?;

    for target in TestedCrate::iter() {
        let test_file = target.tests_file()?;
        let actual = fs::read_to_string(&test_file).with_context(|| {
            format!(
                "Failed to read the file with {target} tests ({})",
                test_file.display()
            )
        })?;
        for case in &test_cases {
            let expected = case.to_string();
            assert!(
                actual.contains(&expected),
                "Test case `{expected}` is missing in {target} (expected because {:?} exists).\n\
                !!!! Consider running `cargo x update-listings`. !!!!",
                case.src_path.display()
            )
        }
    }
}
