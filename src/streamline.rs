use crate::{
    cancel::Cancel,
    progress::{Progress, RevertProgress},
    state::State,
};
use futures::{
    channel::oneshot::{self, Receiver},
    stream, Stream,
};
use std::sync::Arc;

/// Streamlines represent the streams of states configured for a particular Context, Error type,
/// and `State`-implementing type
pub struct Streamline<C, E, S>
where
    S: State<Context = C, Error = E>,
{
    cancellation_handle: Option<Receiver<()>>,
    context: Option<C>,
    current: Progress<S, E, C>,
}

impl<C, E, S> Streamline<C, E, S>
where
    S: State<Context = C, Error = E>,
{
    /// Create a `Streamline` from an initial state
    pub fn build(state: S) -> Self {
        Self {
            cancellation_handle: None,
            context: None,
            current: Progress::from(state),
        }
    }

    /// Add an (optional) context to an existing `Streamline`
    pub fn context(mut self, context: C) -> Self {
        self.context = Some(context);

        self
    }

    /// Generate a Stream of states, consuming the `Streamline`
    pub fn run(self) -> impl Stream<Item = Progress<S, E, C>> {
        stream::unfold(Some(self), Self::reduce)
    }

    /// Return a Stream of states and a cancellation handle
    pub fn run_preemptible(mut self) -> (impl Stream<Item = Progress<S, E, C>>, Cancel) {
        let (sender, receiver) = oneshot::channel::<()>();

        self.cancellation_handle = Some(receiver);

        (self.run(), Cancel::from(sender))
    }

    async fn reduce(state_machine: Option<Self>) -> Option<(Progress<S, E, C>, Option<Self>)> {
        if let Some(mut state_machine) = state_machine {
            let context = state_machine.context.as_ref();
            let next_state = match &state_machine.current {
                Progress::Ok(inner) => {
                    let cancellation_handle = match state_machine.cancellation_handle {
                        Some(_) => std::mem::take(&mut state_machine.cancellation_handle),
                        None => None,
                    };

                    // Before moving to the next state, check that the current
                    // streamline hasn't been cancelled externally
                    let cancelled_state = match cancellation_handle {
                        Some(mut reciever) => match reciever.try_recv() {
                            Ok(Some(_)) => Some(Progress::Revert(RevertProgress::Reverting {
                                step: inner.clone(),
                                source: None,
                            })),
                            _ => {
                                // replace the original receiver if one existed in the first place
                                std::mem::replace(
                                    &mut state_machine.cancellation_handle,
                                    Some(reciever),
                                );

                                None
                            }
                        },
                        _ => None,
                    };

                    if cancelled_state.is_some() {
                        cancelled_state
                    } else {
                        match inner.next(context).await {
                            Ok(None) => None,
                            Ok(Some(next)) => Some(Progress::Ok(next)),
                            Err(source) => Some(Progress::Revert(RevertProgress::Reverting {
                                step: inner.clone(),
                                source: Some(Arc::new(source)),
                            })),
                        }
                    }
                }
                Progress::Revert(RevertProgress::Reverting { step, source }) => {
                    match step.revert(context).await {
                        Ok(None) => Some(Progress::Revert(RevertProgress::Reverted {
                            source: source.clone(),
                        })),
                        Ok(Some(next)) => Some(Progress::Revert(RevertProgress::Reverting {
                            step: next,
                            source: source.clone(),
                        })),
                        Err(error) => Some(Progress::Revert(RevertProgress::Failure {
                            source: source.clone(),
                            error,
                        })),
                    }
                }
                _ => None,
            };

            if let Some(next_state) = next_state {
                let current = std::mem::replace(&mut state_machine.current, next_state);

                Some((current, Some(state_machine)))
            } else {
                Some((state_machine.current, None))
            }
        } else {
            None
        }
    }
}
