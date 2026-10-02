// normalize-stderr-test: "(\n)\n\z" -> "$1"

mod user_profile {
    use std::fmt;

    fn normalize_name(raw: &str) -> String {
        raw.trim().to_owned()
    }

    struct UserProfile {
        name: String,
    }

    impl fmt::Display for UserProfile {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(&self.name)
        }
    }
}

mod payment_status {
    const DEFAULT_OPEN: bool = true;

    enum PaymentStatus {
        Open,
        Closed,
    }

    fn default_status() -> PaymentStatus {
        if DEFAULT_OPEN {
            PaymentStatus::Open
        } else {
            PaymentStatus::Closed
        }
    }
}

mod raw_record {
    fn default_bits() -> u32 {
        0
    }

    union RawRecord {
        bits: u32,
    }

    fn read_bits(record: RawRecord) -> u32 {
        unsafe { record.bits }
    }

    fn default_record() -> RawRecord {
        RawRecord {
            bits: default_bits(),
        }
    }
}

mod request_id {
    fn fallback() -> u64 {
        0
    }

    type RequestId = u64;

    fn parse(raw: u64) -> RequestId {
        raw.max(fallback())
    }
}

pub mod public_order {
    pub fn helper() -> bool {
        true
    }

    pub struct PublicOrder {
        pub id: u64,
    }
}

mod parent_record {
    mod child_record {
        fn helper() -> bool {
            true
        }

        struct ChildRecord;
    }

    struct ParentRecord;
}

mod order_summary {
    use std::fmt;

    struct OrderSummary {
        total: u64,
    }

    impl fmt::Display for OrderSummary {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(formatter, "{}", self.total)
        }
    }
}

mod account_id {
    type AccountId = u64;

    fn parse(raw: u64) -> AccountId {
        raw
    }
}

mod no_representative_type {
    fn helper() -> bool {
        true
    }

    struct OtherType;
}

mod api_client {
    fn helper() -> bool {
        true
    }

    struct APIClient;
}

mod cfg_stripped_helper {
    #[cfg(any())]
    fn omitted_helper() -> bool {
        true
    }

    struct CfgStrippedHelper;
}

mod cfg_stripped_representative {
    fn helper() -> bool {
        true
    }

    #[cfg(any())]
    struct CfgStrippedRepresentative;
}

macro_rules! generated_helper {
    () => {
        fn helper() -> bool {
            true
        }
    };
}

mod macro_helper_before_type {
    generated_helper!();

    struct MacroHelperBeforeType;
}

macro_rules! generated_representative {
    () => {
        struct MacroGeneratedType;
    };
}

mod macro_generated_type {
    fn helper() -> bool {
        true
    }

    generated_representative!();
}

fn main() {}
