extern crate getopts;

use std::env;

use getopts::Options;

pub enum OptCommand {
    Validate,
    Dump,
    Check,
    Graph,
    ExampleConfig,
}

pub struct Opts {
    pub command: OptCommand,
    pub config_file: String,
    pub root_dir: String,
}

fn usage(opts: &Options, exec: &str) {
    let brief = format!(
        r#"Usage: {} COMMAND [OPTIONS]

Commands:
    validate - pubspec dependency validation
    graph    - generate a dot dependency graph
    check    - check all external dependencies' versions
    dump     - dump package dependencies
    example  - print example configuration"#,
        exec
    );
    print!("{}", opts.usage(&brief));
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
        usage(&opts, &args[0]);
        std::process::exit(0);
    }

    let config_file = matches.opt_str("c").unwrap_or("flcheck.yaml".to_owned());
    let root_dir = matches.opt_str("d").unwrap_or(".".to_owned());

    let cmd = match matches.free.len() {
        0 => {
            eprintln!("missing command");
            eprintln!();
            usage(&opts, &args[0]);
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
        eprintln!();
        usage(&opts, &args[0]);
        std::process::exit(1)
    }
}

impl OptCommand {
    pub fn from(value: &str) -> Option<OptCommand> {
        match value {
            "validate" => Some(OptCommand::Validate),
            "dump" => Some(OptCommand::Dump),
            "check" => Some(OptCommand::Check),
            "graph" => Some(OptCommand::Graph),
            "example" => Some(OptCommand::ExampleConfig),
            _ => None,
        }
    }
}
