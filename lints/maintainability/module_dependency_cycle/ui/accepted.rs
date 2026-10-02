#![allow(dead_code)]

mod leaf {
    pub fn call() {}
}
mod middle {
    pub fn call() {
        crate::leaf::call();
    }
}
mod root {
    pub fn call() {
        crate::middle::call();
        crate::generated_back_edge::call();
    }
    pub fn leaf() {}
}

mod one_top_level_module {
    pub mod left {
        pub fn call() {
            super::right::call();
        }
        pub fn leaf() {}
    }
    pub mod right {
        pub fn call() {
            super::left::leaf();
        }
    }
}

mod generated_back_edge {
    macro_rules! call_root {
        () => {
            crate::root::leaf()
        };
    }
    pub fn call() {
        call_root!();
    }
}

fn external_crates_are_excluded() {
    std::mem::drop(String::new());
}

struct RootContract;

mod root_consumer {
    pub struct Worker;

    impl Worker {
        pub fn work(&self, _contract: &crate::RootContract) {}
    }
}

mod method_caller {
    macro_rules! generated_method_call {
        ($worker:expr) => {
            $worker.work(&crate::RootContract)
        };
    }

    pub fn run(worker: &crate::root_consumer::Worker) {
        worker.work(&crate::RootContract);
        generated_method_call!(worker);
        let _length = String::new().len();
    }
}

fn root_calls_children() {
    method_caller::run(&root_consumer::Worker);
}

fn main() {}
