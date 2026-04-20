//! Implementation of the `workgraph create` command.

use anyhow::{Context, anyhow};
use tokio::fs;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive};
use wg_trigger::{TriggerMutationService, load_trigger};

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::output::{CreateOutcome, CreateOutput};
use crate::services::mutation::PrimitiveMutationService;
use crate::util::fields::split_body_and_frontmatter;
use crate::util::slug::{slugify, unique_slug};
use crate::util::stdin::{merge_fields, parse_create_stdin_payload};

/// Creates a new primitive and appends a matching ledger entry.
///
/// # Errors
///
/// Returns an error when the workspace metadata is missing, the primitive type is unknown,
/// validation fails, or persistence cannot be completed.
pub async fn handle(
    app: &AppContext,
    primitive_type: &str,
    title: Option<&str>,
    fields: &[KeyValueInput],
    dry_run: bool,
    stdin: bool,
) -> anyhow::Result<CreateOutput> {
    let registry = app.load_registry().await?;
    let runtime_registry = app.load_runtime_registry().await?;
    let primitive_definition = runtime_registry
        .get_type(primitive_type)
        .ok_or_else(|| anyhow!("unknown primitive type '{primitive_type}'"))?;

    let (requested_title, merged_fields) = resolve_create_inputs(title, fields, stdin)?;
    let id = slugify(&requested_title);
    let primitive_path = app.workspace().primitive_path(primitive_type, &id);
    let (body, extra_fields) =
        split_body_and_frontmatter(Some(primitive_definition), &merged_fields);
    let mut primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: primitive_type.to_owned(),
            id,
            title: requested_title,
            extra_fields,
        },
        body,
    };

    if fs::try_exists(primitive_path.as_path())
        .await
        .context("failed to inspect existing primitive path")?
    {
        let reference = format!("{primitive_type}/{}", primitive.frontmatter.id);
        let existing = read_primitive(app.workspace(), primitive_type, &primitive.frontmatter.id)
            .await
            .with_context(|| format!("failed to read existing primitive '{reference}'"))?;
        if existing == primitive {
            let path = primitive_path.as_path().display().to_string();
            return Ok(CreateOutput {
                outcome: CreateOutcome::Noop,
                reference,
                path,
                primitive,
                ledger_entry: None,
            });
        }

        let unique_id = unique_slug(
            app.workspace(),
            primitive_type,
            &primitive.frontmatter.title,
        )
        .await?;
        primitive.frontmatter.id = unique_id;
    }

    let reference = format!("{primitive_type}/{}", primitive.frontmatter.id);
    let path = app
        .workspace()
        .primitive_path(primitive_type, &primitive.frontmatter.id)
        .as_path()
        .display()
        .to_string();

    if dry_run {
        return Ok(CreateOutput {
            outcome: CreateOutcome::DryRun,
            reference,
            path,
            primitive,
            ledger_entry: None,
        });
    }

    let actor = app.effective_actor_id().await?;

    let (path, ledger_entry) = if primitive_type == "trigger" {
        let trigger = load_trigger_payload(&primitive)?;
        TriggerMutationService::new(app.workspace())
            .save_trigger_as(&trigger, actor.clone())
            .await?;
        let stored_trigger = load_trigger(app.workspace(), &trigger.id).await?;
        let stored_primitive = read_primitive(app.workspace(), "trigger", &stored_trigger.id)
            .await
            .with_context(|| {
                format!(
                    "failed to read stored trigger 'trigger/{}'",
                    stored_trigger.id
                )
            })?;
        primitive = stored_primitive;
        let ledger_entry = app
            .read_ledger_entries()
            .await?
            .into_iter()
            .rev()
            .find(|entry| entry.primitive_type == "trigger" && entry.primitive_id == trigger.id)
            .ok_or_else(|| anyhow!("failed to locate ledger entry for trigger/{}", trigger.id))?;
        (
            app.workspace()
                .primitive_path("trigger", &trigger.id)
                .as_path()
                .display()
                .to_string(),
            ledger_entry,
        )
    } else {
        PrimitiveMutationService::new(app, &registry)
            .create(actor, &primitive)
            .await?
    };

    Ok(CreateOutput {
        outcome: CreateOutcome::Created,
        reference,
        path,
        primitive,
        ledger_entry: Some(ledger_entry),
    })
}

fn resolve_create_inputs(
    cli_title: Option<&str>,
    cli_fields: &[KeyValueInput],
    stdin: bool,
) -> anyhow::Result<(String, Vec<KeyValueInput>)> {
    let stdin_payload = if stdin {
        Some(parse_create_stdin_payload()?)
    } else {
        None
    };
    let title = cli_title
        .map(ToOwned::to_owned)
        .or_else(|| {
            stdin_payload
                .as_ref()
                .and_then(|payload| payload.title.clone())
        })
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            anyhow!("missing title; provide --title \"<title>\" or include title in --stdin JSON")
        })?;
    let fields = if let Some(payload) = stdin_payload {
        merge_fields(cli_fields, &payload.fields)
    } else {
        cli_fields.to_vec()
    };
    Ok((title, fields))
}

fn load_trigger_payload(primitive: &StoredPrimitive) -> anyhow::Result<wg_trigger::Trigger> {
    let trigger = wg_trigger::trigger_from_primitive(primitive)
        .context("failed to decode trigger payload from create request")?;
    Ok(trigger)
}
