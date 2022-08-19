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
        overridden: Box<Option<Dependency>>,
    },
    Git {
        name: String,
        git: String,
        path: String,
        overridden: Box<Option<Dependency>>,
    },
    Public {
        name: String,
        version: String,
        overridden: Box<Option<Dependency>>,
    },
}

impl Dependency {
    pub fn name(&self) -> &String {
        match self {
            Dependency::Local { name, .. } => name,
            Dependency::Git { name, .. } => name,
            Dependency::Public { name, .. } => name,
        }
    }

    pub fn with_override(self, override_dependency: Dependency) -> Self {
        match self {
            Dependency::Local { name, path, .. } => Dependency::Local {
                name,
                path,
                overridden: Box::new(Some(override_dependency)),
            },
            Dependency::Git {
                name, path, git, ..
            } => Dependency::Git {
                name,
                path,
                git,
                overridden: Box::new(Some(override_dependency)),
            },
            Dependency::Public { name, version, .. } => Dependency::Public {
                name,
                version,
                overridden: Box::new(Some(override_dependency)),
            },
        }
    }

    pub fn effective(&self) -> &Dependency {
        match self {
            Dependency::Local { overridden, .. } => overridden.as_ref().as_ref().unwrap_or(self),
            Dependency::Git { overridden, .. } => overridden.as_ref().as_ref().unwrap_or(self),
            Dependency::Public { overridden, .. } => overridden.as_ref().as_ref().unwrap_or(self),
        }
    }

    pub fn is_public(&self) -> bool {
        match self {
            Dependency::Public { .. } => true,
            _ => false,
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
            .flat_map(|dep| {
                vec![
                    self.allowed_dependency(dep, config, packages),
                    self.cyclic_dependency(dep, packages, vec![self.dir_path.clone()]),
                ]
                .into_iter()
                .flatten()
            })
            .collect()
    }

    fn resolve_dependency<'a>(
        &self,
        dep: &Dependency,
        packages: &'a Vec<Pubspec>,
    ) -> Option<&'a Pubspec> {
        match dep.effective() {
            Dependency::Local { path, .. } => {
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

    fn cyclic_dependency(
        &self,
        dep: &Dependency,
        packages: &Vec<Pubspec>,
        seen: Vec<String>,
    ) -> Option<PackageValidation> {
        match self.resolve_dependency(dep, packages) {
            Some(rev_dep) => {
                if let Some(idx) = seen.iter().position(|d| *d == rev_dep.dir_path) {
                    // we only want to report the cyclic dependency for the involved packages only
                    if self.dir_path != rev_dep.dir_path {
                        return None;
                    }

                    let mut route = seen.clone();
                    let mut prepared: Vec<_> = route
                        .drain(idx..)
                        .flat_map(|path| file_name(&path))
                        .collect();
                    prepared.push(rev_dep.dir_name.clone());

                    return Some(PackageValidation {
                        package_name: self.name.clone(),
                        error: format!("cyclic dependency {}", prepared.join(" -> ")),
                        code: "validation:dependency:cyclic".to_owned(),
                    });
                } else {
                    for inner_dep in rev_dep.dependencies.iter() {
                        let mut dep_path = seen.clone();
                        dep_path.push(rev_dep.dir_path.clone());

                        let cyclic = self.cyclic_dependency(inner_dep, packages, dep_path);
                        if cyclic.is_some() {
                            return cyclic;
                        }
                    }
                    return None;
                }
            }
            None => None,
        }
    }

    fn allowed_dependency(
        &self,
        dep: &Dependency,
        config: &Config,
        packages: &Vec<Pubspec>,
    ) -> Option<PackageValidation> {
        // public/external dependencies are allowed anyways
        if dep.is_public() {
            return None;
        }

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
                code: "validation:dependency:unknown".to_owned(),
            }),
            Some(dep_pubspec) => {
                let non_valid = !valid_prefixes
                    .iter()
                    .any(|prefix| dep_pubspec.dir_name.starts_with(prefix));
                if non_valid {
                    Some(PackageValidation {
                        package_name: self.name.clone(),
                        error: format!("dependency to {} is not allowed", dep.name()),
                        code: "validation:dependency:unallowed".to_owned(),
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
            let new_dependency =
                if let Some(dep_override) = extract_dependency(key, &dependency_overrides[key]) {
                    dep.with_override(dep_override)
                } else {
                    dep
                };

            deps.push(new_dependency);
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
            overridden: Box::new(None),
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
            overridden: Box::new(None),
        });
    }

    // try public (external) dependency at last
    value
        .as_str()
        .map(|str| str.to_owned())
        .or_else(|| value.as_f64().map(|num| format!("{}", num)))
        .map(|version| Dependency::Public {
            name: key.to_owned(),
            version: version,
            overridden: Box::new(None),
        })
}

fn file_name(path: &str) -> Option<String> {
    PathBuf::from(path)
        .file_name()
        .and_then(|path| path.to_str())
        .map(|path| path.to_owned())
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
