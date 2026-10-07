// CLI argument handling.
use std::env;
use std::path::PathBuf;
use std::process;

/// Test configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestConfig {
    pub file: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub body: Option<String>,
}

impl TestConfig {
    pub fn new() -> Self {
        TestConfig {
            file: None,
            method: None,
            path: None,
            body: None,
        }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Run mode for the interpreter
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunMode {
    Repl,
    Run(String),          // Run a single .zun file
    Serve(PathBuf, bool), // Serve mode: path, show_routertab
    Test(TestConfig),     // Test mode: configuration
}

/// Parse CLI arguments and return the run mode.
pub fn parse_args() -> RunMode {
    let args: Vec<String> = env::args().collect();
    match parse_args_from(args.iter().map(|arg| arg.as_str())) {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("Error: {message}");
            process::exit(1);
        }
    }
}

fn parse_args_from<'a>(args: impl IntoIterator<Item = &'a str>) -> Result<RunMode, String> {
    let args: Vec<&str> = args.into_iter().collect();

    if args.len() >= 2 {
        match args[1] {
            "run" if args.len() >= 3 => {
                let filename = args[2];
                if filename.ends_with(".zun") {
                    Ok(RunMode::Run(filename.to_string()))
                } else {
                    Err("Only .zun files can be executed".to_string())
                }
            }
            "serve" => {
                let mut show_routertab = false;
                let mut path: Option<PathBuf> = None;

                for arg in args.iter().skip(2) {
                    if *arg == "--routertab" {
                        show_routertab = true;
                        continue;
                    }
                    if arg.starts_with("--") {
                        return Err(format!("Unknown serve argument '{}'", arg));
                    }
                    if path.is_some() {
                        return Err(format!("Unexpected extra serve argument '{}'", arg));
                    }
                    path = Some(PathBuf::from(arg));
                }

                Ok(RunMode::Serve(
                    path.unwrap_or_else(|| PathBuf::from(".")),
                    show_routertab,
                ))
            }
            "test" => {
                // Parse test command arguments
                let mut config = TestConfig::new();

                let mut i = 2;
                while i < args.len() {
                    match args[i] {
                        "--route" => {
                            // Parse "METHOD /path" - needs at least 3 args: --route METHOD /path
                            if i + 2 < args.len() {
                                config.method = Some(args[i + 1].to_string());
                                config.path = Some(args[i + 2].to_string());
                                i += 3;
                            } else if i + 1 < args.len() {
                                // Try to parse "METHOD /path" from single argument
                                let route = &args[i + 1];
                                if let Some(space_idx) = route.find(' ') {
                                    config.method = Some(route[..space_idx].to_string());
                                    config.path = Some(route[space_idx + 1..].to_string());
                                    i += 2;
                                } else {
                                    return Err(
                                        "--route requires METHOD and path (e.g., --route GET /hello)"
                                            .to_string(),
                                    );
                                }
                            } else {
                                return Err(
                                    "--route requires METHOD and path (e.g., --route GET /hello)"
                                        .to_string(),
                                );
                            }
                        }
                        "--body" | "-b" => {
                            if i + 1 < args.len() {
                                config.body = Some(args[i + 1].to_string());
                                i += 2;
                            } else {
                                return Err("--body requires a value".to_string());
                            }
                        }
                        arg if arg.ends_with(".zun") => {
                            config.file = Some(arg.to_string());
                            i += 1;
                        }
                        "-h" | "--help" => {
                            print_help();
                            process::exit(0);
                        }
                        _ => {
                            return Err(format!("Unknown test argument '{}'", args[i]));
                        }
                    }
                }

                Ok(RunMode::Test(config))
            }
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            _ => Err(format!(
                "Unknown command '{}'. Use 'zun run <file>' to run a file, or 'zun serve' to start server mode.",
                args[1]
            )),
        }
    } else {
        Ok(RunMode::Repl)
    }
}

pub fn print_help() {
    println!("Zun - a tiny, fast language for HTTP services");
    println!();
    println!("Usage:");
    println!("  zun               Start REPL mode");
    println!("  zun run <file>    Run a .zun file");
    println!("  zun serve [path]  Start server mode (default: current directory)");
    println!("  zun test [options] Run tests (default: main.zun)");
    println!();
    println!("Test Options:");
    println!("  zun test [file]   Run tests from .zun file (default: main.zun)");
    println!("  zun test --route METHOD /path  Test a specific route");
    println!("  zun test --body JSON   Request body for POST/PUT");
    println!();
    println!("Server Options:");
    println!("  -h, --help           Show this help message");
    println!("  --routertab          Show registered routes table on server start");
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{RunMode, parse_args_from};

    #[test]
    fn parse_serve_args_accepts_routertab_flag() {
        let mode = parse_args_from(["zun", "serve", ".", "--routertab"]).expect("serve args");
        assert_eq!(mode, RunMode::Serve(PathBuf::from("."), true));
    }

    #[test]
    fn parse_serve_args_rejects_unknown_flag() {
        let err = parse_args_from(["zun", "serve", ".", "--unknown"]).expect_err("should fail");
        assert!(err.contains("Unknown serve argument"));
    }

    #[test]
    fn parse_serve_args_rejects_extra_positional_arg() {
        let err = parse_args_from(["zun", "serve", ".", "extra"]).expect_err("should fail");
        assert!(err.contains("Unexpected extra serve argument"));
    }
}
