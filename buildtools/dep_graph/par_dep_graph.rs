#[cfg(feature = "parallel-build")]
use rayon::iter::{
    plumbing::{bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer},
    IndexedParallelIterator, IntoParallelIterator, ParallelIterator,
};
#[cfg(feature = "parallel-build")]
use crossbeam_channel::{Receiver, Sender};

impl<I> IntoParallelIterator for DepGraph<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = Wrapper<I>;
    type Iter = DepGraphParIter<I>;

    fn into_par_iter(self) -> Self::Iter {
        DepGraphParIter::new(self.ready_nodes, self.deps, self.reverse_deps)
    }
}

#[derive(Clone)]
pub struct Wrapper<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    /// Wrapped item
    inner: I,
    /// Reference to the number of items being currently processed
    // counter: Arc<AtomicUsize>,
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
    // counter: Arc<AtomicUsize>,
    pub fn new(inner: I, item_done_tx: Sender<I>) -> Self {
        // (*counter).fetch_add(1, Ordering::SeqCst);
        Self {
            inner,
            // counter,
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
        // (*self.counter).fetch_sub(1, Ordering::SeqCst);
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

#[cfg(feature = "parallel-build")]
{
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
}

/// Parallel iterator for DepGraph
#[cfg(feature = "parallel-build")]
pub struct DepGraphParIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    // timeout: Arc<RwLock<Option<Duration>>>,
    // counter: Arc<AtomicUsize>,
    concurrency: usize,
    item_ready_rx: Receiver<I>,
    item_done_tx: Sender<I>,
}

#[cfg(feature = "parallel-build")]
impl<I> DepGraphParIter<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    /// Create a new parallel iterator
    ///
    /// This will create a thread and crossbeam channels to listen/send
    /// available and processed nodes.
    pub fn new(
        ready_nodes: Vec<I>,
        deps: Graph<I>,
        reverse_deps: Graph<I>,
        concurrency: Option<usize>,
    ) -> Self {
        // let timeout = Arc::new(RwLock::new(DEFAULT_TIMEOUT));
        // let counter = Arc::new(AtomicUsize::new(0));

        // Create communication channel for processed nodes
        let (item_ready_tx, item_ready_rx): (Sender<I>, Receiver<I>) = mpsc::channel();
        let (item_done_tx, item_done_rx): (Sender<I>, Receiver<I>) = mpsc::channel();
        // let (item_ready_tx, item_ready_rx) = crossbeam_channel::unbounded::<I>();
        // let (item_done_tx, item_done_rx) = crossbeam_channel::unbounded::<I>();

        // Inject ready nodes
        ready_nodes
            .iter()
            .for_each(|node| item_ready_tx.send(node.clone()).unwrap());

        // Clone Arcs for dispatcher thread
        // let dispatcher_timeout = timeout.clone();
        // let dispatcher_counter = counter.clone();

        // Start dispatcher thread
        thread::spawn(move || {
            while let Ok(id) = item_done_rx.recv() {
                println!("cargo:warning=item done: {:?}", id);
            }
            // loop {
            //     crossbeam_channel::select! {
            //         // Grab a processed node ID
            //         recv(item_done_rx) -> id => {
            //             let id = id.unwrap();
            //             // Remove the node from all reverse dependencies
            //             let next_nodes = remove_node_id::<I>(id, &deps, &reverse_deps)?;

            //             // Send the next available nodes to the channel.
            //             next_nodes
            //                 .iter()
            //                 .for_each(|node_id| item_ready_tx.send(node_id.clone()).unwrap());

            //             // If there are no more nodes, leave the loop
            //             if deps.read().unwrap().is_empty() {
            //                 break;
            //             }
            //         },
            //         // Timeout
            //         default(*loop_timeout.read().unwrap()) => {
            //             let deps = deps.read().unwrap();
            //             let counter_val = loop_counter.load(Ordering::SeqCst);
            //             if deps.is_empty() {
            //                 break;
            //             // There are still some items processing.
            //             } else if counter_val > 0 {
            //                 continue;
            //             } else {
            //                 return Err(Error::ResolveGraphError("circular dependency detected"));
            //             }
            //         },
            //     };
            // }

            // drop ready channel and stop threads listening to it
            drop(item_ready_tx);
            // Ok(())
        });

        DepGraphParIter {
            // timeout,
            // counter,
            concurrency: concurrency.unwrap_or(num_cpus::get()),
            item_ready_rx,
            item_done_tx,
        }
    }

    // pub fn with_timeout(self, timeout: Duration) -> Self {
    //     *self.timeout.write().unwrap() = timeout;
    //     self
    // }
}

#[cfg(feature = "parallel-build")]
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

#[cfg(feature = "parallel-build")]
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
            // counter: self.counter.clone(),
            item_ready_rx: self.item_ready_rx,
            item_done_tx: self.item_done_tx,
        })
    }
}

#[cfg(feature = "parallel-build")]
struct DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    // counter: Arc<AtomicUsize>,
    item_ready_rx: Receiver<I>,
    item_done_tx: Sender<I>,
}

#[cfg(feature = "parallel-build")]
impl<I> Iterator for DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = Wrapper<I>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.item_ready_rx.recv() {
            Ok(item) => Some(Wrapper::new(
                item,
                // self.counter.clone(),
                self.item_done_tx.clone(),
            )),
            Err(_) => None,
        }
    }
}

#[cfg(feature = "parallel-build")]
impl<I> DoubleEndedIterator for DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

#[cfg(feature = "parallel-build")]
impl<I> ExactSizeIterator for DepGraphProducer<I> where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static
{
}

#[cfg(feature = "parallel-build")]
impl<I> Producer for DepGraphProducer<I>
where
    I: Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync + 'static,
{
    type Item = Wrapper<I>;
    type IntoIter = Self;

    fn into_iter(self) -> Self::IntoIter {
        Self {
            // counter: self.counter.clone(),
            item_ready_rx: self.item_ready_rx.clone(),
            item_done_tx: self.item_done_tx,
        }
    }

    fn split_at(self, _: usize) -> (Self, Self) {
        (
            Self {
                // counter: self.counter.clone(),
                item_ready_rx: self.item_ready_rx.clone(),
                item_done_tx: self.item_done_tx.clone(),
            },
            Self {
                // counter: self.counter.clone(),
                item_ready_rx: self.item_ready_rx.clone(),
                item_done_tx: self.item_done_tx,
            },
        )
    }
}
