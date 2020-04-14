use async_trait::async_trait;
use futures::StreamExt;
use streamline::{State, Streamline};
use tokio::runtime::Runtime;
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex };

#[test]
fn handles_context() {
    lazy_static! {
        static ref CONTEXT: Arc<Mutex<Context>> = Arc::new(Mutex::new(Context { items: vec![] }));
    }

    #[derive(Clone)]
    struct Context {
        items: Vec<u8>
    }

    #[derive(Clone, Debug, PartialEq)]
    enum MyState {
        Start,
        End,
    }

    #[async_trait(?Send)]
    impl State for MyState {
        type Context = Arc<Mutex<Context>>;
        type Error = ();

        async fn next(
            &self,
            context: Option<&mut Self::Context>,
        ) -> Result<Option<Self>, Self::Error> {
            let mut context = context.ok_or(())?.lock().expect("could not get lock on context");

            context.items.push(0);

            let next_state = match self {
                MyState::Start => Some(Self::End),
                _ => None,
            };

            Ok(next_state)
        }
    }

    Runtime::new().unwrap().block_on(async {
        Streamline::build(MyState::Start)
            .context(CONTEXT.clone())
            .run()
            .collect::<Vec<_>>()
            .await;

        let final_context = CONTEXT.lock().expect("could not get final context value");

        assert_eq!(final_context.items, [0, 0]);
    });
}
