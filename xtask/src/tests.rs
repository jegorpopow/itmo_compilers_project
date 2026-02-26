use super::*;

use anyhow::bail;

#[throws]
fn check_has_all_tests(actual: &TestDirContents, expected: &TestDirContents) {
    let missing: Vec<_> = expected.names.difference(&actual.names).collect();
    if !missing.is_empty() {
        let mut msg = "Some tests are missing:".to_string();
        for name in missing {
            use core::fmt::Write;
            write!(
                &mut msg,
                "{:?} (expected because {:?} exists)",
                actual.name_to_path(name),
                expected.name_to_path(name)
            )
            .expect("Writing to String can't fail")
        }
        bail!("Some tests are missing: {msg}")
    }
}

#[test]
#[throws]
fn all_files_are_used() {
    let srcs = test_sources()?;
    assert_eq!(srcs.extension, "i");
    let lexer_tests = lexer_tests()?;
    assert_eq!(lexer_tests.extension, "txt");
    check_has_all_tests(&lexer_tests, &srcs)?;
    check_has_all_tests(&srcs, &lexer_tests)?;
}

#[test]
#[throws]
fn lexer_has_all_test_cases() {
    let lexer_tests_file = workspace_root()?.join("compiler/src/lexer/tests.rs");
    let actual = fs::read_to_string(&lexer_tests_file).with_context(|| {
        format!("Failed to read the file with lexer tests ({lexer_tests_file:?})")
    })?;
    let expected = lexer_tests()?;
    #[expect(
        clippy::iter_over_hash_type,
        reason = "Do we really care about order of errors?"
    )]
    for name in &expected.names {
        ensure!(
            actual.contains(&*format!(" => {name:?},")),
            "Test case {name:?} is missing in {lexer_tests_file:?} (expected because {:?} exists)",
            expected.name_to_path(name)
        )
    }
}
