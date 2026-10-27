use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use regex::Regex;

const BLOCKS_SOURCES: [&str; 2] = ["Blocks.txt", "emoji-data.txt"];
const ZWJ_SOURCES: [&str; 4] = [
    "emoji-zwj-sequences.txt",
    "emoji-sequences.txt",
    "emoji-variation-sequences.txt",
    "emoji-test.txt",
];

fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l))
}

fn fields(line: &str) -> Vec<&str> {
    line.split([' ', '\t']).filter(|f| !f.is_empty()).collect()
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("valid regex")
}

fn decode_hex(s: &str) -> i64 {
    s.chars().fold(0, |n, c| {
        let d = if c.is_ascii_digit() {
            c as i64 - '0' as i64
        } else {
            match c.to_ascii_lowercase() {
                c @ 'a'..='f' => c as i64 - 'a' as i64 + 10,
                _ => 0,
            }
        };
        n * 16 + d
    })
}

struct Blocks {
    alias: HashMap<&'static str, &'static str>,
    rules: Vec<(Regex, Option<&'static str>)>,
    subs: Vec<(Regex, &'static str)>,
}

impl Blocks {
    fn new() -> Self {
        let alias = HashMap::from([
            ("ipa extensions", "phonetic"),
            ("letterlike symbols", "symbol"),
            ("number forms", "symbol"),
            ("miscellaneous technical", "symbol"),
            ("control pictures", "symbol"),
            ("optical character recognition", "symbol"),
            ("enclosed alphanumerics", "symbol"),
            ("box drawing", "symbol"),
            ("block elements", "symbol"),
            ("miscellaneous symbols", "symbol"),
            ("miscellaneous symbols supplement", "symbol"),
            ("symbols for legacy computing", "symbol"),
            ("symbols for legacy computing supplement", "symbol"),
            ("cjk strokes", "cjk-misc"),
            ("cjk symbols and punctuation", "cjk-misc"),
            ("halfwidth and fullwidth forms", "cjk-misc"),
            ("yijing hexagram symbols", "cjk-misc"),
            ("common indic number forms", "north-indic-number"),
            ("kanbun", "han"),
            ("vertical forms", "cjk-misc"),
        ]);
        let rules = [
            ("for symbols", Some("symbol")),
            (
                "latin|combining .* marks|spacing modifier|tone letters|alphabetic presentation",
                Some("latin"),
            ),
            ("cjk|enclosed ideograph|kangxi", Some("han")),
            ("arabic", Some("arabic")),
            ("^greek", Some("greek")),
            ("^coptic", Some("coptic")),
            ("cuneiform number", Some("cuneiform")),
            ("cuneiform", Some("cuneiform")),
            ("mathematical alphanumeric symbol", Some("mathematical")),
            (
                "punctuation|mathematical|arrows|currency|superscript|small form variants|geometric|dingbats|enclosed|alchemical|pictograph|emoticon|transport",
                Some("symbol"),
            ),
            ("canadian aboriginal", Some("canadian-aboriginal")),
            ("katakana|hiragana", Some("kana")),
            ("myanmar", Some("burmese")),
            ("hangul", Some("hangul")),
            ("khmer", Some("khmer")),
            ("braille", Some("braille")),
            ("^yi ", Some("yi")),
            ("surrogates|private use|variation selectors", None),
            ("^(specials|tags)$", None),
            ("linear b", Some("linear-b")),
            ("aramaic", Some("aramaic")),
            ("rumi num", Some("arabic")),
            ("duployan|shorthand", Some("duployan-shorthand")),
            ("sutton signwriting", Some("sutton-sign-writing")),
            ("sinhala archaic number", Some("sinhala")),
            ("tangut components", Some("tangut")),
            ("jurchen radicals", Some("jurchen")),
        ]
        .into_iter()
        .map(|(p, a)| (re(p), a))
        .collect();
        let subs = [
            ("^small ", ""),
            (" (extended|extensions*|supplement).*", ""),
            ("numbers", "number"),
            ("numerals", "numeral"),
            ("symbols", "symbol"),
            ("forms$", "form"),
            ("tiles$", "tile"),
            ("^new ", ""),
            (
                " (characters|hieroglyphs|cursive|hieroglyph format controls)$",
                "",
            ),
        ]
        .into_iter()
        .map(|(p, r)| (re(p), r))
        .collect();
        Blocks { alias, rules, subs }
    }

    fn name2alias(&self, name: &str) -> Option<String> {
        let mut name = name.to_lowercase();
        if let Some(a) = self.alias.get(name.as_str()) {
            return Some((*a).to_string());
        }
        for (pattern, result) in &self.rules {
            if pattern.is_match(&name) {
                return result.map(str::to_string);
            }
        }
        for (pattern, replacement) in &self.subs {
            name = pattern.replacen(&name, 1, *replacement).into_owned();
        }
        Some(name.replace(' ', "-"))
    }
}

#[derive(Default)]
struct Table {
    start: Vec<String>,
    end: Vec<String>,
    name: Vec<String>,
    alt: Vec<String>,
}

fn set(v: &mut Vec<String>, i: usize, s: &str) {
    if v.len() <= i {
        v.resize(i + 1, String::new());
    }
    v[i] = s.to_string();
}

fn get(v: &[String], i: usize) -> &str {
    v.get(i).map_or("", String::as_str)
}

pub fn charscript_el(blocks: &str, emoji_data: &str) -> String {
    let b = Blocks::new();
    let mut t = Table::default();
    let mut i: usize = 0;
    let hex_line = re("^[0-9A-F]");
    for line in lines(blocks) {
        if !hex_line.is_match(line) {
            continue;
        }
        let f = fields(line);
        let first = f.first().copied().unwrap_or("");
        let sep = first.find("..").map_or(0, |p| p + 1);
        let len = first.len();
        let s = first.get(..sep.saturating_sub(1)).unwrap_or("");
        let e = first
            .get(sep + 1..(sep + 1 + len.saturating_sub(sep + 2)).min(len))
            .unwrap_or("");
        let rest = f.get(1..).unwrap_or(&[]).join(" ");
        i += 1;
        set(&mut t.start, i, if s == "0080" { "00A0" } else { s });
        set(&mut t.end, i, e);
        set(&mut t.name, i, &rest);
        if get(&t.start, i) == "3300" {
            set(&mut t.end, i, "3357");
            set(&mut t.name, i, "Katakana");
            set(&mut t.alt, i, "kana");
            i += 1;
            set(&mut t.start, i, "3358");
            set(&mut t.end, i, "33FF");
            set(&mut t.name, i, "CJK Compatibility");
        } else if get(&t.start, i) == "FFF0" {
            set(&mut t.start, i, "FFFC");
            set(&mut t.end, i, "FFFD");
            set(&mut t.name, i, "Replacement Symbols");
            set(&mut t.alt, i, "symbol");
            i += 1;
            set(&mut t.start, i, "FFFE");
            set(&mut t.end, i, "FFFF");
            set(&mut t.name, i, "Specials");
        }
        let alt = b.name2alias(get(&t.name, i)).unwrap_or_default();
        set(&mut t.alt, i, &alt);
        if alt.is_empty() {
            i -= 1;
            continue;
        }
        if get(&t.alt, i) == get(&t.alt, i - 1)
            && decode_hex(get(&t.start, i)) == 1 + decode_hex(get(&t.end, i - 1))
        {
            let end = get(&t.end, i).to_string();
            set(&mut t.end, i - 1, &end);
            let name = format!("{}, {}", get(&t.name, i - 1), get(&t.name, i));
            set(&mut t.name, i - 1, &name);
            i -= 1;
        }
        let splits: &[(&str, &str, &str)] = match get(&t.start, i) {
            "0370" => &[
                ("03E1", "", ""),
                ("03E2", "03EF", "coptic"),
                ("03F0", "03FF", "greek"),
            ],
            "FB00" => &[
                ("FB06", "", ""),
                ("FB13", "FB17", "armenian"),
                ("FB1D", "FB4F", "hebrew"),
            ],
            "FF00" => &[
                ("FF60", "", ""),
                ("FF61", "FF9F", "kana"),
                ("FFA0", "FFDF", "hangul"),
                ("FFE0", "FFEF", "cjk-misc"),
            ],
            _ => &[],
        };
        for (k, &(a, z, alt)) in splits.iter().enumerate() {
            if k == 0 {
                set(&mut t.end, i, a);
            } else {
                i += 1;
                set(&mut t.start, i, a);
                set(&mut t.end, i, z);
                set(&mut t.alt, i, alt);
            }
        }
    }
    let emoji_line = re("^[0-9A-F].*; Emoji_Presentation ");
    for line in lines(emoji_data) {
        if !emoji_line.is_match(line) {
            continue;
        }
        let f = fields(line);
        let first = f.first().copied().unwrap_or("");
        let (s, e) = match first.find("..") {
            Some(p) => (&first[..p], &first[p + 2..]),
            None => (first, first),
        };
        i += 1;
        set(&mut t.start, i, s);
        set(&mut t.end, i, e);
        set(&mut t.alt, i, "emoji");
        set(&mut t.name, i, "Autogenerated emoji");
    }
    i += 1;
    set(&mut t.start, i, "FE0F");
    set(&mut t.end, i, "FE0F");
    set(&mut t.alt, i, "emoji");
    set(&mut t.name, i, "Autogenerated emoji (override)");
    let mut out = String::new();
    out.push_str(";;; charscript.el --- character script table  -*- lexical-binding:t -*-\n");
    out.push_str(";;; Automatically generated from admin/unidata/{Blocks,emoji-data}.txt\n");
    out.push_str("(let (script-list)\n");
    out.push_str("  (dolist (elt '(\n");
    for j in 1..=i {
        let (start, end, alt, name) = (
            get(&t.start, j),
            get(&t.end, j),
            get(&t.alt, j),
            get(&t.name, j),
        );
        out.push_str(&format!("    (#x{start} #x{end} {alt})"));
        let lname = name.to_lowercase();
        if !name.is_empty() && alt != lname && (!alt.contains('-') || lname.contains("supplement"))
        {
            out.push_str(&format!(" ; {name}"));
        }
        out.push('\n');
    }
    out.push_str("    ))\n");
    out.push_str("    (set-char-table-range char-script-table\n");
    out.push_str("\t\t\t  (cons (car elt) (nth 1 elt)) (nth 2 elt))\n");
    out.push_str("    (or (memq (nth 2 elt) script-list)\n");
    out.push_str("\t(setq script-list (cons (nth 2 elt) script-list))))\n");
    out.push_str("  (set-char-table-extra-slot char-script-table 0 (nreverse script-list)))\n");
    out.push_str("\n(map-char-table\n");
    out.push_str(" (lambda (ch script)\n");
    out.push_str("   (and (eq script 'symbol)\n");
    out.push_str("\t(modify-category-entry ch ?5)))\n");
    out.push_str(" char-script-table)\n");
    out.push_str("\n(provide 'charscript)\n");
    out
}

pub fn emoji_zwj_el(sources: &[&str]) -> String {
    let sequence = re("^[0-9A-F].*; RGI_Emoji_(ZWJ|Modifier)_Sequence");
    let qualified = re("^[0-9A-F].*; (un|minimally-)qualified");
    let zwj = re(".+200D.+;");
    let unqualified = re("; unqualified");
    let tail = re(" *;.*");
    let style = re("^[0-9A-F]+ FE0F *; *emoji style;");
    let mut order: Vec<String> = Vec::new();
    let mut vec: HashMap<String, String> = HashMap::new();
    let mut tails: Vec<String> = Vec::new();
    let mut tty: Vec<String> = Vec::new();
    for source in sources {
        for line in lines(source) {
            let mut record = line.to_string();
            if sequence.is_match(&record) || (qualified.is_match(&record) && zwj.is_match(&record))
            {
                let is_unqualified = unqualified.is_match(&record);
                record = tail.replacen(&record, 1, "").into_owned();
                let elts = fields(&record);
                let key = elts.first().copied().unwrap_or("").to_string();
                let entry = match vec.get_mut(&key) {
                    Some(v) => {
                        v.push('\n');
                        v
                    }
                    None => {
                        order.push(key.clone());
                        vec.entry(key.clone()).or_default()
                    }
                };
                entry.push('"');
                for elt in &elts {
                    entry.push_str(&format!("\\N{{U+{elt}}}"));
                }
                entry.push('"');
                if is_unqualified && elts.get(1) == Some(&"200D") {
                    tails.push(elts.get(2).copied().unwrap_or("").to_string());
                }
            }
            if style.is_match(&record) {
                tty.push(fields(&record).first().copied().unwrap_or("").to_string());
            }
        }
    }
    let triggers = [
        "261D", "26F9", "270C", "270D", "2764", "1F3CB", "1F3CC", "1F3F3", "1F3F4", "1F441",
        "1F574", "1F575", "1F590", "20E3", "26D3",
    ];
    let mut out = String::new();
    out.push_str(
        ";;; emoji-zwj.el --- emoji zwj character composition table  -*- lexical-binding:t -*-\n",
    );
    out.push_str(";;; Automatically generated from admin/unidata/emoji-{zwj-,}sequences.txt\n");
    out.push_str("(eval-when-compile (require 'regexp-opt))\n");
    let codepoint_list = |out: &mut String, var: &str, items: &[&str]| {
        out.push_str(&format!("(setq {var}\n'(\n"));
        for item in items {
            out.push_str(&format!("?\\N{{U+{item}}}\n"));
        }
        out.push_str("))\n");
    };
    codepoint_list(
        &mut out,
        "auto-composition-emoji-eligible-codepoints",
        &triggers,
    );
    let tty: Vec<&str> = tty.iter().map(String::as_str).collect();
    codepoint_list(
        &mut out,
        "auto-composition-emoji-tty-eligible-codepoints",
        &tty,
    );
    let tails: Vec<&str> = tails.iter().map(String::as_str).collect();
    codepoint_list(
        &mut out,
        "auto-composition-emoji-unqualified-tail-initials",
        &tails,
    );
    out.push_str("(dolist (elt (eval-when-compile `(\n");
    for elt in &order {
        out.push_str(&format!("(#x{elt} .\n,(regexp-opt\n'(\n{}\n", vec[elt]));
        out.push_str(&format!(
            "\"\\N{{U+{elt}}}\\N{{U+FE0E}}\"\n\"\\N{{U+{elt}}}\\N{{U+FE0F}}\"\n)))\n"
        ));
    }
    out.push_str(")))\n");
    out.push_str("  (set-char-table-range composition-function-table\n");
    out.push_str("                        (car elt)\n");
    out.push_str(
        "                        (nconc (char-table-range composition-function-table (car elt))\n",
    );
    out.push_str("                               (list (vector (cdr elt)\n");
    out.push_str("                                             0\n");
    out.push_str("                                             #'compose-gstring-and-emoji)))))\n");
    out.push_str(";; The following two blocks are derived by hand from emoji-sequences.txt\n");
    out.push_str(
        ";; FIXME: add support for Emoji_Keycap_Sequence once we learn how to respect FE0F/VS-16\n",
    );
    out.push_str(";; for ASCII characters.\n");
    out.push_str(";; Flags\n");
    out.push_str("(set-char-table-range composition-function-table\n");
    out.push_str("                      '(#x1F1E6 . #x1F1FF)\n");
    out.push_str("                      (nconc (char-table-range composition-function-table '(#x1F1E6 . #x1F1FF))\n");
    out.push_str(r#"                             (list (vector "[\U0001F1E6-\U0001F1FF][\U0001F1E6-\U0001F1FF]""#);
    out.push('\n');
    out.push_str("                                           0\n");
    out.push_str("                                           #'compose-gstring-and-emoji))))\n");
    out.push_str(";; UK Flags\n");
    out.push_str("(set-char-table-range composition-function-table\n");
    out.push_str("                      #x1F3F4\n");
    out.push_str(
        "                      (nconc (char-table-range composition-function-table #x1F3F4)\n",
    );
    out.push_str(r#"                             (list (vector "\U0001F3F4\U000E0067\U000E0062\\(?:\U000E0065\U000E006E\U000E0067\\|\U000E0073\U000E0063\U000E0074\\|\U000E0077\U000E006C\U000E0073\\)\U000E007F""#);
    out.push('\n');
    out.push_str("                                           0\n");
    out.push_str("                                           #'compose-gstring-and-emoji))))\n");
    out.push_str("\n(provide 'emoji-zwj)");
    out
}

fn read(dir: &Path, name: &str) -> io::Result<String> {
    Ok(String::from_utf8_lossy(&fs::read(dir.join(name))?).into_owned())
}

pub fn generate(top: &Path) -> io::Result<()> {
    let unidata = top.join("admin/unidata");
    let out = top.join("lisp/international");
    let charscript = charscript_el(
        &read(&unidata, "Blocks.txt")?,
        &read(&unidata, "emoji-data.txt")?,
    );
    fs::write(out.join("charscript.el"), charscript)?;
    let zwj: Vec<String> = ZWJ_SOURCES
        .iter()
        .map(|name| read(&unidata, name))
        .collect::<io::Result<_>>()?;
    let zwj: Vec<&str> = zwj.iter().map(String::as_str).collect();
    fs::write(out.join("emoji-zwj.el"), emoji_zwj_el(&zwj))?;
    Ok(())
}

fn modified(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

pub fn up_to_date(top: &Path) -> bool {
    let unidata = top.join("admin/unidata");
    let out = top.join("lisp/international");
    let check = |target: &str, sources: &[&str], script: &str| {
        let Some(t) = modified(&out.join(target)) else {
            return false;
        };
        sources
            .iter()
            .chain(std::iter::once(&script))
            .all(|s| modified(&unidata.join(s)).is_some_and(|m| m <= t))
    };
    check("charscript.el", &BLOCKS_SOURCES, "blocks.awk")
        && check("emoji-zwj.el", &ZWJ_SOURCES, "emoji-zwj.awk")
}
