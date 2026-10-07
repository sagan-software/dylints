use std::env;
use std::env::var as read_var;

fn local_var() {
    fn var(_name: &str) -> Option<String> {
        None
    }

    let _ = var("LOCAL");
}

fn ordinary_business_logic() {
    let _ = std::env::var("API_ENDPOINT");
    let _ = std::env::var_os("API_TOKEN");
    let _ = env::var("FEATURE_FLAG");
    let _ = read_var("READ_ALIAS");
}

fn write_env_is_not_a_read() {
    unsafe {
        std::env::set_var("FEATURE_FLAG", "on");
    }
}

fn main() {
    let _ = std::env::var("LOG_LEVEL");
}

mod cli {
    pub fn parse() {
        let _ = std::env::var("CLI_MODE");
    }
}

mod config {
    pub fn load() {
        let _ = std::env::var_os("DATABASE_URL");
    }
}

fn bootstrap() {
    let _ = std::env::var("BOOTSTRAP_MODE");
}

fn load_settings() {
    let _ = std::env::var("SETTINGS_PATH");
}

fn read_env() {
    let _ = std::env::var("APP_ENV");
}

#[test]
fn reads_env_in_test() {
    let _ = std::env::var("TEST_ONLY");
}

mod tests {
    pub fn helper() {
        let _ = std::env::var("HELPER_TEST_ENV");
    }
}
