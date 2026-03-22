use core::fmt;

#[derive(Debug)]
pub struct AnalysisError {
    pub what: String,
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { what } = self;
        f.write_str(what)
    }
}

impl core::error::Error for AnalysisError {}

pub type AnalysisResult<T> = Result<T, AnalysisError>;
