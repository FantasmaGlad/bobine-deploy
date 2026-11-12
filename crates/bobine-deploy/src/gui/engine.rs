use bobine_deploy_engine::{execute, Sink};
use bobine_deploy_proto::{Command, Event};
use iced::futures::channel::mpsc;
use iced::futures::SinkExt;
use iced::Task;

use super::message::{Job, Message};

const EVENT_BUFFER: usize = 4096;

struct Forward(mpsc::Sender<Message>);

impl Sink for Forward {
    fn event(&self, event: Event) {
        let _ = self.0.clone().try_send(Message::Event(event));
    }
}

pub fn run(job: Job, command: Command) -> Task<Message> {
    Task::run(
        iced::stream::channel(EVENT_BUFFER, async move |mut output: mpsc::Sender<Message>| {
            let result = execute(command, &Forward(output.clone())).await;
            let _ = output.send(Message::Finished(job, result)).await;
        }),
        |message| message,
    )
}
