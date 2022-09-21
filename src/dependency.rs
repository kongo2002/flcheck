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

    pub fn is_git(&self) -> bool {
        match self {
            Dependency::Git { .. } => true,
            _ => false,
        }
    }

    pub fn is_public(&self) -> bool {
        match self {
            Dependency::Public { .. } => true,
            _ => false,
        }
    }
}
