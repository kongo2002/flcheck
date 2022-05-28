extern crate walkdir;

use crate::config::PackageType;
use crate::error::FlError;
use crate::error::PackageValidation;
use crate::util::load_yaml;
use crate::Config;
use crate::FlError::ConfigValidation;
use std::path::PathBuf;
use walkdir::WalkDir;
use yaml_rust::Yaml;

#[derive(Debug)]
pub struct Pubspec {
    pub name: String,
    pub path: String,
    pub dir_name: String,
    pub dir_path: String,
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug)]
pub enum Dependency {
    Local {
        name: String,
        path: String,
    },
    Git {
        name: String,
        git: String,
        path: String,
    },
}

impl Dependency {
    pub fn name(&self) -> &String {
        match self {
            Dependency::Local { name, path: _ } => name,
            Dependency::Git {
                name,
                path: _,
                git: _,
            } => name,
        }
    }
}

impl Pubspec {
    pub fn load(path: &str) -> Result<Pubspec, FlError> {
        let yaml = load_yaml(path)?;
        let name = yaml["name"].as_str().unwrap_or("").to_owned();

        pubspec_dir(path)
            .ok_or(ConfigValidation(format!(
                "cannot determine parent directory for {}",
                path
            )))
            .map(|(dir_name, dir_path)| Pubspec {
                name: name,
                path: path.to_owned(),
                dir_name: dir_name,
                dir_path: dir_path,
                dependencies: get_dependencies(&yaml),
            })
    }

    pub fn validate(&self, config: &Config, packages: &Vec<Pubspec>) -> Vec<PackageValidation> {
        if config.is_blacklisted(&self.path) {
            return vec![];
        }

        self.dependencies
            .iter()
            .flat_map(|dep| self.valid_dependency(dep, config, packages))
            .collect()
    }

    fn resolve_dependency<'a>(&self, dep: &Dependency, packages: &'a Vec<Pubspec>) -> Option<&'a Pubspec> {
        match dep {
            Dependency::Local { name: _, path } => {
                let full_path = PathBuf::from(format!("{}/{}", self.dir_path, path));
                let canonicalized = std::fs::canonicalize(full_path).ok()?;
                let full_str = canonicalized.to_str()?;
                let pubspec = packages
                    .iter()
                    .find(|pubspec| pubspec.dir_path == full_str)?;

                Some(pubspec)
            }
            _ => None,
        }
    }

    fn valid_dependency(
        &self,
        dep: &Dependency,
        config: &Config,
        packages: &Vec<Pubspec>,
    ) -> Option<PackageValidation> {
        let valid_prefixes: Vec<_> = config
            .package_types
            .iter()
            .filter(|pkg_type| self.dir_name.starts_with(&pkg_type.prefix))
            .flat_map(|include| valid_include_prefixes(include, config))
            .collect();

        match self.resolve_dependency(dep, packages) {
            None => Some(PackageValidation {
                package_name: self.name.clone(),
                error: format!("unable to find dependency {}", dep.name()),
            }),
            Some(dep_pubspec) => {
                let non_valid = !valid_prefixes
                    .iter()
                    .any(|prefix| dep_pubspec.dir_name.starts_with(prefix));
                if non_valid {
                    Some(PackageValidation {
                        package_name: self.name.clone(),
                        error: format!(
                            "dependency to {} is not allowed",
                            dep.name()
                        ),
                    })
                } else {
                    None
                }
            }
        }
    }
}

fn valid_include_prefixes(pkg_type: &PackageType, config: &Config) -> Vec<String> {
    let mut prefixes = vec![];
    config.package_types.iter().for_each(|pkg| {
        if pkg_type.includes.iter().any(|inc| *inc == pkg.name) {
            if !prefixes.contains(&pkg.prefix) {
                prefixes.push(pkg.prefix.clone());

                if pkg.name != pkg_type.name {
                    for prefix in valid_include_prefixes(pkg, config) {
                        if !prefixes.contains(&prefix) {
                            prefixes.push(prefix);
                        }
                    }
                }
            }
        }
    });
    prefixes
}

pub fn find_pubspecs(root_dir: &str) -> Vec<String> {
    let mut pubspecs = vec![];

    let walker = WalkDir::new(root_dir)
        .into_iter()
        // filter hidden files/directories
        .filter_entry(|e| {
            !e.file_name()
                .to_str()
                .map(|s| s.starts_with("."))
                .unwrap_or(false)
        })
        // skip errors (e.g. non permission directories)
        .filter_map(|e| e.ok());

    for entry in walker {
        let filename = entry.file_name().to_str().unwrap_or("").to_lowercase();
        let is_pubspec = filename == "pubspec.yaml" || filename == "pubspec.yml";

        if is_pubspec {
            if let Some(path) = entry.path().to_str() {
                pubspecs.push(path.to_owned());
            }
        }
    }

    pubspecs
}

fn get_dependencies(yaml: &Yaml) -> Vec<Dependency> {
    let dependencies = &yaml["dependencies"];
    let dependency_overrides = &yaml["dependency_overrides"];
    let empty = Default::default();

    let mut deps = vec![];

    for (key, value) in dependencies.as_hash().unwrap_or(&empty).iter() {
        let key = key.as_str().unwrap_or("");
        if key.is_empty() {
            continue;
        }

        if let Some(dep) = extract_dependency(key, value) {
            // we found a "normal" dependency
            // before collecting this one, we have to check if there is
            // a dependency override set for that same dependency
            if let Some(dep_override) = extract_dependency(key, &dependency_overrides[key]) {
                deps.push(dep_override);
            } else {
                deps.push(dep);
            }
        }
    }

    deps
}

fn extract_dependency(key: &str, value: &Yaml) -> Option<Dependency> {
    let path = value["path"].as_str().unwrap_or("");

    // check local dependency first
    if !path.is_empty() {
        return Some(Dependency::Local {
            name: key.to_owned(),
            path: path.to_owned(),
        });
    }

    // check git dependency
    let git_node = &value["git"];
    let git_url = git_node["url"].as_str().unwrap_or("");
    let git_path = git_node["path"].as_str().unwrap_or("");

    if !git_url.is_empty() && !git_path.is_empty() {
        return Some(Dependency::Git {
            name: key.to_owned(),
            git: git_url.to_owned(),
            path: git_path.to_owned(),
        });
    }

    None
}

fn pubspec_dir(path: &str) -> Option<(String, String)> {
    let full_path = std::path::Path::new(path);
    let dir_name = full_path
        .parent()
        .and_then(|d| d.file_name())
        .and_then(|f| f.to_str())?;
    let full_dir = full_path.parent().and_then(|f| f.to_str())?;

    Some((dir_name.to_owned(), full_dir.to_owned()))
}
