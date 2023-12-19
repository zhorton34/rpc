use thiserror::Error;
use reqwest;
use lopdf;

#[derive(Debug, Error)]
pub enum PdfExtractError {
    #[error("Failed to download PDF")]
    DownloadError(#[from] reqwest::Error),
    #[error("Failed to extract text from PDF")]
    ExtractionError(#[from] lopdf::Error),
}

#[derive(Debug, Error)]
pub enum ParseSouthLawPropertiesError {
    #[error("Empty input error")]
    EmptyInputError,
    
    #[error("No valid data found in the input")]
    NoValidDataError,

    #[error("Unexpected content encountered in data")]
    UnexpectedContentError,
}
