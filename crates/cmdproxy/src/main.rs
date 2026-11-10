#[cfg(windows)]
fn main() {
    std::process::exit(cmdproxy::run());
}

#[cfg(not(windows))]
fn main() {}
