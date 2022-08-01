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
use task::{IntoTask, State, Task, TaskNode, Tasks};
use tokio::sync::{broadcast, RwLock};

enum PoolResult<O> {
    Shutdown,
    Task(O),
}

type Trace<I> = Vec<(I, Vec<I>)>;

type Context<'a, C> = Box<dyn FnMut() -> C + Send + Sync + 'a>;

type Pool<I, O, E> =
    FuturesUnordered<Pin<Box<dyn Future<Output = PoolResult<(I, Result<O, E>)>> + Send + Sync>>>;

pub struct Scheduler<'a, P, I, C, O, E>
where
    P: Policy + Send + Sync,
    I: Clone + std::fmt::Debug,
    E: Clone + std::fmt::Debug,
    O: Clone,
{
    /// pool of pending tasks
    pool: RwLock<Pool<I, O, E>>,
    /// scheduler policy
    policy: P,
    /// task context factory function
    ctx_factory: Context<'a, C>,
    /// map of all tasks and their state of execution
    tasks: RwLock<Tasks<I, C, O, E>>,
    /// task schedule DAG
    schedule: RwLock<Schedule<I>>,
    /// execution trace
    trace: Trace<I>,
    // /// shutdown receiver channel
    // shutdown_rx: broadcast::Receiver<bool>,
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
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            pool: RwLock::new(FuturesUnordered::new()),
            policy: GreedyPolicy::new(),
            ctx_factory: Box::new(|| ()),
            tasks: RwLock::new(Tasks::new()),
            schedule: RwLock::new(Schedule::new()),
            trace: Vec::new(),
            config: Config::default(),
            shutdown_tx,
        }
    }
}

