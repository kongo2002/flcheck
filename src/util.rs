use crate::error::FlError;
use crate::FlError::NoConfigFound;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use yaml_rust::Yaml;
use yaml_rust::YamlLoader;

pub fn load_yaml(config_file: &str) -> Result<Yaml, FlError> {
    if !std::path::Path::new(config_file).exists() {
        return Err(NoConfigFound(config_file.to_owned()));
    }

    let config_content = std::fs::read_to_string(config_file)?;
    let mut docs = YamlLoader::load_from_str(&config_content)?;

    if docs.is_empty() {
        Err(NoConfigFound(config_file.to_owned()))
    } else {
        Ok(docs.remove(0))
    }
}

pub fn yaml_str_list(yaml: &Yaml) -> Vec<String> {
    let empty_list = Default::default();

    yaml.as_vec()
        .unwrap_or(&empty_list)
        .into_iter()
        .flat_map(|entry| entry.as_str().map(|x| x.to_owned()))
        .filter(|value| !value.is_empty())
        .collect()
}

/// Helper function that normalizes (or canonicalizes) the given `path_str`. This function does not
/// care if the actual directories exist or not.
///
/// This is in contrast to `std::fs::canonicalize` which is the actual motivation for this
/// function in the first place.
pub fn normalize_path_str(path_str: String) -> PathBuf {
    let path = Path::new(path_str.as_str());
    return normalize_path(path);
}

/// Helper function that normalizes (or canonicalizes) the given `path`. This function does not
/// care if the actual directories exist or not.
///
/// This is in contrast to `std::fs::canonicalize` which is the actual motivation for this
/// function in the first place.
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}
