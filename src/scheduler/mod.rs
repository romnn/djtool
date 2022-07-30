// pub mod builder;
pub mod error;
pub mod job;

use async_trait::async_trait;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
// use std::ops::Deref;
// pub use builder::SchedulerBuilder;
pub use error::{Error, ScheduleError};
use futures::stream::{FuturesUnordered, StreamExt};
// use futures_util::{Stream, StreamExt};
// use job::{IntoTask, Task};
use job::{IntoTask, Task, TaskNode};
use std::cmp::Eq;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;
use std::future::Future;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;

#[async_trait]
pub trait Policy // <P, I, C, O, E>
{
    async fn arbitrate<'a, P, I, C, O, E>(
        &self,
        scheduler: &Scheduler<'a, P, I, C, O, E>,
    ) -> Option<I>
    where
        P: Policy + Send + Sync,
        C: Send + Sync + 'static,
        O: Send + Sync + 'static,
        E: 'static,
        I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug;
}

pub struct GreedyPolicy {}

// <P, I, C, O, E>
// <I, C, O, E>
#[async_trait]
impl Policy for GreedyPolicy {
    async fn arbitrate<'a, P, I, C, O, E>(
        &self,
        scheduler: &Scheduler<'a, P, I, C, O, E>,
    ) -> Option<I>
    where
        P: Policy + Send + Sync,
        C: Send + Sync + 'static,
        O: Send + Sync + 'static,
        E: 'static,
        I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug,
    {
        let ready = scheduler.ready().await;
        ready.into_iter().next()
    }
}

impl GreedyPolicy {
    fn new() -> Self {
        Self {}
    }
}

// #[derive(Debug)]
// pub struct Scheduler<'a, P, I, L, C, O, E>
pub struct Scheduler<'a, P, I, C, O, E>
where
    P: Policy + Send + Sync,
    I: std::fmt::Debug,
    // F: Future<Output = Result<O, E>>,
{
    // phantom1: std::marker::PhantomData<P>,
    // phantom2: std::marker::PhantomData<I>,
    // phantom3: std::marker::PhantomData<L>,
    // phantom4: std::marker::PhantomData<C>,
    // phantom5: std::marker::PhantomData<O>,
    // phantom6: std::marker::PhantomData<E>,
    // pool: FuturesUnordered<Box<dyn Future<Output = Result<O, E>>>,
    // pool: RwLock<FuturesUnordered<Box<mut dyn Future<Output = Result<O, E>> + Send + Sync>>>,
    pool: RwLock<FuturesUnordered<Pin<Box<dyn Future<Output = Result<(I, O), E>> + Send + Sync>>>>,
    // pool: RwLock<FuturesUnordered<Pin<Box<dyn Future<Output = Result<O, E>>>>>>,
    // constraints: Vec<Box<dyn Fn(&Scheduler) -> bool>>,
    // arbitrator: Box<dyn Fn(&Scheduler) -> bool>,
    policy: P,
    ctx_factory: Box<dyn FnMut() -> C + Send + Sync + 'a>,
    // ctx_factory: Box<dyn FnMut() -> C + 'a>,
    schedule: RwLock<Schedule<I, C, O, E>>,
    // schedule: RwLock<Schedule<I, L, C, O, E>>,
}

// type InnerGraph<I> = HashMap<I, HashSet<I>>;
// type DAG<I> = Arc<RwLock<InnerGraph<I>>>;
type DAG<I> = HashMap<I, HashSet<I>>;

// #[derive(Debug)]
// pub struct Schedule<I, L, C, O, E> {
pub struct Schedule<I, C, O, E> {
    // tasks: HashMap<I, Box<dyn Task<I, L, C, O, E>>>,
    tasks: HashMap<I, Task<I, C, O, E>>,
    // tasks: HashMap<I, Rc<RefCell<Task<I, C, O, E>>>>,
    ready: HashSet<I>,
    deps: DAG<I>,
    reverse_deps: DAG<I>,
}

// impl<I, L, C, O, E> Schedule<I, L, C, O, E> {
impl<I, C, O, E> Schedule<I, C, O, E> {
    pub fn new() -> Self {
        Self {
            ready: HashSet::new(),
            deps: HashMap::new(),
            reverse_deps: HashMap::new(),
            tasks: HashMap::new(),
        }
    }
}

