#[cfg(feature = "testing")]
use core::fmt;

pub mod paths;

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Pass,
    Fail,
}

impl From<Mode> for &'static str {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Pass => "pass",
            Mode::Fail => "fail",
        }
    }
}

pub type TestResult = anyhow::Result<()>;

#[track_caller]
#[cfg(feature = "testing")]
pub fn run_test<E: fmt::Debug>(
    folder: &str,
    out_extension: &str,
    run: fn(&str) -> Result<String, E>,
    mode: Mode,
    test: &str,
) -> TestResult {
    use anyhow::{Context, bail};
    let test_src = paths::src_for(test);
    let source = std::fs::read_to_string(&test_src)
        .with_context(|| format!("Error reading {}", test_src.display()))?;
    let result = run(&source);
    let expected =
        ::expect_test::expect_file![paths::output_for(folder, test, out_extension, mode)];
    match mode {
        Mode::Pass => expected.assert_eq(&result.expect("Test failed but expected to pass")),
        Mode::Fail => match result {
            Ok(actual) => {
                bail!("Test expected to fail, but passed with the following result:\n{actual}")
            }
            Err(e) => expected.assert_debug_eq(&e),
        },
    }
    Ok(())
}

#[cfg(feature = "testing")]
#[macro_export]
macro_rules! tests {
    (
        folder = $folder:literal
        extension = $extension:literal
        fun = $tester:ident
        pass = [
            $($pass_ident:ident => $pass_name:literal)+
        ]
        fail = [
            $($fail_ident:ident => $fail_name:literal)*
        ]
    ) => {
        $(
            #[test]
            fn $pass_ident() -> $crate::TestResult {
                $crate::run_test(
                    $folder,
                    $extension,
                    $tester,
                    $crate::Mode::Pass,
                    $pass_name,
                )
            }
        )+
        $(
            #[test]
            fn $fail_ident() -> $crate::TestResult {
                $crate::run_test(
                    $folder,
                    $extension,
                    $tester,
                    $crate::Mode::Fail,
                    $fail_name,
                )
            }
        )*
    };
}
