//! Thin orchestration wrappers for review export/apply commands.
//!
//! The review crate owns CSV shape and validation. This module wires checked
//! project paths to that crate and prints human-readable counts.

use anyhow::Result;
use camino::Utf8PathBuf;

use crate::open_project;

pub fn export_review(project: &str) -> Result<u64> {
    let (paths, _) = open_project(project)?;
    let rows = tinotax_review::export_review(&paths)?;
    println!(
        "exported {rows} rows needing review to {}",
        paths.out().join("manual_review.csv")
    );
    Ok(rows)
}

pub fn apply_review(project: &str, file: &str) -> Result<u64> {
    let (paths, _) = open_project(project)?;
    let applied = tinotax_review::apply_review(&paths, &Utf8PathBuf::from(file))?;
    println!(
        "recorded {applied} review decisions to {}",
        paths.staging().join("review_overrides.jsonl")
    );
    Ok(applied)
}