// impl<I, L, C, O, E> Schedule<I, L, C, O, E>
impl<I, C, O, E> Schedule<I, C, O, E>
where
    I: Clone + Eq + Hash + std::fmt::Debug,
{
    // pub fn add_task<T: IntoTask<I, L, C, O, E>>(
    pub fn completed(&mut self, id: I) -> Result<(), ScheduleError<I>> {
        if let Some(dependants) = self.rev_deps.get(&id) {};
        Ok(())
        // .ok_or(UnsatisfiedDependencies(HashSet<I>)ScheduleError
    }

    // pub fn add_task<T: IntoTask<I, L, C, O, E>>(
    pub fn add_task<T: IntoTask<I, C, O, E>>(&mut self, task: T) -> Result<(), ScheduleError<I>> {
        let deps: DAG<I> = HashMap::new();
        let mut seen = HashSet::<I>::new();
        // let mut stack = Vec::<Box<dyn Task<I, L, C, O, E>>>::new();
        // let mut stack = Vec::<TaskNode<I, L, C, O, E>>::new();
        let mut stack = Vec::<TaskNode<I, C, O, E>>::new();
        // Vec::<Task<I, L, C, O, E>>::new();

        // let root = task.into_task();
        // let root_id = root.id.clone();
        // self.tasks.insert(root.id.clone(), task.into_task());
        // stack.push(&self.tasks[&root_id]);
        // stack.push(Box::new(task));
        stack.push(Box::new(task).into_task());

        while let Some(current) = stack.pop() {
            // let current_task = current.into_task();
            // let id = current.task.id();
            seen.insert(current.task.id());
            // let id = current.id();
            let mut deps = deps.entry(current.task.id()).or_insert(HashSet::new());

            // let current_id = current.id();
            for dep in current.dependencies.into_iter() {
                let dep_task = dep.into_task();
                // let dep_task_id = dep_task.id.clone();
                deps.insert(dep_task.task.id());
                // self.tasks.insert(dep_task_id.clone(), dep_task);
                if !seen.contains(&dep_task.task.id()) {
                    stack.push(dep_task);
                }
            }
            self.tasks.insert(
                current.task.id(),
                // Rc::new(RefCell::new(current.task)),
                current.task,
                // Task {
                //     // move all but the dependencies
                //     id: current.id(),
                //     labels: current.labels,
                //     task: current.task,
                // },
            );
        }
        self.extend(deps)?;
        Ok(())
    }
}

