use crate::error::FlError;
use crate::error::FlError::ConfigValidation;
use crate::util::load_yaml;
use crate::util::yaml_str_list;
use regex::Regex;

#[derive(Debug)]
pub struct Config {
    pub package_types: Vec<PackageType>,
    blacklist: Vec<Regex>,
}

#[derive(Debug)]
pub struct PackageType {
    pub name: String,
    pub prefix: String,
    pub includes: Vec<String>,
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
        let empty = Default::default();

        let package_types = config_yaml["package_types"]
            .as_hash()
            .unwrap_or(&empty)
            .into_iter()
            .flat_map(|(key, value)| {
                let name = key.as_str().unwrap_or("").to_owned();
                let prefix = value["prefix"].as_str().unwrap_or("").to_owned();
                let includes = yaml_str_list(&value["includes"]);

                if name.is_empty() {
                    None
                } else {
                    Some(PackageType {
                        name,
                        prefix,
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
            return Err(ConfigValidation("no package types configured".to_owned()))
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
                        if package.prefix.is_empty() {
                            let err = format!("package '{}': empty prefix", package.name);
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
