//! End-to-end integration coverage for the Phase 0 WorkGraph flow.

use std::collections::BTreeMap;

use serde_yaml::Value;
use tempfile::tempdir;
use tokio::fs;
use wg_cli::execute;
use wg_graph::{NeighborDirection, NodeRef, build_graph};
use wg_ledger::verify_chain;
use wg_mission::{add_thread_to_mission, create_mission, mission_progress};
use wg_orientation::{brief, checkpoint, status};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, PolicyEngine};
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, write_primitive};
use wg_thread::{add_message, claim_thread, complete_thread, create_thread, open_thread};
use wg_types::{ActorId, Registry};

#[tokio::test]
async fn init_create_query_and_verify_ledger_chain() {
    let temp_dir = tempdir().expect("temporary directory should be created");

    let init_output = execute(["workgraph", "--json", "init"], temp_dir.path())
        .await
        .expect("workspace initialization should succeed");
    let init_json: serde_json::Value =
        serde_json::from_str(&init_output).expect("init output should be valid JSON");
    assert_eq!(init_json["schema_version"], "workgraph.cli.v1alpha2");
    assert_eq!(init_json["success"], true);
    assert_eq!(init_json["command"], "init");
    assert_eq!(
        init_json["result"]["config"]["config_file"],
        temp_dir
            .path()
            .join(".workgraph")
            .join("config.yaml")
            .display()
            .to_string()
    );
    assert!(
        fs::try_exists(temp_dir.path().join(".workgraph").join("config.yaml"))
            .await
            .expect("config file existence check should succeed")
    );

    execute(
        [
            "workgraph",
            "--json",
            "create",
            "org",
            "--title",
            "Versatly",
            "--field",
            "summary=AI-native company",
        ],
        temp_dir.path(),
    )
    .await
    .expect("org creation should succeed");

    execute(
        [
            "workgraph",
            "--json",
            "create",
            "client",
            "--title",
            "Hale Pet Door",
            "--field",
            "account_owner=pedro",
        ],
        temp_dir.path(),
    )
    .await
    .expect("client creation should succeed");

    execute(
        [
            "workgraph",
            "--json",
            "create",
            "decision",
            "--title",
            "Rust for WorkGraph v4",
            "--field",
            "status=decided",
        ],
        temp_dir.path(),
    )
    .await
    .expect("decision creation should succeed");

    let query_output = execute(["workgraph", "--json", "query", "client"], temp_dir.path())
        .await
        .expect("client query should succeed");
    let query_json: serde_json::Value =
        serde_json::from_str(&query_output).expect("query output should be valid JSON");
    assert_eq!(query_json["success"], true);
    assert_eq!(query_json["command"], "query");
    assert_eq!(query_json["result"]["count"], 1);
    assert_eq!(
        query_json["result"]["items"][0]["frontmatter"]["id"],
        "hale-pet-door"
    );

    let status_output = execute(["workgraph", "--json", "status"], temp_dir.path())
        .await
        .expect("status should succeed");
    let status_json: serde_json::Value =
        serde_json::from_str(&status_output).expect("status output should be valid JSON");
    assert_eq!(status_json["success"], true);
    assert_eq!(status_json["command"], "status");
    assert_eq!(status_json["result"]["type_counts"]["org"], 1);
    assert_eq!(status_json["result"]["type_counts"]["client"], 1);
    assert_eq!(status_json["result"]["type_counts"]["decision"], 1);
    assert_eq!(
        status_json["result"]["last_entry"]["primitive_type"],
        "decision"
    );

    let brief_output = execute(
        ["workgraph", "--json", "brief", "--lens", "workspace"],
        temp_dir.path(),
    )
    .await
    .expect("brief should succeed");
    let brief_json: serde_json::Value =
        serde_json::from_str(&brief_output).expect("brief output should be valid JSON");
    assert_eq!(brief_json["success"], true);
    assert_eq!(brief_json["command"], "brief");
    assert_eq!(brief_json["result"]["lens"], "workspace");
    assert_eq!(
        brief_json["result"]["sections"][0]["items"][0]["title"],
        "Versatly"
    );

    let capabilities_output = execute(["workgraph", "--json", "capabilities"], temp_dir.path())
        .await
        .expect("capabilities should succeed");
    let capabilities_json: serde_json::Value = serde_json::from_str(&capabilities_output)
        .expect("capabilities output should be valid JSON");
    assert_eq!(capabilities_json["command"], "capabilities");
    assert!(capabilities_json["result"]["commands"].is_array());
    assert!(capabilities_json["result"]["primitive_contracts"].is_array());
    assert_eq!(
        capabilities_json["result"]["primitive_contracts"]
            .as_array()
            .expect("primitive contracts should be an array")
            .len(),
        Registry::builtins().list_types().len()
    );

    let schema_output = execute(["workgraph", "--json", "schema", "create"], temp_dir.path())
        .await
        .expect("schema should succeed");
    let schema_json: serde_json::Value =
        serde_json::from_str(&schema_output).expect("schema output should be valid JSON");
    assert_eq!(schema_json["command"], "schema");
    assert_eq!(schema_json["result"]["commands"][0]["name"], "create");
    assert!(schema_json["result"]["primitive_contracts"].is_array());
    assert_eq!(
        schema_json["result"]["primitive_contracts"]
            .as_array()
            .expect("primitive contracts should be an array")
            .len(),
        Registry::builtins().list_types().len()
    );

    let dry_run_output = execute(
        [
            "workgraph",
            "--json",
            "--dry-run",
            "create",
            "project",
            "--title",
            "Dealer Portal",
            "--field",
            "status=active",
        ],
        temp_dir.path(),
    )
    .await
    .expect("dry-run create should succeed");
    let dry_run_json: serde_json::Value =
        serde_json::from_str(&dry_run_output).expect("dry-run output should be valid JSON");
    assert_eq!(dry_run_json["command"], "create");
    assert_eq!(dry_run_json["result"]["dry_run"], true);
    assert_eq!(dry_run_json["result"]["reference"], "project/dealer-portal");
    assert_eq!(dry_run_json["result"]["ledger_entry"], serde_json::Value::Null);
    assert!(
        !fs::try_exists(temp_dir.path().join("projects").join("dealer-portal.md"))
            .await
            .expect("dry-run project existence check should succeed")
    );

    let idempotent_create_output = execute(
        [
            "workgraph",
            "--json",
            "create",
            "org",
            "--title",
            "Versatly",
            "--field",
            "summary=AI-native company",
        ],
        temp_dir.path(),
    )
    .await
    .expect("idempotent org creation should succeed");
    let idempotent_create_json: serde_json::Value = serde_json::from_str(&idempotent_create_output)
        .expect("idempotent create output should be valid JSON");
    assert_eq!(idempotent_create_json["result"]["idempotent"], true);
    assert_eq!(
        idempotent_create_json["result"]["ledger_entry"],
        serde_json::Value::Null
    );

    let workspace = WorkspacePath::new(temp_dir.path());

    create_thread(
        &workspace,
        "kernel-thread-1",
        "Kernel implementation thread",
        Some("phase-1-kernel"),
    )
    .await
    .expect("thread should be created");
    open_thread(&workspace, "kernel-thread-1")
        .await
        .expect("thread should open");
    claim_thread(&workspace, "kernel-thread-1", ActorId::new("pedro"))
        .await
        .expect("thread should be claimed");
    add_message(
        &workspace,
        "kernel-thread-1",
        ActorId::new("agent:cursor"),
        "Linking implementation to [[decision/rust-for-workgraph-v4]].",
    )
    .await
    .expect("thread message should be added");
    complete_thread(&workspace, "kernel-thread-1")
        .await
        .expect("thread should complete");

    create_mission(
        &workspace,
        "phase-1-kernel",
        "Phase 1 kernel implementation",
        "Implement remaining kernel crates and tests.",
    )
    .await
    .expect("mission should be created");
    add_thread_to_mission(&workspace, "phase-1-kernel", "kernel-thread-1")
        .await
        .expect("thread should be attached to mission");
    let progress = mission_progress(&workspace, "phase-1-kernel")
        .await
        .expect("mission progress should compute");
    assert_eq!(progress.completed_threads, 1);
    assert_eq!(progress.total_threads, 1);

    let policy_primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: "policy".to_owned(),
            id: "decision-guard".to_owned(),
            title: "Decision create guard".to_owned(),
            extra_fields: BTreeMap::from([
                (
                    "scope".to_owned(),
                    Value::Sequence(vec![Value::String("decision".to_owned())]),
                ),
                (
                    "rules".to_owned(),
                    Value::Sequence(vec![
                        Value::Mapping(
                            [
                                (
                                    Value::String("effect".to_owned()),
                                    Value::String("allow".to_owned()),
                                ),
                                (
                                    Value::String("actions".to_owned()),
                                    Value::Sequence(vec![Value::String("create".to_owned())]),
                                ),
                                (
                                    Value::String("actors".to_owned()),
                                    Value::Sequence(vec![Value::String("pedro".to_owned())]),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        Value::Mapping(
                            [
                                (
                                    Value::String("effect".to_owned()),
                                    Value::String("deny".to_owned()),
                                ),
                                (
                                    Value::String("actions".to_owned()),
                                    Value::Sequence(vec![Value::String("create".to_owned())]),
                                ),
                                (
                                    Value::String("actors".to_owned()),
                                    Value::Sequence(vec![Value::String("intern".to_owned())]),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                    ]),
                ),
            ]),
        },
        body: "Policy body".to_owned(),
    };
    write_primitive(&workspace, &Registry::builtins(), &policy_primitive)
        .await
        .expect("policy should be written");
    let policy_engine = PolicyEngine::load(&workspace)
        .await
        .expect("policy engine should load");
    assert_eq!(
        policy_engine.evaluate(
            &ActorId::new("pedro"),
            PolicyAction::Create,
            "decision",
            &PolicyContext::default(),
        ),
        PolicyDecision::Allow
    );
    assert_eq!(
        policy_engine.evaluate(
            &ActorId::new("intern"),
            PolicyAction::Create,
            "decision",
            &PolicyContext::default(),
        ),
        PolicyDecision::Deny
    );

    let graph = build_graph(&workspace)
        .await
        .expect("graph should build from workspace primitives");
    let thread_node = NodeRef::new("thread", "kernel-thread-1");
    assert!(
        graph
            .neighbors(&thread_node, NeighborDirection::Outbound)
            .contains(&NodeRef::new("decision", "rust-for-workgraph-v4"))
    );

    let workspace_status = status(&workspace)
        .await
        .expect("orientation status should load");
    assert_eq!(workspace_status.type_counts.get("thread"), Some(&1));
    assert_eq!(workspace_status.type_counts.get("mission"), Some(&1));

    let actor_brief = brief(&workspace, &ActorId::new("pedro"))
        .await
        .expect("orientation brief should load");
    assert_eq!(actor_brief.assigned_threads.len(), 1);
    assert_eq!(actor_brief.assigned_missions.len(), 1);
    assert!(
        actor_brief
            .warnings
            .iter()
            .any(|warning| warning.contains("Graph issue"))
    );

    let checkpoint_primitive = checkpoint(
        &workspace,
        "Kernel crate implementation",
        "Finalize tests and quality checks",
    )
    .await
    .expect("checkpoint primitive should be saved");
    assert_eq!(checkpoint_primitive.frontmatter.r#type, "checkpoint");
    let actor_thread = wg_thread::load_thread(&workspace, "kernel-thread-1")
        .await
        .expect("thread should still be readable");
    assert_eq!(actor_thread.messages.len(), 1);

    verify_chain(temp_dir.path())
        .await
        .expect("ledger chain should remain valid");
}

#[tokio::test]
async fn coordination_cli_commands_cover_thread_mission_run_trigger_and_checkpoint_workflows() {
    let temp_dir = tempdir().expect("temporary directory should be created");

    execute(["workgraph", "--json", "init"], temp_dir.path())
        .await
        .expect("workspace initialization should succeed");

    let thread_create = execute(
        [
            "workgraph",
            "--json",
            "thread",
            "create",
            "--id",
            "launch-thread",
            "--title",
            "Launch readiness",
        ],
        temp_dir.path(),
    )
    .await
    .expect("thread create should succeed");
    let thread_create_json: serde_json::Value =
        serde_json::from_str(&thread_create).expect("thread create output should be valid JSON");
    assert_eq!(thread_create_json["command"], "thread");
    assert_eq!(thread_create_json["result"]["action"], "create");
    assert_eq!(thread_create_json["result"]["reference"], "thread/launch-thread");

    execute(
        ["workgraph", "--json", "thread", "open", "launch-thread"],
        temp_dir.path(),
    )
    .await
    .expect("thread open should succeed");
    execute(
        [
            "workgraph",
            "--json",
            "thread",
            "claim",
            "launch-thread",
            "--actor",
            "pedro",
        ],
        temp_dir.path(),
    )
    .await
    .expect("thread claim should succeed");
    execute(
        [
            "workgraph",
            "--json",
            "thread",
            "add-exit-criterion",
            "launch-thread",
            "--id",
            "criterion-1",
            "--title",
            "Verification complete",
        ],
        temp_dir.path(),
    )
    .await
    .expect("thread exit criterion should succeed");
    execute(
        [
            "workgraph",
            "--json",
            "thread",
            "add-evidence",
            "launch-thread",
            "--id",
            "evidence-1",
            "--title",
            "Verifier report",
            "--satisfies",
            "criterion-1",
            "--source",
            "manual",
        ],
        temp_dir.path(),
    )
    .await
    .expect("thread evidence should succeed");
    let thread_complete = execute(
        ["workgraph", "--json", "thread", "complete", "launch-thread"],
        temp_dir.path(),
    )
    .await
    .expect("thread complete should succeed");
    let thread_complete_json: serde_json::Value = serde_json::from_str(&thread_complete)
        .expect("thread complete output should be valid JSON");
    assert_eq!(
        thread_complete_json["result"]["thread"]["frontmatter"]["status"],
        "done"
    );

    let mission_create = execute(
        [
            "workgraph",
            "--json",
            "mission",
            "create",
            "--id",
            "launch-mission",
            "--title",
            "Launch mission",
            "--objective",
            "Ship safely.",
        ],
        temp_dir.path(),
    )
    .await
    .expect("mission create should succeed");
    let mission_create_json: serde_json::Value = serde_json::from_str(&mission_create)
        .expect("mission create output should be valid JSON");
    assert_eq!(mission_create_json["command"], "mission");
    assert_eq!(mission_create_json["result"]["action"], "create");
    execute(
        [
            "workgraph",
            "--json",
            "mission",
            "add-thread",
            "launch-mission",
            "launch-thread",
        ],
        temp_dir.path(),
    )
    .await
    .expect("mission add-thread should succeed");
    let mission_progress_output = execute(
        [
            "workgraph",
            "--json",
            "mission",
            "progress",
            "launch-mission",
        ],
        temp_dir.path(),
    )
    .await
    .expect("mission progress should succeed");
    let mission_progress_json: serde_json::Value = serde_json::from_str(&mission_progress_output)
        .expect("mission progress output should be valid JSON");
    assert_eq!(
        mission_progress_json["result"]["progress"]["completed_threads"],
        1
    );
    assert_eq!(mission_progress_json["result"]["progress"]["total_threads"], 1);

    let run_create = execute(
        [
            "workgraph",
            "--json",
            "run",
            "create",
            "--id",
            "run-1",
            "--title",
            "Cursor analysis",
            "--actor",
            "agent:cursor",
            "--thread",
            "launch-thread",
            "--mission",
            "launch-mission",
        ],
        temp_dir.path(),
    )
    .await
    .expect("run create should succeed");
    let run_create_json: serde_json::Value =
        serde_json::from_str(&run_create).expect("run create output should be valid JSON");
    assert_eq!(run_create_json["command"], "run");
    assert_eq!(run_create_json["result"]["action"], "create");
    execute(
        ["workgraph", "--json", "run", "start", "run-1"],
        temp_dir.path(),
    )
    .await
    .expect("run start should succeed");
    let run_complete = execute(
        [
            "workgraph",
            "--json",
            "run",
            "complete",
            "run-1",
            "--summary",
            "Completed successfully",
        ],
        temp_dir.path(),
    )
    .await
    .expect("run complete should succeed");
    let run_complete_json: serde_json::Value =
        serde_json::from_str(&run_complete).expect("run complete output should be valid JSON");
    assert_eq!(
        run_complete_json["result"]["run"]["frontmatter"]["status"],
        "succeeded"
    );

    let trigger_save = execute(
        [
            "workgraph",
            "--json",
            "trigger",
            "save",
            "--id",
            "trigger-1",
            "--title",
            "React to completed threads",
            "--status",
            "active",
            "--event-source",
            "ledger",
            "--op",
            "done",
            "--primitive-type",
            "thread",
            "--field-name",
            "evidence",
            "--action-kind",
            "rebrief_actor",
            "--action-target",
            "agent/cursor",
            "--action-instruction",
            "Refresh the brief",
        ],
        temp_dir.path(),
    )
    .await
    .expect("trigger save should succeed");
    let trigger_save_json: serde_json::Value =
        serde_json::from_str(&trigger_save).expect("trigger save output should be valid JSON");
    assert_eq!(trigger_save_json["command"], "trigger");
    assert_eq!(trigger_save_json["result"]["action"], "save");

    let trigger_eval = execute(
        [
            "workgraph",
            "--json",
            "trigger",
            "evaluate",
            "--entry-index",
            "5",
        ],
        temp_dir.path(),
    )
    .await
    .expect("trigger evaluation should succeed");
    let trigger_eval_json: serde_json::Value = serde_json::from_str(&trigger_eval)
        .expect("trigger evaluation output should be valid JSON");
    assert_eq!(trigger_eval_json["result"]["action"], "evaluate");
    assert_eq!(trigger_eval_json["result"]["matches"][0]["trigger_id"], "trigger-1");

    let checkpoint_output = execute(
        [
            "workgraph",
            "--json",
            "checkpoint",
            "--working-on",
            "Kernel implementation",
            "--focus",
            "Finalize trigger CLI",
        ],
        temp_dir.path(),
    )
    .await
    .expect("checkpoint command should succeed");
    let checkpoint_json: serde_json::Value = serde_json::from_str(&checkpoint_output)
        .expect("checkpoint output should be valid JSON");
    assert_eq!(checkpoint_json["command"], "checkpoint");
    assert_eq!(
        checkpoint_json["result"]["checkpoint"]["frontmatter"]["type"],
        "checkpoint"
    );

    verify_chain(temp_dir.path())
        .await
        .expect("ledger chain should remain valid after CLI workflows");
}
