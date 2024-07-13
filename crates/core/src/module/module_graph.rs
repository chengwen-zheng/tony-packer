use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    vec,
};

use petgraph::{csr::DefaultIx, graph::NodeIndex, stable_graph::StableDiGraph, EdgeDirection};
use toy_farm_macro_cache_item::cache_item;

use crate::plugin::ResolveKind;

use super::{Module, ModuleId};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cache_item]
pub struct ModuleGraphEdgeDataItem {
    /// the source of this edge, for example, `./index.css`
    pub source: String,
    pub kind: ResolveKind,
    // the order of this edge, for example, for:
    /// ```js
    /// import a from './a';
    /// import b from './b';
    /// ```
    /// the edge `./a`'s order is 0 and `./b`'s order is 1 (starting from 0).
    pub order: usize,
}
#[cache_item]
#[derive(PartialEq, Debug, Clone)]
pub struct ModuleGraphEdge(pub(crate) Vec<ModuleGraphEdgeDataItem>);

impl Default for ModuleGraphEdge {
    fn default() -> Self {
        Self::new()
    }
}
impl ModuleGraphEdge {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn items(&self) -> &[ModuleGraphEdgeDataItem] {
        &self.0
    }

    pub fn update_kind(&mut self, kind: ResolveKind) {
        for item in self.0.iter_mut() {
            item.kind = kind.clone();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ModuleGraphEdgeDataItem> {
        self.0.iter()
    }

    pub fn contains(&self, item: &ModuleGraphEdgeDataItem) -> bool {
        self.0.contains(item)
    }

    // true if all of the edge data items are dynamic
    pub fn is_dynamic(&self) -> bool {
        self.0
            .iter()
            .all(|item| item.kind == ResolveKind::DynamicImport)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub struct ModuleGraph {
    /// internal graph
    g: StableDiGraph<Module, ModuleGraphEdge>,

    /// to index module in the graph using [ModuleId]
    id_index_map: HashMap<ModuleId, NodeIndex<DefaultIx>>,
    /// file path to module ids, e.g src/index.scss -> [src/index.scss, src/index.scss?raw]
    file_module_ids_map: HashMap<ModuleId, Vec<ModuleId>>,
    /// entry modules of this module graph.
    /// (Entry Module Id, Entry Name)
    pub entries: HashMap<ModuleId, String>,
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self {
            g: StableDiGraph::new(),
            id_index_map: HashMap::new(),
            file_module_ids_map: HashMap::new(),
            entries: HashMap::new(),
        }
    }

    pub fn add_edge_item(
        &mut self,
        from: &ModuleId,
        to: &ModuleId,
        item: ModuleGraphEdgeDataItem,
    ) -> anyhow::Result<()> {
        let from_index = self
            .id_index_map
            .get(from)
            .unwrap_or_else(|| panic!("module {:?} not found in the module graph", from));
        let to_index = self
            .id_index_map
            .get(to)
            .unwrap_or_else(|| panic!("module {:?} not found in the module graph", to));

        // if the edge already exists, we should update the edge info
        if let Some(edge_index) = self.g.find_edge(*from_index, *to_index) {
            if !self.g[edge_index].contains(&item) {
                self.g[edge_index].0.push(item);
            }
            return Ok(());
        }

        // using update_edge instead of add_edge to avoid duplicated edges, see https://docs.rs/petgraph/latest/petgraph/graph/struct.Graph.html#method.update_edge
        self.g
            .update_edge(*from_index, *to_index, ModuleGraphEdge(vec![item]));

        Ok(())
    }

    /// Get the dep module of the specified module which imports the dep module using the specified source.
    /// Used to get module by (module, source) pair, for example, for `module a`:
    /// ```js
    /// import b from './b';
    /// ```
    /// we can get `module b` by `(module a, "./b")`.
    ///
    /// Panic if the dep does not exist or the source is not correct
    pub fn get_dep_by_source_optional(
        &self,
        module_id: &ModuleId,
        source: &str,
        kind: Option<ResolveKind>,
    ) -> Option<ModuleId> {
        let i = self
            .id_index_map
            .get(module_id)
            .unwrap_or_else(|| panic!("module {:?} not found in the module graph", module_id));

        let mut edges = self
            .g
            .neighbors_directed(*i, EdgeDirection::Outgoing)
            .detach();

        while let Some((edge_index, node_index)) = edges.next(&self.g) {
            if self.g[edge_index].iter().any(|e| {
                e.source == *source && (kind.is_none() || e.kind == *kind.as_ref().unwrap())
            }) {
                return Some(self.g[node_index].id.clone());
            }
        }

        None
    }

    /// get dependencies of the specific module, sorted by the order of the edge.
    /// for example, for `module a`:
    /// ```js
    /// import c from './c';
    /// import b from './b';
    /// ```
    /// return `['module c', 'module b']`, ensure the order of original imports.
    pub fn dependencies(&self, module_id: &ModuleId) -> Vec<(ModuleId, &ModuleGraphEdge)> {
        let i = self
            .id_index_map
            .get(module_id)
            .unwrap_or_else(|| panic!("module_id {:?} should in the module graph", module_id));
        let mut edges = self
            .g
            .neighbors_directed(*i, EdgeDirection::Outgoing)
            .detach();

        let mut deps = vec![];

        while let Some((edge_index, node_index)) = edges.next(&self.g) {
            deps.push((self.g[node_index].id.clone(), &self.g[edge_index]));
        }

        deps.sort_by(|a, b| {
            if a.1.is_empty() || b.1.is_empty() {
                return Ordering::Equal;
            }

            let a_minimum_order = a.1.iter().map(|item| item.order).min().unwrap();
            let b_minimum_order = b.1.iter().map(|item| item.order).min().unwrap();

            a_minimum_order.cmp(&b_minimum_order)
        });

        deps
    }

    /// sort the module graph topologically using post order dfs, note this topo sort also keeps the original import order.
    /// return (topologically sorted modules, cyclic modules stack)
    ///
    /// **Unsupported Situation**: if the two entries shares the same dependencies but the import order is not the same, may cause one entry don't keep original import order, this may bring problems in css as css depends on the order.
    pub fn topo_sort(&self) -> (Vec<ModuleId>, Vec<Vec<ModuleId>>) {
        fn dfs(
            entry: &ModuleId,
            graph: &ModuleGraph,
            stack: &mut Vec<ModuleId>,
            visited: &mut HashSet<ModuleId>,
            result: &mut Vec<ModuleId>,
            cyclic: &mut Vec<Vec<ModuleId>>,
        ) {
            // cycle detected
            if let Some(pos) = stack.iter().position(|m| m == entry) {
                cyclic.push(stack.clone()[pos..].to_vec());
                return;
            } else if visited.contains(entry) {
                // skip visited module
                return;
            }

            visited.insert(entry.clone());
            stack.push(entry.clone());

            let deps = graph.dependencies(entry);

            for (dep, _) in &deps {
                dfs(dep, graph, stack, visited, result, cyclic)
            }

            // visit current entry
            result.push(stack.pop().unwrap());
        }

        let mut visited = HashSet::new();
        let mut result = vec![];
        let mut cyclic = vec![];
        let mut stack = vec![];

        // sort entries to make sure it is stable
        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort();

        for (entry, _) in entries {
            let res = vec![];
            dfs(
                entry,
                self,
                &mut stack,
                &mut visited,
                &mut result,
                &mut cyclic,
            );
            result.extend(res);
        }

        result.reverse();

        (result, cyclic)
    }

    pub fn add_module(&mut self, module: Module) {
        let id = module.id.clone();
        let index = self.g.add_node(module);

        if !id.query_string().is_empty() {
            let rel_path = id.relative_path();
            self.file_module_ids_map
                .entry(rel_path.into())
                .or_default()
                .push(id.clone())
        }

        self.id_index_map.insert(id, index);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        module::{Module, ModuleId},
        plugin::ResolveKind,
    };

    use super::{ModuleGraph, ModuleGraphEdge, ModuleGraphEdgeDataItem};

    /// construct a test module graph like below:
    /// ```plain
    ///           A   B
    ///          / \ / \
    ///         C   D   E
    ///          \ /    |
    ///           F     G
    /// ```
    /// * **dynamic dependencies**: `A -> D`, `C -> G`, `D -> G`, `E -> H`
    /// * others are static dependencies
    /// * cyclic dependencies from `F -> A`
    fn construct_test_module_graph() -> ModuleGraph {
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
                    source: "./F".to_string(),
                    kind: ResolveKind::Import,
                    order: 0,
                },
            )
            .unwrap();

        graph.entries =
            HashMap::from([("A".into(), "A".to_string()), ("B".into(), "B".to_string())]);

        graph
    }

    #[test]
    fn toposort() {
        let graph = construct_test_module_graph();
        let (sorted, cycle) = graph.topo_sort();

        assert_eq!(cycle, vec![vec!["A".into(), "C".into(), "F".into()],]);
        assert_eq!(
            sorted,
            vec!["B", "E", "G", "A", "D", "C", "F"]
                .into_iter()
                .map(|m| m.into())
                .collect::<Vec<ModuleId>>()
        );
    }

    #[test]
    fn dependencies() {
        let graph = construct_test_module_graph();

        let deps = graph.dependencies(&"A".into());
        assert_eq!(
            deps,
            vec![
                (
                    "C".into(),
                    &ModuleGraphEdge(vec![ModuleGraphEdgeDataItem {
                        source: "./C".to_string(),
                        kind: ResolveKind::Import,
                        order: 0
                    }])
                ),
                (
                    "D".into(),
                    &ModuleGraphEdge(vec![ModuleGraphEdgeDataItem {
                        source: "./D".to_string(),
                        kind: ResolveKind::DynamicImport,
                        order: 1
                    }])
                ),
            ]
        );
    }
}
