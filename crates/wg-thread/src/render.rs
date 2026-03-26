use wg_error::{Result, WorkgraphError};
use wg_types::{ActorId, ConversationMessage, CoordinationAction, MessageKind};

use crate::Thread;

pub(crate) fn render_thread_body(thread: &Thread) -> Result<String> {
    let mut rendered = String::new();

    rendered.push_str("## Exit Criteria\n");
    if thread.exit_criteria.is_empty() {
        rendered.push_str("None recorded.\n");
    } else {
        for criterion in &thread.exit_criteria {
            let required = if criterion.required {
                "required"
            } else {
                "optional"
            };
            rendered.push_str(&format!(
                "- {} [{}] ({})\n",
                criterion.title, criterion.id, required
            ));
            if let Some(description) = &criterion.description {
                rendered.push_str(&format!("  {}\n", description.trim()));
            }
            if let Some(reference) = &criterion.reference {
                rendered.push_str(&format!("  reference: {}\n", reference));
            }
        }
    }

    rendered.push_str("\n## Evidence\n");
    if thread.evidence.is_empty() {
        rendered.push_str("None recorded.\n");
    } else {
        for evidence in &thread.evidence {
            rendered.push_str(&format!("- {} ({})\n", evidence.title, evidence.id));
            if !evidence.satisfies.is_empty() {
                rendered.push_str(&format!("  satisfies: {}\n", evidence.satisfies.join(", ")));
            }
            if let Some(reference) = &evidence.reference {
                rendered.push_str(&format!("  reference: {}\n", reference));
            }
            if let Some(source) = &evidence.source {
                rendered.push_str(&format!("  source: {}\n", source));
            }
        }
    }

    rendered.push_str("\n## Update Actions\n");
    render_actions(&mut rendered, &thread.update_actions);
    rendered.push_str("\n## Completion Actions\n");
    render_actions(&mut rendered, &thread.completion_actions);

    let yaml = serde_yaml::to_string(&thread.messages).map_err(encoding_error)?;
    let yaml = yaml
        .strip_prefix("---\n")
        .or_else(|| yaml.strip_prefix("---\r\n"))
        .unwrap_or(yaml.as_str());
    let trailing_newline = if yaml.ends_with('\n') { "" } else { "\n" };
    rendered.push_str("\n## Conversation\n\n```yaml\n");
    rendered.push_str(yaml);
    rendered.push_str(trailing_newline);
    rendered.push_str("```\n");

    Ok(rendered)
}

fn render_actions(rendered: &mut String, actions: &[CoordinationAction]) {
    if actions.is_empty() {
        rendered.push_str("None planned.\n");
        return;
    }

    for action in actions {
        rendered.push_str(&format!(
            "- {} ({}) [{}]\n",
            action.title, action.id, action.kind
        ));
        if let Some(target_reference) = &action.target_reference {
            rendered.push_str(&format!("  target: {}\n", target_reference));
        }
        if let Some(description) = &action.description {
            rendered.push_str(&format!("  {}\n", description.trim()));
        }
    }
}

pub(crate) fn parse_conversation_messages(body: &str) -> Result<Vec<ConversationMessage>> {
    let Some(opening) = body.find("```yaml") else {
        return Ok(Vec::new());
    };

    let after_opening = &body[opening + "```yaml".len()..];
    let after_newline = after_opening.strip_prefix('\n').unwrap_or(after_opening);
    let Some(closing) = after_newline.find("\n```") else {
        return Err(WorkgraphError::ValidationError(
            "thread conversation body is missing closing ``` fence".to_owned(),
        ));
    };
    let yaml = &after_newline[..closing];

    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_yaml::from_str::<Vec<ConversationMessage>>(yaml).map_err(encoding_error)
}

pub(crate) fn infer_message_kind(actor: &ActorId) -> MessageKind {
    let value = actor.as_str();
    if value.starts_with("agent:") || value.starts_with("agent/") {
        MessageKind::Agent
    } else {
        MessageKind::Human
    }
}

pub(crate) fn encoding_error(error: impl std::fmt::Display) -> WorkgraphError {
    WorkgraphError::EncodingError(error.to_string())
}
