// run-rustfix
// rustfix-only-machine-applicable
use schemars::JsonSchema;
use serde::Serialize;

fn default_text() -> String {
    String::new()
}

fn other_default_text() -> String {
    String::new()
}

#[derive(JsonSchema, Serialize)]
struct Example {
    #[serde(default)]
    #[schemars(default)]
    bad: String,
    #[serde(default)]
    good: String,
}

#[derive(JsonSchema, Serialize)]
struct DefaultValues {
    #[serde(default = "default_text")]
    #[schemars(default = r#"default_text"#)]
    matching: String,
    #[serde(default = "default_text")]
    #[schemars(default = "other_default_text")]
    override_value: String,
    #[serde(default)]
    #[schemars(default = "default_text")]
    different_form: String,
}

macro_rules! duplicate_default {
    ($name:ident) => {
        #[derive(JsonSchema, Serialize)]
        struct $name {
            #[serde(default)]
            #[schemars(default)]
            field: String,
        }
    };
}

duplicate_default!(MacroExample);

fn local_default() {
    #[derive(JsonSchema, Serialize)]
    struct Local {
        #[serde(
            rename = r#"https://field,name"#,
            default,
            skip_serializing_if = "Option::is_none"
        )]
        #[schemars(default, /* keep this override */ rename = "https://schema,name")]
        value: Option<String>,
    }

    let _local = Local { value: None };
}

fn main() {
    let _defaults = DefaultValues {
        matching: String::new(),
        override_value: String::new(),
        different_form: String::new(),
    };
    local_default();
}
