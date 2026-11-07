use clap::Parser;
use subraumctl::cli::Cli;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    subraumctl::commands::run(cli).await?;
    Ok(())
}
