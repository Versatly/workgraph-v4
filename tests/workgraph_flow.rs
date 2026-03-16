//! End-to-end integration coverage for the Phase 0 WorkGraph flow.

use tempfile::tempdir;
use tokio::fs;
use wg_cli::execute;
use wg_ledger::verify_chain;

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

    verify_chain(temp_dir.path())
        .await
        .expect("ledger chain should remain valid");
}
