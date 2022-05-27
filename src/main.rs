use crate::cli::Opts;
use crate::config::Config;
use crate::error::FlError;
use crate::pubspec::Pubspec;

extern crate yaml_rust;

mod cli;
mod config;
mod error;
mod pubspec;
mod util;

fn run(opts: Opts) -> Result<(), FlError> {
    let config = Config::load(&opts.config_file)?;

    let pubspecs: Result<Vec<Pubspec>, _> = pubspec::find_pubspecs(&opts.root_dir)
        .iter()
        .filter(|path| !config.is_blacklisted(path))
        .map(|pubspec| Pubspec::load(pubspec))
        .collect();

    for pubspec in pubspecs?.iter() {
        println!("{:?}", pubspec);
    }

    Ok(())
}

fn main() {
    let opts = cli::get_opts();

    if let Err(err) = run(opts) {
        eprintln!("{}", err);
        std::process::exit(1);
    }
}
