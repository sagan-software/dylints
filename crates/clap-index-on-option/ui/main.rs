use clap::Arg;

fn main() {
    let _ = Arg::new("bad").long("bad").index(1);
    let name = 'n';
    let _ = Arg::new("named").short(name).index(1);
    let _ = Arg::new("option").long("option");
    let _ = Arg::new("positional").index(1);
    let _ = Arg::new("removed").long("removed").long(None).index(1);
    let _ = Arg::new("reset").long("reset").index(1).index(None);
    let _ = Arg::new("cloned").long("cloned").clone().index(1);
}
