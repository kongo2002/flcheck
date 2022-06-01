extern crate getopts;

use std::env;

use getopts::Options;

pub enum OptCommand {
    Validate,
    Dump,
    Check,
}

pub struct Opts {
    pub command: OptCommand,
    pub config_file: String,
    pub root_dir: String,
}

pub fn get_opts() -> Opts {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optopt("c", "config", "config file (default: flcheck.yaml)", "FILE");
    opts.optopt("d", "dir", "apps directory", "DIR");
    opts.optflag("h", "help", "show help");

    let matches = match opts.parse(&args[1..]) {
        Ok(parsed) => parsed,
        Err(f) => {
            eprintln!("{}", f.to_string());
            std::process::exit(1)
        }
    };

    if matches.opt_present("h") {
        let brief = format!(
            r#"Usage: {} COMMAND [OPTIONS]

Commands:
    validate - pubspec dependency validation"#,
            args[0]
        );
        print!("{}", opts.usage(&brief));
        std::process::exit(0);
    }

    let config_file = matches.opt_str("c").unwrap_or("flcheck.yaml".to_owned());
    let root_dir = matches.opt_str("d").unwrap_or(".".to_owned());

    let cmd = match matches.free.len() {
        0 => {
            eprintln!("missing command");
            std::process::exit(1)
        }
        _ => OptCommand::from(&matches.free[0]),
    };

    if let Some(command) = cmd {
        Opts {
            command,
            config_file,
            root_dir,
        }
    } else {
        eprintln!("unknown command");
        std::process::exit(1)
    }
}

impl OptCommand {
    pub fn from(value: &str) -> Option<OptCommand> {
        match value {
            "validate" => Some(OptCommand::Validate),
            "dump" => Some(OptCommand::Dump),
            "check" => Some(OptCommand::Check),
            _ => None,
        }
    }
}
