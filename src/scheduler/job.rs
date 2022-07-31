use super::error::Error;
use async_trait::async_trait;
use downcast_rs::{impl_downcast, Downcast};
use std::future::Future;
use std::pin::Pin;

// add dependencies:
// lets say: outputs artwork image data
// idea: for all the Tasks, make them return the same outcome type
// then, each Task with prerequisits can get them as a vector of those outcomes

pub type TaskFun<C, O, E> = Box<
    dyn FnOnce(C, Vec<O>) -> Pin<Box<dyn Future<Output = Result<O, E>> + Send + Sync>>
        + Send
        + Sync,
>;

/// a task
pub struct Task<I, C, O, E> {
    /// unique identifier for this task
    pub id: I,
    /// task function
    pub task: TaskFun<C, O, E>,
}

impl<I, C, O, E> Task<I, C, O, E>
where
    I: Clone,
{
    pub fn id(&self) -> I {
        self.id.clone()
    }
}

/// task node with dependencies
pub struct TaskNode<I, C, O, E> {
    // pub task: TaskFun<C, O, E>,
    pub task: Task<I, C, O, E>,
    pub dependencies: Vec<Box<dyn IntoTask<I, C, O, E>>>,
}

/// state of a task during execution
pub enum State<C, O, E> {
    Pending(TaskFun<C, O, E>),
    Running,
    Success(O),
    Failed(E),
}

impl<C, O, E> State<C, O, E> {
    fn success(&self) -> bool {
        match self {
            State::Success(_) => true,
            _ => false,
        }
    }
}

// pub trait IntoTask<I, L, C, O, E>: std::fmt::Debug {
// pub trait IntoTask<I, L, C, O, E> {
#[async_trait]
pub trait IntoTask<I, C, O, E> {
    // pub trait IntoTask<I, L, C, O, E> {
    // : Downcast {
    // fn dependencies(&self) -> Vec<Dependency> {
    //     // let mut dep = Dependency::new(&self);
    // }
    // fn id(&self) -> I;
    //     // must return a unique id for each task here

    // }
    // pub id: I,
    //     pub labels: Vec<L>,
    //     pub dependencies: Vec<Box<dyn IntoTask<I, L, C, O, E>>>,
    //     pub task: TaskFun<C, O, E>,

    // fn id(&self) -> I;
    // fn labels(&self) -> &Vec<L>;
    // fn dependencies(&self) -> &Vec<Box<dyn Task<I, L, C, O, E>>>;
    // async fn task(
    //     self,
    //     ctx: C,
    //     prereqs: Vec<O>,
    // ) -> Pin<Box<dyn Future<Output = Result<O, E>> + Send + Sync>>;
    // Task<I, L, C, O, E>;

    // fn into_task(self: Box<Self>) -> TaskNode<I, L, C, O, E>;
    fn into_task(self: Box<Self>) -> TaskNode<I, C, O, E>;
    // fn into_task(self: Self) -> TaskNode<I, L, C, O, E>;

    // fn plan(&self, plan: &mut PlanBuilder<C, O, E>) -> Result<(), Error<E>> {
    //     // this is the default trait implementation
    //     #![allow(unused_variables)]

    //     Ok(())
    // }
}

// impl_downcast!(IntoTask<C, O, E>);
