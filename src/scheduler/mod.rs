// pub mod builder;
pub mod error;
pub mod job;

use async_trait::async_trait;
// pub use builder::SchedulerBuilder;
pub use error::{Error, ScheduleError};
use futures::stream::{FuturesUnordered, StreamExt};
use job::{IntoTask, Task};
use std::cmp::Eq;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;
use std::future::Future;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait Policy {
    async fn schedule(&self) -> u32;
}

pub struct GreedyPolicy {}

#[async_trait]
impl Policy for GreedyPolicy {
    async fn schedule(&self) -> u32 {
        23
    }
}

impl GreedyPolicy {
    fn new() -> Self {
        Self {}
    }
}

// #[derive(Debug)]
pub struct Scheduler<'a, P, I, L, C, O, E>
where
    P: Policy,
    // F: Future<Output = Result<O, E>>,
{
    // phantom1: std::marker::PhantomData<P>,
    // phantom2: std::marker::PhantomData<I>,
    // phantom3: std::marker::PhantomData<L>,
    // phantom4: std::marker::PhantomData<C>,
    // phantom5: std::marker::PhantomData<O>,
    // phantom6: std::marker::PhantomData<E>,
    // pool: FuturesUnordered<Box<dyn Future<Output = Result<O, E>>>,
    pool: RwLock<FuturesUnordered<Box<dyn Future<Output = Result<O, E>>>>>,
    // constraints: Vec<Box<dyn Fn(&Scheduler) -> bool>>,
    // arbitrator: Box<dyn Fn(&Scheduler) -> bool>,
    policy: P,
    ctx_factory: Box<dyn FnMut() -> C + 'a>,
    tasks: RwLock<HashMap<I, Task<I, L, C, O, E>>>,
}

// type InnerGraph<I> = HashMap<I, HashSet<I>>;
// type DAG<I> = Arc<RwLock<InnerGraph<I>>>;
type DAG<I> = HashMap<I, HashSet<I>>;

#[derive(Debug)]
pub struct Schedule<I> {
    ready_nodes: HashSet<I>,
    deps: DAG<I>,
    reverse_deps: DAG<I>,
}

impl<I> Schedule<I>
where
    I: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            ready_nodes: HashSet::new(),
            deps: HashMap::new(),
            reverse_deps: HashMap::new(),
        }
    }

    pub fn extend(&mut self, nodes: DAG<I>) -> Result<(), ScheduleError<I>> {
        // think: new nodes must be disjoint components of a graph
        // cannot have dependencies to
        // new nodes must not exist before but can have dependencies to existing nodes
        // assume node exists already
        // new node: always okay to insert (can have existing dep)
        // existing node: only okay if does not have any new dependencies, since it might have been
        // scheduled already
        for (node, new_deps) in nodes.into_iter() {
            match self.deps.entry(node.clone()) {
                Entry::Occupied(_) => {
                    // if node existed, check if there are any new dependencies
                    let deps = self.deps.get(&node).unwrap();
                    let diff: HashSet<_> = new_deps.difference(&deps).cloned().collect();
                    if !diff.is_empty() {
                        Err(ScheduleError::UnsatisfiedDependencies(diff))
                    } else {
                        Ok(())
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(new_deps);
                    Ok(())
                }
            }?;

            // if let Err(OccupiedError { value }) = self.deps.try_insert(node.clone(), new_deps) {
            //     // if node existed, check if there are any new dependencies
            //     if !diff.is_empty() {
            //         return Err(ScheduleError::UnsatisfiedDependencies(diff));
            //     }
            // }
            // let mut deps = self.deps.entry(node.clone()).or_insert(HashSet::new());
            // deps.extend(new_deps);

            let deps = self.deps.get(&node).unwrap();
            if deps.is_empty() {
                self.ready_nodes.insert(node.clone());
            }

            // this should be fine?
            for dep in deps.iter() {
                let mut rev_deps = self
                    .reverse_deps
                    .entry(dep.to_owned())
                    .or_insert(HashSet::new());
                rev_deps.insert(node.clone());
            }
        }
        // todo: check for cycles
        Ok(())
    }

    fn remove(&mut self, node: &I) -> Result<HashSet<I>, ScheduleError<I>> {
        self.deps.remove(&node);
        let empty = HashSet::<I>::new();
        let dependants = self.reverse_deps.get(node).unwrap_or(&empty);

        let free_nodes: HashSet<_> = dependants
            .iter()
            .filter_map(|dependant| match self.deps.get_mut(&dependant) {
                Some(dependant_dependencies) => {
                    dependant_dependencies.remove(node);
                    if dependant_dependencies.is_empty() {
                        Some(dependant.clone())
                    } else {
                        None
                    }
                }
                None => None,
            })
            .collect();
        Ok(free_nodes)
    }

    pub fn schedule(&mut self, node: I) -> Result<(), ScheduleError<I>> {
        self.ready_nodes.remove(&node);
        let next_nodes = self.remove(&node)?;
        self.ready_nodes.extend(next_nodes);
        Ok(())
    }
}

