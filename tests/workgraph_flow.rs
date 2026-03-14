//! End-to-end integration coverage for the Phase 0 WorkGraph flow.

use tempfile::tempdir;
use wg_cli::execute;
use wg_ledger::verify_chain;

#[tokio::test]
async fn init_create_query_and_verify_ledger_chain() {
    let temp_dir = tempdir().expect("temporary directory should be created");

    execute(["workgraph", "init"], temp_dir.path())
        .await
        .expect("workspace initialization should succeed");

    execute(
        [
            "workgraph",
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
    assert_eq!(status_json["command"], "status");
    assert_eq!(status_json["result"]["type_counts"]["org"], 1);
    assert_eq!(status_json["result"]["type_counts"]["client"], 1);
    assert_eq!(status_json["result"]["type_counts"]["decision"], 1);
    assert_eq!(
        status_json["result"]["last_entry"]["primitive_type"],
        "decision"
    );

    verify_chain(temp_dir.path())
        .await
        .expect("ledger chain should remain valid");
}
