use crate::error::FlError;
use crate::error::FlError::ConfigValidation;
use crate::error::ValidationLevel;
use crate::error::ValidationType;
use crate::util::load_yaml;
use crate::util::yaml_str_list;
use regex::Regex;
use yaml_rust::Yaml;

#[derive(Debug, PartialEq)]
pub struct PackageType {
    pub name: String,
    pub prefixes: Vec<String>,
    pub includes: Vec<String>,
}

impl PackageType {
    pub fn matches_prefix(&self, dir_name: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| dir_name.starts_with(prefix))
    }
}

#[derive(Debug)]
pub struct Config {
    pub package_types: Vec<PackageType>,
    pub blacklist: Vec<Regex>,
    pub validations: Vec<(ValidationType, ValidationLevel)>,
    pub public_repositories: Vec<Regex>,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.package_types == other.package_types
            && self.validations == other.validations
            && regex_str_list(&self.blacklist) == regex_str_list(&other.blacklist)
            && regex_str_list(&self.public_repositories)
                == regex_str_list(&other.public_repositories)
    }
}

impl Config {
    pub fn is_valid(&self) -> bool {
        !self.package_types.is_empty()
    }

    pub fn is_blacklisted(&self, full_path: &str) -> bool {
        self.blacklist.iter().any(|regex| regex.is_match(full_path))
    }

    pub fn is_public_repo(&self, git_repo: &str) -> bool {
        self.public_repositories
            .iter()
            .any(|regex| regex.is_match(git_repo))
    }

    /// Determine the configured `ValidationLevel` for the given
    /// `ValidationType`.
    ///
    /// Defaults to `ValidationLevel::Error` if not configured.
    pub fn validation_level(&self, validation_type: &ValidationType) -> ValidationLevel {
        self.validations
            .iter()
            .find(|(typ, _)| typ == validation_type)
            .map(|(_, level)| level.clone())
            .unwrap_or(ValidationLevel::Error)
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

        let validations = config_yaml["validations"]
            .as_hash()
            .unwrap_or(&empty)
            .into_iter()
            .map(|(key, value)| {
                let type_str = key.as_str().unwrap_or("");
                let level_str = value.as_str().unwrap_or("");
                let validation_type = ValidationType::from_str(type_str)
                    .ok_or(FlError::InvalidValidationType(type_str.to_owned()))?;
                let validation_level = ValidationLevel::from_str(level_str).ok_or(
                    FlError::InvalidValidationLevel(level_str.to_owned(), type_str.to_owned()),
                )?;

                Ok((validation_type, validation_level))
            })
            .collect::<Result<Vec<(ValidationType, ValidationLevel)>, FlError>>()?;

        let public_repositories = regex_list(
            yaml_str_list(&config_yaml["public_repositories"]),
            "public repository",
        )?;

        let blacklist = regex_list(yaml_str_list(&config_yaml["blacklist"]), "blacklist")?;

        let config = Config {
            package_types: package_types.collect(),
            blacklist: blacklist,
            validations: validations,
            public_repositories: public_repositories,
        };

        config.validate()
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

fn regex_str_list(regexes: &Vec<Regex>) -> Vec<&str> {
    regexes.iter().map(|rgx| rgx.as_str()).collect::<Vec<_>>()
}

fn regex_list(strings: Vec<String>, config_type: &str) -> Result<Vec<Regex>, FlError> {
    strings
        .iter()
        .map(|entry| {
            Regex::new(entry).map_err(|_| {
                ConfigValidation(format!(
                    "invalid regex in {} entry: '{}'",
                    config_type, entry
                ))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::config::PackageType;
    use crate::Config;
    use regex::Regex;
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
        let mut docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix:
    ",
        )
        .unwrap();
        let config = Config::load_from_yaml(docs.remove(0));

        assert_eq!(config.is_err(), true);
    }

    #[test]
    fn load_config_minimal() {
        let mut docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix: app
    ",
        )
        .unwrap();
        let config = Config::load_from_yaml(docs.remove(0)).unwrap();

        assert_eq!(
            config,
            Config {
                package_types: vec![PackageType {
                    name: "app".to_owned(),
                    prefixes: vec!["app".to_owned()],
                    includes: Vec::new()
                }],
                blacklist: Vec::new(),
                validations: Vec::new(),
                public_repositories: Vec::new(),
            }
        )
    }

    #[test]
    fn load_config_blacklist() {
        let mut docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix: app
blacklist:
- one
- two
    ",
        )
        .unwrap();
        let config = Config::load_from_yaml(docs.remove(0)).unwrap();

        assert_eq!(
            config,
            Config {
                package_types: vec![PackageType {
                    name: "app".to_owned(),
                    prefixes: vec!["app".to_owned()],
                    includes: Vec::new()
                }],
                blacklist: vec![Regex::new("one").unwrap(), Regex::new("two").unwrap()],
                validations: Vec::new(),
                public_repositories: Vec::new(),
            }
        )
    }

    #[test]
    fn load_config_public_repositories() {
        let mut docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix: app
public_repositories:
- one
- two
    ",
        )
        .unwrap();
        let config = Config::load_from_yaml(docs.remove(0)).unwrap();

        assert_eq!(
            config,
            Config {
                package_types: vec![PackageType {
                    name: "app".to_owned(),
                    prefixes: vec!["app".to_owned()],
                    includes: Vec::new()
                }],
                blacklist: Vec::new(),
                validations: Vec::new(),
                public_repositories: vec![Regex::new("one").unwrap(), Regex::new("two").unwrap()],
            }
        )
    }

    #[test]
    fn load_config_invalid_yaml() {
        let docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix: app
public_repositories:
- **
    ",
        );

        assert_eq!(docs.is_err(), true)
    }

    #[test]
    fn load_config_invalid_public_repositories() {
        let mut docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix: app
public_repositories:
- '**'
    ",
        )
        .unwrap();
        let config = Config::load_from_yaml(docs.remove(0));

        assert_eq!(config.is_err(), true)
    }

    #[test]
    fn load_config_multiple_prefixes() {
        let mut docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix:
    - 'app-'
    - 'app_'
    ",
        )
        .unwrap();
        let config = Config::load_from_yaml(docs.remove(0)).unwrap();

        assert_eq!(
            config,
            Config {
                package_types: vec![PackageType {
                    name: "app".to_owned(),
                    prefixes: vec!["app-".to_owned(), "app_".to_owned()],
                    includes: Vec::new()
                }],
                blacklist: Vec::new(),
                validations: Vec::new(),
                public_repositories: Vec::new(),
            }
        )
    }

    #[test]
    fn load_config_includes() {
        let mut docs = YamlLoader::load_from_str(
            "
package_types:
  app:
    dir_prefix: app
  pkg:
    dir_prefix: pkg
    includes:
    - app
    ",
        )
        .unwrap();
        let config = Config::load_from_yaml(docs.remove(0)).unwrap();

        assert_eq!(
            config,
            Config {
                package_types: vec![
                    PackageType {
                        name: "app".to_owned(),
                        prefixes: vec!["app".to_owned()],
                        includes: Vec::new()
                    },
                    PackageType {
                        name: "pkg".to_owned(),
                        prefixes: vec!["pkg".to_owned()],
                        includes: vec!["app".to_owned()]
                    }
                ],
                blacklist: Vec::new(),
                validations: Vec::new(),
                public_repositories: Vec::new(),
            }
        )
    }
}
