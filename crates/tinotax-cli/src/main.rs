//! Thin binary: parse arguments, set up tracing, dispatch to `tinotax-app`.

mod cli;

use anyhow::Result;
use clap::Parser;

use cli::{
    CalculateCommand, Cli, Command, LedgerCommand, PackCommand, PricesCommand, ProjectCommand,
    ReviewCommand,
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tinotax=info,warn".into()),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Doctor => tinotax_app::doctor().await,

        Command::Demo {
            config,
            out,
            resume,
        } => tinotax_app::run_demo(&config, &out, resume).await,

        Command::Project { command } => match command {
            ProjectCommand::Init { config, out } => {
                tinotax_app::project_init(&config, &out).map(|_| ())
            }
        },

        Command::Fetch { project, resume } => tinotax_app::fetch_project(&project.project, resume)
            .await
            .map(|_| ()),

        Command::ImportCex { project } => tinotax_app::import_cex(&project.project),

        Command::Normalise { project } => {
            tinotax_app::normalise_project(&project.project).map(|_| ())
        }

        Command::Diagnose { project } => {
            tinotax_app::diagnose_project(&project.project).map(|_| ())
        }

        Command::Report { project } => tinotax_app::export_reports(&project.project),

        Command::Review { command } => match command {
            ReviewCommand::ExportAll { project } => {
                tinotax_app::export_review_all(&project.project).map(|_| ())
            }
            ReviewCommand::ExportUncertain { project } => {
                tinotax_app::export_review(&project.project).map(|_| ())
            }
            ReviewCommand::Apply { project, file } => {
                tinotax_app::apply_review(&project.project, &file).map(|_| ())
            }
        },

        Command::Ledger { command } => match command {
            LedgerCommand::Build { project } => tinotax_app::ledger_build(&project.project),
            LedgerCommand::Price { project } => tinotax_app::ledger_price(&project.project),
        },

        Command::Prices { command } => match command {
            PricesCommand::Missing { project } => tinotax_app::prices_missing(&project.project),
            PricesCommand::Import { project, file } => {
                tinotax_app::prices_import(&project.project, &file)
            }
            PricesCommand::Fetch { project, provider } => {
                tinotax_app::prices_fetch(&project.project, &provider).await
            }
        },

        Command::Calculate { command } => match command {
            CalculateCommand::Uk {
                project,
                tax_year,
                allow_unpriced,
            } => tinotax_app::calculate_uk(&project.project, &tax_year, allow_unpriced),
        },

        Command::Pack { command } => match command {
            PackCommand::Hmrc { project, tax_year } => {
                tinotax_app::pack_hmrc(&project.project, &tax_year)
            }
        },
    }
}
