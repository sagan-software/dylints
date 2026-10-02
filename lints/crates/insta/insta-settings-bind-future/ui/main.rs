#![allow(dead_code)]

use insta::Settings;

async fn work() {}

async fn invalid() {
    let settings = Settings::clone_current();
    let future = settings.bind(|| async { work().await });
    future.await;
}

async fn valid_async_binding() {
    let settings = Settings::clone_current();
    settings.bind_async(async { work().await }).await;
}

fn valid_synchronous_binding() {
    let settings = Settings::clone_current();
    settings.bind(|| {});
}

struct OtherSettings;

impl OtherSettings {
    fn bind<T>(&self, function: impl FnOnce() -> T) -> T {
        function()
    }
}

async fn same_name_is_not_insta() {
    let settings = OtherSettings;
    settings.bind(|| async { work().await }).await;
}

fn main() {}
