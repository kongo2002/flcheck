use crate::config::Config;

extern crate yaml_rust;

mod cli;
mod config;
mod error;
mod pubspec;
mod util;

fn main() {
    let opts = cli::get_opts();
    match Config::load(&opts.config_file) {
        Ok(config) => {
            if !config.is_valid() {
                eprintln!("no package types configured");
                std::process::exit(1);
            }

            for yaml in pubspec::find_pubspecs(&opts.root_dir) {
                if !config.is_blacklisted(&yaml) {
                    println!("{}", yaml);
                }
            }
        }
        Err(err) => {
            eprintln!("failed to load configuration: {}", err);
            std::process::exit(1);
        }
    }
}
