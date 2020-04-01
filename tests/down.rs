use async_trait::async_trait;
use futures::StreamExt;
use strmline::{Progress, RevertProgress, State, Streamline};
use tokio::runtime::Runtime;

#[test]
fn completes_down() {
    #[derive(Debug)]
    struct Context;

    #[allow(dead_code)]
    #[derive(Clone, Debug, PartialEq)]
    enum MyState {
        Start,
        Middle(String),
        End(String),
    }

    impl Default for MyState {
        fn default() -> Self {
            Self::Start
        }
    }

    #[derive(Debug, PartialEq)]
    struct MyError(&'static str);

    #[async_trait(?Send)]
    impl State for MyState {
        type Context = Context;
        type Error = MyError;

        async fn next(
            &self,
            _context: Option<&Self::Context>,
        ) -> Result<Option<Self>, Self::Error> {
            let next_state = match self {
                MyState::Start => Some(Self::Middle("hooray!".into())),
                MyState::Middle(_) => return Err(MyError("Something went wrong!")),
                _ => None,
            };

            Ok(next_state)
        }

        async fn revert(
            &self,
            _context: Option<&Self::Context>,
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
            Some(Progress::Revert(RevertProgress::Reverted { source })) => {
                assert_eq!(**source, MyError("Something went wrong!"))
            }
            _ => panic!("incorrect terminal state found"),
        }
    });
}
