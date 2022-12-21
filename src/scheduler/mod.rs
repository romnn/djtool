pub mod builder;
pub mod error;
pub mod policy;
pub mod schedule;
pub mod task;

use async_trait::async_trait;
pub use builder::*;
pub use error::{Error, ScheduleError, TaskError};
use futures::stream::{FuturesUnordered, StreamExt};
pub use policy::{GreedyPolicy, Policy};
pub use schedule::Schedule;
use std::cell::RefCell;
use std::cmp::Eq;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
pub use task::{IntoTask, Task, TaskNode};
use task::{State, Tasks};
use tokio::sync::{broadcast, Mutex, RwLock};

enum PoolResult<O> {
    Shutdown,
    Task(O),
}

type Trace<I> = Vec<(I, Vec<I>)>;

type Context<'a, C> = Box<dyn FnMut() -> C + Send + 'a>;

type Pool<I, O, E> =
    FuturesUnordered<Pin<Box<dyn Future<Output = PoolResult<(I, Result<O, E>)>> + Send>>>;

enum SchedulerState {
    Paused,
    Running,
}

pub struct Scheduler<'a, P, I, C, O, E>
where
    P: Policy + Send,
    I: Clone + std::fmt::Debug,
    E: Clone + std::fmt::Debug,
    O: Clone,
{
    /// mutex for locking scheduler
    running_lock: Mutex<u8>,
    /// state
    state: Mutex<SchedulerState>,
    /// pool of pending tasks
    pool: Mutex<Pool<I, O, E>>,
    /// scheduler policy
    policy: P,
    /// task context factory function
    ctx_factory: Mutex<Context<'a, C>>,
    /// map of all tasks and their state of execution
    tasks: RwLock<Tasks<I, C, O, E>>,
    /// task schedule DAG
    schedule: RwLock<Schedule<I>>,
    /// execution trace
    trace: Mutex<Trace<I>>,
    /// shutdown sender channel
    shutdown_tx: broadcast::Sender<bool>,
    /// scheduler config
    config: Config,
}

pub type GreedyScheduler<'a, I, O, E> = Scheduler<'a, GreedyPolicy, I, (), O, E>;

impl<'a, I, O, E> Scheduler<'a, GreedyPolicy, I, (), O, E>
where
    I: Clone + std::fmt::Debug,
    E: Clone + std::fmt::Debug,
    O: Clone,
{
    pub fn greedy() -> Self {
        Self::new(GreedyPolicy::new(), Box::new(|| ()))
    }
}

impl<'a, P, I, C, O, E> Scheduler<'a, P, I, C, O, E>
where
    P: Policy + Send,
    I: Clone + std::fmt::Debug,
    E: Clone + std::fmt::Debug,
    O: Clone,
    // P: Policy + Send,
    // I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug + 'static,
    // C: Send + 'static,
    // O: Clone + Send + std::fmt::Debug + 'static,
    // E: Clone + Send + std::fmt::Debug + 'static,
{
    // pub fn new(policy: P, ctx_factory: Context<'a, C>) -> Self {
    pub fn new<CF: FnMut() -> C + Send + 'a>(policy: P, ctx_factory: CF) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            running_lock: Mutex::new(0),
            state: Mutex::new(SchedulerState::Paused),
            pool: Mutex::new(FuturesUnordered::new()),
            policy,
            ctx_factory: Mutex::new(Box::new(ctx_factory)),
            tasks: RwLock::new(Tasks::new()),
            schedule: RwLock::new(Schedule::new()),
            trace: Mutex::new(Vec::new()),
            config: Config::default(),
            shutdown_tx,
        }
    }
}

