//! Implementation of the `workgraph ledger` command.

use crate::app::AppContext;
use crate::output::LedgerOutput;

/// Returns recent immutable ledger entries.
///
/// # Errors
///
/// Returns an error when the workspace ledger cannot be read.
pub async fn handle(app: &AppContext, last: Option<usize>) -> anyhow::Result<LedgerOutput> {
    let mut entries = app.read_ledger_entries().await?;
    entries.reverse();
    let limit = last.unwrap_or(10).max(1);
    entries.truncate(limit);

    Ok(LedgerOutput {
        count: entries.len(),
        entries,
    })
}
