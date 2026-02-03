use palm_cli::CliResult;

#[tokio::main]
async fn main() -> CliResult<()> {
    palm_cli::run().await
}
