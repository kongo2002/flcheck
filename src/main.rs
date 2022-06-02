use crate::cli::{OptCommand, Opts};
use crate::config::Config;
use crate::error::FlError;
use crate::pubspec::Pubspec;
use crate::FlError::NoInputFiles;

mod cli;
mod command;
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
        OptCommand::Validate => command::validate(config, pubspecs),
        OptCommand::Dump => command::dump(pubspecs),
        OptCommand::Check => command::check(pubspecs).await,
        OptCommand::Graph => command::graph(pubspecs),
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
