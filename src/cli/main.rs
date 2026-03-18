use clap::{Parser, Subcommand};

mod client;
mod commands;
mod config;

#[derive(Parser)]
#[command(name = "press", about = "Printing Press CLI", version)]
struct Cli {
    /// Environment: development (default), prd/production
    #[arg(short, long, default_value = "development")]
    env: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Publish a post to subscribers
    Publish {
        /// Post slug or URL (e.g., "my-post" or "https://www.philipithomas.com/my-post")
        slug: String,
        /// Force send even if some subscribers already received it
        #[arg(long)]
        force: bool,
        /// Send to a single email address (test mode)
        #[arg(long)]
        to: Option<String>,
    },
    /// Import subscribers from a CSV file
    Import {
        /// CSV format: ghost-members or postcard
        #[arg(long)]
        format: String,
        /// Path to CSV file
        path: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let env_config = config::resolve_env(&cli.env)?;

    match cli.command {
        Commands::Publish { slug, force, to } => {
            commands::publish::run(&env_config, &slug, force, to.as_deref()).await?;
        }
        Commands::Import { format, path } => {
            commands::import::run(&env_config, &format, &path).await?;
        }
    }

    Ok(())
}
