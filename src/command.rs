use crate::pubdev::fetch_dep_versions;
use crate::pubspec::Dependency;
use crate::Config;
use crate::FlError;
use crate::FlError::ValidationError;
use crate::Pubspec;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::collections::HashSet;

pub fn graph(pubspecs: Vec<Pubspec>) -> Result<(), FlError> {
    println!("//");
    println!("// automatically generated by flcheck <https://github.com/kongo2002/flcheck>");
    println!("//");
    println!("digraph dependencies {{");
    println!("  ranksep =\"2.0 equally\";");

    for pubspec in pubspecs {
        println!("  // {}", pubspec.name);
        println!("  {} []", pubspec.name);

        for dep in pubspec.dependencies {
            match dep {
                Dependency::Local { name, path: _ } => println!("  {} -> {};", pubspec.name, name),
                Dependency::Git {
                    name,
                    path: _,
                    git: _,
                } => println!("  {} -> {};", pubspec.name, name),
                _ => {}
            }
        }
    }

    println!("}}");
    Ok(())
}

pub async fn check(pubspecs: Vec<Pubspec>) -> Result<(), FlError> {
    let unique_packages = pubspecs
        .iter()
        .flat_map(|pkg| {
            pkg.dependencies.iter().flat_map(|dep| match dep {
                Dependency::Public { name, version: _ } => Some(name),
                _ => None,
            })
        })
        .collect::<HashSet<_>>();

    let versions = try_join_all(
        unique_packages
            .iter()
            .map(|package| fetch_dep_versions(package)),
    )
    .await?;

    let lookup = versions
        .into_iter()
        .map(|pubversion| (pubversion.name.clone(), pubversion))
        .collect::<HashMap<_, _>>();

    for pubspec in pubspecs {
        println!("{}", pubspec.name);

        for dep in pubspec.dependencies {
            match dep {
                Dependency::Public { name, version } => {
                    let pub_version = lookup.get(&name).map(|vsn| &vsn.latest);
                    println!(
                        "  {}: {} [{}]",
                        name,
                        version,
                        pub_version.map_or("<unknown>", String::as_str)
                    );
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub fn dump(pubspecs: Vec<Pubspec>) -> Result<(), FlError> {
    for pubspec in pubspecs {
        // TODO: implement proper Display
        println!("{:?}", pubspec)
    }
    Ok(())
}

pub fn validate(config: Config, pubspecs: Vec<Pubspec>) -> Result<(), FlError> {
    let mut num_errors = 0u32;
    for pubspec in pubspecs.iter() {
        for val in pubspec.validate(&config, &pubspecs) {
            num_errors += 1;
            eprintln!("{:?}", val)
        }
    }

    if num_errors > 0 {
        Err(ValidationError(num_errors))
    } else {
        Ok(())
    }
}

pub fn example_config() {
    println!(r#"# Package types list rules for packages that describe
# what package is allowed to depend on each other.
#
# The typical recommended setup is a hierachy like the following:
# - main: the main app that is shipped and distributed to app stores and
#   assembles the functionalities of one or multiple (sub) apps
# - app: one or multiple (sub) apps that encapsulate functionalities of
#   usually one domain per app - must not depend on each other
# - shared: few shared libraries that can be used the glue together
#   multiple apps, e.g. for routing, navigation - must import packages
#   and other shared libraries only
# - package: general purpose libraries (e.g. utilities) that do not contain
#   domain specific logic, may be included from any package type/layer
package_types:

  main:
    # the dir_prefix is used to associate every dart package to one
    # of the package types listed here, is applied to the directory
    # name of the package
    dir_prefix: 'main'
    # list of package types all packages of this type may import from
    # (here: main is allowed to import all apps and everything that apps
    # are allowed to import themselves)
    includes:
      - app

  app:
    dir_prefix: 'app_'
    includes:
      - shared

  shared:
    dir_prefix: 'shared_'
    includes:
      - shared
      - package

  package:
    dir_prefix: 'pkg_'
    includes:
      - package

# List of patterns (regular expressions) that match package directories
# that should be excluded from all validations and checks.
# Here: exclude all auto-generated "example" packages from native dart
# packages.
blacklist:
  - '/example'
"#);
}