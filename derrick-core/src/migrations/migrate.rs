use super::history::{ExistingMigration, HistoryTable};
use super::migration::{AppliedMigration, Migration};
use super::source::MigrationSource;
use crate::error::Error;

use futures_core::future::BoxFuture;

/// The runtime for applying a migration set.
///
/// There are required methods for interacting with the
/// schema history table, but additionally `apply_tx` and
/// `apply_no_tx`.  These should (respectively, should _not_)
/// run an individual migration within a transaction.
///
/// _Note_: If a migration is not ran in a transaction,
/// an outcome  where the history table reaches an
/// erroneous state is possible: when the migration
/// query itself succeeds but the query to update
/// the history with a new row does not succeed.
pub trait Migrate
where
    Self: Send + Sync,
{
    /// A `Migrate` has to manage the history table.
    type History: HistoryTable + Clone + Send + Sync;

    /// Additional data needed to initialize.
    type Init: Clone + Send + Sync;

    /// Create a new value.
    fn initialize(
        db_url: String,
        history: Self::History,
        data: Self::Init,
    ) -> BoxFuture<'static, Result<Self, Error>>
    where
        Self: Sized;

    /// Create the history table if it does not exist.
    fn check_history_table(&mut self) -> BoxFuture<'_, Result<(), Error>>;

    /// Get the full history table.
    fn get_history_table(&mut self) -> BoxFuture<'_, Result<Vec<ExistingMigration>, Error>>;

    /// Insert a newly applied migration.
    fn insert_new_applied<'a, 'c: 'a>(
        &'c mut self,
        applied: &'a AppliedMigration,
    ) -> BoxFuture<'a, Result<(), Error>>;

    /// Apply a migration and update history outside of a
    /// transaction.
    fn apply_no_tx<'a, 'c: 'a>(
        &'c mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>>;

    /// Apply a migration and update history in a transaction.
    fn apply_tx<'a, 'c: 'a>(
        &'c mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>>;

    /// Get the most recent applied migration version.
    fn current_version(&mut self) -> BoxFuture<'_, Result<Option<i64>, Error>> {
        Box::pin(async move {
            let current =
                self.get_history_table()
                    .await?
                    .into_iter()
                    .fold(None::<i64>, |acc, m| match acc {
                        None => Some(m.version),
                        Some(v) if m.version > v => Some(m.version),
                        _ => acc,
                    });

            Ok(current)
        })
    }

    /// Apply a migration.
    fn apply<'a, 'c: 'a>(
        &'c mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, Result<AppliedMigration, Error>> {
        if migration.no_tx {
            self.apply_no_tx(migration)
        } else {
            self.apply_tx(migration)
        }
    }

    /// Enforce rules about source migrations.
    fn validate_source(
        source: Vec<MigrationSource>,
        history: Vec<ExistingMigration>,
    ) -> Result<(), Error> {
        NoValidation::validate(source, history)
    }
}

/// Empty method for `validate_source`.
#[derive(Clone)]
pub struct NoValidation;

impl NoValidation {
    fn validate(
        _source: Vec<MigrationSource>,
        _applied: Vec<ExistingMigration>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
