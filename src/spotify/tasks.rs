use crate::download;
use crate::proto;
use crate::scheduler::{self, IntoTask, Policy, Task, TaskNode};
use crate::utils;
use crate::Sink;
use async_trait::async_trait;
use futures_util::FutureExt;
use std::collections::HashSet;
use std::path::PathBuf;
use tempdir::TempDir;

#[derive(thiserror::Error, Clone, Debug)]
pub(super) enum Error {
    #[error("test")]
    Test,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub(super) enum TaskLabel {
    Download,
    Transcode,
    Match,
    Tag,
    Move,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub(super) struct TaskId {
    label: TaskLabel,
    track: proto::djtool::TrackId,
}

#[derive(Clone, Debug)]
pub(super) enum TaskResult {
    Test,
}

#[derive(Debug)]
pub(super) struct SpotifyTrack {
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

// #[derive(Debug)]
pub(super) struct DownloadTrackAudioTask {
    // id of the current task
    pub track: proto::djtool::Track,
    pub output: TempDir,
    pub sink: Sink,
    // sink: &'a Sink,
    // url to download
    // path to save result to
    // handle to the progress bar manager
}

impl IntoTask<TaskId, (), TaskResult, Error> for DownloadTrackAudioTask {
    fn into_task(
        self: Box<Self>,
    ) -> Result<TaskNode<TaskId, (), TaskResult, Error>, scheduler::Error<Error, TaskId>> {
        // Err(scheduler::Error::InvalidConfiguration("test".into()))
        // Err(Box::new(anyhow::anyhow!("no such id").into()).into())
        // Err(scheduler::Error::Build::from(Box::new(
        //     anyhow::anyhow!("no such id").into(),
        // )))
        //     // )))?
        let dependencies = vec![
            // add dependencies
        ];
        let id = TaskId {
            track: self.track.id.as_ref().unwrap().clone(),
            // .ok_or(scheduler::Error::Build(Box::new(
            //     anyhow::anyhow!("no such id").into(),
            // )))?
            // // .map_err(scheduler::Error::from)?
            // .clone(),
            label: TaskLabel::Download,
        };
        let candidate_filename =
            utils::sanitize_filename(&format!("{} - {}", &self.track.name, self.track.artist));

        Ok(TaskNode {
            dependencies,
            task: Task {
                id,
                task: Box::new(|ctx, prereqs| {
                    Box::pin(async move {
                        let downloaded = self
                            .sink
                            .download(
                                &self.track,
                                // &self.output,
                                &self
                                    .output
                                    .path()
                                    .join(format!("original_{}", &candidate_filename)),
                                None,
                                Some(Box::new(move |progress: download::DownloadProgress| {
                                    // bar_clone.set_message("downloading".to_string());
                                    // bar_clone.set_position(progress.downloaded as u64);
                                    // bar_clone.set_length(progress.total.unwrap() as u64);
                                    // bar_clone.tick();
                                    // println!(
                                    //     "downloaded: {} {:?}",
                                    //     progress.downloaded,
                                    //     progress.total.unwrap()
                                    // );
                                    // io::stdout().flush().unwrap();
                                    // io::stderr().flush().unwrap();
                                })),
                            )
                            // todo: map download error here
                            .await
                            .unwrap();

                        Ok(TaskResult::Test)
                    })
                }),
            },
        })
    }
}
