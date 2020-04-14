use crate::{
    progress::{Progress, RevertProgress},
    state::State,
};
use futures::{stream, Stream};
use std::sync::Arc;

/// Streamlines represent the streams of states configured for a particular Context, Error type,
/// and `State`-implementing type
pub struct Streamline<C, E, S>
where
    S: State<Context = C, Error = E>,
{
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

    async fn reduce(state_machine: Option<Self>) -> Option<(Progress<S, E, C>, Option<Self>)> {
        if let Some(mut state_machine) = state_machine {
            let context = state_machine.context.as_ref();
            let next_state = match &state_machine.current {
                Progress::Ok(inner) => match inner.next(context).await {
                    Ok(None) => None,
                    Ok(Some(next)) => Some(Progress::Ok(next)),
                    Err(source) => Some(Progress::Revert(RevertProgress::Reverting {
                        step: inner.clone(),
                        source: Arc::new(source),
                    })),
                },
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
