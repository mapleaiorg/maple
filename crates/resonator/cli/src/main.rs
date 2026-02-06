use resonator_cli::CliResult;

#[tokio::main]
async fn main() -> CliResult<()> {
    resonator_cli::run().await
}
