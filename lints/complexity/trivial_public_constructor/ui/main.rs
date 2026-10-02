pub struct UserId;

pub type UserAlias = User;

pub struct User {
    pub id: UserId,
    pub name: String,
}

impl User {
    pub fn new(id: UserId, name: String) -> UserAlias {
        UserAlias { id, name }
    }

    fn checked(id: UserId) -> Self {
        Self {
            id,
            name: String::new(),
        }
    }
}

pub struct PrivateField {
    pub id: UserId,
    name: String,
}

impl PrivateField {
    pub fn new(id: UserId, name: String) -> Self {
        Self { id, name }
    }
}

pub struct RestrictedField {
    pub id: UserId,
    pub(crate) name: String,
}

impl RestrictedField {
    pub fn new(id: UserId, name: String) -> Self {
        Self { id, name }
    }
}

pub struct Newtype(pub String);

impl Newtype {
    pub fn new(value: String) -> Self {
        Self(value)
    }
}

pub struct Wrapper<T> {
    pub value: T,
}

impl<T> Wrapper<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

fn main() {}
