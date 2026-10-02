// compile-flags: --test

use test_case::test_case;

#[test_case(1_u8 ; "silently unregistered")]
#[test_case(3_u8 ; "second unregistered case")]
async fn unregistered(_value: u8) {}

#[test_case(2_u8 ; "registered with Tokio")]
#[tokio::test]
async fn registered(_value: u8) {}

#[test_case(4_u8 ; "synchronous")]
fn synchronous(_value: u8) {}

fn main() {}
