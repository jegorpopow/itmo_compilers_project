#![allow(dead_code)]

pub struct AnalysisError {
    pub what: String,
}

pub type AnalysisResult<T> = Result<T, AnalysisError>;