impl<'a, P, I, C, O, E> Scheduler<'a, P, I, C, O, E>
where
    P: Policy + Send,
    I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug + 'static,
    C: Send + 'static,
    O: Clone + Send + std::fmt::Debug + 'static,
    E: Clone + Send + std::fmt::Debug + 'static,
{
    // pub fn new(policy: P, ctx_factory: Context<'a, C>) -> Self {
    //     let (shutdown_tx, _) = broadcast::channel(1);
    //     Self {
    //         running_lock: Mutex::new(0),
    //         pool: Mutex::new(FuturesUnordered::new()),
    //         policy,
    //         ctx_factory: Mutex::new(ctx_factory),
    //         tasks: RwLock::new(Tasks::new()),
    //         schedule: RwLock::new(Schedule::new()),
    //         trace: Mutex::new(Vec::new()),
    //         config: Config::default(),
    //         shutdown_tx,
    //     }
    // }

    /// allows concurrently adding tasks to the scheduler
    pub async fn add_task<T: IntoTask<I, C, O, E>>(&self, task: T) -> Result<(), Error<E, I>> {
        let mut deps: schedule::DAG<I> = HashMap::new();
        let mut seen = HashSet::<I>::new();
        let mut stack = Vec::<TaskNode<I, C, O, E>>::new();

        stack.push(Box::new(task).into_task()?);

        while let Some(current) = stack.pop() {
            seen.insert(current.task.id());
            let mut current_deps = deps.entry(current.task.id()).or_insert(HashSet::new());

            // consumes dependencies
            for dep in current.dependencies.into_iter() {
                let dep_task = dep.into_task()?;
                current_deps.insert(dep_task.task.id());
                if !seen.contains(&dep_task.task.id()) {
                    stack.push(dep_task);
                }
            }

            // consumes task
            // should not be called before the schedule is happy?
            let mut tasks = self.tasks.write().await;
            tasks.insert(current.task.id(), State::Pending(current.task.task));
        }
        let mut schedule = self.schedule.write().await;
        // this can leave the schedule in invalid condition
        // schedule should check compatibility first
        schedule.extend(deps)?;
        Ok(())
    }

    /// shutdown the scheduler
    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        self.pool.lock().await.clear();
    }

    /// number of running tasks in task pool
    pub async fn running(&self) -> usize {
        self.pool.lock().await.len()
    }

    /// enable execution trace
    pub fn enable_trace(&mut self, enabled: bool) {
        self.config.trace = enabled
    }

    /// get trace
    // pub async fn trace(&'a self) -> impl Iterator<Item = &'a (I, Vec<I>)> + Clone + 'a {
    // pub async fn trace(&'a self) -> impl Iterator<Item = &'a (I, Vec<I>)> + Clone + 'a {
    pub async fn trace(&self) -> Trace<I> {
        // impl Iterator<Item = &'a (I, Vec<I>)> + Clone + 'a {
        let trace = self.trace.lock().await;
        trace.clone()
    }

    pub async fn finish(&self) -> Result<(), Error<E, I>> {
        // let handle = tokio::task::spawn(async { self.start().await });
        // self.
        let mut tasks = self.tasks.read().await;
        // let mut tasks = self.tasks.write().await;
        // match self.config.result_config {
        //     ResultConfig::KeepNone => tasks.clear(),
        //     _ => {}
        // }
        let errs = tasks.failed().map(|(id, err)| (id.clone(), err.clone()));
        let errs: HashMap<I, TaskError<I, E>> = HashMap::from_iter(errs);
        if errs.len() > 0 {
            Err(Error::Failed(errs))
        } else {
            Ok(())
        }
        // Ok(())
    }

    // pub async fn run(&self) -> Result<(), Error<E, I>> {
    //     tokio::task::spawn(async { self.start().await });
    //     // finish the executor, wait for the first results
    //     self.finish().await
    // }

    /// produce tasks until scheduler is shut down
    async fn produce_tasks(&self) {
        let mut ctx_factory = self.ctx_factory.lock().await;
        loop {
            // lock the schedule, pool, and tasks
            let mut schedule = self.schedule.write().await;
            let mut tasks = self.tasks.write().await;
            match self.policy.arbitrate(&tasks, &schedule) {
                Some(id) => {
                    schedule.schedule(&id);
                    // eprintln!("scheduled {:?}", &id);

                    let dependencies: Vec<(_, _)> = schedule
                        .dependencies(&id)
                        .map(|(_, dep)| (dep.clone(), tasks.get(&dep)))
                        .collect();

                    assert!(dependencies.iter().all(|(_, state)| match state {
                        Some(State::Success(res)) => true,
                        _ => false,
                    }));

                    let prereqs: HashMap<I, O> = HashMap::from_iter(
                        dependencies.iter().filter_map(|(id, state)| match state {
                            Some(State::Success(res)) => Some((id.clone(), res.clone())),
                            _ => None,
                        }),
                    );

                    let ctx = (ctx_factory)();
                    // task is owned by replacing it
                    match tasks
                            .insert(id.clone(), State::Running)
                            // .ok_or(TaskError::NoTask(id.clone()))?
                        {
                            Some(State::Pending(mut task)) => {
                                let id = id.clone();
                                self.pool.lock().await.push(Box::pin(async move {
                                    let res = (task)(ctx, prereqs).await;
                                    PoolResult::Task((id, res))
                                }));
                            }
                            None => unreachable!("scheduled task that does not exist"),
                            _ => unreachable!("scheduled non pending task"),
                        };

                    if self.config.trace {
                        self.trace
                            .lock()
                            .await
                            .push((id.clone(), tasks.running().cloned().collect::<Vec<I>>()));
                    }
                }
                None => break,
            };
        }
    }

    /// consume finished tasks
    async fn consume_tasks(&self) {
        // eprintln!("waiting for task to complete");
        // todo: the pool could be empty, because no task have been added yet
        // to avoid waiting forever, while
        // even better: run producer and consumer concurrently
        match self.pool.lock().await.next().await {
            Some(PoolResult::Task((id, res))) => {
                // eprintln!("got task result");
                let mut tasks = self.tasks.write().await;
                let mut schedule = self.schedule.write().await;

                match res {
                    Ok(res) => {
                        // first, mark task as succeeded
                        schedule.set_state(id.clone(), schedule::State::Success);
                        tasks.insert(id.clone(), State::Success(res));
                        // for (_, dep) in vec![(0, id.clone())]
                        //     .into_iter()
                        //     .chain(schedule.recursive_dependencies(&id))
                        // {
                        let dependencies: Vec<(_, _)> =
                            schedule.recursive_dependencies(&id).collect();

                        // crate::debug!(&dependencies);
                        for (_, dep) in dependencies {
                            if schedule
                                .dependants(&dep)
                                .states()
                                .all(|(_, _, state)| match state {
                                    Some(schedule::State::Pending) => false,
                                    _ => true,
                                })
                            {
                                // can remove the dependency
                                match self.config.result_config {
                                    ResultConfig::KeepAll => {}
                                    ResultConfig::KeepNone | ResultConfig::KeepRoots => {
                                        // dependency can not be root
                                        // eprintln!("removing {:?}", &dep);
                                        tasks.remove(&dep);
                                    }
                                }
                            }
                        }
                        schedule.update_ready_nodes(&id);
                        // match self.config.result_config {
                        //     // if schedule.dependants(&dep).all( == 0 {
                        //     //      tasks.insert(dep.clone(), State::Failed(cause.clone()));
                        //     //  } else {
                        //     //      tasks.remove(&dep);
                        //     //  }

                        //     // ResultConfig::KeepAll => {
                        //     //     tasks.insert(dep.clone(), State::Failed(cause.clone()));
                        //     // }
                        //     // ResultConfig::KeepRoots => {
                        //     //     if schedule.dependants(&dep).count() == 0 {
                        //     //         tasks.insert(dep.clone(), State::Failed(cause.clone()));
                        //     //     } else {
                        //     //         tasks.remove(&dep);
                        //     //     }
                        //     // }
                        //     // ResultConfig::KeepNone => {
                        //     //     tasks.remove(&dep);
                        //     // }
                        // }
                    }
                    Err(err) => {
                        // mark all dependants as failed
                        // todo: this should be all reachable in component
                        let cause = TaskError::Precondition(id.clone());
                        for (_, dep) in vec![(0, id.clone())]
                            .into_iter()
                            .chain(schedule.recursive_dependants(&id))
                        {
                            match self.config.result_config {
                                ResultConfig::KeepAll => {
                                    tasks.insert(dep.clone(), State::Failed(cause.clone()));
                                }
                                ResultConfig::KeepRoots => {
                                    if schedule.dependants(&dep).count() == 0 {
                                        tasks.insert(dep.clone(), State::Failed(cause.clone()));
                                    } else {
                                        tasks.remove(&dep);
                                    }
                                }
                                ResultConfig::KeepNone => {
                                    tasks.remove(&dep);
                                }
                            }
                        }
                        schedule.remove_dependants(&id);
                    }
                }
            }
            Some(PoolResult::Shutdown) => return,
            _ => {
                panic!("job pool unexpectedly empty");
            }
        }
    }

    // pub async fn start(&self) -> Result<(), Error<E, I>> {
    // pub async fn start(&self) -> Result<(), Error<E, I>> {
    pub async fn run(&self) {
        // todo: think about when and how the locking should take place
        // lock the scheduler work loop
        let lock = self.running_lock.lock().await;

        let mut state = self.state.lock().await;
        *state = SchedulerState::Running;
        drop(state);

        futures::join!(self.produce_tasks(), self.consume_tasks());

        // let mut shutdown_rx = self.shutdown_tx.subscribe();
        // self.pool.lock().await.push(Box::pin(async move {
        //     let _ = shutdown_rx.recv().await;
        //     PoolResult::Shutdown
        // }));

        // loop {
        // if !self.config.long_running && self.running().await == 1 {
        //     // exit as soon as task pool is empty
        //     // note: the remaining task in the pool is the task waiting for shutdown
        //     break;
        // }

        // eprintln!("next round");
        // }

        // cancel all futures in the pool
        // self.pool.lock().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::future::Future;
    use std::pin::Pin;
    use tokio::time::{sleep, Duration};

    // struct CustomPolicy {}

    // #[async_trait]
    // impl Policy for CustomPolicy {
    //     async fn schedule(&self) -> u32 {
    //         23
    //     }
    // }

    #[derive(thiserror::Error, Clone, Debug)]
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
        trace_id: usize,
        labels: Vec<L>,
    }

    type CustomResult = usize;

    type Dependencies<I, C, O, E> = Vec<Box<dyn IntoTask<I, C, O, E> + Send>>;

    struct CustomTask {
        id: CustomId<CustomLabel>,
        dependencies: Dependencies<CustomId<CustomLabel>, (), CustomResult, CustomError>,
    }

    impl CustomTask {
        pub fn new(
            id: usize,
            trace_id: usize,
            dependencies: Dependencies<CustomId<CustomLabel>, (), CustomResult, CustomError>,
        ) -> Self {
            Self {
                id: CustomId {
                    id,
                    trace_id,
                    labels: vec![],
                },
                dependencies,
            }
        }
    }

    #[async_trait]
    impl IntoTask<CustomId<CustomLabel>, (), CustomResult, CustomError> for CustomTask {
        fn into_task(
            self: Box<Self>,
        ) -> Result<
            TaskNode<CustomId<CustomLabel>, (), CustomResult, CustomError>,
            Error<CustomError, CustomId<CustomLabel>>,
        > {
            let id = self.id.id.clone();
            Ok(TaskNode {
                task: Task {
                    id: self.id,
                    task: Box::new(move |ctx, prereqs| {
                        Box::pin(async move {
                            crate::debug!(id);
                            crate::debug!(ctx);
                            crate::debug!(prereqs);
                            sleep(Duration::from_secs(2)).await;
                            Ok(id)
                        })
                    }),
                },
                dependencies: self.dependencies,
            })
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_default_scheduler() -> Result<()> {
        let mut scheduler = Scheduler::greedy();
        scheduler.enable_trace(true);
        // : GreedyScheduler<CustomId<CustomLabel>, CustomResult, CustomError> =
        // Scheduler::new();

        scheduler
            .add_task(CustomTask::new(
                0,
                1,
                vec![
                    Box::new(CustomTask::new(1, 0, vec![])),
                    Box::new(CustomTask::new(2, 0, vec![])),
                ],
            ))
            .await?;
        scheduler.run().await;
        let trace = scheduler.trace().await;
        let trace = trace
            .iter()
            .map(|(task, _)| task.trace_id)
            .collect::<Vec<usize>>();
        assert_eq!(trace, vec![0, 0, 1]);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_greedy_policy_limit() -> Result<()> {
        let policy = GreedyPolicy::max_tasks(Some(3));
        let mut scheduler = SchedulerBuilder::new(policy, Box::new(|| ())).build();
        scheduler.enable_trace(true);
        let deps = (1..)
            .take(10)
            .map(|id| Box::new(CustomTask::new(id, 0, vec![])))
            .map(|task| Box::<dyn IntoTask<_, _, _, _> + Send>::from(task))
            .collect();
        scheduler.add_task(CustomTask::new(0, 1, deps)).await?;
        scheduler.run().await;
        let trace = scheduler.trace().await;
        let active = trace.iter().map(|(_, active)| active.len());
        // crate::debug!(active.clone().collect::<Vec<usize>>());
        assert!(active.max().unwrap() <= 3);
        Ok(())
    }

    // #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    // async fn test_greedy_policy_limit() -> Result<()> {
    //     let policy = GreedyPolicy::max_tasks(Some(3));
    //     let mut scheduler = SchedulerBuilder::new(policy, Box::new(|| ())).build();
    //     scheduler.enable_trace(true);
    //     let deps = (1..)
    //         .take(10)
    //         .map(|id| Box::new(CustomTask::new(id, 0, vec![])))
    //         .map(|task| Box::<dyn IntoTask<_, _, _, _> + Send>::from(task))
    //         .collect();
    //     scheduler.add_task(CustomTask::new(0, 1, deps)).await?;
    //     let results = scheduler.run().await?;
    //     let trace = scheduler.trace().await;
    //     let active = trace.iter().map(|(_, active)| active.len());
    //     // crate::debug!(active.clone().collect::<Vec<usize>>());
    //     assert!(active.max().unwrap() <= 3);
    //     Ok(())
    // }
}
