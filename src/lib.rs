use async_trait::async_trait;
use futures::{stream, Stream};
use std::{fmt, sync::Arc};

#[derive(Debug)]
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
    pub fn build(state: S) -> Self {
        Self {
            context: None,
            current: Progress::from(state),
        }
    }

    pub fn context(mut self, context: C) -> Self {
        self.context = Some(context);

        self
    }

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
                let current = std::mem::take(&mut state_machine.current);

                Some((current, None))
            }
        } else {
            None
        }
    }
}

#[async_trait(?Send)]
pub trait State: Clone + fmt::Debug + Default + PartialEq {
    type Context;
    type Error;

    async fn next(&self, context: Option<&Self::Context>) -> Result<Option<Self>, Self::Error>;
    async fn revert(&self, context: Option<&Self::Context>) -> Result<Option<Self>, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub enum RevertProgress<S, E, C>
where
    S: State<Context = C, Error = E>,
{
    Reverting { step: S, source: Arc<E> },
    Reverted { source: Arc<E> },
    Failure { source: Arc<E>, error: E },
}

#[derive(Debug, PartialEq)]
pub enum Progress<S, E, C>
where
    S: State<Context = C, Error = E>,
{
    Ok(S),
    Revert(RevertProgress<S, E, C>),
}

impl<S, E, C> Default for Progress<S, E, C>
where
    S: State<Context = C, Error = E>,
{
    fn default() -> Self {
        Self::Ok(S::default())
    }
}

impl<S, E, C> From<S> for Progress<S, E, C>
where
    S: State<Context = C, Error = E>,
{
    fn from(state: S) -> Self {
        Self::Ok(state)
    }
}
