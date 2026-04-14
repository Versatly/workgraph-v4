#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Minimal MCP stdio adapter for WorkGraph.

use std::sync::Arc;

use anyhow::{Context, bail};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use wg_types::{RemoteCommandRequest, RemoteCommandResponse};

/// MCP command executor used by the stdio server.
#[async_trait]
pub trait McpCommandExecutor: Send + Sync {
    /// Executes one CLI-shaped command request on behalf of a tool call.
    async fn execute(&self, request: RemoteCommandRequest)
    -> anyhow::Result<RemoteCommandResponse>;
}

/// Serves a minimal MCP stdio endpoint until stdin closes.
///
/// # Errors
///
/// Returns an error when the stdio protocol cannot be parsed or writing fails.
pub async fn serve_stdio(executor: Arc<dyn McpCommandExecutor>) -> anyhow::Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = tokio::io::BufWriter::new(stdout);

    while let Some(message) = read_message(&mut reader).await? {
        if let Some(response) = handle_message(&executor, message).await? {
            write_message(&mut writer, &response).await?;
        }
    }

    writer.flush().await.context("failed to flush MCP stdout")
}

async fn handle_message(
    executor: &Arc<dyn McpCommandExecutor>,
    message: Value,
) -> anyhow::Result<Option<Value>> {
    let method = message
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let id = message.get("id").cloned();

    match method {
        "initialize" => Ok(id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "serverInfo": {
                        "name": "workgraph-mcp",
                        "version": "0.1.0"
                    },
                    "capabilities": {
                        "tools": {
                            "listChanged": false
                        }
                    }
                }
            })
        })),
        "notifications/initialized" => Ok(None),
        "tools/list" => Ok(id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": tool_definitions()
                }
            })
        })),
        "tools/call" => {
            let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
            let tool_name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow::anyhow!("missing MCP tool name"))?;
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let request = tool_request(tool_name, arguments)?;
            let result = executor.execute(request).await?;
            Ok(id.map(|id| {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": result.rendered
                            }
                        ],
                        "isError": !result.success
                    }
                })
            }))
        }
        "" => Ok(None),
        other => Ok(id.map(|id| {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32601,
                    "message": format!("unsupported MCP method '{other}'")
                }
            })
        })),
    }
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "whoami",
            "description": "Show the effective WorkGraph actor and connection mode.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "brief",
            "description": "Return a structured workspace brief.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "lens": { "type": "string" }
                }
            }
        },
        {
            "name": "status",
            "description": "Return workspace status and graph hygiene.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "query",
            "description": "Query primitives by type.",
            "inputSchema": {
                "type": "object",
                "required": ["primitive_type"],
                "properties": {
                    "primitive_type": { "type": "string" }
                }
            }
        },
        {
            "name": "show",
            "description": "Show one primitive by reference.",
            "inputSchema": {
                "type": "object",
                "required": ["reference"],
                "properties": {
                    "reference": { "type": "string" }
                }
            }
        },
        {
            "name": "create",
            "description": "Create one primitive.",
            "inputSchema": {
                "type": "object",
                "required": ["primitive_type", "title"],
                "properties": {
                    "primitive_type": { "type": "string" },
                    "title": { "type": "string" },
                    "dry_run": { "type": "boolean" },
                    "fields": {
                        "type": "object",
                        "additionalProperties": { "type": "string" }
                    }
                }
            }
        },
        {
            "name": "claim",
            "description": "Claim a thread for the configured actor.",
            "inputSchema": {
                "type": "object",
                "required": ["thread_id"],
                "properties": {
                    "thread_id": { "type": "string" }
                }
            }
        },
        {
            "name": "complete",
            "description": "Complete a thread once evidence is satisfied.",
            "inputSchema": {
                "type": "object",
                "required": ["thread_id"],
                "properties": {
                    "thread_id": { "type": "string" }
                }
            }
        },
        {
            "name": "run_create",
            "description": "Create a queued run bound to a thread.",
            "inputSchema": {
                "type": "object",
                "required": ["title", "thread_id"],
                "properties": {
                    "title": { "type": "string" },
                    "thread_id": { "type": "string" },
                    "actor_id": { "type": "string" },
                    "kind": { "type": "string" },
                    "source": { "type": "string" },
                    "summary": { "type": "string" },
                    "dry_run": { "type": "boolean" }
                }
            }
        },
        {
            "name": "run_start",
            "description": "Mark a queued run as running.",
            "inputSchema": {
                "type": "object",
                "required": ["run_id"],
                "properties": {
                    "run_id": { "type": "string" }
                }
            }
        },
        {
            "name": "run_complete",
            "description": "Mark a run as completed.",
            "inputSchema": {
                "type": "object",
                "required": ["run_id"],
                "properties": {
                    "run_id": { "type": "string" },
                    "summary": { "type": "string" }
                }
            }
        },
        {
            "name": "actor_register",
            "description": "Register a person or agent actor.",
            "inputSchema": {
                "type": "object",
                "required": ["actor_type", "id", "title"],
                "properties": {
                    "actor_type": { "type": "string" },
                    "id": { "type": "string" },
                    "title": { "type": "string" },
                    "runtime": { "type": "string" },
                    "email": { "type": "string" },
                    "parent_actor_id": { "type": "string" },
                    "root_actor_id": { "type": "string" },
                    "lineage_mode": { "type": "string" },
                    "capabilities": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                }
            }
        },
        {
            "name": "actor_list",
            "description": "List registered people and agents.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "actor_type": { "type": "string" }
                }
            }
        }
    ])
}

