use futures::channel::oneshot::Sender;

/// Cancellation handle returned by `run_preemptible` that can be used to trigger `Streamline`
/// revert processes from outside the `next` method
pub struct Cancel {
    sender: Sender<()>,
}

impl From<Sender<()>> for Cancel {
    fn from(sender: Sender<()>) -> Self {
        Self { sender }
    }
}

impl Cancel {
    /// Cancellation method for cancelling a `Streamline` associated with a parent `Cancel`
    pub fn cancel(self) -> Result<(), ()> {
        self.sender.send(())
    }
}
