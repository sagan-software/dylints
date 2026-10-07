#![allow(dead_code)]

// These directed acyclic graphs reduce dependency directions from popular crates.

// Case 101: serde_json-style input flows from deserializer to value.
mod serde_json_value {
    pub fn build() {}
}
mod serde_json_de {
    pub fn parse() {
        crate::serde_json_value::build();
    }
}
mod serde_json_io {
    pub fn read() {
        crate::serde_json_de::parse();
    }
}

// Case 102: tokio-style runtime ownership flows toward tasks.
mod tokio_task {
    pub fn schedule() {}
}
mod tokio_runtime {
    pub fn tick() {
        crate::tokio_task::schedule();
    }
}
mod tokio_builder {
    pub fn build() {
        crate::tokio_runtime::tick();
    }
}

// Case 103: clap-style command construction flows into parsing and output.
mod clap_output {
    pub fn render() {}
}
mod clap_parser {
    pub fn parse() {
        crate::clap_output::render();
    }
}
mod clap_command {
    pub fn run() {
        crate::clap_parser::parse();
    }
}

// Case 104: reqwest-style client execution flows toward transport.
mod reqwest_transport {
    pub fn send() {}
}
mod reqwest_redirect {
    pub fn check() {
        crate::reqwest_transport::send();
    }
}
mod reqwest_client {
    pub fn execute() {
        crate::reqwest_redirect::check();
    }
}

// Case 105: regex-style source lowers through a compiler pipeline.
mod regex_automata {
    pub fn compile() {}
}
mod regex_hir {
    pub fn lower() {
        crate::regex_automata::compile();
    }
}
mod regex_parser {
    pub fn parse() {
        crate::regex_hir::lower();
    }
}

// Case 106: tracing-style events flow toward a subscriber.
mod tracing_subscriber {
    pub fn record() {}
}
mod tracing_dispatch {
    pub fn dispatch() {
        crate::tracing_subscriber::record();
    }
}
mod tracing_event {
    pub fn emit() {
        crate::tracing_dispatch::dispatch();
    }
}

// Case 107: rayon-style iterator execution flows into plumbing.
mod rayon_plumbing {
    pub fn bridge() {}
}
mod rayon_iter {
    pub fn drive() {
        crate::rayon_plumbing::bridge();
    }
}
mod rayon_user {
    pub fn execute() {
        crate::rayon_iter::drive();
    }
}

// Case 108: anyhow-style context wraps an underlying error.
mod anyhow_source {
    pub fn source() {}
}
mod anyhow_context {
    pub fn attach() {
        crate::anyhow_source::source();
    }
}
mod anyhow_error {
    pub fn construct() {
        crate::anyhow_context::attach();
    }
}

// Case 109: axum-style routing flows through extraction to response.
mod axum_response {
    pub fn render() {}
}
mod axum_extract {
    pub fn extract() {
        crate::axum_response::render();
    }
}
mod axum_routing {
    pub fn route() {
        crate::axum_extract::extract();
    }
}

// Case 110: bytes-style values delegate to buffer operations.
mod bytes_buf {
    pub fn access() {}
}
mod bytes_value {
    pub fn read() {
        crate::bytes_buf::access();
    }
}
mod bytes_api {
    pub fn consume() {
        crate::bytes_value::read();
    }
}

fn main() {}