fn tool_request(tool_name: &str, arguments: Value) -> anyhow::Result<RemoteCommandRequest> {
    let mut args = vec!["workgraph".to_owned(), "--json".to_owned()];
    match tool_name {
        "whoami" => args.push("whoami".to_owned()),
        "brief" => {
            args.push("brief".to_owned());
            if let Some(lens) = string_arg(&arguments, "lens") {
                push_flag(&mut args, "--lens", &lens);
            }
        }
        "status" => args.push("status".to_owned()),
        "query" => {
            args.push("query".to_owned());
            args.push(required_string_arg(&arguments, "primitive_type")?);
        }
        "show" => {
            args.push("show".to_owned());
            args.push(required_string_arg(&arguments, "reference")?);
        }
        "create" => {
            args.push("create".to_owned());
            args.push(required_string_arg(&arguments, "primitive_type")?);
            push_flag(
                &mut args,
                "--title",
                &required_string_arg(&arguments, "title")?,
            );
            if arguments
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                args.push("--dry-run".to_owned());
            }
            append_fields(&mut args, arguments.get("fields"));
        }
        "claim" => {
            args.push("claim".to_owned());
            args.push(required_string_arg(&arguments, "thread_id")?);
        }
        "complete" => {
            args.push("complete".to_owned());
            args.push(required_string_arg(&arguments, "thread_id")?);
        }
        "run_create" => {
            args.extend(["run".to_owned(), "create".to_owned()]);
            push_flag(
                &mut args,
                "--title",
                &required_string_arg(&arguments, "title")?,
            );
            push_flag(
                &mut args,
                "--thread-id",
                &required_string_arg(&arguments, "thread_id")?,
            );
            append_optional_flag(&mut args, "--actor-id", string_arg(&arguments, "actor_id"));
            append_optional_flag(&mut args, "--kind", string_arg(&arguments, "kind"));
            append_optional_flag(&mut args, "--source", string_arg(&arguments, "source"));
            append_optional_flag(&mut args, "--summary", string_arg(&arguments, "summary"));
            if arguments
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                args.push("--dry-run".to_owned());
            }
        }
        "run_start" => {
            args.extend(["run".to_owned(), "start".to_owned()]);
            args.push(required_string_arg(&arguments, "run_id")?);
        }
        "run_complete" => {
            args.extend(["run".to_owned(), "complete".to_owned()]);
            args.push(required_string_arg(&arguments, "run_id")?);
            append_optional_flag(&mut args, "--summary", string_arg(&arguments, "summary"));
        }
        "actor_register" => {
            args.extend(["actor".to_owned(), "register".to_owned()]);
            push_flag(
                &mut args,
                "--type",
                &required_string_arg(&arguments, "actor_type")?,
            );
            push_flag(&mut args, "--id", &required_string_arg(&arguments, "id")?);
            push_flag(
                &mut args,
                "--title",
                &required_string_arg(&arguments, "title")?,
            );
            append_optional_flag(&mut args, "--runtime", string_arg(&arguments, "runtime"));
            append_optional_flag(&mut args, "--email", string_arg(&arguments, "email"));
            append_optional_flag(
                &mut args,
                "--parent-actor-id",
                string_arg(&arguments, "parent_actor_id"),
            );
            append_optional_flag(
                &mut args,
                "--root-actor-id",
                string_arg(&arguments, "root_actor_id"),
            );
            append_optional_flag(
                &mut args,
                "--lineage-mode",
                string_arg(&arguments, "lineage_mode"),
            );
            if let Some(capabilities) = arguments.get("capabilities").and_then(Value::as_array) {
                for capability in capabilities.iter().filter_map(Value::as_str) {
                    push_flag(&mut args, "--capability", capability);
                }
            }
        }
        "actor_list" => {
            args.extend(["actor".to_owned(), "list".to_owned()]);
            append_optional_flag(&mut args, "--type", string_arg(&arguments, "actor_type"));
        }
        other => bail!("unsupported MCP tool '{other}'"),
    }

    Ok(RemoteCommandRequest {
        args,
        actor_id: None,
    })
}

