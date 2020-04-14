use crate::state::State;
use std::sync::Arc;

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

impl<S, E, C> From<S> for Progress<S, E, C>
where
    S: State<Context = C, Error = E>,
{
    fn from(state: S) -> Self {
        Self::Ok(state)
    }
}