// pub struct Scheduler<'a, P, F, I, L, C, O, E>
// where
//     P: Policy,
//     F: Future<Output = Result<O, E>>,
pub type GreedyScheduler<'a, I, L, O, E> = Scheduler<'a, GreedyPolicy, I, L, (), O, E>;

impl<'a, I, L, O, E> Scheduler<'a, GreedyPolicy, I, L, (), O, E>
// impl Scheduler<'static, GreedyPolicy, _, _, _, (), _, _>
// where
//     F: Future<Output = Result<O, E>>,
{
    pub fn new() -> Self
// where
    //     F: Future<Output = Result<O, E>>,
    {
        Self {
            // phantom1: std::marker::PhantomData,
            // phantom2: std::marker::PhantomData,
            // phantom3: std::marker::PhantomData,
            // phantom4: std::marker::PhantomData,
            // phantom5: std::marker::PhantomData,
            // phantom6: std::marker::PhantomData,
            pool: RwLock::new(FuturesUnordered::new()),
            // constraints: Vec::new(),
            policy: GreedyPolicy::new(),
            ctx_factory: Box::new(|| ()),
            tasks: RwLock::new(HashMap::new()),
            // <I, Task<I, L, C, O, E>>>,
            // deps: InnerGraph::<I>::new(),
            // reverse_deps: InnerGraph::<I>::new(),
            // ready_nodes: HashSet::<I>::new(),
        }
    }
}

// struct PlanBuilderEntry<C, E> {
//     job: Rc<dyn IntoJob<C, E>>,
//     dependencies: HashSet<usize>,
//     dependents: HashSet<usize>,
// }

// type Dep = Clone + fmt::Debug + Eq + Hash + PartialEq + Send + Sync;
// pub trait Dep: Clone + std::fmt::Debug + Eq + Hash + PartialEq + Send + Sync {}
// impl<T: PartialOrd + Display> PartialDisplay for T {}

// #[derive(Clone, Debug)]
// pub struct Dependency<I>
// where
//     I: Dep,
// {
//     id: I,
//     deps: HashSet<I>,
//     rev_deps: HashSet<I>,
// }

// impl<I> Dependency<I>
// where
//     I: Dep,
// {
//     pub fn new(id: I) -> Dependency<I> {
//         Dependency {
//             id,
//             deps: HashSet::new(),
//             rev_deps: HashSet::new(),
//         }
//     }

//     pub fn id(&self) -> &I {
//         &self.id
//     }
//     pub fn deps(&self) -> &HashSet<I> {
//         &self.deps
//     }
//     pub fn add_dep(&mut self, dep: I) {
//         self.deps.insert(dep);
//     }
// }

// pub struct Scheduler<'a, P, O, E, C>
// where
//     P: Policy,

