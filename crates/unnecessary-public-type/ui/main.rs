pub struct UnusedStruct {
    value: String,
}

pub enum UnusedEnum {
    Ready,
}

pub type UnusedAlias = usize;

pub struct UsedStruct {
    value: String,
}

fn takes_used(value: UsedStruct) -> String {
    value.value
}

struct PrivateType;

/// Documented public type; assumed to be intentionally exported.
pub struct DocumentedPublicType;

#[allow(dead_code)]
pub struct AllowedPublicType;

#[repr(C)]
pub struct FfiPublicType {
    value: u8,
}

fn main() {
    let _ = takes_used(UsedStruct {
        value: String::new(),
    });
    let _ = PrivateType;
}

// A sibling crate in the workspace uses this type.
pub struct SharedWithSibling;
