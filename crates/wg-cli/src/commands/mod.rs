//! Individual command handlers and dispatch logic for the WorkGraph CLI.

mod brief;
mod create;
mod init;
mod query;
mod schema;
mod show;
mod skills;
mod status;

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
        Command::Skills => Ok(CommandOutput::Skills(skills::handle())),
        Command::Schema { command } => {
            Ok(CommandOutput::Schema(schema::handle(command.as_deref())))
        }
        Command::Create {
            primitive_type,
            title,
            fields,
        } => Ok(CommandOutput::Create(
            create::handle(app, &primitive_type, &title, &fields).await?,
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
    }
}
