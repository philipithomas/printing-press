use clap::{Parser, Subcommand};

mod client;
mod commands;
mod config;

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
    /// Send physical mail for a post via Lob
    Mail {
        /// Post slug (e.g., "my-post")
        slug: String,
        /// Force send even if some recipients already received it
        #[arg(long)]
        force: bool,
        /// Send to a single email address (test mode — looks up Stripe customer by email)
        #[arg(long)]
        to: Option<String>,
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
        Commands::Mail { slug, force, to } => {
            commands::mail::run(&env_config, &slug, force, to.as_deref()).await?;
        }
    }

    Ok(())
}
