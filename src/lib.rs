/*!
This crates provides a state machine implementation that emits states as a `std::futures::Stream`,
groups sources of external state into a single `Context`, and handles automatic conversion between states
(both forwards and backwards) through the `State` trait.
*/
#![deny(missing_docs)]
use async_trait::async_trait;
use futures::{stream, Stream};
use std::{fmt, sync::Arc};

/// Streamlines represent the streams of states configured for a particular Context, Error type,
/// and `State`-implementing type
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
                let current = std::mem::take(&mut state_machine.current);

                Some((current, None))
            }
        } else {
            None
        }
    }
}

/// The `State` trait defines the way that a `Streamline` progresses to (or from) the next state.
#[async_trait(?Send)]
pub trait State: Clone + fmt::Debug + Default + PartialEq {
    /// Global state shared between all `Streamline` states.
    type Context;
    /// The Error shared between all states progressions.
    type Error;

    /// Derives the next state when progressing through a `Streamline` that has not encountered any
    /// errors. There are no limits to how many times a state can be visited, but all mappings
    /// between user states must be explicit. If `Err(Self::Error)` is returned from this method,
    /// the reversion process is triggered. If `Ok(None)` is returned, the `Streamline` ends. If
    /// `Ok(Some(Self))` is returned, the stream continues to the next iteration of `next`
    async fn next(&self, context: Option<&Self::Context>) -> Result<Option<Self>, Self::Error>;

    /// Handles the mapping between a state and its previous state in the case of reversion on
    /// `Err` from `next()`. By default, `revert` simply ends the `Streamline`
    async fn revert(&self, _context: Option<&Self::Context>) -> Result<Option<Self>, Self::Error> {
        Ok(None)
    }
}

/// An internal state machine that represents the process of reverting previous progress.
#[derive(Debug, PartialEq)]
pub enum RevertProgress<S, E, C>
where
    S: State<Context = C, Error = E>,
{
    /// An in-flight `State` reversion
    Reverting {
        /// the state variant in the process of being reverted
        step: S,
        /// the original error that triggered the reversion process
        source: Arc<E>,
    },
    /// The final state of a successful reversion
    Reverted {
        /// the original error that triggered the reversion process
        source: Arc<E>,
    },
    /// The final state of a failed reversion
    Failure {
        /// the original error that triggered the reversion process
        source: Arc<E>,
        /// the error that caused the reversion process to fail
        error: E,
    },
}

/// The state emitted by a `Streamline`
#[derive(Debug, PartialEq)]
pub enum Progress<S, E, C>
where
    S: State<Context = C, Error = E>,
{
    /// All user-provided states run as part of `Progress::Ok` until they trigger a reversion
    Ok(S),
    /// Once a reversion has been triggered, `Progress` tracks the state of the reversion through
    /// a `RevertProgress` `enum`
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
