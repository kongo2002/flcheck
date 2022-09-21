use serde::Serialize;

#[derive(Debug, Serialize)]
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

    /// Return a copy including an override dependency.
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

    /// Return the "effective" dependency meaning either itself or the
    /// overridden dependency (if existing).
    pub fn effective(&self) -> &Dependency {
        match self {
            Dependency::Local { overridden, .. } => overridden.as_ref().as_ref().unwrap_or(self),
            Dependency::Git { overridden, .. } => overridden.as_ref().as_ref().unwrap_or(self),
            Dependency::Public { overridden, .. } => overridden.as_ref().as_ref().unwrap_or(self),
        }
    }

    /// Whether this dependency is a "local" dependency, meaning
    /// it references a package in the current/same repository.
    pub fn is_local(&self) -> bool {
        match self {
            Dependency::Local { .. } => true,
            _ => false,
        }
    }

    /// Whether this dependency is a reference to a git repository.
    pub fn is_git(&self) -> bool {
        match self {
            Dependency::Git { .. } => true,
            _ => false,
        }
    }

    /// Whether the dependency is a package hosted on pub.dev
    pub fn is_public(&self) -> bool {
        match self {
            Dependency::Public { .. } => true,
            _ => false,
        }
    }
}
