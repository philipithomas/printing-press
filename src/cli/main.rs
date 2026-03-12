use clap::{Parser, Subcommand};

mod client;
mod commands;
mod config;
mod keystore;

#[derive(Parser)]
#[command(name = "pp", about = "Printing Press CLI", version)]
struct Cli {
    /// Environment: development (default), prd/production
    #[arg(short, long, default_value = "development")]
    env: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Store API key for an environment
    Login,
    /// Publish a post to subscribers
    Publish {
        /// Post slug (e.g., "my-post")
        slug: String,
        /// Force send even if some subscribers already received it
        #[arg(long)]
        force: bool,
        /// Send to a single email address (test mode)
        #[arg(long)]
        to: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let env_config = config::resolve_env(&cli.env)?;

    match cli.command {
        Commands::Login => {
            commands::login::run(&env_config)?;
        }
        Commands::Publish { slug, force, to } => {
            commands::publish::run(&env_config, &slug, force, to.as_deref()).await?;
        }
    }

    Ok(())
}
