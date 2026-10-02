// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 ; "silently unregistered")]
async fn unregistered(_value: u8) {}

#[test_case(2_u8 ; "registered with Tokio")]
#[tokio::test]
async fn registered(_value: u8) {}

fn main() {}
