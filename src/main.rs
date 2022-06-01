use crate::cli::{OptCommand, Opts};
use crate::config::Config;
use crate::error::FlError;
use crate::pubspec::{Dependency, Pubspec};
use crate::FlError::NoInputFiles;
use crate::FlError::ValidationError;
use futures::future::try_join_all;
use std::collections::HashMap;
use std::collections::HashSet;

mod cli;
mod config;
mod error;
mod pubdev;
mod pubspec;
mod util;

async fn run(opts: Opts) -> Result<(), FlError> {
    let config = Config::load(&opts.config_file)?;

    let loaded_pubspecs: Result<Vec<Pubspec>, _> = pubspec::find_pubspecs(&opts.root_dir)
        .iter()
        .map(|pubspec| Pubspec::load(pubspec))
        .collect();

    let pubspecs = loaded_pubspecs?;
    if pubspecs.is_empty() {
        return Err(NoInputFiles(opts.root_dir));
    }

    match opts.command {
        OptCommand::Validate => validate(config, pubspecs),
        OptCommand::Dump => dump(pubspecs),
        OptCommand::Check => check(pubspecs).await,
    }
}

async fn check(pubspecs: Vec<Pubspec>) -> Result<(), FlError> {
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
            .map(|package| pubdev::fetch_dep_versions(package)),
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
                    let unknown = "<unknown>".to_owned();
                    let pub_version = lookup.get(&name).and_then(|vsn| vsn.versions.last());
                    println!(
                        "  {}: {} [{}]",
                        name,
                        version,
                        pub_version.unwrap_or(&unknown)
                    );
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn dump(pubspecs: Vec<Pubspec>) -> Result<(), FlError> {
    for pubspec in pubspecs {
        // TODO: implement proper Display
        println!("{:?}", pubspec)
    }
    Ok(())
}

fn validate(config: Config, pubspecs: Vec<Pubspec>) -> Result<(), FlError> {
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

#[tokio::main]
async fn main() {
    let opts = cli::get_opts();

    if let Err(err) = run(opts).await {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
