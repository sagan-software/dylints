fn parse_port(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>().map_err(|error| error.to_string())
}

fn main() {
    let _port = parse_port("8080").unwrap();
}
