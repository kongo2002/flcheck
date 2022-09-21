use serde::Serialize;
use serde::Serializer;
use std::fmt::Display;

#[derive(thiserror::Error, Debug)]
pub enum FlError {
    #[error("failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("no configuration file found (tried: {0})")]
    NoConfigFound(String),
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
pub enum ValidationType {
    GitDevDependency,
    UnknownDependency,
    DependencyNotAllowed,
    CyclicDependency,
}

impl ValidationType {
    fn as_str(&self) -> &str {
        match self {
            ValidationType::GitDevDependency => "validation:dev-dependency:git",
            ValidationType::UnknownDependency => "validation:dependency:unknown",
            ValidationType::DependencyNotAllowed => "validation:dependency:unallowed",
            ValidationType::CyclicDependency => "validation:dependency:cyclic",
        }
    }
}

impl Display for ValidationType {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for ValidationType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Serialize)]
pub struct PackageValidation {
    pub package_name: String,
    pub error: String,
    pub code: ValidationType,
}
