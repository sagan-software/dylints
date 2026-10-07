#![allow(dead_code)]

use serde::Deserialize;

#[cfg(any())]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CfgDisabledOuterDenied {
    id: String,
    #[serde(flatten)]
    extra: Extra,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OuterDenied {
    id: String,
    #[serde(flatten)]
    extra: Extra,
}

#[derive(Deserialize)]
struct InnerDeniedOuter {
    id: String,
    #[serde(flatten)]
    extra: StrictExtra,
}

#[derive(Deserialize)]
struct Extra {
    trace_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictExtra {
    trace_id: String,
}

type StrictAlias = StrictExtra;

#[derive(Deserialize)]
struct AliasedInnerDeniedOuter {
    id: String,
    #[serde(flatten)]
    extra: StrictAlias,
}

mod nested {
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct StrictNested {
        pub trace_id: String,
    }
}

#[derive(Deserialize)]
struct QualifiedInnerDeniedOuter {
    id: String,
    #[serde(flatten)]
    extra: nested::StrictNested,
}

#[derive(Deserialize)]
struct OptionalInnerDeniedOuter {
    id: String,
    #[serde(flatten)]
    extra: Option<StrictExtra>,
}

mod shadow {
    use serde::Deserialize;

    // Same name as the denied struct above, but it accepts unknown fields.
    #[derive(Deserialize)]
    pub struct StrictExtra {
        pub trace_id: String,
    }
}

#[derive(Deserialize)]
struct ShadowedName {
    id: String,
    #[serde(flatten)]
    extra: shadow::StrictExtra,
}

#[derive(serde::Serialize)]
#[serde(deny_unknown_fields)]
struct SerializeOnlyOuter {
    id: String,
    #[serde(flatten)]
    extra: SerializeExtra,
}

#[derive(serde::Serialize)]
struct SerializeExtra {
    trace_id: String,
}

#[derive(Deserialize)]
struct Plain {
    id: String,
    extra: PlainExtra,
}

#[derive(Deserialize)]
struct PlainExtra {
    trace_id: String,
}

fn main() {}
