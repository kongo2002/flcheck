extern crate walkdir;

use crate::Config;
use crate::FlError::ConfigValidation;
use crate::config::PackageType;
use crate::dependency::Dependency;
use crate::error::FlError;
use crate::error::PackageValidation;
use crate::error::ValidationType;
use crate::util::load_yaml;
use crate::util::normalize_path_str;

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
                name,
                dir_name,
                dir_path,
                path: path.to_owned(),
                dependencies: get_dependencies(&yaml),
                dev_dependencies: get_dev_dependencies(&yaml),
                is_public: is_public_package(&yaml),
            })
    }

    pub fn validate(&self, config: &Config, packages: &Vec<Pubspec>) -> Vec<PackageValidation> {
        if config.is_blacklisted(&self.path) {
            return vec![];
        }

        let all_dependencies = self.dependencies.iter().chain(self.dev_dependencies.iter());
        let all_dependency_validations = all_dependencies.flat_map(|dep| {
            self.cyclic_dependency(config, dep, packages, vec![self.dir_path.clone()])
        });

        let dependency_validations = self.dependencies.iter().flat_map(|dep| {
            vec![
                self.allowed_dependency(dep, config, packages),
                self.public_package_git_dependencies_only(config, dep),
            ]
            .into_iter()
            .flatten()
        });

        let dev_dependency_validations = self.dev_dependencies.iter().flat_map(|dep| {
            vec![self.git_packages_in_dev_dependencies(config, dep)]
                .into_iter()
                .flatten()
        });

        dependency_validations
            .chain(all_dependency_validations)
            .chain(dev_dependency_validations)
            .collect()
    }

    fn resolve_dependency<'a>(
        &self,
        dep: &Dependency,
        packages: &'a [Pubspec],
    ) -> Option<&'a Pubspec> {
        match dep.effective() {
            Dependency::Local { path, .. } => {
                let full_path = format!("{}/{}", self.dir_path, path);
                let normalized = normalize_path_str(full_path);
                let full_str = normalized.to_str()?;

                packages.iter().find(|pubspec| pubspec.dir_path == full_str)
            }
            _ => None,
        }
    }

    fn cyclic_dependency(
        &self,
        config: &Config,
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

                    prepared.push(format!("'{}'", rev_dep.dir_name));

                    Some(self.validation(
                        config,
                        format!("cyclic dependency {}", prepared.join(" -> ")),
                        ValidationType::CyclicDependency,
                        None,
                    ))
                } else {
                    let all_dependencies = rev_dep
                        .dependencies
                        .iter()
                        .chain(rev_dep.dev_dependencies.iter());
                    for inner_dep in all_dependencies {
                        let mut dep_path = seen.clone();
                        dep_path.push(rev_dep.dir_path.clone());

                        let cyclic = self.cyclic_dependency(config, inner_dep, packages, dep_path);
                        if cyclic.is_some() {
                            return cyclic;
                        }
                    }
                    None
                }
            }
            None => None,
        }
    }

    fn git_packages_in_dev_dependencies(
        &self,
        config: &Config,
        dep: &Dependency,
    ) -> Option<PackageValidation> {
        if dep.is_public(config) {
            Some(self.validation(
                config,
                format!("git dependency in dev_dependencies {}", dep.name()),
                ValidationType::GitDevDependency,
                None,
            ))
        } else {
            None
        }
    }

    fn public_package_git_dependencies_only(
        &self,
        config: &Config,
        dep: &Dependency,
    ) -> Option<PackageValidation> {
        if !self.is_public || !dep.is_local() {
            None
        } else {
            Some(self.validation(
                config,
                format!("non-git dependency '{}' in public package", dep.name()),
                ValidationType::NonGitDependencyInPublicPackage,
                None,
            ))
        }
    }

    fn allowed_dependency(
        &self,
        dep: &Dependency,
        config: &Config,
        packages: &[Pubspec],
    ) -> Option<PackageValidation> {
        // dependencies are not analyzed with an empty configuration
        if config.is_empty() {
            return None;
        }

        // public/external dependencies are allowed/ignored anyways
        if dep.is_pubdev() {
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
                config,
                format!("unable to find dependency '{}'", dep.name()),
                ValidationType::UnknownDependency,
                None,
            )),
            Some(dep_pubspec) => {
                let non_valid = !valid_prefixes
                    .iter()
                    .any(|prefix| dep_pubspec.dir_name.starts_with(prefix));
                if non_valid {
                    let mut valid_packages = valid_prefixes
                        .iter()
                        .map(|prefix| format!("'{}'", prefix))
                        .collect::<Vec<_>>();

                    valid_packages.sort_unstable();
                    valid_packages.dedup();

                    Some(self.validation(
                        config,
                        format!("dependency to '{}' is not allowed", dep.name()),
                        ValidationType::DependencyNotAllowed,
                        format!(
                            "packages with the following directory prefixes are allowed only: {}",
                            valid_packages.join(", ")
                        ),
                    ))
                } else {
                    None
                }
            }
        }
    }

    /// Create a new `PackageValidation` instance for this `Pubspec`
    fn validation<T: Into<Option<String>>>(
        &self,
        config: &Config,
        error: String,
        code: ValidationType,
        description: T,
    ) -> PackageValidation {
        let level = config.validation_level(&code);

        PackageValidation {
            package_name: self.name.clone(),
            description: description.into(),
            error,
            code,
            level,
        }
    }
}

