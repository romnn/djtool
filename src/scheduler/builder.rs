use super::Policy;
use super::Scheduler;
use std::cmp::Eq;
use std::hash::Hash;
use std::future::Future;

pub struct SchedulerBuilder<'a, P, C>
// where
//     P: Policy,
{
    // constraints: Vec<Box<dyn Fn(&Scheduler) -> bool>>,
    policy: P,
    // tasks: RwLock<HashMap<I, Task<I, L, C, O, E>>>,
    ctx_factory: Box<dyn FnMut() -> C + 'a>,
}

impl<'a, P, C> SchedulerBuilder<'a, P, C>
where
    P: Policy,
{
    pub fn new() -> &mut Self {
        Self {
            ctx_factory: Box<dyn FnMut() -> C + 'a>,
        }
    }

    pub fn context<F>(&mut self, factory: F) -> &mut Self
    where
        F: FnMut() -> C + 'a,
    {
        self.ctx_factory = Box::new(factory);
        self
    }

    pub fn policy(&mut self, policy: P) -> &mut Self
// where
    //     C: Fn(&Scheduler) -> bool,
    {
        self.policy = policy;
        self
    }

    // pub fn contrained<C>(&mut self, constraint: C) -> &mut Self
    // where
    //     C: Fn(&Scheduler) -> bool,
    // {
    //     self.constraints.push(Box::new(constraint));
    //     self
    // }

    pub fn build<F, I, L, O, E>(&mut self) -> Scheduler<P, F, I, L, C, O, E>
    where
        F: Future<Output = Result<O, E>>,
        I: Copy + Hash + Eq,
    {
        Scheduler {
            ctx_factory: self.ctx_factory,
            // constraints: self.constraints,
            policy: self.policy,
        }
    }
}
