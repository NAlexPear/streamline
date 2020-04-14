use async_trait::async_trait;
use futures::StreamExt;
use streamline::{Progress, State, Streamline};
use tokio::runtime::Runtime;

#[test]
fn completes_up() {
    struct Context;

    #[derive(Clone, Debug, PartialEq)]
    enum MyState {
        Start,
        Middle(String),
        End(String),
    }

    #[async_trait(?Send)]
    impl State for MyState {
        type Context = Context;
        type Error = ();

        async fn next(
            &self,
            _context: Option<&mut Self::Context>,
        ) -> Result<Option<Self>, Self::Error> {
            let next_state = match self {
                MyState::Start => Some(Self::Middle("hooray!".into())),
                MyState::Middle(content) => Some(Self::End(content.into())),
                _ => None,
            };

            Ok(next_state)
        }

        async fn revert(
            &self,
            _context: Option<&mut Self::Context>,
        ) -> Result<Option<Self>, Self::Error> {
            let next_state = match self {
                MyState::End(content) => Some(Self::Middle(content.to_string())),
                MyState::Middle(_) => Some(Self::Start),
                _ => None,
            };

            Ok(next_state)
        }
    }

    Runtime::new().unwrap().block_on(async {
        let states: Vec<_> = Streamline::build(MyState::Start)
            .context(Context)
            .run()
            .collect()
            .await;

        match states.first() {
            Some(Progress::Ok(state)) => assert_eq!(state, &MyState::Start),
            _ => panic!("incorrect start state found"),
        };

        match states.last() {
            Some(Progress::Ok(state)) => assert_eq!(state, &MyState::End("hooray!".into())),
            _ => panic!("incorrect terminal state found"),
        }
    });
}