fn valid_include_prefixes(pkg_type: &PackageType, config: &Config) -> Vec<String> {
    let mut prefixes = vec![];
    config.package_types.iter().for_each(|pkg| {
        if pkg_type.includes.contains(&pkg.name) {
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

        if is_pubspec && let Some(path) = entry.path().to_str() {
            pubspecs.push(path.to_owned());
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
        .map(|version| Dependency::PubDev {
            name: key.to_owned(),
            version,
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
    use crate::Config;
    use crate::Pubspec;
    use crate::dependency::Dependency;
    use crate::error::ValidationType;
    use crate::pubspec::PackageType;
    use crate::pubspec::PackageValidation;

    fn base_config() -> Config {
        return Config::from_packages(vec![
            PackageType {
                name: "app".to_owned(),
                prefixes: vec!["app_".to_owned()],
                includes: vec!["shared".to_owned()],
            },
            PackageType {
                name: "shared".to_owned(),
                prefixes: vec!["shared_".to_owned()],
                includes: vec!["shared".to_owned(), "package".to_owned()],
            },
            PackageType {
                name: "package".to_owned(),
                prefixes: vec!["pkg_".to_owned()],
                includes: vec!["package".to_owned()],
            },
        ]);
    }

    fn pkg(name: &str, path: &str) -> Pubspec {
        return Pubspec {
            name: name.to_owned(),
            path: format!("{}/pubspec.yaml", path),
            dir_name: name.to_owned(),
            dir_path: path.to_owned(),
            dependencies: Vec::new(),
            dev_dependencies: Vec::new(),
            is_public: false,
        };
    }

    fn codes(validations: Vec<PackageValidation>) -> Vec<ValidationType> {
        return validations.into_iter().map(|v| v.code).collect();
    }

    #[test]
    fn empty_dependencies() {
        let config = base_config();
        let all = vec![pkg("test", "/tmp/test")];

        let errors = all[0].validate(&config, &all);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn multiple_packages() {
        let config = base_config();
        let all = vec![
            pkg("foo", "/tmp/foo"),
            pkg("bar", "/tmp/bar"),
            pkg("ham", "/tmp/ham"),
            pkg("eggs", "/tmp/eggs"),
        ];

        for pkg in all.iter() {
            let errors = pkg.validate(&config, &all);
            assert_eq!(errors.len(), 0);
        }
    }

    #[test]
    fn git_dependency() {
        let config = base_config();
        let all = vec![Pubspec {
            dependencies: vec![Dependency::Git {
                name: "git".to_owned(),
                git: "git://repo".to_owned(),
                path: "".to_owned(),
                overridden: Box::new(None),
            }],
            ..pkg("foo", "/tmp/foo")
        }];

        let errors = all[0].validate(&config, &all);
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn unknown_dependency() {
        let config = base_config();
        let all = vec![Pubspec {
            dependencies: vec![Dependency::Local {
                name: "bar".to_owned(),
                path: "../bar".to_owned(),
                overridden: Box::new(None),
            }],
            ..pkg("foo", "/tmp/foo")
        }];

        let errors = all[0].validate(&config, &all);
        let error_codes = codes(errors);

        assert_eq!(error_codes, vec![ValidationType::UnknownDependency]);
    }

    #[test]
    fn unconfigured_dependency() {
        let config = base_config();
        let all = vec![
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "bar".to_owned(),
                    path: "../bar".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("foo", "/tmp/foo")
            },
            Pubspec {
                dependencies: vec![],
                ..pkg("bar", "/tmp/bar")
            },
        ];

        let errors = all[0].validate(&config, &all);
        let error_codes = codes(errors);

        assert_eq!(error_codes, vec![ValidationType::DependencyNotAllowed]);
    }

    #[test]
    fn basic_dependency() {
        let config = base_config();
        let all = vec![
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "pkg_bar".to_owned(),
                    path: "../pkg_bar".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("app_foo", "/tmp/app_foo")
            },
            Pubspec {
                dependencies: vec![],
                ..pkg("pkg_bar", "/tmp/pkg_bar")
            },
        ];

        let errors = all[0].validate(&config, &all);
        let error_codes = codes(errors);

        assert_eq!(error_codes, Vec::new());
    }

    #[test]
    fn unallowed_dependency() {
        let config = base_config();
        let all = vec![
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "app_bar".to_owned(),
                    path: "../app_bar".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("pkg_foo", "/tmp/pkg_foo")
            },
            Pubspec {
                dependencies: vec![],
                ..pkg("app_bar", "/tmp/app_bar")
            },
        ];

        let errors = all[0].validate(&config, &all);
        let error_codes = codes(errors);

        assert_eq!(error_codes, vec![ValidationType::DependencyNotAllowed]);
    }

    #[test]
    fn cyclic_dependency() {
        let config = base_config();
        let all = vec![
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "pkg_bar".to_owned(),
                    path: "../pkg_bar".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("pkg_foo", "/tmp/pkg_foo")
            },
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "pkg_foo".to_owned(),
                    path: "../pkg_foo".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("pkg_bar", "/tmp/pkg_bar")
            },
        ];

        let errors = all[0].validate(&config, &all);
        let error_codes = codes(errors);

        assert_eq!(error_codes, vec![ValidationType::CyclicDependency]);
    }

    #[test]
    fn cyclic_and_unallowed_dependency() {
        let config = base_config();
        let all = vec![
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "app_bar".to_owned(),
                    path: "../app_bar".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("app_foo", "/tmp/app_foo")
            },
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "app_foo".to_owned(),
                    path: "../app_foo".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("app_bar", "/tmp/app_bar")
            },
        ];

        let errors = all[0].validate(&config, &all);
        let error_codes = codes(errors);

        assert_eq!(
            error_codes,
            vec![
                ValidationType::DependencyNotAllowed,
                ValidationType::CyclicDependency
            ]
        );
    }

    #[test]
    fn cyclic_dev_dependencies() {
        let config = base_config();
        let all = vec![
            Pubspec {
                dependencies: vec![Dependency::Local {
                    name: "pkg_bar".to_owned(),
                    path: "../pkg_bar".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("pkg_foo", "/tmp/pkg_foo")
            },
            Pubspec {
                dev_dependencies: vec![Dependency::Local {
                    name: "pkg_foo".to_owned(),
                    path: "../pkg_foo".to_owned(),
                    overridden: Box::new(None),
                }],
                ..pkg("pkg_bar", "/tmp/pkg_bar")
            },
        ];

        let errors = all[0].validate(&config, &all);
        let error_codes = codes(errors);

        assert_eq!(error_codes, vec![ValidationType::CyclicDependency]);
    }
}
