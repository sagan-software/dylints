fn main() {
    if runtime_flag() {
        panic!("missing configuration");
    }
    if runtime_flag() {
        todo!("wire later");
    }
    if runtime_flag() {
        unimplemented!("wire later");
    }
    if runtime_flag() {
        assert!(true);
    }
    if runtime_flag() {
        assert_eq!(1, 1);
    }
    if runtime_flag() {
        assert_ne!(1, 2);
    }
    if runtime_flag() {
        let value: Result<u8, &str> = Ok(1);
        let _value = value.unwrap();
    }
    if runtime_flag() {
        let value: Result<u8, &str> = Ok(1);
        let _value = value.expect("value exists");
    }
}

fn runtime_flag() -> bool {
    std::env::var_os("PANIC_IN_MAIN").is_some()
}

fn non_main_panics() {
    panic!("allowed outside main");
}

mod nested_non_entry_main {
    pub fn main() {
        let value: Result<u8, &str> = Ok(1);
        let _value = value.unwrap();
        assert_eq!(_value, 1);
        let value: Result<u8, &str> = Ok(1);
        let _value = value.expect("value exists");
    }
}

mod result_main {
    pub fn main() -> Result<(), Box<dyn std::error::Error>> {
        let value: Result<u8, &str> = Ok(1);
        let _value = value?;
        Ok(())
    }
}

mod qualified_result_main {
    pub fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

mod test_main {
    #[test]
    pub fn main() {
        assert!(true);
    }
}
