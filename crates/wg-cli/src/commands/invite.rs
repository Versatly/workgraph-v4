//! Implementation of `workgraph invite` credential management commands.

use anyhow::{anyhow, bail};
use wg_types::{ActorId, HostedCredential, RemoteAccessScope};

use crate::app::AppContext;
use crate::output::{InviteCreateOutput, InviteListOutput, InviteRevokeOutput, InviteSummary};
use crate::util::slug::slugify;
use crate::util::token::{generate_token, token_hash};

/// Arguments for `workgraph invite create`.
#[derive(Debug, Clone)]
pub struct InviteCreateArgs {
    /// Stable human-readable label for the invite.
    pub label: String,
    /// Actor identity bound to this invite.
    pub actor_id: String,
    /// Server URL agents should use to connect.
    pub server: String,
    /// Hosted access scope granted to the actor.
    pub access_scope: Option<RemoteAccessScope>,
}

/// Creates one actor-bound hosted invite credential.
///
/// # Errors
///
/// Returns an error when the credential store cannot be read or written.
pub async fn create(
    app: &AppContext,
    args: InviteCreateArgs,
) -> anyhow::Result<InviteCreateOutput> {
    let label = normalize_label(&args.label)?;
    let id = format!("invite-{label}");
    let actor_id_input = args.actor_id.trim();
    if actor_id_input.is_empty() {
        bail!("invite create requires --actor-id");
    }
    let actor_id = ActorId::new(actor_id_input);
    let access_scope = args.access_scope.unwrap_or(RemoteAccessScope::Operate);
    let token = generate_token();
    let credential = HostedCredential {
        id: id.clone(),
        label: label.clone(),
        actor_id,
        access_scope,
        token_hash: token_hash(&token),
        revoked: false,
    };

    let mut store = app.load_credentials().await?;
    if let Some(existing) = store
        .credentials
        .iter()
        .find(|credential| credential.id == id || credential.label == label)
    {
        if !existing.revoked {
            bail!(
                "invite '{}' already exists; revoke it first or choose a different --label",
                existing.label
            );
        }
    }
    store
        .credentials
        .retain(|credential| credential.id != id && credential.label != label);
    store.credentials.push(credential.clone());
    app.write_credentials(&store).await?;

    let server = args.server.trim_end_matches('/').to_owned();
    let connect_command = format!(
        "workgraph connect --server {server} --token {token} --actor-id {}",
        credential.actor_id
    );

    Ok(InviteCreateOutput {
        credential: InviteSummary::from_credential(&credential),
        server,
        token,
        connect_command,
        credentials_path: app.credentials_path().display().to_string(),
    })
}

/// Lists hosted invite credentials.
///
/// # Errors
///
/// Returns an error when the credential store cannot be read.
pub async fn list(app: &AppContext) -> anyhow::Result<InviteListOutput> {
    let store = app.load_credentials().await?;
    let mut credentials = store
        .credentials
        .iter()
        .map(InviteSummary::from_credential)
        .collect::<Vec<_>>();
    credentials.sort_by(|left, right| left.label.cmp(&right.label).then(left.id.cmp(&right.id)));
    Ok(InviteListOutput {
        count: credentials.len(),
        credentials,
        credentials_path: app.credentials_path().display().to_string(),
    })
}

/// Revokes one hosted invite credential.
///
/// # Errors
///
/// Returns an error when the credential does not exist or the store cannot be written.
pub async fn revoke(app: &AppContext, label_or_id: &str) -> anyhow::Result<InviteRevokeOutput> {
    let target = label_or_id.trim();
    if target.is_empty() {
        bail!("invite revoke requires a credential label or id");
    }

    let mut store = app.load_credentials().await?;
    let credential = store
        .credentials
        .iter_mut()
        .find(|credential| credential.id == target || credential.label == target)
        .ok_or_else(|| anyhow!("invite credential '{target}' was not found"))?;
    credential.revoked = true;
    let summary = InviteSummary::from_credential(credential);
    app.write_credentials(&store).await?;

    Ok(InviteRevokeOutput {
        credential: summary,
        credentials_path: app.credentials_path().display().to_string(),
    })
}

fn normalize_label(label: &str) -> anyhow::Result<String> {
    let label = slugify(label.trim());
    if label.is_empty() {
        bail!("invite label must contain at least one letter or number");
    }
    Ok(label)
}
