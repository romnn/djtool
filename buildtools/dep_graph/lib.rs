use anyhow::Result;
use std::cmp;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use std::ops;
use std::ops::Deref;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};
use std::thread;

#[derive(Debug)]
pub enum DepGraphError {
    CloseNodeError(String, &'static str),
    /// The list of dependencies is empty
    EmptyListError,
    IteratorDropped,
    NoAvailableNodeError,
    ResolveGraphError(&'static str),
}

impl fmt::Display for DepGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CloseNodeError(name, reason) => {
                write!(f, "Failed to close node {}: {}", name, reason)
            }
            Self::EmptyListError => write!(f, "The dependency list is empty"),
            Self::IteratorDropped => write!(
                f,
                "The iterator attached to the coordination thread dropped"
            ),
            Self::NoAvailableNodeError => write!(f, "No node are currently available"),
            Self::ResolveGraphError(reason) => write!(f, "Failed to resolve the graph: {}", reason),
        }
    }
}

impl error::Error for DepGraphError {}

#[derive(Clone, Debug)]
pub struct Dependency<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync,
{
    id: I,
    deps: HashSet<I>,
}

impl<I> Dependency<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync,
{
    pub fn new(id: I) -> Dependency<I> {
        Dependency {
            id,
            deps: HashSet::default(),
        }
    }

    pub fn id(&self) -> &I {
        &self.id
    }
    pub fn deps(&self) -> &HashSet<I> {
        &self.deps
    }
    pub fn add_dep(&mut self, dep: I) {
        self.deps.insert(dep);
    }
}

pub type InnerGraph<I> = HashMap<I, HashSet<I>>;
pub type Graph<I> = Arc<RwLock<InnerGraph<I>>>;

pub struct DepGraph<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    pub ready_nodes: Vec<I>,
    pub deps: Graph<I>,
    pub reverse_deps: Graph<I>,
}

impl<I> DepGraph<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    pub fn new(nodes: &[Dependency<I>]) -> Result<Self> {
        let (ready_nodes, deps, reverse_deps) = DepGraph::parse_nodes(nodes);

        // check for cyclic dependencies
        if TarjanStronglyConnectedComponents::new(&deps).has_circles() {
            panic!("has circles");
        }

        Ok(DepGraph {
            ready_nodes,
            deps: Arc::new(RwLock::new(deps)),
            reverse_deps: Arc::new(RwLock::new(reverse_deps)),
        })
    }

    fn parse_nodes(nodes: &[Dependency<I>]) -> (Vec<I>, InnerGraph<I>, InnerGraph<I>) {
        let mut deps = InnerGraph::<I>::default();
        let mut reverse_deps = InnerGraph::<I>::default();
        let mut ready_nodes = Vec::<I>::default();

        for node in nodes {
            deps.insert(node.id().clone(), node.deps().clone());

            if node.deps().is_empty() {
                ready_nodes.push(node.id().clone());
            }

            for node_dep in node.deps() {
                if !reverse_deps.contains_key(node_dep) {
                    let mut dep_reverse_deps = HashSet::new();
                    dep_reverse_deps.insert(node.id().clone());
                    reverse_deps.insert(node_dep.clone(), dep_reverse_deps.clone());
                } else {
                    let dep_reverse_deps = reverse_deps.get_mut(node_dep).unwrap();
                    dep_reverse_deps.insert(node.id().clone());
                }
            }
        }

        (ready_nodes, deps, reverse_deps)
    }
}

impl<I> IntoIterator for DepGraph<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = I;
    type IntoIter = DepGraphIter<I>;

    fn into_iter(self) -> Self::IntoIter {
        DepGraphIter::<I>::new(
            self.ready_nodes.clone(),
            self.deps.clone(),
            self.reverse_deps,
        )
    }
}

#[derive(Clone)]
pub struct DepGraphIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    ready_nodes: Vec<I>,
    deps: Graph<I>,
    reverse_deps: Graph<I>,
}

impl<I> DepGraphIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    pub fn new(ready_nodes: Vec<I>, deps: Graph<I>, reverse_deps: Graph<I>) -> Self {
        Self {
            ready_nodes,
            deps,
            reverse_deps,
        }
    }
}

fn remove_node_id<I>(
    id: I,
    deps: &Graph<I>,
    reverse_deps: &Graph<I>,
) -> Result<Vec<I>, DepGraphError>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    let rdep_ids = {
        match reverse_deps.read().unwrap().get(&id) {
            Some(node) => node.clone(),
            // If no node depends on a node, it will not appear
            // in reverse_deps.
            None => Default::default(),
        }
    };

    let mut deps = deps.write().unwrap();
    let next_nodes = rdep_ids
        .iter()
        .filter_map(|rdep_id| {
            let rdep = match deps.get_mut(&rdep_id) {
                Some(rdep) => rdep,
                None => return None,
            };

            rdep.remove(&id);

            if rdep.is_empty() {
                Some(rdep_id.clone())
            } else {
                None
            }
        })
        .collect();

    // Remove the current node from the list of dependencies.
    deps.remove(&id);

    Ok(next_nodes)
}

impl<I> Iterator for DepGraphIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(id) = self.ready_nodes.pop() {
            // Remove dependencies and retrieve next available nodes, if any.
            let next_nodes =
                remove_node_id::<I>(id.clone(), &self.deps, &self.reverse_deps).unwrap();

            // Push ready nodes
            self.ready_nodes.extend_from_slice(&next_nodes);

            // Return the node ID
            Some(id)
        } else {
            // No available node
            None
        }
    }
}
