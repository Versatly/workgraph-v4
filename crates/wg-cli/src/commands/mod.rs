//! Individual command handlers and dispatch logic for the WorkGraph CLI.

mod actor;
mod brief;
mod capabilities;
mod checkpoint;
mod claim;
mod connect;
mod create;
mod init;
mod ledger;
mod query;
mod run;
mod schema;
pub(crate) mod serve;
mod show;
mod status;
mod thread_complete;
mod trigger;

use crate::app::AppContext;
use crate::args::{ActorCommand, Command, McpCommand, RunCommand, TriggerCommand};
use crate::output::CommandOutput;

/// Executes the selected CLI command using the shared application context.
///
/// # Errors
///
/// Returns an error when the command cannot be completed successfully.
pub async fn execute(app: &AppContext, command: Command) -> anyhow::Result<CommandOutput> {
    match command {
        Command::Init => Ok(CommandOutput::Init(init::handle(app).await?)),
        Command::Connect {
            server,
            token,
            actor_id,
        } => Ok(CommandOutput::Connect(
            connect::handle(app, &server, &token, &actor_id).await?,
        )),
        Command::Whoami => Ok(CommandOutput::Whoami(connect::whoami(app).await?)),
        Command::Serve {
            listen,
            actor_id,
            access_scope,
            ..
        } => Ok(CommandOutput::Serve(serve::describe_http(
            app,
            &listen,
            Some(actor_id.as_str()),
            access_scope.map(|scope| scope.0),
        )?)),
        Command::Brief { lens } => Ok(CommandOutput::Brief(brief::handle(app, lens.0).await?)),
        Command::Status => Ok(CommandOutput::Status(status::handle(app).await?)),
        Command::Claim { thread_id } => {
            Ok(CommandOutput::Claim(claim::handle(app, &thread_id).await?))
        }
        Command::Complete { thread_id } => Ok(CommandOutput::Complete(
            thread_complete::handle(app, &thread_id).await?,
        )),
        Command::Checkpoint { working_on, focus } => Ok(CommandOutput::Checkpoint(
            checkpoint::handle(app, &working_on, &focus).await?,
        )),
        Command::Ledger { last } => Ok(CommandOutput::Ledger(ledger::handle(app, last).await?)),
        Command::Capabilities => Ok(CommandOutput::Capabilities(capabilities::handle())),
        Command::Schema { primitive_type } => Ok(CommandOutput::Schema(
            schema::handle(app, primitive_type.as_deref()).await?,
        )),
        Command::Create {
            primitive_type,
            title,
            fields,
            dry_run,
            stdin,
        } => Ok(CommandOutput::Create(
            create::handle(
                app,
                &primitive_type,
                title.as_deref(),
                &fields,
                dry_run,
                stdin,
            )
            .await?,
        )),
        Command::Query {
            primitive_type,
            filters,
        } => Ok(CommandOutput::Query(
            query::handle(app, &primitive_type, &filters).await?,
        )),
        Command::Show { reference } => {
            Ok(CommandOutput::Show(show::handle(app, &reference).await?))
        }
        Command::Run { command } => match command {
            RunCommand::Create {
                title,
                thread_id,
                actor_id,
                kind,
                source,
                executor_id,
                mission_id,
                parent_run_id,
                summary,
                dry_run,
            } => Ok(CommandOutput::RunCreate(
                run::create(
                    app,
                    run::RunCreateArgs {
                        title,
                        thread_id,
                        actor_id,
                        kind,
                        source,
                        executor_id,
                        mission_id,
                        parent_run_id,
                        summary,
                        dry_run,
                    },
                )
                .await?,
            )),
            RunCommand::Start { run_id } => {
                Ok(CommandOutput::RunLifecycle(run::start(app, &run_id).await?))
            }
            RunCommand::Complete { run_id, summary } => Ok(CommandOutput::RunLifecycle(
                run::complete(app, &run_id, summary.as_deref()).await?,
            )),
            RunCommand::Fail { run_id, summary } => Ok(CommandOutput::RunLifecycle(
                run::fail(app, &run_id, summary.as_deref()).await?,
            )),
            RunCommand::Cancel { run_id, summary } => Ok(CommandOutput::RunLifecycle(
                run::cancel(app, &run_id, summary.as_deref()).await?,
            )),
        },
        Command::Trigger { command } => match command {
            TriggerCommand::Validate { reference } => Ok(CommandOutput::TriggerValidate(
                trigger::validate(app, &reference).await?,
            )),
            TriggerCommand::Replay { last } => Ok(CommandOutput::TriggerReplay(
                trigger::replay(app, last).await?,
            )),
            TriggerCommand::Ingest {
                source,
                event_id,
                event_name,
                provider,
                fields,
            } => Ok(CommandOutput::TriggerIngest(
                trigger::ingest(
                    app,
                    trigger::TriggerIngestArgs {
                        source,
                        event_id: event_id.unwrap_or_else(|| "manual-ingest".to_owned()),
                        event_name,
                        provider,
                        actor_id: trigger::field_value(&fields, "actor_id"),
                        subject_reference: trigger::field_value(&fields, "subject_reference"),
                        primitive_type: trigger::field_value(&fields, "primitive_type"),
                        primitive_id: trigger::field_value(&fields, "primitive_id"),
                        op: trigger::field_value(&fields, "op"),
                        fields,
                    },
                )
                .await?,
            )),
        },
        Command::Actor { command } => match command {
            ActorCommand::Register {
                actor_type,
                id,
                title,
                email,
                runtime,
                parent_actor_id,
                root_actor_id,
                lineage_mode,
                capabilities,
            } => Ok(CommandOutput::ActorRegister(
                actor::register(
                    app,
                    actor::ActorRegisterArgs {
                        actor_type,
                        id,
                        title,
                        email,
                        runtime,
                        parent_actor_id,
                        root_actor_id,
                        lineage_mode,
                        capabilities,
                    },
                )
                .await?,
            )),
            ActorCommand::List { actor_type } => Ok(CommandOutput::ActorList(
                actor::list(app, actor_type.as_deref()).await?,
            )),
            ActorCommand::Show { reference } => Ok(CommandOutput::ActorShow(
                actor::show(app, &reference).await?,
            )),
        },
        Command::Mcp { command } => match command {
            McpCommand::Serve {
                actor_id,
                access_scope,
            } => Ok(CommandOutput::Serve(serve::describe_mcp(
                app,
                Some(actor_id.as_str()),
                access_scope.map(|scope| scope.0),
            )?)),
        },
    }
}
