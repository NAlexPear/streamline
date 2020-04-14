# `streamline`
Reversible Stream-based state machine library for Rust

## Example
Here's an example of how to use a `Streamline` to create a GitHub repo, Tweet about it, and reverse the whole process if something goes wrong.

```rust
// recommended, but not required
use async_trait::async_trait;
// some example clients for communicating through third-party APIs
use clients::{Github, Twitter};
use futures::StreamExt;
use streamline::{State, Streamline};

const MY_USERNAME: &'static str = "my-github-username";

// clients are stored in context for shared access throughout the life of the streamline
#[derive(Debug)]
struct Context {
  github: Github,
  twitter: Twitter,
}

#[derive(Debug, PartialEq)]
// you should create better errors than this
type MyError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone, Debug, PartialEq)]
enum MyState {
  Github { repo_name: String },
  Twitter { repo_id: i32, repo_name: String }
  Done { repo_id: i32, repo_name: String, tweet_id: i32 }
}

#[async_trait(?Send)]
impl State for MyState {
  type Context = Context;
  type Error = MyError;

  // every state needs to be mapped to the next
  async fn next(&self, context: Option<&mut Self::Context>) -> Result<Option<Self>, Self::Error> {
    let context = context.ok_or_else(|_| Box::new("No context supplied!"))?;

    let next_state = match self {
        MyState::Github { repo_name } => {
          context
            .github
            .add_repo(&repo_name)
            .await?
            .map(|response| Some(MyState::Twitter { repo_id: &response.repo_id, repo_url: repo_name }))
        },
        MyState::Twitter { repo_name, .. } => {
          context
            .twitter
            .tweet(&format("Look at my new Github repo at https://github.com/{}/{}!", &repo_name))
            .await?
            .map(|response| Some(MyState::Done { tweet_id: response.tweet_id }))
        },
        MyState::Done { .. } => None // returning Ok(None) stops the stream!
      };

      Ok(next_state)
  }

  // optionally, old states can be cleaned up if something goes wrong
  async fn revert(&self, context: Option<&mut Self::Context>) -> Result<Option<Self>, Self::Error> {
    let context = context.ok_or_else(|_| Box::new("No context supplied!"))?;

    let next_state = match self {
      MyState::Done { tweet_id, repo_id, repo_name } => {
        context
          .twitter
          .delete_tweet(tweet_id)
          .await?;

        Some(MyState::Twitter { repo_id, repo_name })
      },
      MyState::Twitter { repo_id, repo_name } => {
        context
          .github
          .delete_repo(repo_id)
          .await?;

        Some(MyState::Github { repo_id, repo_name })
      },
      MyState::Github { .. } => None
    };

    Ok(next_state)
  }
}

async fn handle_tweeting_repo(repo_name: String) {
  let context = Context {
    github: Github::new(),
    twitter: Twitter::new(),
  };

  Streamline::build(MyState { repo_name })
    .context(context)
    .run()
    .for_each(|state| println!("Next state {:?}", &state))
    .await;
}
```

## Motivation
If one wants to move from one state to the next within a process, it makes sense in Rust to look towards some of the many [state machine patterns](https://hoverbear.org/blog/rust-state-machine-pattern/) available through the type system. `enum`s, in particular, are a great way of modeling the progress of a process in a way that excludes impossible states along the way. But there's less certainty around handling state for the following scenarios:

1. _Non-blocking state updates_: it's often the case that we care _most_ about the final state of a state machine, but we would also like to be updated when the state changes along the way to that terminal state. In state machines, this is often implemented as a side effect (e.g. through channels or an event broker).
2. _Super-statefulness_: while it's common to carry state around inside individual variants of an `enum`, it's much trickier to know when to handle updates to state that is not directly attached to a single variant of the state machine. How does one, for example, handle updating a value in a database as a state machine progresses? What about interacting with third-party services? When should these parts of state be handled?
3. _Reversibility_: most processes need to know how to clean up after themselves. Modeling these fallible processes _and_ their path towards full reversion to the original state (or failure to do so) is a complex and boilerplate-heavy process.
4. _Cancellation_: interrupting the execution of a `Stream` is easy... just drop the `Stream`! But cleaning up after a stream that represents some in-progress state is much more difficult.

`streamline` solves addresses those problems in the following ways:

1. _`futures::Stream`-compatibility_: rather than using side effects during state machine execution, this library models every update to a state machine as an `Item` in a `std::futures::Stream`.
2. _Consistent `Context`_: all super-variant state can be accessed with a consistent `Context`.
3. _Automatic Reversion_: whenever a process returns an `Err`, `streamline` will (optionally) revert all the states up to that point, returning the original error.
4. _Manual Cancellation_: the `run_preemptible` method returns a `Stream` _and_ a `Cancel` handler that can be used to trigger the revert process of a working stream.
