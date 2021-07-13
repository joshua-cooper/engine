use {
    log::error,
    std::{env, fs::File, io, process},
};

const DEFAULT_FILE: &str = "transactions.csv";

fn main() {
    env_logger::init();

    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from(DEFAULT_FILE));

    match File::open(&path) {
        Ok(file) => {
            if let Err(e) = engine::run(file, io::stdout()) {
                error!("Fatal error: {}", e);
                process::exit(1);
            }
        }
        Err(e) => {
            error!("Error opening \"{}\": {}", path, e);
            process::exit(1);
        }
    }
}
