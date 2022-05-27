use crate::error::FlError;
use yaml_rust::Yaml;
use yaml_rust::YamlLoader;

pub fn load_yaml(config_file: &str) -> Result<Yaml, FlError> {
    let config_content = std::fs::read_to_string(config_file)?;
    let mut docs = YamlLoader::load_from_str(&config_content)?;
    return Ok(docs.remove(0));
}

pub fn yaml_str_list(yaml: &Yaml) -> Vec<String> {
    let empty_list = Default::default();

    yaml.as_vec()
        .unwrap_or(&empty_list)
        .into_iter()
        .flat_map(|entry| entry.as_str().map(|x| x.to_owned()))
        .collect()
}
