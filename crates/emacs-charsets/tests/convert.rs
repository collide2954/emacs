use std::path::PathBuf;

use emacs_charsets as cs;

fn lines(text: &str) -> Vec<Vec<u8>> {
    cs::read_lines(text.as_bytes())
}

fn text(lines: &[Vec<u8>]) -> String {
    let mut out = String::new();
    for l in lines {
        out.push_str(std::str::from_utf8(l).unwrap());
        out.push('\n');
    }
    out
}

fn top() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn read_lines_strips_carriage_returns() {
    assert_eq!(lines("a\r\nb\n"), vec![b"a".to_vec(), b"b".to_vec()]);
    assert!(lines("").is_empty());
    assert_eq!(lines("x"), vec![b"x".to_vec()]);
}

#[test]
fn compact_merges_runs() {
    let input = lines("# head\n0x00 0x0000\n0x01 0x0001\n0x02 0x0002\n0xA1 0x0104\n0xA2 0x02D8\n");
    assert_eq!(
        text(&cs::compact(&input)),
        "# head\n0x00-0x02 0x0000\n0xA1 0x0104\n0xA2 0x02D8\n"
    );
}

#[test]
fn compact_uses_wide_codes_above_255() {
    let input = lines("0x2121 0x3000\n0x2122 0x3001\n0x2124 0xFF0C\n");
    assert_eq!(
        text(&cs::compact(&input)),
        "0x2121-0x2122 0x3000\n0x2124 0xFF0C\n"
    );
}

#[test]
fn big5_converts_to_iso_layout() {
    let input = lines("0xa140 0x3000\n0xa17e 0xFE5A\n0xc940 0x4E42\n");
    assert_eq!(
        text(&cs::big5(&input)),
        "0x2121 0x3000\n0x215F 0xFE5A\n0x2121 0x4E42\n"
    );
}

#[test]
fn kuten_offsets_rows_and_cells() {
    let input = lines("# c\n0x0101 0x3000\n0x1601 0x4E9C\n");
    assert_eq!(text(&cs::kuten(&input)), "0x2121 0x3000\n0x3021 0x4E9C\n");
}

#[test]
fn cp932_tags_and_sorts() {
    let input = lines("0x8140 0x3000\n0x8740 0x2460\n0xED40 0x7E8A\n0xFA40 0x2170\n");
    let out = cs::sort_numeric_field4(&cs::cp932(&input));
    let s = text(&out);
    assert!(s.starts_with("0x8140 0x3000 # 0 2121\n0x8740 0x2460 # 1 2D21\n0xFA40 0x2170 # 2\n0xED40 0x7E8A # 3 7921\n"));
    assert!(s.ends_with("0xF9FC 0xE757 # 4\n"));
}

#[test]
fn gb180302_compresses_index_runs() {
    let input = lines("0x8140 0x4E02\n0x8141 0x4E04\n0x8142 0x4E05\n0x8143 0x4E06\n");
    assert_eq!(
        text(&cs::gb180302(&input)),
        "0x8140 0x4E02\n0x8141-0x8143 0x4E04\n"
    );
}

#[test]
fn gb180304_lists_unmapped_runs() {
    let input = lines("# h\n0x8140-0x8141 0x0080\n");
    assert_eq!(
        text(&cs::gb180304(&input)),
        "# h\n0x81308130-0x8630A437\t0x0082\n"
    );
}

#[test]
fn cp51932_el_selects_nec_rows() {
    let input = lines("# Generated\n0x8740 0x2460 # 1 2D21\n0x8140 0x3000 # 0 2121\n");
    let el = String::from_utf8(cs::cp51932_el(&input)).unwrap();
    assert!(el.contains("'(;JISEXT<->UNICODE\n\t (#x2D21 . #x2460))))\n"));
    assert!(!el.contains("#x2121"));
}

#[test]
fn eucjp_ms_el_tracks_sections() {
    let input = lines(
        "% JIS X 0208\n<U3000>     /xa1/xa1  X\n<U2460>     /xad/xa1  CIRCLED\n<U2461>     /xad/xa2  CIRCLED\n% JIS X 0212\n<U2170>     /x8f/xf3/xf3  ROMAN\nEND CHARMAP\n",
    );
    let el = String::from_utf8(cs::eucjp_ms_el(&input)).unwrap();
    assert!(el.contains("\n\t (#xada1 . #x2460)\n\t (#xada2 . #x2461)\n\t (#xf3f3 #x2170))))\n"));
    assert!(!el.contains("#xa1a1"));
}

#[test]
fn iso_8859_2_map_matches_glibc() {
    let out = cs::glibc_map(
        &top(),
        "ISO-8859-2.gz",
        "^<.*[ \t]/x",
        None,
        cs::Format::Glibc1,
        cs::Awk::Compact,
    )
    .unwrap();
    let s = text(&out);
    assert!(s.starts_with("# Generated from ISO-8859-2 in localedata/charmaps of glibc\n0x00-0xA0 0x0000\n0xA1 0x0104\n0xA2 0x02D8\n"));
}
