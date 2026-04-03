//! Implementation of the `workgraph mission` command family.

use anyhow::Context;
use wg_mission::{
    activate_mission, add_run_to_mission, add_thread_to_mission, block_mission, complete_mission,
    create_mission, load_mission, mission_progress,
};

use crate::app::AppContext;
use crate::args::MissionCommand;
use crate::output::{MissionOutput, MissionProgressOutput};
use crate::services::codec::mission_to_stored;

/// Executes a mission workflow command.
///
/// # Errors
///
/// Returns an error when mission persistence, lifecycle validation, or output assembly fails.
pub async fn handle(app: &AppContext, command: MissionCommand) -> anyhow::Result<MissionOutput> {
    match command {
        MissionCommand::Create {
            id,
            title,
            objective,
        } => {
            let reference = format!("mission/{id}");
            if app.dry_run() {
                return Ok(MissionOutput {
                    action: "create".to_owned(),
                    dry_run: true,
                    reference,
                    mission: None,
                    progress: None,
                });
            }

            let mission = create_mission(app.workspace(), &id, &title, &objective)
                .await
                .with_context(|| format!("failed to create mission '{id}'"))?;
            Ok(MissionOutput {
                action: "create".to_owned(),
                dry_run: false,
                reference: format!("mission/{}", mission.id),
                mission: Some(mission_to_stored(&mission)?),
                progress: None,
            })
        }
        MissionCommand::Activate { mission_id } => {
            if app.dry_run() {
                return Ok(MissionOutput {
                    action: "activate".to_owned(),
                    dry_run: true,
                    reference: format!("mission/{mission_id}"),
                    mission: None,
                    progress: None,
                });
            }

            let mission = activate_mission(app.workspace(), &mission_id)
                .await
                .with_context(|| format!("failed to activate mission '{mission_id}'"))?;
            Ok(MissionOutput {
                action: "activate".to_owned(),
                dry_run: false,
                reference: format!("mission/{}", mission.id),
                mission: Some(mission_to_stored(&mission)?),
                progress: None,
            })
        }
        MissionCommand::Block { mission_id } => {
            if app.dry_run() {
                return Ok(MissionOutput {
                    action: "block".to_owned(),
                    dry_run: true,
                    reference: format!("mission/{mission_id}"),
                    mission: None,
                    progress: None,
                });
            }

            let mission = block_mission(app.workspace(), &mission_id)
                .await
                .with_context(|| format!("failed to block mission '{mission_id}'"))?;
            Ok(MissionOutput {
                action: "block".to_owned(),
                dry_run: false,
                reference: format!("mission/{}", mission.id),
                mission: Some(mission_to_stored(&mission)?),
                progress: None,
            })
        }
        MissionCommand::Complete { mission_id } => {
            if app.dry_run() {
                return Ok(MissionOutput {
                    action: "complete".to_owned(),
                    dry_run: true,
                    reference: format!("mission/{mission_id}"),
                    mission: None,
                    progress: None,
                });
            }

            let mission = complete_mission(app.workspace(), &mission_id)
                .await
                .with_context(|| format!("failed to complete mission '{mission_id}'"))?;
            Ok(MissionOutput {
                action: "complete".to_owned(),
                dry_run: false,
                reference: format!("mission/{}", mission.id),
                mission: Some(mission_to_stored(&mission)?),
                progress: None,
            })
        }
        MissionCommand::AddThread {
            mission_id,
            thread_id,
        } => {
            if app.dry_run() {
                return Ok(MissionOutput {
                    action: "add_thread".to_owned(),
                    dry_run: true,
                    reference: format!("mission/{mission_id}"),
                    mission: None,
                    progress: None,
                });
            }

            let mission = add_thread_to_mission(app.workspace(), &mission_id, &thread_id)
                .await
                .with_context(|| {
                    format!("failed to attach thread '{thread_id}' to mission '{mission_id}'")
                })?;
            Ok(MissionOutput {
                action: "add_thread".to_owned(),
                dry_run: false,
                reference: format!("mission/{}", mission.id),
                mission: Some(mission_to_stored(&mission)?),
                progress: None,
            })
        }
        MissionCommand::AddRun { mission_id, run_id } => {
            if app.dry_run() {
                return Ok(MissionOutput {
                    action: "add_run".to_owned(),
                    dry_run: true,
                    reference: format!("mission/{mission_id}"),
                    mission: None,
                    progress: None,
                });
            }

            let mission = add_run_to_mission(app.workspace(), &mission_id, &run_id)
                .await
                .with_context(|| {
                    format!("failed to attach run '{run_id}' to mission '{mission_id}'")
                })?;
            Ok(MissionOutput {
                action: "add_run".to_owned(),
                dry_run: false,
                reference: format!("mission/{}", mission.id),
                mission: Some(mission_to_stored(&mission)?),
                progress: None,
            })
        }
        MissionCommand::Progress { mission_id } => {
            let mission = load_mission(app.workspace(), &mission_id)
                .await
                .with_context(|| format!("failed to load mission '{mission_id}'"))?;
            let progress = mission_progress(app.workspace(), &mission_id)
                .await
                .with_context(|| {
                    format!("failed to compute progress for mission '{mission_id}'")
                })?;
            Ok(MissionOutput {
                action: "progress".to_owned(),
                dry_run: app.dry_run(),
                reference: format!("mission/{}", mission.id),
                mission: Some(mission_to_stored(&mission)?),
                progress: Some(MissionProgressOutput {
                    completed_threads: progress.completed_threads,
                    total_threads: progress.total_threads,
                }),
            })
        }
    }
}
