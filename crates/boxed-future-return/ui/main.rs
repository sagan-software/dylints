// compile-flags: --edition=2024

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
type LocalBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
type LocalJob<'a> = Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>>;

struct Ready;

impl Future for Ready {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(())
    }
}

fn refresh() -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(Ready)
}

fn load<'a>() -> BoxFuture<'a, ()> {
    Box::pin(Ready)
}

fn load_local<'a>() -> LocalBoxFuture<'a, ()> {
    Box::pin(Ready)
}

fn load_job<'a>() -> LocalJob<'a> {
    Box::pin(Ready)
}

fn load_fully_qualified() -> Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
    Box::pin(Ready)
}

fn load_box_direct() -> Box<dyn std::future::Future<Output = ()> + Send> {
    Box::new(Ready)
}

trait Service {
    fn run<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + 'a>>;
}

fn native_future() -> Ready {
    Ready
}

async fn native_async() {}

mod domain_box {
    use super::Future;

    pub struct Box<T: ?Sized>(*const T);

    pub fn custom_box() -> Box<dyn Future<Output = ()>> {
        todo!()
    }
}

trait Loader {
    fn load(&self) -> LocalBoxFuture<'_, ()>;
}

// The trait fixes the impl's signature, so only the trait declaration is reported.
impl Loader for Ready {
    fn load(&self) -> LocalBoxFuture<'_, ()> {
        Box::pin(Ready)
    }
}

// A macro-generated boxed future, like `#[async_trait]` output, is not reported.
macro_rules! boxed_method {
    ($name:ident) => {
        fn $name() -> Pin<Box<dyn Future<Output = ()> + Send>> {
            Box::pin(Ready)
        }
    };
}

boxed_method!(generated);

fn boxed_value() -> Box<u8> {
    Box::new(1)
}

fn boxed_iterator() -> Box<dyn Iterator<Item = u8>> {
    Box::new(std::iter::empty())
}

fn nested_in_option<'a>() -> Option<(u8, &'a BoxFuture<'a, ()>)> {
    None
}

fn main() {}
