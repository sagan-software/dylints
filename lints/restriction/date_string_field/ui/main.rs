mod chrono {
    pub struct NaiveDate;
}

mod time {
    pub struct Date;
}

struct Date;

struct Profile<'a> {
    date_of_birth: String,
    dob: &'a str,
    birth_date: String,
    birthdate: String,
    created_date: String,
    updated_date: &'a str,
    expires_date: String,
    renewal_date: String,
    display_name: String,
    candidate: String,
    created_at: String,
    semantic_birth_date: chrono::NaiveDate,
    custom_birth_date: Date,
    time_birth_date: time::Date,
}

fn main() {}
