extern crate getopts;

use std::env;

use getopts::Options;

pub struct Opts {
    pub config_file: String,
    pub root_dir: String,
}

pub fn get_opts() -> Opts {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt("c", "config", "config file (default: flcheck.yaml)", "FILE");
    opts.optflag("h", "help", "show help");

    let matches = match opts.parse(&args[1..]) {
        Ok(parsed) => parsed,
        Err(f) => {
            eprintln!("{}", f.to_string());
            std::process::exit(1)
        }
    };

    if matches.opt_present("h") {
        let brief = format!("Usage: {} DIR [OPTIONS]", args[0]);
        print!("{}", opts.usage(&brief));
        std::process::exit(0);
    }

    let config_file = matches.opt_str("c").unwrap_or("flcheck.yaml".to_owned());
    let root_dir = if !matches.free.is_empty() { matches.free[0].clone() } else { ".".to_owned() };

    return Opts{
        config_file,
        root_dir,
    }
}
