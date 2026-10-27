use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let top = manifest.join("../..");
    println!("cargo:rerun-if-changed=../../admin/charsets/glibc");
    println!("cargo:rerun-if-changed=../../admin/charsets/mapfiles");
    println!("cargo:rerun-if-changed=../../admin/charsets/charsets.stamp");
    println!("cargo:rerun-if-changed=../../admin/unidata");
    println!("cargo:rerun-if-changed=../../lisp/international/charscript.el");
    println!("cargo:rerun-if-changed=../../lisp/international/emoji-zwj.el");
    if !emacs_charsets::up_to_date(&top) {
        emacs_charsets::generate(&top).expect("generate charset maps");
    }
    if !emacs_unidata::up_to_date(&top) {
        emacs_unidata::generate(&top).expect("generate unidata Lisp files");
    }
}
