use futures::channel::oneshot::Sender;

pub struct Cancel {
    sender: Sender<()>,
}

impl From<Sender<()>> for Cancel {
    fn from(sender: Sender<()>) -> Self {
        Self { sender }
    }
}

impl Cancel {
    pub fn cancel(self) -> Result<(), ()> {
        self.sender.send(())
    }
}
