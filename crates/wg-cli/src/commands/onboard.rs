//! Implementation of `workgraph onboard` first-run setup.

use anyhow::Context;
use serde_yaml::Value;
use tokio::fs;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive};

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::commands::{actor, init};
use crate::output::{OnboardCreatedPrimitive, OnboardOutput};
use crate::services::mutation::PrimitiveMutationService;
use crate::util::slug::slugify;

/// Arguments for `workgraph onboard`.
#[derive(Debug, Clone)]
pub struct OnboardArgs {
    /// Durable person actor id for the operator.
    pub person_id: String,
    /// Human-readable operator name.
    pub person_title: String,
    /// Optional operator email.
    pub email: Option<String>,
    /// Optional initial org title.
    pub org_title: Option<String>,
    /// Optional initial project title.
    pub project_title: Option<String>,
    /// Optional initial mission title.
    pub mission_title: Option<String>,
    /// Optional initial thread title.
    pub thread_title: Option<String>,
    /// Initial agent actor ids and runtimes.
    pub agents: Vec<KeyValueInput>,
}

/// Bootstraps an operator, optional initial work primitives, and initial agents.
///
/// # Errors
///
/// Returns an error when workspace initialization or primitive creation fails.
pub async fn handle(app: &AppContext, args: OnboardArgs) -> anyhow::Result<OnboardOutput> {
    let init = init::handle(app).await?;
    let mut config = app.load_config().await?;
    let person_id = args.person_id.trim().to_owned();
    config.default_actor_id = Some(wg_types::ActorId::new(&person_id));
    app.write_config(&config).await?;

    let person = actor::register(
        app,
        actor::ActorRegisterArgs {
            actor_type: "person".to_owned(),
            id: person_id.clone(),
            title: args.person_title,
            email: args.email,
            runtime: None,
            parent_actor_id: None,
            root_actor_id: None,
            lineage_mode: None,
            capabilities: Vec::new(),
        },
    )
    .await?;

    let mut registered_agents = Vec::new();
    let mut agent_ids = Vec::new();
    for agent in args.agents {
        let agent_id = agent.key.trim().to_owned();
        let runtime = agent.value.trim().to_owned();
        let title = agent_title(&agent_id, &runtime);
        agent_ids.push(agent_id.clone());
        registered_agents.push(
            actor::register(
                app,
                actor::ActorRegisterArgs {
                    actor_type: "agent".to_owned(),
                    id: agent_id,
                    title,
                    email: None,
                    runtime: Some(runtime),
                    parent_actor_id: Some(person_id.clone()),
                    root_actor_id: Some(person_id.clone()),
                    lineage_mode: Some("opaque".to_owned()),
                    capabilities: vec!["agentic-work".to_owned()],
                },
            )
            .await?,
        );
    }

    let registry = app.load_registry().await?;
    let mutation = PrimitiveMutationService::new(app, &registry);
    let mut created_primitives = Vec::new();

    if let Some(title) = args.org_title {
        created_primitives
            .push(ensure_primitive(app, &mutation, "org", &title, Default::default()).await?);
    }
    if let Some(title) = args.project_title {
        let mut fields = std::collections::BTreeMap::new();
        if let Some(org) = created_primitives
            .iter()
            .find(|primitive| primitive.primitive.frontmatter.r#type == "org")
        {
            fields.insert(
                "client_id".to_owned(),
                Value::String(org.primitive.frontmatter.id.clone()),
            );
        }
        created_primitives.push(ensure_primitive(app, &mutation, "project", &title, fields).await?);
    }
    if let Some(title) = args.mission_title {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("status".to_owned(), Value::String("draft".to_owned()));
        if let Some(first_agent_id) = agent_ids.first() {
            fields.insert(
                "owner_actor_id".to_owned(),
                Value::String(first_agent_id.clone()),
            );
        } else {
            fields.insert(
                "owner_actor_id".to_owned(),
                Value::String(person_id.clone()),
            );
        }
        created_primitives.push(ensure_primitive(app, &mutation, "mission", &title, fields).await?);
    }
    if let Some(title) = args.thread_title {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("status".to_owned(), Value::String("open".to_owned()));
        if let Some(mission) = created_primitives
            .iter()
            .find(|primitive| primitive.primitive.frontmatter.r#type == "mission")
        {
            fields.insert(
                "parent_mission_id".to_owned(),
                Value::String(mission.primitive.frontmatter.id.clone()),
            );
        }
        if let Some(first_agent_id) = agent_ids.first() {
            fields.insert(
                "assigned_actor".to_owned(),
                Value::String(first_agent_id.clone()),
            );
        }
        created_primitives.push(ensure_primitive(app, &mutation, "thread", &title, fields).await?);
    }

    Ok(OnboardOutput {
        init,
        person,
        agents: registered_agents,
        created_primitives,
        default_actor_id: person_id,
    })
}

async fn ensure_primitive(
    app: &AppContext,
    mutation: &PrimitiveMutationService<'_>,
    primitive_type: &str,
    title: &str,
    extra_fields: std::collections::BTreeMap<String, Value>,
) -> anyhow::Result<OnboardCreatedPrimitive> {
    let id = slugify(title);
    let path = app.workspace().primitive_path(primitive_type, &id);
    if fs::try_exists(path.as_path())
        .await
        .context("failed to inspect existing onboard primitive path")?
    {
        let primitive = read_primitive(app.workspace(), primitive_type, &id)
            .await
            .with_context(|| format!("failed to read existing {primitive_type}/{id}"))?;
        return Ok(OnboardCreatedPrimitive {
            reference: format!("{primitive_type}/{id}"),
            created: false,
            primitive,
        });
    }

    let body = if primitive_type == "mission" {
        format!("{title}\n")
    } else {
        String::new()
    };
    let primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: primitive_type.to_owned(),
            id: id.clone(),
            title: title.to_owned(),
            extra_fields,
        },
        body,
    };
    mutation
        .create(app.effective_actor_id().await?, &primitive)
        .await?;

    Ok(OnboardCreatedPrimitive {
        reference: format!("{primitive_type}/{id}"),
        created: true,
        primitive,
    })
}

fn agent_title(actor_id: &str, runtime: &str) -> String {
    let title = actor_id
        .rsplit_once([':', '/'])
        .map(|(_, suffix)| suffix)
        .unwrap_or(actor_id)
        .split(['-', '_'])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();
    if title.is_empty() {
        format!("{runtime} agent")
    } else {
        title
    }
}