// impl<I, L, C, O, E> Schedule<I, L, C, O, E>
impl<I, C, O, E> Schedule<I, C, O, E>
where
    I: Clone + Eq + Hash + std::fmt::Debug,
{
    pub fn extend(&mut self, nodes: DAG<I>) -> Result<(), ScheduleError<I>> {
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

            let deps = self.deps.get(&node).unwrap();
            if deps.is_empty() {
                self.ready.insert(node.clone());
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
        self.ready.remove(&node);
        let next_nodes = self.remove(&node)?;
        self.ready.extend(next_nodes);
        Ok(())
    }
}

// pub struct Scheduler<'a, P, F, I, L, C, O, E>
// where
//     P: Policy,
//     F: Future<Output = Result<O, E>>,
// pub type GreedyScheduler<'a, I, L, O, E> = Scheduler<'a, GreedyPolicy, I, L, (), O, E>;
pub type GreedyScheduler<'a, I, O, E> = Scheduler<'a, GreedyPolicy, I, (), O, E>;

// impl<'a, I, L, O, E> Scheduler<'a, GreedyPolicy, I, L, (), O, E>
impl<'a, I, O, E> Scheduler<'a, GreedyPolicy, I, (), O, E>
// impl Scheduler<'static, GreedyPolicy, _, _, _, (), _, _>
where
    I: std::fmt::Debug,
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
            // tasks: RwLock::new(HashMap::new()),
            schedule: RwLock::new(Schedule::new()),
            // <I, Task<I, L, C, O, E>>>,
            // deps: InnerGraph::<I>::new(),
            // reverse_deps: InnerGraph::<I>::new(),
            // ready: HashSet::<I>::new(),
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

// impl<'a, P, I, L, C, O, E> Scheduler<'a, P, I, L, C, O, E>
impl<'a, P, I, C, O, E> Scheduler<'a, P, I, C, O, E>
where
    P: Policy + Send + Sync,
    // F: Future<Output = Result<O, E>>,
    C: Send + Sync + 'static,
    O: Send + Sync + 'static,
    E: 'static,
    I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug,
{
    // pub async fn add_task<T: IntoTask<I, L, C, O, E>>(&self, task: T) -> Result<(), Error<E, I>> {
    pub async fn add_task<T: IntoTask<I, C, O, E>>(&self, task: T) -> Result<(), Error<E, I>> {
        // add to graph here
        // let task = task.into_task();
        let mut schedule = self.schedule.write().await;
        schedule.add_task(task)?;
        // let mut tasks = self.tasks.write().await;
        // tasks.insert(task.id.clone(), task);

        // let mut deps = InnerGraph::<I>::default();
        // let mut reverse_deps = InnerGraph::<I>::default();
        // let mut ready= HashSet::<I>::default();

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Error<E, I>> {
        Ok(())
    }

    pub async fn trace(&self) -> Result<usize, Error<E, I>> {
        let pool = self.pool.read().await;
        Ok(pool.len())
        // Ok(0)
    }

    pub async fn running(&self) -> usize {
        let pool = self.pool.read().await;
        pool.len()
        // Ok(0)
    }

    // pub async fn mark_complete(&self) -> usize {
    //     let pool = self.pool.read().await;
    //     pool.len()
    //     // Ok(0)
    // }

    // pub async fn ready(&self) -> Result<HashSet<I>, Error<E, I>> {
    pub async fn ready(&self) -> HashSet<I> {
        // let scheduler = self.scheduler.read().await;
        let schedule = self.schedule.read().await;
        // &self.schedule.read().await.ready
        // scheduler.ready.map(|id: I| s
        schedule.ready.iter().cloned().collect()
        // .iter().cloned().collect()
        // Ok(schedule.ready.iter().cloned().collect())
        // Ok(0)
    }

    /// Marks a job as completed and updates the ready queue with any new jobs that
    /// are now ready to execute as a result.
    async fn mark_complete(&mut self, id: I, res: Result<O, E>) {
        // self.results
        // self.jobs[job_idx].state = match res {
        //     Ok(outcome) => State::Success(outcome),
        //     Err(err) => State::Failed(err),
        // };

        let schedule = self.schedule.write().await;
        schedule.completed(id);
        // for dep_idx in &self.jobs[job_idx].dependents {
        //     let is_ready = self.jobs[*dep_idx]
        //         .dependencies
        //         .iter()
        //         .all(|i| self.jobs[*i].state.success());
        //     if is_ready {
        //         self.ready.push(*dep_idx);
        //     }
        // }
    }
    pub async fn run(&mut self) -> Result<(), Error<E, I>> {
        loop {
            while let Some(id) = self.policy.arbitrate(&self).await {
                let ctx = (self.ctx_factory)();
                let mut schedule = self.schedule.write().await;
                // this task is now owned
                let mut task = schedule
                    .tasks
                    .remove(&id)
                    .ok_or(ScheduleError::NoTask(id.clone()))?;

                let mut pool = self.pool.write().await;
                pool.push(Box::pin(async move {
                    // todo: get the dependencies results
                    let id = task.id();
                    let res = (task.task)(ctx, vec![]).await;
                    (id, res)
                }));
            }

            if self.running().await == 0 {
                // run exists when all tasks are complete
                break;
            }

            match self.pool.write().await.next().await {
                Some((id, res)) => {
                    self.mark_complete(id, res).await;
                }
                _ => {
                    panic!("job pool unexpectedly empty");
                }
            }
        }

        Ok(())
    }

    pub async fn start(&self) -> Result<(), Error<E, I>> {
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
    use std::pin::Pin;
    // use super::builder::SchedulerBUilder*;

    // struct CustomPolicy {}

    // #[async_trait]
    // impl Policy for CustomPolicy {
    //     async fn schedule(&self) -> u32 {
    //         23
    //     }
    // }

    #[derive(thiserror::Error, Debug)]
    enum CustomError {
        #[error("test")]
        Test,
    }

    #[derive(Clone, Eq, PartialEq, Hash, Debug)]
    enum CustomLabel {
        A,
        B,
        C,
    }

    #[derive(Clone, Hash, Eq, PartialEq, Debug)]
    struct CustomId<L>
    where
        L: Clone + Hash + Eq + PartialEq,
    {
        id: usize,
        labels: Vec<L>,
    }

    // type CustomId = usize;
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
        id: CustomId<CustomLabel>,
        // labels: Vec<CustomLabel>,
        dependencies: Vec<Box<dyn IntoTask<CustomId<CustomLabel>, (), CustomResult, CustomError>>>,
    }

    #[async_trait]
    impl IntoTask<CustomId<CustomLabel>, (), CustomResult, CustomError> for CustomTask {
        // fn id(&self) -> CustomId {
        //     self.id
        // }

        // fn labels(&self) -> &Vec<CustomLabel> {
        //     &self.labels
        // }

        // fn dependencies(
        //     &self,
        // ) -> &Vec<Box<dyn Task<CustomId, CustomLabel, (), CustomResult, CustomError>>> {
        //     self.dependencies.cloned()
        // }

        // async fn task(
        //     self,
        //     ctx: (),
        //     prereqs: Vec<CustomResult>,
        // ) -> Pin<Box<dyn Future<Output = Result<CustomResult, CustomError>> + Send + Sync>>
        // {
        //     Box::pin(async move { Ok(12) })
        // }

        fn into_task(
            self: Box<Self>,
            // self: Self,
        ) -> TaskNode<CustomId<CustomLabel>, (), CustomResult, CustomError> {
            let id = self.id.id.clone();
            TaskNode {
                task: Task {
                    id: self.id,
                    // labels: self.labels, // .iter().cloned().collect(),
                    task: Box::new(move |ctx, prereqs| Box::pin(async move { Ok(id) })),
                },
                dependencies: self.dependencies, // .iter().cloned().collect(),
                                                 // dependencies: vec![],
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
        let mut scheduler: GreedyScheduler<CustomId<CustomLabel>, CustomResult, CustomError> =
            Scheduler::new();
        // let running = scheduler.running().await?;

        scheduler
            .add_task(CustomTask {
                id: CustomId {
                    id: 0,
                    labels: vec![],
                },
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
        assert_eq!(0, 1);
        Ok(())
    }
}
