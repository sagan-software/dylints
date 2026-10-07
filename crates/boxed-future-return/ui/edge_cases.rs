#![allow(dead_code)]

use std::future::{Future, ready};
use std::pin::Pin;

struct Service;

trait Loader {
    fn load(&self) -> Pin<Box<dyn Future<Output = usize> + Send + '_>>;
}

impl Service {
    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(ready(()))
    }

    fn run_native(&self) -> impl Future<Output = ()> + '_ {
        ready(())
    }
}

impl Loader for Service {
    fn load(&self) -> Pin<Box<dyn Future<Output = usize> + Send + '_>> {
        Box::pin(ready(1))
    }
}

fn main() {}
