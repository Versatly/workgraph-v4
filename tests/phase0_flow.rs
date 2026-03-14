use std::collections::BTreeMap;

use tempfile::tempdir;
use wg_cli::{Cli, Commands, run};
use wg_ledger::verify_chain;
use wg_paths::WorkspacePath;
use wg_store::query;
use wg_types::PrimitiveType;

#[test]
fn integration_flow_init_create_query_verify_ledger() {
    let tempdir = tempdir().expect("tempdir should be created");
    let workspace = tempdir.path().to_path_buf();

    run(Cli {
        workspace: workspace.clone(),
        command: Commands::Init,
    })
    .expect("init command should succeed");

    run(Cli {
        workspace: workspace.clone(),
        command: Commands::Create {
            primitive_type: "org".to_owned(),
            title: "Acme Org".to_owned(),
            field: vec!["mission=Build context graph".to_owned()],
        },
    })
    .expect("org create should succeed");

    run(Cli {
        workspace: workspace.clone(),
        command: Commands::Create {
            primitive_type: "client".to_owned(),
            title: "Globex Client".to_owned(),
            field: vec!["status=active".to_owned()],
        },
    })
    .expect("client create should succeed");

    run(Cli {
        workspace: workspace.clone(),
        command: Commands::Query {
            primitive_type: "client".to_owned(),
            filter: vec!["status=active".to_owned()],
        },
    })
    .expect("query command should succeed");

    let workspace = WorkspacePath::new(workspace);
    let results = query(&workspace, PrimitiveType::Client, &BTreeMap::new())
        .expect("store query should succeed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].frontmatter.title, "Globex Client");

    verify_chain(&workspace).expect("ledger verification should succeed");
}
