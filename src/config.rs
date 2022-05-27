use crate::error::FlError;
use crate::error::FlError::ConfigValidation;
use crate::util::load_yaml;

#[derive(Debug)]
pub struct Config {
    pub package_types: Vec<PackageType>,
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

    pub fn load(file: &str) -> Result<Config, FlError> {
        let config_yaml = load_yaml(file)?;
        let empty = Default::default();

        let package_types = config_yaml["package_types"]
            .as_hash()
            .unwrap_or(&empty)
            .into_iter()
            .flat_map(|(key, value)| {
                let no_includes = vec![];
                let name = key.as_str().unwrap_or("").to_owned();
                let prefix = value["prefix"].as_str().unwrap_or("").to_owned();
                let includes = value["includes"]
                    .as_vec()
                    .unwrap_or(&no_includes)
                    .into_iter()
                    .flat_map(|entry| entry.as_str().map(|x| x.to_owned()));

                if name.is_empty() {
                    None
                } else {
                    Some(PackageType {
                        name,
                        prefix,
                        includes: includes.collect(),
                    })
                }
            });

        let config = Config {
            package_types: package_types.collect(),
        };
        config.validate().map(Err).unwrap_or(Ok(config))
    }

    fn package_exists(&self, package_name: &str) -> bool {
        self.package_types
            .iter()
            .any(|package| package.name == package_name)
    }

    fn validate(&self) -> Option<FlError> {
        self.package_types
            .iter()
            .fold(None, |acc, package| match acc {
                Some(_) => acc,
                None => {
                    let unknown_include = package
                        .includes
                        .iter()
                        .find(|include| !self.package_exists(include));

                    unknown_include.map(|include| {
                        let err =
                            format!("package '{}': unknown include '{}'", package.name, include);
                        ConfigValidation(err.to_owned())
                    }).or_else(|| {
                        if package.prefix.is_empty() {
                            let err = format!("package '{}': empty prefix", package.name);
                            Some(ConfigValidation(err.to_owned()))
                        } else {
                            None
                        }
                    })
                }
            })
    }
}
