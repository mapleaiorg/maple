use clap::{Parser, Subcommand};
use std::ffi::OsString;
use std::fs;

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
    Validate {
        #[arg(short, long)]
        file: String,
    },

    /// UAL parsing and compilation
    Ual {
        #[command(subcommand)]
        command: UalCommands,
    },

    /// PALM operations (forwarded to palm)
    #[command(alias = "ops")]
    Palm {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },

    /// Direct PALM operations shortcut (e.g. `maple spec list`)
    #[command(external_subcommand)]
    PalmShortcut(Vec<OsString>),
}

#[derive(Subcommand)]
enum UalCommands {
    /// Parse UAL into an AST
    Parse {
        #[arg(short, long)]
        file: String,
    },
    /// Compile UAL into RCF and PALM operations
    Compile {
        #[arg(short, long)]
        file: String,
    },
    /// Validate UAL commitments (RCF validation)
    Validate {
        #[arg(short, long)]
        file: String,
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
        Commands::Ual { command } => {
            if let Err(err) = handle_ual(command) {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Commands::Palm { args } => {
            let mut forwarded = Vec::with_capacity(args.len() + 1);
            forwarded.push(OsString::from("palm"));
            forwarded.extend(args);

            if let Err(err) = palm::run_with_args(forwarded).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
        Commands::PalmShortcut(args) => {
            let mut forwarded = Vec::with_capacity(args.len() + 1);
            forwarded.push(OsString::from("palm"));
            forwarded.extend(args);

            if let Err(err) = palm::run_with_args(forwarded).await {
                eprintln!("{err}");
                std::process::exit(1);
            }
        }
    }
}

fn handle_ual(command: UalCommands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        UalCommands::Parse { file } => {
            let input = fs::read_to_string(file)?;
            let ast = ual_parser::parse(&input)?;
            println!("{}", serde_json::to_string_pretty(&ast)?);
            Ok(())
        }
        UalCommands::Compile { file } => {
            let input = fs::read_to_string(file)?;
            let ast = ual_parser::parse(&input)?;
            let compiled = ual_compiler::compile(&ast)?;
            println!("{}", serde_json::to_string_pretty(&compiled)?);
            Ok(())
        }
        UalCommands::Validate { file } => {
            let input = fs::read_to_string(file)?;
            let ast = ual_parser::parse(&input)?;
            let compiled = ual_compiler::compile(&ast)?;
            let validator = rcf_validator::RcfValidator::new();
            for item in compiled {
                if let ual_compiler::UalCompiled::Commitment(commitment) = item {
                    validator.validate_commitment(&commitment)?;
                }
            }
            println!("UAL validation succeeded.");
            Ok(())
        }
    }
}
