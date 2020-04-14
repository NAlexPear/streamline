use async_trait::async_trait;
use futures::StreamExt;
use streamline::{Progress, RevertProgress, State, Streamline};
use tokio::runtime::Runtime;

#[test]
fn cancels() {
    #[derive(Debug)]
    struct Context;

    #[derive(Clone, Debug, PartialEq)]
    enum MyState {
        Start,
        Middle(String),
        End(String),
    }

    #[derive(Debug, PartialEq)]
    struct MyError(&'static str);

    #[async_trait(?Send)]
    impl State for MyState {
        type Context = Context;
        type Error = MyError;

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
        let (streamline, cancellation_handle) = Streamline::build(MyState::Start)
            .context(Context)
            .run_preemptible();

        let mut stream = streamline.boxed_local();

        let next_step = stream.next().await;

        match next_step {
            Some(Progress::Ok(state)) => assert_eq!(&state, &MyState::Start),
            _ => panic!("incorrect start state found"),
        };

        cancellation_handle.cancel().expect("could not send value through channel");

        let mut last_step = Progress::Ok(MyState::Start);

        while let Some(step) = stream.next().await {
            last_step = step;
        }

        match last_step {
            Progress::Revert(RevertProgress::Reverted { source }) => {
                assert_eq!(source, None)
            }
            _ => panic!("incorrect terminal state found"),
        }
    });
}
