use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    let top = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    emacs_charsets::generate(&top)
}
