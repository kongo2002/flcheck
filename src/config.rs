use crate::error::FlError;
use crate::error::FlError::ConfigValidation;
use crate::util::load_yaml;
use crate::util::yaml_str_list;
use regex::Regex;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct Config {
    pub package_types: Vec<PackageType>,
    blacklist: Vec<Regex>,
}

#[derive(Debug, PartialEq)]
pub struct PackageType {
    pub name: String,
    pub prefixes: Vec<String>,
    pub includes: Vec<String>,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.package_types == other.package_types
            && self
                .blacklist
                .iter()
                .map(|rgx| rgx.as_str())
                .collect::<Vec<_>>()
                == other
                    .blacklist
                    .iter()
                    .map(|rgx| rgx.as_str())
                    .collect::<Vec<_>>()
    }
}

impl PackageType {
    pub fn matches_prefix(&self, dir_name: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| dir_name.starts_with(prefix))
    }
}

impl Config {
    pub fn is_valid(&self) -> bool {
        !self.package_types.is_empty()
    }

    pub fn is_blacklisted(&self, full_path: &str) -> bool {
        self.blacklist.iter().any(|entry| entry.is_match(full_path))
    }

    pub fn load(file: &str) -> Result<Config, FlError> {
        let config_yaml = load_yaml(file)?;
        return Config::load_from_yaml(config_yaml);
    }

    fn load_from_yaml(config_yaml: Yaml) -> Result<Config, FlError> {
        let empty = Default::default();

        let package_types = config_yaml["package_types"]
            .as_hash()
            .unwrap_or(&empty)
            .into_iter()
            .flat_map(|(key, value)| {
                let name = key.as_str().unwrap_or("").to_owned();
                let includes = yaml_str_list(&value["includes"]);

                let prefix = value["dir_prefix"].as_str().unwrap_or("").to_owned();
                let prefixes = if prefix.is_empty() {
                    yaml_str_list(&value["dir_prefix"])
                } else {
                    vec![prefix]
                };

                if name.is_empty() {
                    None
                } else {
                    Some(PackageType {
                        name,
                        prefixes,
                        includes: includes,
                    })
                }
            });

        let blacklist: Result<Vec<Regex>, _> = yaml_str_list(&config_yaml["blacklist"])
            .iter()
            .map(|entry| {
                Regex::new(entry)
                    .map_err(|_| ConfigValidation(format!("invalid blacklist entry: '{}'", entry)))
            })
            .collect();

        blacklist
            .map(|bl| Config {
                package_types: package_types.collect(),
                blacklist: bl,
            })
            .and_then(|c| c.validate())
    }

    fn package_exists(&self, package_name: &str) -> bool {
        self.package_types
            .iter()
            .any(|package| package.name == package_name)
    }

    fn validate(self) -> Result<Config, FlError> {
        if !self.is_valid() {
            return Err(ConfigValidation("no package types configured".to_owned()));
        }

        self.package_types
            .iter()
            .flat_map(|package| {
                let unknown_include = package
                    .includes
                    .iter()
                    .find(|include| !self.package_exists(include));

                unknown_include
                    .map(|include| {
                        let err =
                            format!("package '{}': unknown include '{}'", package.name, include);
                        ConfigValidation(err.to_owned())
                    })
                    .or_else(|| {
                        if package.prefixes.is_empty() {
                            let err = format!("package '{}': empty dir_prefix", package.name);
                            Some(ConfigValidation(err.to_owned()))
                        } else {
                            None
                        }
                    })
            })
            .next()
            .map(Err)
            .unwrap_or(Ok(self))
    }
}

#[cfg(test)]
mod tests {
    use crate::config::PackageType;
    use crate::Config;
    use yaml_rust::YamlLoader;

    #[test]
    fn load_config_empty() {
        let docs = YamlLoader::load_from_str("").unwrap();

        assert_eq!(docs.is_empty(), true);
    }

    #[test]
    fn load_config_no_package_types() {
        let mut docs = YamlLoader::load_from_str("package_types:").unwrap();
        let config = Config::load_from_yaml(docs.remove(0));

        assert_eq!(config.is_err(), true);
    }

    #[test]
    fn load_config_missing_dir_prefix() {
        let mut docs =
            YamlLoader::load_from_str("package_types:\n  app:\n    dir_prefix:").unwrap();
        let config = Config::load_from_yaml(docs.remove(0));

        assert_eq!(config.is_err(), true);
    }

    #[test]
    fn load_config_minimal() {
        let mut docs =
            YamlLoader::load_from_str("package_types:\n  app:\n    dir_prefix: app").unwrap();
        let config = Config::load_from_yaml(docs.remove(0)).unwrap();

        assert_eq!(
            config,
            Config {
                package_types: vec![PackageType {
                    name: "app".to_owned(),
                    prefixes: vec!["app".to_owned()],
                    includes: Vec::new()
                }],
                blacklist: Vec::new()
            }
        )
    }
}
