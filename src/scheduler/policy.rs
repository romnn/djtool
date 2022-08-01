use async_trait::async_trait;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use super::error::{Error, ScheduleError};
use super::{Schedule, Scheduler};
use futures::stream::{FuturesUnordered, StreamExt};
use super::task::{IntoTask, State, Task, TaskNode, Tasks};
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
    // async fn arbitrate<'a, P, I, C, O, E>(
    // async fn arbitrate<'a, I>(
    async fn arbitrate<I, C, O, E>(
        // &'_ self,
        &self,
        // scheduler: &'a Schedule<I>,
        tasks: &Tasks<I, C, O, E>,
        scheduler: &Schedule<I>,
        // scheduler: &Scheduler<'a, P, I, C, O, E>,
    ) -> Option<I>
    where
        I: Clone + Send + Sync + Eq + Hash + std::fmt::Debug + 'static,
        C: Send + Sync + 'static,
        O: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + std::fmt::Debug + 'static;
}

pub struct GreedyPolicy {
    max_tasks: Option<usize>,
}

impl GreedyPolicy {
    pub fn new() -> Self {
        Self { max_tasks: None }
    }

    pub fn max_tasks(max_tasks: Option<usize>) -> Self {
        Self { max_tasks }
    }
}

#[async_trait]
impl Policy for GreedyPolicy {
    // this should take mutable reference to the schedule
    // this should take immutable reference to tasks
    // executionstats (total ids, failed ids, running ids)
    // async fn arbitrate<'a, P, I, C, O, E>(
    // async fn arbitrate<'a, I>(
    async fn arbitrate<I, C, O, E>(
        // &'_ self,
        &self,
        // scheduler: &Scheduler<'a, P, I, C, O, E>,
        tasks: &Tasks<I, C, O, E>,
        schedule: &Schedule<I>,
        // schedule: &'a Schedule<I>,
        // ) -> Option<&'a I>
    ) -> Option<I>
    where
        // P: Policy + Send + Sync,
        I: Clone + Send + Sync + Eq + Hash + std::fmt::Debug + 'static,
        C: Send + Sync + 'static,
        O: Clone + Send + Sync + 'static,
        E: Clone + Send + Sync + std::fmt::Debug + 'static,
        // I: Clone + Eq + Hash + Send + Sync + std::fmt::Debug,
    {
        // returning some id -> task with id will be executed
        // returning None -> no task can be scheduled at the moment, wait for any task tofinish
        // before attempting to schedule again

        // take any ready job unless the max number of jobs is exceeded
        if let Some(max_tasks) = self.max_tasks {
            if tasks.running().count() >= max_tasks {
                return None;
            }
        }
        schedule.ready().cloned().next()
        // let ready = scheduler.ready().await;
        // ready.into_iter().next()
        // None
    }
}
