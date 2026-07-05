//! Command-line surface. Parsing only — every command body lives in
//! `tinotax-app`; no business logic here.

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "tinotax",
    version,
    about = "Reviewed-ledger UK crypto tax CLI: fetch, review, price, calculate, evidence pack"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// `--project ./fox-project` — the folder created by `project init`.
#[derive(Debug, Args)]
pub struct ProjectArg {
    /// Path to the project folder
    #[arg(long)]
    pub project: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check config, environment and provider reachability
    Doctor,

    /// Run the whole ingestion pipeline in one shot (init → fetch →
    /// normalise → diagnose → review exports → reports)
    Demo {
        /// Wallet/source config, e.g. wallets.toml
        #[arg(long)]
        config: String,
        /// Project folder to create
        #[arg(long)]
        out: String,
        /// Reuse already-fetched raw pages
        #[arg(long)]
        resume: bool,
    },

    /// Project folder management
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },

    /// Fetch wallet history from chain APIs into the immutable raw cache
    Fetch {
        #[command(flatten)]
        project: ProjectArg,
        /// Reuse already-fetched raw pages
        #[arg(long)]
        resume: bool,
    },

    /// Import the CEX CSV exports declared as cex_csvs entries in project.toml
    ImportCex {
        #[command(flatten)]
        project: ProjectArg,
    },

    /// Normalise raw wallet pages into staging/normalised_events.jsonl
    Normalise {
        #[command(flatten)]
        project: ProjectArg,
    },

    /// Data quality and completeness diagnostics
    Diagnose {
        #[command(flatten)]
        project: ProjectArg,
    },

    /// Export normalised transactions CSV + audit manifest
    Report {
        #[command(flatten)]
        project: ProjectArg,
    },

    /// Review and edit the data (all human changes are recorded as
    /// overrides; raw and normalised data are never mutated)
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },

    /// Build and price the reviewed tax ledger
    Ledger {
        #[command(subcommand)]
        command: LedgerCommand,
    },

    /// Historical GBP prices
    Prices {
        #[command(subcommand)]
        command: PricesCommand,
    },

    /// Tax calculations
    Calculate {
        #[command(subcommand)]
        command: CalculateCommand,
    },

    /// Client-facing deliverables
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommand {
    /// Create a project folder from a config file
    Init {
        /// Wallet/source config, e.g. wallets.toml
        #[arg(long)]
        config: String,
        /// Project folder to create
        #[arg(long)]
        out: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ReviewCommand {
    /// Export every event to out/review_all_transactions.csv for full review
    ExportAll {
        #[command(flatten)]
        project: ProjectArg,
    },
    /// Export only rows flagged as uncertain to out/manual_review.csv
    ExportUncertain {
        #[command(flatten)]
        project: ProjectArg,
    },
    /// Apply an edited review CSV (records decisions, never mutates data)
    Apply {
        #[command(flatten)]
        project: ProjectArg,
        /// The edited review CSV
        #[arg(long)]
        file: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum LedgerCommand {
    /// Derive staging/reviewed_ledger.jsonl from events + review overrides
    Build {
        #[command(flatten)]
        project: ProjectArg,
    },
    /// Value the reviewed ledger in GBP using the price book
    Price {
        #[command(flatten)]
        project: ProjectArg,
    },
}

#[derive(Debug, Subcommand)]
pub enum PricesCommand {
    /// List (asset, date) pairs that still need a GBP price
    Missing {
        #[command(flatten)]
        project: ProjectArg,
    },
    /// Import a manual price CSV (asset_symbol,date,price_gbp and
    /// optionally source,note)
    Import {
        #[command(flatten)]
        project: ProjectArg,
        /// CSV of prices to import
        #[arg(long)]
        file: String,
    },
    /// Fetch missing daily GBP prices from a provider
    Fetch {
        #[command(flatten)]
        project: ProjectArg,
        /// Price provider
        #[arg(long, default_value = "coingecko")]
        provider: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum CalculateCommand {
    /// UK CGT (same-day, 30-day, Section 104) + income for one tax year
    Uk {
        #[command(flatten)]
        project: ProjectArg,
        /// Tax year label, e.g. 2024-2025
        #[arg(long)]
        tax_year: String,
        /// Exclude unpriced/unresolved rows (reported, not silently dropped)
        /// instead of refusing to calculate
        #[arg(long)]
        allow_unpriced: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum PackCommand {
    /// Build the HMRC / Self Assessment evidence pack for one tax year
    Hmrc {
        #[command(flatten)]
        project: ProjectArg,
        /// Tax year label, e.g. 2024-2025
        #[arg(long)]
        tax_year: String,
    },
}