impl<'a, P, I, C, O, E> Scheduler<'a, P, I, C, O, E>
where
    P: Policy + Send + Sync,
    I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug + 'static,
    C: Send + Sync + 'static,
    O: Clone + Send + Sync + std::fmt::Debug + 'static,
    E: Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// allows concurrently adding tasks to the scheduler
    pub async fn add_task<T: IntoTask<I, C, O, E>>(&self, task: T) -> Result<(), Error<E, I>> {
        let mut deps: schedule::DAG<I> = HashMap::new();
        let mut seen = HashSet::<I>::new();
        let mut stack = Vec::<TaskNode<I, C, O, E>>::new();

        stack.push(Box::new(task).into_task());

        while let Some(current) = stack.pop() {
            seen.insert(current.task.id());
            let mut current_deps = deps.entry(current.task.id()).or_insert(HashMap::new());

            // consumes dependencies
            for dep in current.dependencies.into_iter() {
                let dep_task = dep.into_task();
                current_deps.insert(dep_task.task.id(), schedule::State::Pending);
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

    pub async fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// number of running tasks in task pool
    async fn running(&self) -> usize {
        self.pool.read().await.len()
    }

    /// get trace
    pub fn trace(&self) -> &Trace<I> {
        &self.trace
    }

    // pub async fn mark_complete(&self) -> usize {
    //     let pool = self.pool.read().await;
    //     pool.len()
    //     // Ok(0)
    // }

    // pub async fn ready(&self) -> Result<HashSet<I>, Error<E, I>> {
    // pub async fn ready(&self) -> HashSet<I> {
    // pub async fn ready(&self) -> tokio::sync::RwLockWriteGuard<'_, HashSet<I>> {
    //     // let scheduler = self.scheduler.read().await;
    //     let schedule = self.schedule.read().await;
    //     // schedule
    //     // &self.schedule.read().await.ready
    //     // scheduler.ready.map(|id: I| s
    //     schedule.ready // .iter().cloned().collect()
    //                    // .iter().cloned().collect()
    //                    // Ok(schedule.ready.iter().cloned().collect())
    //                    // Ok(0)
    // }

    // /// Marks a job as completed and updates the ready queue with any new jobs that
    // /// are now ready to execute as a result.
    // async fn mark_complete(&self, id: I, res: Result<O, E>) {
    //     // store the result
    //     self.schedule.write().await.completed(&id);

    //     self.tasks.write().await.insert(
    //         id.clone(),
    //         match res {
    //             Ok(res) => State::Success(res),
    //             Err(err) => State::Failed(TaskError::Failed(err)),
    //         },
    //     );

    //     // update
    //     // match res {
    //     //     Err(err) => State::Failed(err),
    //     //     _
    //     // };

    //     // update the schedule to compute new ready tasks
    //     // let mut schedule = self.schedule.write().await;
    //     // for dep_idx in &self.jobs[job_idx].dependents {
    //     //     let is_ready = self.jobs[*dep_idx]
    //     //         .dependencies
    //     //         .iter()
    //     //         .all(|i| self.jobs[*i].state.success());
    //     //     if is_ready {
    //     //         self.ready.push(*dep_idx);
    //     //     }
    //     // }
    // }

    pub async fn run(&mut self) -> Result<(), Error<E, I>> {
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        self.pool.write().await.push(Box::pin(async move {
            let _ = shutdown_rx.recv().await;
            PoolResult::Shutdown
        }));

        loop {
            // produce until no task can be produced
            loop {
                // lock the schedule, pool, and tasks
                let mut schedule = self.schedule.write().await;
                let mut pool = self.pool.write().await;
                let mut tasks = self.tasks.write().await;
                match self.policy.arbitrate(&tasks, &schedule).await {
                    Some(id) => {
                        // schedule.ready().remove(&id);
                        schedule.schedule(&id)?;

                        // let (prereqs, errs) = {
                        if let Err(err) = (|| {
                            // crate::debug!(&id, &schedule.deps); // .get(&id));
                            let dependencies = schedule.dependencies(&id);
                            // .ok_or(TaskError::NoTask(id.clone()))?;
                            // let failed =
                            // crate::debug!(dependencies);
                            // let dependencies: Vec<(I, Option<&State<_, _, _, _>>)> = dependencies
                            let dependencies: Vec<(_, _)> = dependencies
                                .map(|(_, dep)| (dep.clone(), tasks.get(&dep)))
                                .collect();

                            // let test = dependencies.iter().map(|(x, _)| x).collect::<Vec<&I>>();
                            // crate::debug!(&test);
                            // let dependencies: Vec<&State<_, _, _, _>> = schedule
                            //     .dependencies(&id)
                            //     .map(|dep| tasks.get(dep))
                            //     .collect();
                            // : (Vec<->, Vec<E>) =
                            // let (results, errs) =
                            //     dep_tasks.into_iter().partition(|p| p.succeeded());
                            // let errs

                            // let errs: HashMap<I, TaskError<I, E>> = HashMap::new();
                            // let results: Vec<O> = Vec::new();
                            // let : Vec<O> = Vec::new();

                            // for dep in dependencies.into_iter() {
                            //     match tasks.get(dep) {
                            //         Some(State::Success(res)) => results.push(res),
                            //         Some(State::Pending(_)) | Some(State::Running) => {
                            //             panic!("dependency still pending or running: {:?}", &dep)
                            //         }
                            //         Some(State::Failed(err)) => {
                            //             errs.insert(dep.clone(), TaskError::Failed(err));
                            //         }
                            //         None => {
                            //             errs.insert(dep.clone(), TaskError::NoTask(dep.clone()));
                            //         }
                            //     }
                            // }
                            assert!(dependencies.iter().all(|(_, state)| match state {
                                Some(State::Success(res)) => true,
                                _ => false,
                            }));

                            // let errs: Vec<TaskError<I, E>> = dependencies
                            //     .iter()
                            //     .filter_map(|state| match state {
                            //         (_, Some(State::Failed(err))) => Some(err.clone()),
                            //         (dep, None) => Some(TaskError::NoTask(dep.clone())),
                            //         _ => None,
                            //     })
                            //     .collect();
                            let prereqs: HashMap<I, O> = HashMap::from_iter(
                                dependencies.iter().filter_map(|(id, state)| match state {
                                    Some(State::Success(res)) => Some((id.clone(), res.clone())),
                                    _ => None,
                                }),
                            );
                            // crate::debug!(&results);
                            // crate::debug!(&errs);

                            let ctx = (self.ctx_factory)();
                            // task is owned by replacing it
                            match tasks
                                .insert(id.clone(), State::Running)
                                .ok_or(TaskError::NoTask(id.clone()))?
                            {
                                State::Pending(mut task) => {
                                    let id = id.clone();
                                    pool.push(Box::pin(async move {
                                        let res = (task)(ctx, prereqs).await;
                                        PoolResult::Task((id, res))
                                    }));
                                }
                                _ => panic!("about to schedule non pending task"),
                            };

                            self.trace
                                .push((id.clone(), tasks.running().cloned().collect::<Vec<I>>()));

                            Ok::<(), TaskError<I, E>>(())
                        })() {
                            panic!("{:?}", err);
                        };
                    }
                    None => break,
                };
            }

            if self.running().await == 1 {
                // exit when all tasks are complete
                break;
            }

            let completed = self.pool.write().await.next().await;
            match completed {
                Some(PoolResult::Task((id, res))) => {
                    match res {
                        Ok(res) => {
                            // self.mark_complete(id, res).await;
                            self.schedule.write().await.completed(&id);
                            // mark as complete and
                            self.tasks
                                .write()
                                .await
                                .insert(id.clone(), State::Success(res));
                        }
                        Err(err) => {
                            let mut tasks = self.tasks.write().await;
                            let mut schedule = self.schedule.write().await;
                            // mark task as failed
                            tasks.insert(id.clone(), State::Failed(TaskError::Failed(err)));
                            // mark all dependants as failed
                            let cause = TaskError::Precondition(id.clone());
                            for dep in schedule.dependants(&id) {
                                // if task is root (no dependants), delete the result
                                tasks.insert(id.clone(), State::Failed(cause.clone()));
                            }
                            schedule.remove_dependants(&id);
                        }
                    }
                }
                Some(PoolResult::Shutdown) => break,
                _ => {
                    panic!("job pool unexpectedly empty");
                }
            }
        }

        // cancel all futures in the pool
        self.pool.write().await.clear();
        Ok(())
    }

    pub async fn start(&self) -> Result<(), Error<E, I>> {
        let handle = tokio::spawn(async move { loop {} });
        Ok(())
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

    type Dependencies<I, C, O, E> = Vec<Box<dyn IntoTask<I, C, O, E>>>;

    // #[derive(Debug)]
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
        ) -> TaskNode<CustomId<CustomLabel>, (), CustomResult, CustomError> {
            let id = self.id.id.clone();
            TaskNode {
                task: Task {
                    id: self.id,
                    task: Box::new(move |ctx, prereqs| {
                        Box::pin(async move {
                            crate::debug!(id, ctx, prereqs);
                            sleep(Duration::from_secs(2)).await;
                            Ok(id)
                        })
                    }),
                },
                dependencies: self.dependencies,
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn default_scheduler() -> Result<()> {
        let mut scheduler = Scheduler::new();
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
        let results = scheduler.run().await?;
        // todo: check the results

        let trace = scheduler
            .trace()
            .iter()
            .map(|(task, _)| task.trace_id)
            .collect::<Vec<usize>>();
        assert_eq!(trace, vec![0, 0, 1]);

        // todo: check the trace

        // let (trace, err) = TestPlan::new(vec![(true, vec![])]).trace().await;
        // assert!(err.is_none());
        // assert_eq!(trace[0], Some(0));
        assert_eq!(0, 1);
        Ok(())
    }
}
