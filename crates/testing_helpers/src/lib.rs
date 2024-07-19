use std::{collections::HashMap, path::PathBuf};

use toy_farm_core::{
    relative_path::RelativePath, wax::Glob, Module, ModuleGraph, ModuleGraphEdgeDataItem,
    ResolveKind,
};

pub fn is_update_snapshot_from_env() -> bool {
    std::env::var("FARM_UPDATE_SNAPSHOTS").is_ok()
}

/// construct a test module graph like below:
/// ```plain
///           A   B
///          / \ / \
///         C   D   E
///          \ /    |
///           F     G
/// ```
/// * **dynamic dependencies**: `A -> D`, `C -> F`, `D -> F`, `E -> G`
/// * **cyclic dependencies**: `F -> A`
/// * others are static dependencies
pub fn construct_test_module_graph() -> ModuleGraph {
    let module_ids = vec!["A", "B", "C", "D", "E", "F", "G"]
        .into_iter()
        .map(|i| i.into());
    let mut graph = ModuleGraph::new();

    for id in module_ids {
        let m = Module::new(id);

        graph.add_module(m);
    }

    let static_edges = vec![("A", "C", 0), ("B", "D", 0), ("B", "E", 1)];
    let dynamic_edges = vec![("A", "D", 1), ("C", "F", 0), ("D", "F", 0), ("E", "G", 0)];

    for (from, to, order) in static_edges {
        graph
            .add_edge_item(
                &from.into(),
                &to.into(),
                ModuleGraphEdgeDataItem {
                    source: format!("./{}", to),
                    kind: ResolveKind::Import,
                    order,
                },
            )
            .unwrap();
    }

    for (from, to, order) in dynamic_edges {
        graph
            .add_edge_item(
                &from.into(),
                &to.into(),
                ModuleGraphEdgeDataItem {
                    source: format!("./{}", to),
                    kind: ResolveKind::DynamicImport,
                    order,
                },
            )
            .unwrap();
    }

    graph
        .add_edge_item(
            &"F".into(),
            &"A".into(),
            ModuleGraphEdgeDataItem {
                source: "./A".to_string(),
                kind: ResolveKind::Import,
                order: 0,
            },
        )
        .unwrap();

    graph.entries = HashMap::from([("A".into(), "A".to_string()), ("B".into(), "B".to_string())]);

    graph
}

/// @deprecated using macro fixture instead
pub async fn fixture<F, Fut>(pattern: &str, mut op: F)
where
    F: FnMut(PathBuf, PathBuf) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let base_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let glob = Glob::new(pattern).unwrap();

    for path in glob.walk(base_dir.clone()).flatten() {
        op(path.path().to_path_buf(), base_dir.clone()).await;
    }
}

#[macro_export]
macro_rules! fixture {
    ($pattern:expr, $op:expr) => {
        if cfg!(debug_assertions) {
            toy_farm_testing_helpers::fixture_debug($pattern, file!(), $op).await;
            return;
        }

        toy_farm_testing_helpers::fixture($pattern, $op).await;
    };
}

/// @deprecated using macro fixture instead
pub async fn fixture_debug<F, Fut>(pattern: &str, test_file_path: &str, mut op: F)
where
    F: FnMut(PathBuf, PathBuf) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    // find closest Cargo.toml
    let mut file_path =
        RelativePath::new(test_file_path).to_logical_path(std::env::current_dir().unwrap());
    while let Some(parent) = file_path.parent() {
        if parent.join("Cargo.toml").exists() {
            break;
        }

        file_path = parent.to_path_buf();
    }

    if file_path.parent().is_none() {
        panic!("can't find Cargo.toml");
    }

    let base_dir = file_path.parent().unwrap().to_path_buf();
    let glob = Glob::new(pattern).unwrap();

    let mut exists = false;

    for path in glob.walk(base_dir.clone()).flatten() {
        exists = true;
        op(path.path().to_path_buf(), base_dir.clone()).await;
    }

    if !exists {
        panic!("no fixtures found under {}", pattern);
    }
}
