use super::{remove_node_id, DepGraph, Graph};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use rayon::iter::{
    plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer},
    IndexedParallelIterator, IntoParallelIterator, ParallelIterator,
};
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, RwLock,
};
use std::thread;

impl<I> IntoParallelIterator for DepGraph<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = Wrapper<I>;
    type Iter = DepGraphParIter<I>;

    fn into_par_iter(self) -> Self::Iter {
        DepGraphParIter::new(self.ready_nodes, self.deps, self.reverse_deps, None)
    }
}

#[derive(Debug, Clone)]
pub struct Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    /// Wrapped item
    inner: I,
    /// Channel to notify that the item is done processing (upon drop)
    item_done_tx: Sender<I>,
}

impl<I> Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    /// Create a new Wrapper item
    ///
    /// This needs a reference to the processing counter to keep count of the
    /// number of items currently processed (used to check for circular
    /// dependencies) and the item done channel to notify the dispatcher
    /// thread.
    ///
    pub fn new(inner: I, item_done_tx: Sender<I>) -> Self {
        Self {
            inner,
            item_done_tx,
        }
    }
}

/// Drop implementation to decrement the processing counter and notify the
/// dispatcher thread.
impl<I> Drop for Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    /// Triggered when the wrapper is dropped.
    ///
    /// This will decrement the processing counter and notify the dispatcher thread.
    fn drop(&mut self) {
        self.item_done_tx
            .send(self.inner.clone())
            .expect("could not send message")
    }
}

/// Dereference implementation to access the inner item
///
impl<I> ops::Deref for Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<I> ops::DerefMut for Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<I> Eq for Wrapper<I> where I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static
{}

impl<I> Hash for Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}

impl<I> cmp::PartialEq for Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

/// Parallel iterator for DepGraph
pub struct DepGraphParIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    concurrency: usize,
    item_ready_rx: Receiver<I>,
    item_done_tx: Sender<I>,
}

impl<I> DepGraphParIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    /// Create a new parallel iterator
    ///
    /// This will create a thread and crossbeam channels to listen/send
    /// available and processed nodes.
    pub fn new(
        ready_nodes: HashSet<I>,
        deps: Graph<I>,
        reverse_deps: Graph<I>,
        concurrency: Option<usize>,
    ) -> Self {
        // create communication channel for processed nodes
        let (item_ready_tx, item_ready_rx) = crossbeam_channel::unbounded::<I>();
        let (item_done_tx, item_done_rx) = crossbeam_channel::unbounded::<I>();

        // println!("cargo:warning=deps: {:?}", deps.read().unwrap());
        // println!(
        //     "cargo:warning=reverse deps: {:?}",
        //     reverse_deps.read().unwrap()
        // );

        // inject ready nodes
        ready_nodes
            .iter()
            .for_each(|node| item_ready_tx.send(node.clone()).unwrap());

        // start dispatcher thread
        thread::spawn(move || -> Result<()> {
            while let Ok(id) = item_done_rx.recv() {
                // println!("cargo:warning=item done: {:?}", id);
                // println!("cargo:warning=deps: {:?}", deps.read().unwrap());
                // println!(
                //     "cargo:warning=reverse deps: {:?}",
                //     reverse_deps.read().unwrap()
                // );
                // Remove the node from all reverse dependencies
                let next_nodes = remove_node_id::<I>(id, &deps, &reverse_deps)?;
                // println!("cargo:warning=next nodes: {:?}", next_nodes);

                // send the next available nodes to the channel
                next_nodes
                    .iter()
                    .for_each(|node_id| item_ready_tx.send(node_id.clone()).unwrap());

                // if there are no more nodes, leave the loop
                if deps.read().unwrap().is_empty() {
                    break;
                }
            }
            // println!("cargo:warning=dispatcher end");
            drop(item_ready_tx);
            Ok(())
        });

        DepGraphParIter {
            concurrency: concurrency.unwrap_or(num_cpus::get()),
            item_ready_rx,
            item_done_tx,
        }
    }
}

impl<I> ParallelIterator for DepGraphParIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = Wrapper<I>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }
}

impl<I> IndexedParallelIterator for DepGraphParIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    fn len(&self) -> usize {
        self.concurrency
    }

    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.callback(DepGraphProducer {
            item_ready_rx: self.item_ready_rx,
            item_done_tx: self.item_done_tx,
        })
    }
}

struct DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    item_ready_rx: Receiver<I>,
    item_done_tx: Sender<I>,
}

impl<I> Iterator for DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = Wrapper<I>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.item_ready_rx.recv() {
            Ok(item) => Some(Wrapper::new(item, self.item_done_tx.clone())),
            Err(_) => None,
        }
    }
}

impl<I> DoubleEndedIterator for DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

impl<I> ExactSizeIterator for DepGraphProducer<I> where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static
{
}

impl<I> Producer for DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = Wrapper<I>;
    type IntoIter = Self;

    fn into_iter(self) -> Self::IntoIter {
        Self {
            item_ready_rx: self.item_ready_rx.clone(),
            item_done_tx: self.item_done_tx,
        }
    }

    fn split_at(self, _: usize) -> (Self, Self) {
        (
            Self {
                item_ready_rx: self.item_ready_rx.clone(),
                item_done_tx: self.item_done_tx.clone(),
            },
            Self {
                item_ready_rx: self.item_ready_rx.clone(),
                item_done_tx: self.item_done_tx,
            },
        )
    }
}