impl<'a, P, I, L, C, O, E> Scheduler<'a, P, I, L, C, O, E>
where
    P: Policy,
    // F: Future<Output = Result<O, E>>,
    I: Copy + Hash + Eq,
{
    pub async fn add_task<J: IntoTask<I, L, C, O, E>>(&self, task: J) -> Result<(), Error<E>> {
        // add to graph here
        let task = task.into_task();
        let id = task.id;
        let mut tasks = self.tasks.write().await;
        tasks.insert(id, task);

        // let mut deps = InnerGraph::<I>::default();
        // let mut reverse_deps = InnerGraph::<I>::default();
        // let mut ready_nodes = HashSet::<I>::default();

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Error<E>> {
        Ok(())
    }

    pub async fn trace(&self) -> Result<usize, Error<E>> {
        // let pool = self.pool.read().await;
        // Ok(pool.len())
        Ok(0)
    }

    pub async fn running(&self) -> Result<usize, Error<E>> {
        // let pool = self.pool.read().await;
        // Ok(pool.len())
        Ok(0)
    }

    pub async fn run(&self) -> Result<(), Error<E>> {
        // todo
        Ok(())
    }

    pub async fn start(&self) -> Result<(), Error<E>> {
        let handle = tokio::spawn(async move {
            loop {
                // replace with constraint
                // while pool.len() < self.max_jobs {
                // if let Some((job, idx)) = plan.next_job() {
                //     // check if job can be scheduled
                //     let ctx = (self.ctx_factory)();
                //     pool.push(async move {
                //         let res = job(ctx).await;
                //         (idx, res)
                //     })
                // } else {
                //     break;
                // }
                // }
            }
        });
        Ok(())
        // let mut plan = Plan::new(Task)?;
        // let mut pool = FuturesUnordered::new();

        // loop {
        //     // Add ready Tasks to the pool.
        //     // stop if pool is full or there are no more Tasks
        //     while pool.len() < self.max_Tasks {
        //         if let Some((Task, idx)) = plan.next_Task() {
        //             let ctx = (self.ctx_factory)();
        //             pool.push(async move {
        //                 let res = Task(ctx).await;
        //                 (idx, res)
        //             })
        //         } else {
        //             break;
        //         }
        //     }

        //     if pool.len() == 0 {
        //         // No Tasks ready to execute and no Tasks pending.
        //         // Either we've finished everything or failure
        //         break;
        //     }

        //     // here we wait for new completed Tasks
        //     if let Some((idx, res)) = pool.next().await {
        //         plan.mark_complete(idx, res);
        //     } else {
        //         panic!("Task pool unexpectedly empty");
        //     }
        // }

        // let errs = plan
        //     .Tasks
        //     .iter()
        //     .filter_map(|Task| match Task.state {
        //         State::Failed(err) => Some(err),
        //         _ => None,
        //     })
        //     .collect::<Vec<E>>();
        // // let mut errs = vec![];
        // // for Task in plan.Tasks {
        // //     if let State::Failed(err) = Task.state {
        // //         errs.push(err);
        // //     }
        // // }

        // if errs.len() > 0 {
        //     Err(Error::Failed(errs))
        // } else {
        //     Ok(())
        // }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::future::Future;
    // use super::builder::SchedulerBUilder*;

    struct CustomPolicy {}

    #[async_trait]
    impl Policy for CustomPolicy {
        async fn schedule(&self) -> u32 {
            23
        }
    }

    #[derive(thiserror::Error, Debug)]
    enum CustomError {
        #[error("test")]
        Test,
    }

    #[derive(Debug)]
    enum CustomLabel {
        A,
        B,
        C,
    }

    type CustomId = usize;
    type CustomResult = usize;

    // #[derive(Debug)]
    // enum CustomId {
    //     Test,
    // }

    // #[derive(Debug)]
    // enum CustomResult {
    //     Test,
    // }

    // #[derive(Debug)]
    struct CustomTask {
        id: CustomId,
        labels: Vec<CustomLabel>,
        dependencies: Vec<Box<dyn IntoTask<CustomId, CustomLabel, (), CustomResult, CustomError>>>,
    }

    impl IntoTask<CustomId, CustomLabel, (), CustomResult, CustomError> for CustomTask {
        fn into_task(self) -> Task<CustomId, CustomLabel, (), CustomResult, CustomError> {
            Task {
                id: self.id,
                labels: self.labels,
                dependencies: self.dependencies,
                task: Box::new(move |ctx, prereqs| Box::pin(async move { Ok(self.id) })),
            }
        }
    }
    // where
    //     D: IntoTask<I, L, C, O, E>,
    // {
    // fn id(&self) -> I {
    //     // must return a unique id for each task here

    // }

    // fn into_task(&self) -> Task<TaskId, TaskLabel, (), TaskResult, Error> {

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn default_scheduler() -> Result<()> {
        // #[test]
        // fn default_scheduler() -> Result<()> {
        // let scheduler: Scheduler<'a, P, F, I, L, C, O, E> = Scheduler::new();
        // let scheduler: Scheduler<'_, _, _, CustomId, CustomLabel, _, CustomResult, CustomError> = Scheduler::new();

        // let rt = tokio::runtime::Builder::new_multi_thread()
        //     .enable_all()
        //     .build()
        //     .unwrap();
        // rt.block_on(async {
        //     let scheduler: Arc<
        //         GreedyScheduler<
        //             // dyn Future<Output = Result<CustomResult, CustomError>>,
        //             CustomId,
        //             CustomLabel,
        //             CustomResult,
        //             CustomError,
        //         >,
        //     > = Arc::new(Scheduler::new());
        //     let running = scheduler.running().await?;
        //     Ok::<(), anyhow::Error>(())
        // })?;
        let scheduler: GreedyScheduler<
            // dyn Future<Output = Result<CustomResult, CustomError>>,
            CustomId,
            CustomLabel,
            CustomResult,
            CustomError,
        > = Scheduler::new();
        // let running = scheduler.running().await?;

        scheduler
            .add_task(CustomTask {
                id: 0,
                labels: vec![],
                dependencies: vec![],
            })
            .await?;
        let results = scheduler.run().await?;
        // todo: check the results
        let trace = scheduler.trace().await?;
        // todo: check the trace

        // let (trace, err) = TestPlan::new(vec![(true, vec![])]).trace().await;
        // assert!(err.is_none());
        // assert_eq!(trace[0], Some(0));
        Ok(())
    }
}
