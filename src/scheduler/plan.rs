use super::job::IntoTask;
use super::Error;
use std::collections::HashSet;
use std::rc::Rc;

struct PlanBuilderEntry<C, O, E> {
    Task: Rc<dyn IntoTask<C, O, E>>,
    dependencies: HashSet<usize>,
    dependents: HashSet<usize>,
}

pub struct PlanBuilder<C, O, E> {
    Tasks: Vec<PlanBuilderEntry<C, O, E>>,
    ancestors: HashSet<usize>,
    current_parent: usize,
    ready: Vec<usize>,
}

// impl<C: 'static, O: 'static, E: 'static> PlanBuilder<C, O, E> {
//     fn index_of<J: IntoTask<C, O, E> + PartialEq>(&self, Task: &J) -> Option<usize> {
//         for (idx, entry) in self.Tasks.iter().enumerate() {
//             if let Some(existing_Task) = entry.Task.downcast_ref::<J>() {
//                 if Task == existing_Task {
//                     return Some(idx);
//                 }
//             }
//         }
//         None
//     }

//     pub fn add_dependency<J: IntoTask<C, O, E> + PartialEq>(
//         &mut self,
//         Task: J,
//     ) -> Result<(), Error<E>> {
//         Ok(())
//     }
// }
// struct PlanEntry<C, E> {
//     state: State<C, E>,
//     dependencies: HashSet<usize>,
//     dependents: HashSet<usize>,
// }

// struct Plan<C, E> {
//     Tasks: Vec<PlanEntry<C, E>>,
//     ready: Vec<usize>,
// }

// impl<C, E> Plan<C, E> {
//     fn new<J: IntoTask<C, E>>(Task: J) -> Result<Self, Error<E>> {
//         let Task = Rc::new(Task);

//         let mut builder = PlanBuilder {
//             Tasks: vec![PlanBuilderEntry {
//                 Task: Task.clone(),
//                 dependencies: HashSet::new(),
//                 dependents: HashSet::new(),
//             }],
//             ancestors: HashSet::from_iter(vec![0]),
//             current_parent: 0,
//             ready: vec![],
//         };

//         Task.plan(&mut builder)?;
//         if builder.Tasks[0].dependencies.is_empty() {
//             builder.ready.push(0);
//         }

//         Ok(Self {
//             Tasks: builder
//                 .Tasks
//                 .drain(..)
//                 .map(|e| PlanEntry {
//                     state: State::Pending(e.Task.into_Task()),
//                     dependencies: e.dependencies,
//                     dependents: e.dependents,
//                 })
//                 .collect(),
//             ready: builder.ready,
//         })
//     }
// }
