extern crate walkdir;

use crate::config::PackageType;
use crate::dependency::Dependency;
use crate::error::FlError;
use crate::error::PackageValidation;
use crate::error::ValidationType;
use crate::util::load_yaml;
use crate::Config;
use crate::FlError::ConfigValidation;

use serde::Serialize;
use std::path::PathBuf;
use walkdir::WalkDir;
use yaml_rust::Yaml;

#[derive(Debug, Serialize)]
pub struct Pubspec {
    pub name: String,
    pub path: String,
    pub dir_name: String,
    pub dir_path: String,
    pub dependencies: Vec<Dependency>,
    pub dev_dependencies: Vec<Dependency>,
    pub is_public: bool,
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
                dev_dependencies: get_dev_dependencies(&yaml),
                is_public: is_public_package(&yaml),
            })
    }

    pub fn validate(&self, config: &Config, packages: &Vec<Pubspec>) -> Vec<PackageValidation> {
        if config.is_blacklisted(&self.path) {
            return vec![];
        }

        let dependency_validations = self.dependencies.iter().flat_map(|dep| {
            vec![
                self.allowed_dependency(dep, config, packages),
                self.cyclic_dependency(dep, packages, vec![self.dir_path.clone()]),
                self.public_package_git_dependencies_only(dep),
            ]
            .into_iter()
            .flatten()
        });

        let dev_dependency_validations = self.dev_dependencies.iter().flat_map(|dep| {
            vec![self.git_packages_in_dev_dependencies(dep)]
                .into_iter()
                .flatten()
        });

        return dependency_validations
            .chain(dev_dependency_validations)
            .collect();
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

                    return Some(self.validation(
                        format!("cyclic dependency {}", prepared.join(" -> ")),
                        ValidationType::CyclicDependency,
                    ));
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

    fn git_packages_in_dev_dependencies(&self, dep: &Dependency) -> Option<PackageValidation> {
        if dep.is_git() {
            Some(self.validation(
                format!("git dependency in dev_dependencies {}", dep.name()),
                ValidationType::GitDevDependency,
            ))
        } else {
            None
        }
    }

    fn public_package_git_dependencies_only(&self, dep: &Dependency) -> Option<PackageValidation> {
        if !self.is_public || !dep.is_local() {
            None
        } else {
            Some(self.validation(
                format!("non-git dependency {} in public package", dep.name()),
                ValidationType::NonGitDependencyInPublicPackage,
            ))
        }
    }

    fn allowed_dependency(
        &self,
        dep: &Dependency,
        config: &Config,
        packages: &Vec<Pubspec>,
    ) -> Option<PackageValidation> {
        // public/external dependencies are allowed/ignored anyways
        if dep.is_public() {
            return None;
        }

        // git dependencies are allowed/ignore for now
        // TODO: we might need a concept to identify "internal" git dependencies
        if dep.is_git() {
            return None;
        }

        let valid_prefixes: Vec<_> = config
            .package_types
            .iter()
            .filter(|pkg_type| pkg_type.matches_prefix(&self.dir_name))
            .flat_map(|include| valid_include_prefixes(include, config))
            .collect();

        match self.resolve_dependency(dep, packages) {
            None => Some(self.validation(
                format!("unable to find dependency {}", dep.name()),
                ValidationType::UnknownDependency,
            )),
            Some(dep_pubspec) => {
                let non_valid = !valid_prefixes
                    .iter()
                    .any(|prefix| dep_pubspec.dir_name.starts_with(prefix));
                if non_valid {
                    Some(self.validation(
                        format!("dependency to {} is not allowed", dep.name()),
                        ValidationType::DependencyNotAllowed,
                    ))
                } else {
                    None
                }
            }
        }
    }

    /// Create a new `PackageValidation` instance for this `Pubspec`
    fn validation(&self, error: String, code: ValidationType) -> PackageValidation {
        PackageValidation {
            package_name: self.name.clone(),
            error: error,
            code: code,
        }
    }
}

fn valid_include_prefixes(pkg_type: &PackageType, config: &Config) -> Vec<String> {
    let mut prefixes = vec![];
    config.package_types.iter().for_each(|pkg| {
        if pkg_type.includes.iter().any(|inc| *inc == pkg.name) {
            for prefix in pkg.prefixes.iter() {
                if !prefixes.contains(prefix) {
                    prefixes.push(prefix.clone());

                    if pkg.name != pkg_type.name {
                        for iprefix in valid_include_prefixes(pkg, config) {
                            if !prefixes.contains(&iprefix) {
                                prefixes.push(iprefix);
                            }
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

fn is_public_package(yaml: &Yaml) -> bool {
    let is_public_node = &yaml["flcheck"]["is_public"].as_bool();
    is_public_node.unwrap_or(false)
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

fn get_dev_dependencies(yaml: &Yaml) -> Vec<Dependency> {
    let dependencies = &yaml["dev_dependencies"];
    let empty = Default::default();

    let mut deps = vec![];

    for (key, value) in dependencies.as_hash().unwrap_or(&empty).iter() {
        let key = key.as_str().unwrap_or("");
        if key.is_empty() {
            continue;
        }

        if let Some(dep) = extract_dependency(key, value) {
            deps.push(dep);
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

#[cfg(test)]
mod tests {
    use crate::pubspec::PackageType;
    use crate::Config;
    use crate::Pubspec;

    #[test]
    fn empty_dependencies() {
        let config = Config {
            package_types: vec![PackageType {
                name: "app".to_owned(),
                prefixes: vec!["app".to_owned()],
                includes: Vec::new(),
            }],
            blacklist: Vec::new(),
        };

        let all = vec![Pubspec {
            name: "test".to_owned(),
            path: "/tmp/test".to_owned(),
            dir_name: "test".to_owned(),
            dir_path: "/tmp/test".to_owned(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            is_public: false,
        }];

        let errors = all[0].validate(&config, &all);

        assert_eq!(errors.len(), 0);
    }
}
