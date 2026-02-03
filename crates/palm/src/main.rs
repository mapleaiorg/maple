use palm::CliResult;

#[tokio::main]
async fn main() -> CliResult<()> {
    palm::run().await
}
