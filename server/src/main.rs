use tarrowyn_server::{serve, ServerConfig};

fn main() {
    if let Err(error) = serve(ServerConfig::from_env()) {
        eprintln!("Tarrowyn server stopped: {error}");
        std::process::exit(1);
    }
}
