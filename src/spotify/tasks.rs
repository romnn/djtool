use crate::scheduler::job::{IntoTask, Task, TaskFun};
use crate::scheduler::Policy;
use async_trait::async_trait;
use futures_util::FutureExt;

#[derive(thiserror::Error, Debug)]
pub(super) enum Error {
    #[error("test")]
    Test,
}

#[derive(Debug)]
pub(super) enum TaskLabel {
    Download,
    Transcode,
    Match,
    Tag,
    Move,
}

#[derive(Debug)]
pub(super) enum TaskId {
    Test,
}

#[derive(Debug)]
pub(super) enum TaskResult {
    Test,
}

#[derive(Debug)]
pub(super) struct DownloadFileTask {
    // id of the current task
// url to download
// path to save result to
// handle to the progress bar manager
}

#[derive(Debug)]
pub(super) struct DownloadSpotifyTrack {
    // id of the current task
// url to download
// path to save result to
// handle to the progress bar manager
}

// #[derive(Debug)]
// pub struct SchedulePolicy {
//     // max concurrent items in hash map
// }

// #[async_trait]
// impl Policy for SchedulePolicy {
//     async fn schedule(&self) -> u32 {
//         23
//     }
// }

impl IntoTask<TaskId, TaskLabel, (), TaskResult, Error> for DownloadSpotifyTrack
// where
//     D: IntoTask<I, L, C, O, E>,
{
    // fn id(&self) -> I {
    //     // must return a unique id for each task here

    // }

    fn into_task(self) -> Task<TaskId, TaskLabel, (), TaskResult, Error> {
        // let task: TaskFun<(), TaskResult, Error> = Box::new(|ctx, prereqs| {
        //     Box::pin(async move {
        //         // copy the final result out
        //         Ok(TaskResult::Test)
        //     })
        // });
        let dependencies = vec![
            // add dependencies
        ];
        Task {
            id: TaskId::Test,
            labels: vec![TaskLabel::Download],
            dependencies,
            task: Box::new(|ctx, prereqs| {
                Box::pin(async move {
                    // copy the final result out
                    Ok(TaskResult::Test)
                })
            }),
        }
    }

    // fn dependencies(&self) -> Vec<Dependency> {
    //     let mut dep = Dependency::new(&self);
    //     // for each candidate, transcode it
    //     // self.candidate, candidate_dir.path().join(format!("original_{}", &candidate_filename)),
    //     // dep.add_dep();
    //     dep
    //     // subdep.id.clone());
    // }
}

// impl IntoTask<(), TaskResult, Error> for DownloadFileTask {
//     fn into_task(&self) -> Task<(), TaskResult, Error> {
//         Box::new(|ctx, prereqs| {
//             Box::pin(async move {
//                 eprintln!("{:?}", prereqs);
//                 Ok(TaskResult::Test)
//                 // todo: make lazy static style here
//                 // let sink = &sinks[&proto::djtool::Service::Youtube];
//                 // let bar = Arc::new(mp_clone.add(ProgressBar::new(100)));
//                 // TrackDownloadProgress::style(&bar);
//                 // bar.tick();

//                 // let bar_clone = bar.clone();
//                 // let downloaded = sink
//                 //     .download(
//                 //         &candidate,
//                 //         &candidate_dir
//                 //             .path()
//                 //             .join(format!("original_{}", &candidate_filename)),
//                 //         None,
//                 //         Some(Box::new(move |progress: download::DownloadProgress| {
//                 //             bar_clone.set_message("downloading".to_string());
//                 //             bar_clone.set_position(progress.downloaded as u64);
//                 //             bar_clone.set_length(progress.total.unwrap() as u64);
//                 //             bar_clone.tick();

//                 //             // println!(
//                 //             //     "downloaded: {} {:?}",
//                 //             //     progress.downloaded,
//                 //             //     progress.total.unwrap()
//                 //             // );
//                 //             // io::stdout().flush().unwrap();
//                 //             // io::stderr().flush().unwrap();
//                 //         })),
//                 //     )
//                 //     .await?;
//             })
//         })
//     }
// }
