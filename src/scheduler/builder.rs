use super::policy::{GreedyPolicy, Policy};
use super::schedule::Schedule;
use super::task::{IntoTask, State, Task, TaskNode, Tasks};
use super::{Context, Scheduler};
use async_trait::async_trait;
use super::error::{Error, ScheduleError, TaskError};
use futures::stream::{FuturesUnordered, StreamExt};
use std::cell::RefCell;
use std::cmp::Eq;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

pub enum ResultConfig {
    KeepAll,
    KeepRoots,
    KeepNone,
}

pub struct Config {
    trace: bool,
    result_config: ResultConfig,
    long_running: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            trace: false,
            result_config: ResultConfig::KeepRoots,
            long_running: false,
        }
    }
}

pub struct SchedulerBuilder<'a, P, C>
// where
//     P: Policy + Send + Sync,
{
    /// scheduler policy
    policy: P,
    /// task context factory function
    ctx_factory: Context<'a, C>,
    /// scheduler config
    config: Config,
}

impl<'a, P, C> SchedulerBuilder<'a, P, C> {
    // impl SchedulerBuilder<'a, P, C> {
    // impl Scheduler<'a, P, C> {
    // impl Scheduler {
    pub fn policy(policy: P, ctx_factory: Context<'a, C>) -> Self {
        Self {
            policy,
            ctx_factory,
            config: Config::default(),
        }
    }

    pub fn trace(&mut self, trace: bool) -> &mut Self {
        self.config.trace = trace;
        self
    }

    pub fn results(&mut self, config: ResultConfig) -> &mut Self {
        self.config.result_config = config;
        self
    }

    pub fn long_running(&mut self, long_running: bool) -> &mut Self {
        self.config.long_running = long_running;
        self
    }

    pub fn build<I, O, E>(self) -> Scheduler<'a, P, I, C, O, E>
    where
        P: Policy + Send + Sync,
        I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug + 'static,
        C: Send + Sync + 'static,
        O: Clone + Send + Sync + std::fmt::Debug + 'static,
        E: Clone + Send + Sync + std::fmt::Debug + 'static,
    {
        let (shutdown_tx, _) = broadcast::channel(1);
        // Self {
        //     pool: RwLock::new(FuturesUnordered::new()),
        //     policy: GreedyPolicy::new(),
        //     ctx_factory: Box::new(|| ()),
        //     tasks: RwLock::new(Tasks::new()),
        //     schedule: RwLock::new(Schedule::new()),
        //     trace: Vec::new(),
        //     shutdown_tx,
        // }
        Scheduler {
            pool: RwLock::new(FuturesUnordered::new()),
            policy: self.policy,
            ctx_factory: self.ctx_factory,
            tasks: RwLock::new(Tasks::new()),
            schedule: RwLock::new(Schedule::new()),
            trace: Vec::new(),
            config: self.config,
            shutdown_tx,
        }
    }
}
