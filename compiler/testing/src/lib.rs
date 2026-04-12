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

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).into())
    }
}

pub type TestResult = anyhow::Result<()>;

#[track_caller]
#[cfg(feature = "testing")]
pub fn run_test<E: fmt::Display>(
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
    expected.assert_eq(&match mode {
        Mode::Pass => match result {
            Ok(actual) => actual,
            Err(e) => bail!("Test expected to pass, but failed with the following result:\n{e}"),
        },
        Mode::Fail => match result {
            Ok(actual) => {
                bail!("Test expected to fail, but passed with the following result:\n{actual}")
            }
            Err(e) => format!("{e}\n"),
        },
    });
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
        mod pass {
            use super::*;
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
        }
        mod fail {
            use super::*;
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
        }
    };
}
