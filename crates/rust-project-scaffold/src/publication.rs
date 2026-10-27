use crate::{CancellationToken, Plan, Result, check_cancel};

mod transaction;
use transaction::Transaction;

pub(crate) fn publish(plan: &Plan, token: &CancellationToken) -> Result<()> {
    check_cancel(token)?;
    if plan.options.dry_run {
        return Ok(());
    }
    Transaction::prepare(plan, token)?.commit_remaining(token)
}

#[cfg(test)]
#[path = "../tests/unit/publication.rs"]
mod tests;
