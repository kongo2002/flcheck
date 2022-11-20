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

#[derive(PartialEq)]
pub enum OutputFormat {
    Plain,
    Json,
}

pub struct Opts {
    pub command: OptCommand,
    pub config_file: String,
    pub root_dir: String,
    pub output: OutputFormat,
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

/// Extract command line options.
/// Exits with non-zero exit code on invalid arguments.
pub fn get_opts() -> Opts {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optopt("c", "config", "config file (default: flcheck.yaml)", "FILE");
    opts.optopt("d", "dir", "apps directory", "DIR");
    opts.optopt("o", "output", "output format (plain, json)", "FORMAT");
    opts.optflag("h", "help", "show help");

    let matches = match opts.parse(&args[1..]) {
        Ok(parsed) => parsed,
        Err(f) => {
            eprintln!("{}", f.to_string());
            std::process::exit(1)
        }
    };

    // print help/usage
    if matches.opt_present("h") {
        usage(&opts, &args[0]);
        std::process::exit(0);
    }

    let config_file = matches.opt_str("c").unwrap_or("flcheck.yaml".to_owned());
    let root_dir = matches.opt_str("d").unwrap_or(".".to_owned());
    let output_format = matches.opt_str("o").unwrap_or("plain".to_owned());

    let cmd = match matches.free.len() {
        1 => OptCommand::from(&matches.free[0]),
        0 => {
            eprintln!("missing command");
            eprintln!();
            usage(&opts, &args[0]);
            std::process::exit(1)
        }
        _ => {
            eprintln!("multiple commands are not supported");
            eprintln!();
            usage(&opts, &args[0]);
            std::process::exit(1)
        }
    };

    let output = match parse_format(&output_format) {
        Ok(fmt) => fmt,
        Err(error) => {
            eprintln!("{}", error);
            eprintln!();
            usage(&opts, &args[0]);
            std::process::exit(1)
        }
    };

    if let Some(command) = cmd {
        Opts {
            command,
            config_file,
            root_dir: canonicalize(&root_dir).unwrap_or(root_dir),
            output,
        }
    } else {
        eprintln!("unknown command");
        eprintln!();
        usage(&opts, &args[0]);
        std::process::exit(1)
    }
}

fn parse_format(value: &str) -> Result<OutputFormat, &str> {
    match value {
        "plain" => Ok(OutputFormat::Plain),
        "json" => Ok(OutputFormat::Json),
        _ => Err("invalid output format (valid: json, plain)"),
    }
}

fn canonicalize(path: &String) -> Option<String> {
    let canonicalized = std::fs::canonicalize(path).ok()?;
    let canonical_str = canonicalized.to_str()?;

    Some(canonical_str.to_owned())
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
