/*!
This crates provides a state machine implementation that emits states as a `std::futures::Stream`,
groups sources of external state into a single `Context`, and handles automatic conversion between states
(both forwards and backwards) through the `State` trait.
*/
#![deny(missing_docs, unreachable_pub)]
mod progress;
mod state;
mod streamline;

pub use self::progress::*;
pub use self::state::*;
pub use self::streamline::*;
