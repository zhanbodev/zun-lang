pub mod backends;
pub mod cli;
pub mod repl;
pub mod server;
pub mod test;

pub fn main_entry() {
    match cli::parse_args() {
        cli::RunMode::Run(filename) => {
            repl::run_file(&filename);
        }
        cli::RunMode::Serve(path, show_routertab) => {
            server::run_serve(path, show_routertab);
        }
        cli::RunMode::Test(config) => {
            test::run_test(test::TestConfig {
                file: config.file,
                method: config.method,
                path: config.path,
                body: config.body,
            });
        }
        cli::RunMode::Repl => {
            println!("Zun REPL v0.1.0");
            repl::run_repl();
        }
    }
}
