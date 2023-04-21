pub mod components;

#[cfg(feature = "parallel-build")]
pub mod parallel;

use anyhow::Result;
use components::TarjanStronglyConnectedComponents;
use std::cmp;
use std::cmp::PartialEq;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::iter::{DoubleEndedIterator, ExactSizeIterator};
use std::ops;
use std::ops::Deref;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};

type InnerGraph<I> = HashMap<I, HashSet<I>>;
type Graph<I> = Arc<RwLock<InnerGraph<I>>>;

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

#[derive(Debug)]
pub struct DepGraph<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    pub ready_nodes: HashSet<I>,
    pub deps: Graph<I>,
    pub reverse_deps: Graph<I>,
}

impl<I> DepGraph<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    pub fn new(nodes: Vec<Dependency<I>>) -> Result<Self> {
        let (ready_nodes, deps, reverse_deps) = DepGraph::parse_nodes(nodes);

        // check for cyclic dependencies
        if TarjanStronglyConnectedComponents::new(&deps).has_circles() {
            panic!("has circles");
        }

        // println!("cargo:warning=deps: {:?}", deps);
        // println!("cargo:warning=reverse deps: {:?}", reverse_deps);

        Ok(DepGraph {
            ready_nodes,
            deps: Arc::new(RwLock::new(deps)),
            reverse_deps: Arc::new(RwLock::new(reverse_deps)),
        })
    }

    /// set of all recursive dependencies for node
    pub fn reacheable(&self, node: &I) -> HashSet<I> {
        let mut seen = HashSet::<I>::new();
        let mut stack = Vec::<I>::new();
        stack.push(node.clone());
        while !stack.is_empty() {
            let cur = stack.pop().unwrap();
            seen.insert(cur.clone());
            if let Some(deps) = self
                .deps
                .read()
                .ok()
                .as_ref()
                .and_then(|deps| deps.get(&cur))
            {
                for dep in deps.iter() {
                    if !seen.contains(dep) {
                        stack.push(dep.clone());
                    }
                }
            }
        }
        seen
    }

    pub fn shake(&mut self, nodes: Vec<I>) {
        let mut all_reacheable = HashSet::<I>::new();
        for node in nodes {
            all_reacheable.extend(self.reacheable(&node));
        }
        let remove: HashSet<I> = HashSet::from_iter(
            self.deps
                .read()
                .unwrap()
                .keys()
                .filter(|dep| !all_reacheable.contains(dep))
                .map(|dep| dep.to_owned()),
        );
        for dep in &remove {
            self.ready_nodes.remove(&dep);
            self.deps.write().unwrap().remove(&dep);
            self.reverse_deps.write().unwrap().remove(&dep);
            for (_, deps) in self.deps.write().unwrap().iter_mut() {
                deps.remove(dep);
            }
            for (_, deps) in self.reverse_deps.write().unwrap().iter_mut() {
                deps.remove(dep);
            }
        }
    }

    fn parse_nodes(nodes: Vec<Dependency<I>>) -> (HashSet<I>, InnerGraph<I>, InnerGraph<I>) {
        let mut deps = InnerGraph::<I>::default();
        let mut reverse_deps = InnerGraph::<I>::default();
        let mut ready_nodes = HashSet::<I>::default();

        for node in nodes {
            deps.insert(node.id().clone(), node.deps().clone());

            if node.deps().is_empty() {
                ready_nodes.insert(node.id().clone());
            }

            for node_dep in node.deps() {
                if !reverse_deps.contains_key(node_dep) {
                    reverse_deps.insert(
                        node_dep.clone(),
                        HashSet::from_iter(vec![node.id().clone()]),
                    );
                }

                // if !reverse_deps.contains_key(node_dep) {
                //     // let mut dep_reverse_deps = HashSet::new();
                //     // dep_reverse_deps.insert(node.id().clone());
                //     reverse_deps.insert(
                //         node_dep.clone(),
                //         HashSet::from_iter(vec![node.id().clone()]),
                //     );
                //     // dep_reverse_deps.clone());
                // } else {
                //     let dep_reverse_deps = reverse_deps.get_mut(node_dep).unwrap();
                //     dep_reverse_deps.insert(node.id().clone());
                // }
                // let dep_reverse_deps = reverse_deps.get_mut(node_dep).unwrap();
                reverse_deps
                    .get_mut(node_dep)
                    .unwrap()
                    .insert(node.id().clone());
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
    ready_nodes: HashSet<I>,
    deps: Graph<I>,
    reverse_deps: Graph<I>,
}

impl<I> DepGraphIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    pub fn new(ready_nodes: HashSet<I>, deps: Graph<I>, reverse_deps: Graph<I>) -> Self {
        Self {
            ready_nodes,
            deps,
            reverse_deps,
        }
    }
}

pub fn remove_node_id<I>(id: I, deps: &Graph<I>, reverse_deps: &Graph<I>) -> Result<Vec<I>>
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
        if let Some(id) = self.ready_nodes.iter().next().cloned() {
            self.ready_nodes.remove(&id);
            // remove dependencies and retrieve next available nodes, if any
            let next_nodes =
                remove_node_id::<I>(id.clone(), &self.deps, &self.reverse_deps).ok()?;
            // push ready nodes
            self.ready_nodes.extend(next_nodes);
            return Some(id);
        }
        None
    }
}
