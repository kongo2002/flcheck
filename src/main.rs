use crate::config::Config;

extern crate yaml_rust;

mod cli;
mod config;
mod error;
mod util;

#[derive(Debug)]
pub struct Package {
    pub name: String,
}

fn main() {
    let opts = cli::get_opts();
    match Config::load(&opts.config_file) {
        Ok(config) => {
            if !config.is_valid() {
                eprintln!("no package types configured");
                std::process::exit(1);
            }
            println!("{:?}", config);
        }
        Err(err) => {
            eprintln!("failed to load configuration: {}", err);
            std::process::exit(1);
        }
    }
}
