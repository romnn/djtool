use std::collections::HashSet;

#[derive(thiserror::Error, Debug)]
pub enum Error<E, I>
where
    I: std::fmt::Debug,
    // pub enum Error<'a, E>
    // where
    //     E: std::error::Error,
{
    #[error("invalid configuration: `{0}`")]
    InvalidConfiguration(String),
    // InvalidConfiguration(&'a str),
    #[error("some tasks failed")]
    Failed(Vec<E>),

    #[error("schedule error: `{0}`")]
    Schedule(#[from] ScheduleError<I>),
}

#[derive(thiserror::Error, Debug)]
pub enum ScheduleError<I>
where
    I: std::fmt::Debug,
{
    #[error("dependency cycle detected")]
    Cycle,

    #[error("dependency cycle detected")]
    UnsatisfiedDependencies(HashSet<I>),

    #[error("task does not exist: `{0:?}`")]
    NoTask(I),
}
