fn main() {
    if let Err(error) = cc_profile::run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
