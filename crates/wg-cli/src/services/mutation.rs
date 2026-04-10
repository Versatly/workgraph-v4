//! CLI-side generic primitive mutation services.

use anyhow::{Context, bail};
use wg_clock::RealClock;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::{AuditedWriteRequest, StoredPrimitive, write_primitive_audited};
use wg_trigger::ingest_ledger_entry;
use wg_types::{ActorId, LedgerEntry, LedgerOp, Registry};

use crate::app::AppContext;

/// Domain mutation service for registry-backed primitive writes outside the
/// coordination family crates.
#[derive(Debug, Clone)]
pub struct PrimitiveMutationService<'a> {
    app: &'a AppContext,
    registry: &'a Registry,
}

impl<'a> PrimitiveMutationService<'a> {
    /// Creates a new generic primitive mutation service.
    #[must_use]
    pub fn new(app: &'a AppContext, registry: &'a Registry) -> Self {
        Self { app, registry }
    }

    /// Persists a newly created primitive after policy validation.
    ///
    /// # Errors
    ///
    /// Returns an error when policy evaluation or audited persistence fails.
    pub async fn create(
        &self,
        actor: ActorId,
        primitive: &StoredPrimitive,
    ) -> anyhow::Result<(String, LedgerEntry)> {
        let primitive_type = primitive.frontmatter.r#type.as_str();
        let primitive_id = primitive.frontmatter.id.as_str();

        let policy_decision = evaluate_policy(
            self.app.workspace(),
            &actor,
            PolicyAction::Create,
            primitive_type,
            &PolicyContext::default(),
        )
        .await
        .with_context(|| {
            format!("failed to evaluate policy for {primitive_type}/{primitive_id}")
        })?;
        if policy_decision == PolicyDecision::Deny {
            bail!("policy denied creation of {primitive_type}/{primitive_id} for actor '{actor}'");
        }

        let (path, ledger_entry) = write_primitive_audited(
            self.app.workspace(),
            self.registry,
            primitive,
            AuditedWriteRequest::new(actor, LedgerOp::Create),
            RealClock::new(),
        )
        .await
        .with_context(|| format!("failed to create {primitive_type}/{primitive_id}"))?;

        self.after_mutation(primitive, &ledger_entry).await?;

        Ok((path.as_path().display().to_string(), ledger_entry))
    }

    async fn after_mutation(
        &self,
        _primitive: &StoredPrimitive,
        ledger_entry: &LedgerEntry,
    ) -> anyhow::Result<()> {
        ingest_ledger_entry(self.app.workspace(), ledger_entry)
            .await
            .context("failed to ingest ledger entry into trigger plane")?;
        Ok(())
    }
}
