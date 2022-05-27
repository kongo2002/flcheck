use crate::error::FlError;
use yaml_rust::Yaml;
use yaml_rust::YamlLoader;

pub fn load_yaml(config_file: &str) -> Result<Yaml, FlError> {
    let config_content = std::fs::read_to_string(config_file)?;
    let mut docs = YamlLoader::load_from_str(&config_content)?;
    return Ok(docs.remove(0));
}
