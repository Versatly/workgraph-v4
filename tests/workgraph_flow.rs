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
    assert_eq!(init_json["schema_version"], "workgraph.cli.v1alpha1");
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

    let schema_output = execute(["workgraph", "--json", "schema", "create"], temp_dir.path())
        .await
        .expect("schema should succeed");
    let schema_json: serde_json::Value =
        serde_json::from_str(&schema_output).expect("schema output should be valid JSON");
    assert_eq!(schema_json["command"], "schema");
    assert_eq!(schema_json["result"]["commands"][0]["name"], "create");

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

    let checkpoint_primitive = checkpoint(
        &workspace,
        "Kernel crate implementation",
        "Finalize tests and quality checks",
    )
    .await
    .expect("checkpoint primitive should be saved");
    assert_eq!(checkpoint_primitive.frontmatter.r#type, "checkpoint");

    verify_chain(temp_dir.path())
        .await
        .expect("ledger chain should remain valid");
}
