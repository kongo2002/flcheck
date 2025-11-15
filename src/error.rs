use serde::Serialize;
use serde::Serializer;
use std::fmt::Display;
use std::slice::Iter;

#[derive(thiserror::Error, Debug)]
pub enum FlError {
    #[error("failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("file does not exist (tried: {0})")]
    FileDoesNotExist(String),
    #[error("file is empty ({0})")]
    EmptyFile(String),
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
    #[error("invalid validation type '{0}'")]
    InvalidValidationType(String),
    #[error("invalid validation level '{0}' [{1}] (supported: error, warn, none)")]
    InvalidValidationLevel(String, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationLevel {
    Error,
    Warning,
    None,
}

impl ValidationLevel {
    fn as_str(&self) -> &str {
        match self {
            ValidationLevel::Error => "error",
            ValidationLevel::Warning => "warn",
            ValidationLevel::None => "none",
        }
    }

    pub fn values() -> Iter<'static, ValidationLevel> {
        static LEVELS: [ValidationLevel; 3] = [
            ValidationLevel::Error,
            ValidationLevel::Warning,
            ValidationLevel::None,
        ];
        LEVELS.iter()
    }

    pub fn from_str(input: &str) -> Option<ValidationLevel> {
        ValidationLevel::values()
            .find(|level| level.as_str() == input)
            .cloned()
    }
}

impl Display for ValidationLevel {
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for ValidationLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationType {
    GitDevDependency,
    UnknownDependency,
    DependencyNotAllowed,
    CyclicDependency,
    NonGitDependencyInPublicPackage,
}

impl ValidationType {
    fn as_str(&self) -> &str {
        match self {
            ValidationType::GitDevDependency => "validation:dev-dependency:git",
            ValidationType::UnknownDependency => "validation:dependency:unknown",
            ValidationType::DependencyNotAllowed => "validation:dependency:unallowed",
            ValidationType::CyclicDependency => "validation:dependency:cyclic",
            ValidationType::NonGitDependencyInPublicPackage => {
                "validation:public:dependency:non-git"
            }
        }
    }

    pub fn values() -> Iter<'static, ValidationType> {
        static TYPES: [ValidationType; 5] = [
            ValidationType::GitDevDependency,
            ValidationType::UnknownDependency,
            ValidationType::DependencyNotAllowed,
            ValidationType::CyclicDependency,
            ValidationType::NonGitDependencyInPublicPackage,
        ];
        TYPES.iter()
    }

    pub fn from_str(input: &str) -> Option<ValidationType> {
        ValidationType::values()
            .find(|typ| typ.as_str() == input)
            .cloned()
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

#[derive(Debug, Serialize, PartialEq)]
pub struct PackageValidation {
    pub package_name: String,
    pub error: String,
    pub description: Option<String>,
    pub code: ValidationType,
    pub level: ValidationLevel,
}