fn string_arg(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn required_string_arg(arguments: &Value, key: &str) -> anyhow::Result<String> {
    string_arg(arguments, key)
        .ok_or_else(|| anyhow::anyhow!("missing required MCP argument '{key}'"))
}

fn push_flag(args: &mut Vec<String>, flag: &str, value: &str) {
    args.push(flag.to_owned());
    args.push(value.to_owned());
}

fn append_optional_flag(args: &mut Vec<String>, flag: &str, value: Option<String>) {
    if let Some(value) = value {
        push_flag(args, flag, &value);
    }
}

fn append_fields(args: &mut Vec<String>, value: Option<&Value>) {
    if let Some(fields) = value.and_then(Value::as_object) {
        for (key, value) in fields {
            if let Some(value) = value.as_str() {
                args.push("--field".to_owned());
                args.push(format!("{key}={value}"));
            }
        }
    }
}

async fn read_message(reader: &mut BufReader<tokio::io::Stdin>) -> anyhow::Result<Option<Value>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .await
            .context("failed to read MCP header line")?;
        if read == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(length) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                length
                    .trim()
                    .parse::<usize>()
                    .context("failed to parse MCP Content-Length header")?,
            );
        }
    }

    let content_length = content_length.ok_or_else(|| anyhow::anyhow!("missing Content-Length"))?;
    let mut body = vec![0; content_length];
    reader
        .read_exact(&mut body)
        .await
        .context("failed to read MCP message body")?;
    let message = serde_json::from_slice(&body).context("failed to decode MCP JSON body")?;
    Ok(Some(message))
}

async fn write_message(
    writer: &mut tokio::io::BufWriter<tokio::io::Stdout>,
    message: &Value,
) -> anyhow::Result<()> {
    let body = serde_json::to_vec(message).context("failed to encode MCP response body")?;
    writer
        .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
        .await
        .context("failed to write MCP response header")?;
    writer
        .write_all(&body)
        .await
        .context("failed to write MCP response body")?;
    writer.flush().await.context("failed to flush MCP response")
}

#[cfg(test)]
mod tests {
    use super::tool_request;

    #[test]
    fn actor_register_tool_maps_to_cli_args() {
        let request = tool_request(
            "actor_register",
            serde_json::json!({
                "actor_type": "agent",
                "id": "agent:cursor",
                "title": "Cursor",
                "runtime": "cursor",
                "capabilities": ["coding", "review"]
            }),
        )
        .expect("tool request should build");

        assert_eq!(request.args[0], "workgraph");
        assert!(request.args.contains(&"actor".to_owned()));
        assert!(request.args.contains(&"register".to_owned()));
        assert!(request.args.contains(&"--capability".to_owned()));
    }
}
