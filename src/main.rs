use crate::cli::{OptCommand, Opts};
use crate::config::Config;
use crate::error::FlError;
use crate::pubspec::Pubspec;
use crate::FlError::NoInputFiles;
use crate::FlError::ValidationError;

extern crate yaml_rust;

mod cli;
mod config;
mod error;
mod pubspec;
mod util;

fn run(opts: Opts) -> Result<(), FlError> {
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
    }
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

fn main() {
    let opts = cli::get_opts();

    if let Err(err) = run(opts) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
