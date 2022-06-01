
#[derive(thiserror::Error, Debug)]
pub enum FlError {
    #[error("failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("failed to parse YAML: {0}")]
    YamlReadError(#[from] yaml_rust::ScanError),
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("invalid configuration: {0}")]
    ConfigValidation(String),
    #[error("validation: {0} error(s)")]
    ValidationError(u32),
    #[error("no input files found (directory: {0})")]
    NoInputFiles(String),

}

#[derive(Debug)]
pub struct PackageValidation {
    pub package_name: String,
    pub error: String,
}

