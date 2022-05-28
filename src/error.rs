
#[derive(thiserror::Error, Debug)]
pub enum FlError {
    #[error("failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("failed to parse YAML: {0}")]
    YamlReadError(#[from] yaml_rust::ScanError),
    #[error("invalid configuration: {0}")]
    ConfigValidation(String),
}

#[derive(Debug)]
pub struct PackageValidation {
    pub package_name: String,
    pub error: String,
}

