//! Individual command handlers and dispatch logic for the WorkGraph CLI.

mod brief;
mod capabilities;
mod checkpoint;
mod create;
mod init;
mod mission;
mod query;
mod run;
mod schema;
mod show;
mod status;
mod thread;
mod trigger;

use crate::app::AppContext;
use crate::args::Command;
use crate::output::CommandOutput;

/// Executes the selected CLI command using the shared application context.
///
/// # Errors
///
/// Returns an error when the command cannot be completed successfully.
pub async fn execute(app: &AppContext, command: Command) -> anyhow::Result<CommandOutput> {
    match command {
        Command::Init => Ok(CommandOutput::Init(init::handle(app).await?)),
        Command::Brief { lens } => Ok(CommandOutput::Brief(brief::handle(app, lens.0).await?)),
        Command::Status => Ok(CommandOutput::Status(status::handle(app).await?)),
        Command::Capabilities => Ok(CommandOutput::Capabilities(capabilities::handle())),
        Command::Schema { command } => {
            Ok(CommandOutput::Schema(schema::handle(command.as_deref())))
        }
        Command::Create {
            primitive_type,
            title,
            id,
            body,
            stdin_body,
            fields,
        } => Ok(CommandOutput::Create(
            create::handle(
                app,
                &primitive_type,
                &title,
                id.as_deref(),
                body.as_deref(),
                stdin_body,
                &fields,
            )
            .await?,
        )),
        Command::Query {
            primitive_type,
            filters,
        } => Ok(CommandOutput::Query(
            query::handle(app, &primitive_type, &filters).await?,
        )),
        Command::Thread { command } => {
            Ok(CommandOutput::Thread(thread::handle(app, command).await?))
        }
        Command::Mission { command } => {
            Ok(CommandOutput::Mission(mission::handle(app, command).await?))
        }
        Command::Run { command } => Ok(CommandOutput::Run(run::handle(app, command).await?)),
        Command::Trigger { command } => {
            Ok(CommandOutput::Trigger(trigger::handle(app, command).await?))
        }
        Command::Checkpoint { working_on, focus } => Ok(CommandOutput::Checkpoint(
            checkpoint::handle(app, &working_on, &focus).await?,
        )),
        Command::Show { reference } => {
            Ok(CommandOutput::Show(show::handle(app, &reference).await?))
        }
    }
}
