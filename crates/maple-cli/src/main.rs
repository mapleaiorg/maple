use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "maple", about = "Maple AI Framework CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Version,
    Validate { #[arg(short, long)] file: String },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!("Maple AI Framework v{}", env!("CARGO_PKG_VERSION"));
            println!("\nResonance Architecture - Intelligence free to reason, action bound by obligation.");
        }
        Commands::Validate { file } => println!("Validating: {}", file),
    }
}
