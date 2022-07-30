use std::collections::HashSet;

#[derive(thiserror::Error, Debug)]
pub enum Error<E>
// pub enum Error<'a, E>
// where
//     E: std::error::Error,
{
    #[error("invalid configuration: `{0}`")]
    InvalidConfiguration(String),
    // InvalidConfiguration(&'a str),
    #[error("dependency cycle detected")]
    Cycle,

    #[error("some tasks failed")]
    Failed(Vec<E>),

    #[error("some Tasks could not be planned")]
    Plan(#[from] E),
}

#[derive(thiserror::Error, Debug)]
pub enum ScheduleError<I> {
    #[error("dependency cycle detected")]
    Cycle,
    #[error("dependency cycle detected")]
    UnsatisfiedDependencies(#[from] HashSet<I>),
}
