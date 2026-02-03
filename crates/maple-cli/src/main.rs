use clap::{Parser, Subcommand};
use std::ffi::OsString;

#[derive(Parser)]
#[command(name = "maple", about = "MAPLE AI Framework CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show version information
    Version,

    /// Validate a local file (developer utility)
    Validate { #[arg(short, long)] file: String },

    /// PALM operations (forwarded to palm-cli)
    #[command(alias = "ops")]
    Palm {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!("Maple AI Framework v{}", env!("CARGO_PKG_VERSION"));
            println!("\nResonance Architecture - Intelligence free to reason, action bound by obligation.");
        }
        Commands::Validate { file } => println!("Validating: {}", file),
        Commands::Palm { args } => {
            let mut forwarded = Vec::with_capacity(args.len() + 1);
            forwarded.push(OsString::from("palm"));
            forwarded.extend(args);

            if let Err(err) = palm_cli::run_with_args(forwarded).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    }
}
