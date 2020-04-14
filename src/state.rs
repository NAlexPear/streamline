use async_trait::async_trait;

/// The `State` trait defines the way that a `Streamline` progresses to (or from) the next state.
#[async_trait(?Send)]
pub trait State: Clone + PartialEq {
    /// Global state shared between all `Streamline` states.
    type Context;
    /// The Error shared between all states progressions.
    type Error;

    /// Derives the next state when progressing through a `Streamline` that has not encountered any
    /// errors. There are no limits to how many times a state can be visited, but all mappings
    /// between user states must be explicit. If `Err(Self::Error)` is returned from this method,
    /// the reversion process is triggered. If `Ok(None)` is returned, the `Streamline` ends. If
    /// `Ok(Some(Self))` is returned, the stream continues to the next iteration of `next`
    async fn next(&self, context: Option<&mut Self::Context>) -> Result<Option<Self>, Self::Error>;

    /// Handles the mapping between a state and its previous state in the case of reversion on
    /// `Err` from `next()`. By default, `revert` simply ends the `Streamline`
    async fn revert(&self, _context: Option<&mut Self::Context>) -> Result<Option<Self>, Self::Error> {
        Ok(None)
    }
}
