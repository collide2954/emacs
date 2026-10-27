use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use flate2::read::MultiGzDecoder;
use regex::bytes::Regex;

pub type Line = Vec<u8>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Glibc1,
    Glibc2,
    Glibc27,
    Czyborra,
    Iana,
    Unicode,
    Unicode2,
    Yasuoka,
    KanjiDatabase,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Awk {
    None,
    Compact,
    Cp932,
    Gb180302,
    Kuten,
}

pub fn read_lines(bytes: &[u8]) -> Vec<Line> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    body.split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l).to_vec())
        .collect()
}

fn join(lines: &[Line]) -> Vec<u8> {
    let mut out = Vec::new();
    for l in lines {
        out.extend_from_slice(l);
        out.push(b'\n');
    }
    out
}

fn re(pattern: &str) -> Regex {
    Regex::new(&format!("(?-u){pattern}")).expect("valid pattern")
}

fn fields(line: &[u8]) -> Vec<&[u8]> {
    line.split(|&b| b == b' ' || b == b'\t')
        .filter(|f| !f.is_empty())
        .collect()
}

fn field<'a>(f: &[&'a [u8]], n: usize) -> &'a [u8] {
    f.get(n - 1).copied().unwrap_or(b"")
}

fn substr(s: &[u8], start: usize, len: Option<usize>) -> &[u8] {
    let from = (start - 1).min(s.len());
    let to = len.map_or(s.len(), |l| (from + l).min(s.len()));
    &s[from..to]
}

fn s(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn hex_digit(c: u8, lowercase: bool) -> Option<i64> {
    match c {
        b'0'..=b'9' => Some((c - b'0') as i64),
        b'A'..=b'F' => Some((c - b'A' + 10) as i64),
        b'a'..=b'f' if lowercase => Some((c - b'a' + 10) as i64),
        _ => None,
    }
}

fn compact_hex(text: &[u8], idx: usize) -> i64 {
    let mut n = 0;
    for &c in text.iter().skip(idx - 1) {
        match hex_digit(c, true) {
            Some(d) => n = n * 16 + d,
            None => break,
        }
    }
    n
}

fn awk_hex(text: &[u8], lowercase: bool) -> i64 {
    text.iter()
        .fold(0, |n, &c| n * 16 + hex_digit(c, lowercase).unwrap_or(0))
}

fn awk_num(text: &[u8]) -> i64 {
    let t: Vec<u8> = text
        .iter()
        .copied()
        .skip_while(|c| *c == b' ' || *c == b'\t')
        .collect();
    let (neg, digits) = match t.first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, &t[..]),
    };
    let n = digits
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .fold(0i64, |n, c| n * 10 + (c - b'0') as i64);
    if neg { -n } else { n }
}

fn compact_line(from: i64, to: i64, unicode: i64, end: bool) -> Option<String> {
    let wide = to >= 256;
    if from == to {
        Some(if wide {
            format!("0x{from:04X} 0x{unicode:04X}")
        } else {
            format!("0x{from:02X} 0x{unicode:04X}")
        })
    } else if from < to || end {
        Some(if wide {
            format!("0x{from:04X}-0x{to:04X} 0x{unicode:04X}")
        } else {
            format!("0x{from:02X}-0x{to:02X} 0x{unicode:04X}")
        })
    } else {
        None
    }
}

pub fn compact(input: &[Line]) -> Vec<Line> {
    let mut out = Vec::new();
    let (mut from_code, mut to_code, mut to_unicode, mut from_unicode) = (0i64, -1i64, 0i64, 0i64);
    for line in input {
        if line.first() == Some(&b'#') {
            out.push(line.clone());
            continue;
        }
        let f = fields(line);
        let code = compact_hex(field(&f, 1), 3);
        let unicode = compact_hex(field(&f, 2), 3);
        if code == to_code + 1 && unicode == to_unicode + 1 {
            to_code += 1;
            to_unicode += 1;
        } else {
            if let Some(l) = compact_line(from_code, to_code, from_unicode, false) {
                out.push(l.into_bytes());
            }
            from_code = code;
            to_code = code;
            from_unicode = unicode;
            to_unicode = unicode;
        }
    }
    if let Some(l) = compact_line(from_code, to_code, from_unicode, true) {
        out.push(l.into_bytes());
    }
    out
}

pub fn big5(input: &[Line]) -> Vec<Line> {
    input
        .iter()
        .map(|line| {
            let f = fields(line);
            let big5 = awk_hex(field(&f, 1), true);
            let (b0, b1) = (big5 / 256, big5 % 256);
            let mut idx = if b1 < 127 {
                (b0 - 161) * 157 + (b1 - 64)
            } else {
                (b0 - 161) * 157 + (b1 - 98)
            };
            if b0 >= 201 {
                idx -= 6280;
            }
            let code = (idx / 94 + 33) * 256 + (idx % 94 + 33);
            format!("0x{code:04X} {}", s(field(&f, 2))).into_bytes()
        })
        .collect()
}

pub fn kuten(input: &[Line]) -> Vec<Line> {
    input
        .iter()
        .filter(|line| line.first().is_some_and(|c| c.is_ascii_digit()))
        .map(|line| {
            let f = fields(line);
            let first = field(&f, 1);
            let ku = awk_num(substr(first, 3, Some(2))) + 32;
            let ten = awk_num(substr(first, 5, Some(2))) + 32;
            format!("0x{ku:02X}{ten:02X} {}", s(field(&f, 2))).into_bytes()
        })
        .collect()
}

fn sjis_to_jis(code: i64) -> (i64, i64) {
    let (s1, s2) = (code / 256, code % 256);
    if s2 >= 159 {
        let j1 = if s1 >= 224 {
            s1 * 2 - 352
        } else {
            s1 * 2 - 224
        };
        (j1, s2 - 126)
    } else {
        let j1 = if s1 >= 224 {
            s1 * 2 - 353
        } else {
            s1 * 2 - 225
        };
        (j1, if s2 >= 127 { s2 - 32 } else { s2 - 31 })
    }
}

pub fn cp932(input: &[Line]) -> Vec<Line> {
    let mut out = Vec::new();
    for line in input {
        let text = s(line);
        if line.starts_with(b"0x8") || line.starts_with(b"0x9") || line.starts_with(b"0xE") {
            let f = fields(line);
            let sjis = awk_hex(substr(field(&f, 1), 3, Some(4)), false);
            let (j1, j2) = sjis_to_jis(sjis);
            let ku = j1 - 32;
            let key = if ku == 13 {
                1
            } else if (89..=92).contains(&ku) {
                3
            } else {
                0
            };
            out.push(format!("{text} # {key} {j1:02X}{j2:02X}").into_bytes());
        } else if line.starts_with(b"0xF") {
            out.push(format!("{text} # 2").into_bytes());
        } else {
            out.push(line.clone());
        }
    }
    let mut code = 0xE000;
    for i in 240..250 {
        for j in (64..=126).chain(128..=158).chain(159..=252) {
            out.push(format!("0x{i:02X}{j:02X} 0x{code:04X} # 4").into_bytes());
            code += 1;
        }
    }
    out
}

pub fn sort_numeric_field4(input: &[Line]) -> Vec<Line> {
    let mut keyed: Vec<(i64, Line)> = input
        .iter()
        .map(|l| (awk_num(field(&fields(l), 4)), l.clone()))
        .collect();
    keyed.sort();
    keyed.into_iter().map(|(_, l)| l).collect()
}

fn gb_to_index(gb: i64) -> i64 {
    (gb / 256 - 129) * 191 + gb % 256 - 64
}

fn index_to_gb2(idx: i64) -> i64 {
    (idx / 191 + 129) * 256 + idx % 191 + 64
}

pub fn gb180302(input: &[Line]) -> Vec<Line> {
    let mut out = Vec::new();
    let (mut from_gb, mut to_gb, mut to_unicode, mut from_unicode) = (0i64, -1i64, 0i64, 0i64);
    for line in input {
        if line.first() == Some(&b'#') {
            out.push(line.clone());
            continue;
        }
        let f = fields(line);
        let gb = gb_to_index(awk_hex(substr(field(&f, 1), 3, Some(4)), true));
        let unicode = awk_hex(substr(field(&f, 2), 3, None), true);
        if gb == to_gb + 1 && unicode == to_unicode + 1 {
            to_gb += 1;
            to_unicode += 1;
        } else if gb > to_gb {
            let u = if from_unicode <= 65535 {
                format!("0x{from_unicode:04X}")
            } else {
                format!("0x{from_unicode:08X}")
            };
            if from_gb == to_gb {
                out.push(format!("0x{:04X} {u}", index_to_gb2(from_gb)).into_bytes());
            } else if from_gb < to_gb {
                out.push(
                    format!(
                        "0x{:04X}-0x{:04X} {u}",
                        index_to_gb2(from_gb),
                        index_to_gb2(to_gb)
                    )
                    .into_bytes(),
                );
            }
            from_gb = gb;
            to_gb = gb;
            from_unicode = unicode;
            to_unicode = unicode;
        }
    }
    if from_gb <= to_gb {
        out.push(
            format!(
                "0x{:04X}-0x{:04X} 0x{from_unicode:04X}",
                index_to_gb2(from_gb),
                index_to_gb2(to_gb)
            )
            .into_bytes(),
        );
    }
    out
}

fn index_to_gb4(idx: i64) -> String {
    let b3 = idx % 10 + 48;
    let idx = idx / 10;
    let b2 = idx % 126 + 129;
    let idx = idx / 126;
    let b1 = idx % 10 + 48;
    let b0 = idx / 10 + 129;
    format!("{b0:02X}{b1:02X}{b2:02X}{b3:02X}")
}

pub fn gb180304(input: &[Line]) -> Vec<Line> {
    let mut out = Vec::new();
    let mut table: HashSet<i64> = HashSet::new();
    let range = re("0x....-0x....");
    for line in input {
        if line.first() == Some(&b'#') {
            out.push(line.clone());
            continue;
        }
        let f = fields(line);
        let unicode = awk_hex(substr(field(&f, 2), 3, Some(4)), true);
        if range.is_match(line) {
            let first = field(&f, 1);
            let mut gb_from = gb_to_index(awk_hex(substr(first, 3, Some(4)), true));
            let gb_to = gb_to_index(awk_hex(substr(first, 10, Some(4)), true));
            let mut u = unicode;
            while gb_from <= gb_to {
                table.insert(u);
                u += 1;
                gb_from += 1;
            }
        } else {
            table.insert(unicode);
        }
    }
    let (mut from_gb, mut to_gb, mut from_i) = (-1i64, 0i64, 0i64);
    table.insert(65536);
    for i in 128..=65536 {
        if !table.contains(&i) {
            if !(55296..57344).contains(&i) {
                if from_gb < 0 {
                    from_gb = to_gb;
                    from_i = i;
                }
                to_gb += 1;
            }
        } else if from_gb >= 0 {
            if from_gb + 1 == to_gb {
                out.push(format!("0x{}\t\t0x{from_i:04X}", index_to_gb4(from_gb)).into_bytes());
            } else {
                out.push(
                    format!(
                        "0x{}-0x{}\t0x{from_i:04X}",
                        index_to_gb4(from_gb),
                        index_to_gb4(to_gb - 1)
                    )
                    .into_bytes(),
                );
            }
            from_gb = -1;
        }
    }
    out
}

pub fn cp51932_el(input: &[Line]) -> Vec<u8> {
    let mut out = String::new();
    out.push_str(";;; cp51932.el -- translation table for CP51932  -*- lexical-binding:t -*-\n");
    out.push_str(";;; Automatically generated from CP932-2BYTE.map\n");
    out.push_str("(let ((map\n");
    out.push_str("       '(;JISEXT<->UNICODE");
    let tag = re("# [13]");
    for line in input {
        if tag.is_match(line) {
            let f = fields(line);
            out.push_str(&format!(
                "\n\t (#x{} . #x{})",
                s(field(&f, 5)),
                s(substr(field(&f, 2), 3, Some(4)))
            ));
        }
    }
    for l in [
        ")))",
        "  (setq map (mapcar (lambda (x)",
        "\t\t      (cons (decode-char 'japanese-jisx0208 (car x))",
        "\t\t\t    (cdr x)))",
        "\t\t    map))",
        "  (define-translation-table 'cp51932-decode map)",
        "  (mapc (lambda (x)",
        "\t  (let ((tmp (car x)))",
        "\t    (setcar x (cdr x)) (setcdr x tmp)))",
        "\tmap)",
        "  (define-translation-table 'cp51932-encode map))",
        "",
        "(provide 'cp51932)",
    ] {
        out.push_str(l);
        out.push('\n');
    }
    out.into_bytes()
}

fn regex_fields(line: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < line.len() {
        if line[i] == b' ' || line[i] == b'\t' {
            out.push(&line[start..i]);
            while i < line.len() && (line[i] == b' ' || line[i] == b'\t') {
                i += 1;
            }
            start = i;
        } else {
            i += 1;
        }
    }
    out.push(&line[start..]);
    out
}

fn is_ucs_tag(text: &[u8]) -> bool {
    text.len() >= 7
        && text.starts_with(b"<U")
        && text[2..6]
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        && text[6] == b'>'
}

pub fn eucjp_ms_el(input: &[Line]) -> Vec<u8> {
    const JISX0208_FROM1: &[u8] = b"/xad/xa1";
    const JISX0208_TO1: &[u8] = b"/xad/xfc";
    const JISX0208_FROM2: &[u8] = b"/xf5/xa1";
    const JISX0212_FROM: &[u8] = b"/x8f/xf3/xf3";
    let mut out = String::new();
    out.push_str(";;; eucjp-ms.el --- translation table for eucJP-ms  -*- lexical-binding:t -*-\n");
    out.push_str(";;; Automatically generated from /usr/share/i18n/charmaps/EUC-JP-MS.gz\n");
    out.push_str("(let ((map\n");
    out.push_str("       '(;JISEXT<->UNICODE\n");
    let mut state = 0;
    for line in input {
        if line.starts_with(b"% JIS X 0208") {
            state = 1;
            continue;
        }
        if line.starts_with(b"% JIS X 0212") {
            state = 3;
            continue;
        }
        if line.starts_with(b"END CHARMAP") {
            state = 0;
            continue;
        }
        let f = regex_fields(line);
        let first = field(&f, 1);
        let mut entries = Vec::new();
        if is_ucs_tag(line) && state > 0 {
            entries.push(substr(first, 3, Some(4)).to_vec());
        }
        if line.starts_with(b"%IRREVERSIBLE%") && is_ucs_tag(&line[14..]) && state > 0 {
            entries.push(substr(first, 17, Some(4)).to_vec());
        }
        for unicode in entries {
            let second = field(&f, 2);
            if state == 1 {
                if second == JISX0208_FROM1 || second == JISX0208_FROM2 {
                    state = 2;
                }
            } else if state == 3 && second == JISX0212_FROM {
                state = 4;
            }
            if state == 2 {
                let jis = s(second).replace("/x", "");
                out.push_str(&format!("\n\t (#x{jis} . #x{})", s(&unicode)));
                if second == JISX0208_TO1 {
                    state = 1;
                }
            } else if state == 4 {
                let jis = s(substr(second, 5, Some(8))).replace("/x", "");
                out.push_str(&format!("\n\t (#x{jis} #x{})", s(&unicode)));
            }
        }
    }
    for l in [
        ")))",
        "  (setq map",
        "    (mapcar",
        "\t(lambda (x)",
        "\t    (let ((code (logand (car x) #x7F7F)))",
        "\t      (if (integerp (cdr x))",
        "\t\t  (cons (decode-char 'japanese-jisx0208 code) (cdr x))",
        "\t\t(cons (decode-char 'japanese-jisx0212 code)",
        "\t\t      (cadr x)))))",
        "\tmap))",
        "  (define-translation-table 'eucjp-ms-decode map)",
        "  (mapc (lambda (x)",
        "\t    (let ((tmp (car x)))",
        "\t      (setcar x (cdr x)) (setcdr x tmp)))",
        "\tmap)",
        "  (define-translation-table 'eucjp-ms-encode map))",
        "",
        "(provide 'eucjp-ms)",
    ] {
        out.push_str(l);
        out.push('\n');
    }
    out.into_bytes()
}

fn read_source(path: &Path) -> io::Result<Vec<Line>> {
    let bytes = fs::read(path)?;
    if path.extension().is_some_and(|e| e == "gz") {
        let mut out = Vec::new();
        MultiGzDecoder::new(&bytes[..]).read_to_end(&mut out)?;
        Ok(read_lines(&out))
    } else {
        Ok(read_lines(&bytes))
    }
}

fn header(source: &str, format: Format) -> Vec<Line> {
    let base_gz = source.rsplit('/').next().unwrap_or(source);
    let file = format!("admin/charsets/mapfiles/{base_gz}");
    let base = base_gz.strip_suffix(".gz").unwrap_or(base_gz);
    let (file, url) = match format {
        Format::Glibc1 | Format::Glibc2 | Format::Glibc27 => (format!("{base} in localedata/charmaps of glibc"), None),
        Format::Czyborra => (file, Some(format!("https://czyborra.com/charsets/{base}.gz"))),
        Format::Iana => (file, Some(format!("https://www.iana.org/assignments/charset-reg/{base}"))),
        Format::Unicode => (file, Some(format!("https://www.unicode.org/Public/MAPPINGS/VENDORS/ADOBE/{base}"))),
        Format::Unicode2 => (
            file,
            Some(format!("https://www.unicode.org/Public/MAPPINGS/VENDORS/MICSFT/WINDOWS/{base}")),
        ),
        Format::Yasuoka => (
            file,
            Some(format!("http://kanji.zinbun.kyoto-u.ac.jp/~yasuoka/ftp/CJKtable/{base}.Z")),
        ),
        Format::KanjiDatabase => (
            file,
            Some("http://kanji-database.cvs.sourceforge.net/viewvc/*checkout*/kanji-database/kanji-database/data/cns2ucsdkw.txt?revision=1.4".to_string()),
        ),
    };
    match url {
        Some(u) => vec![
            format!("# Generated from {file} which is a copy of").into_bytes(),
            format!("# {u}").into_bytes(),
        ],
        None => vec![format!("# Generated from {file}").into_bytes()],
    }
}

fn run_awk(awk: Awk, lines: &[Line]) -> Vec<Line> {
    match awk {
        Awk::None => lines.to_vec(),
        Awk::Compact => compact(lines),
        Awk::Cp932 => cp932(lines),
        Awk::Gb180302 => gb180302(lines),
        Awk::Kuten => kuten(lines),
    }
}

fn substitute(line: &[u8], pattern: &Regex, replacement: &str) -> Line {
    pattern.replace(line, replacement.as_bytes()).into_owned()
}

pub fn mapconv(
    path: &Path,
    address: Option<&str>,
    subst: Option<(&str, &str)>,
    format: Format,
    awk: Awk,
) -> io::Result<Vec<Line>> {
    let source = path.to_string_lossy().replace('\\', "/");
    let mut out = header(&source, format);
    let address = address.map(re);
    let subst = subst.map(|(p, r)| (re(&regex::escape(p)), r.to_string()));
    let mut selected = Vec::new();
    for line in read_source(path)? {
        if address.as_ref().is_some_and(|a| !a.is_match(&line)) {
            continue;
        }
        match &subst {
            Some((p, r)) => {
                if p.is_match(&line) {
                    selected.push(substitute(&line, p, r));
                }
            }
            None => selected.push(line),
        }
    }
    let (pattern, replacement) = match format {
        Format::Glibc1 => (r"<U([^>]*)>[ \t]*/x(..).*", "0x${2} 0x${1}"),
        Format::Glibc2 | Format::Glibc27 => {
            (r"<U([^>]*)>[ \t]*/x(..)/x(..).*", "0x${2}${3} 0x${1}")
        }
        Format::Czyborra => (r"=(..)[^U]*U\+([0-9A-F]*).*", "0x${1} 0x${2}"),
        Format::Iana => (r"(0x[0-9A-Fa-f]*)[^0]*(0x[0-9A-Fa-f]*).*", "${1} ${2}"),
        Format::Unicode => (r"([0-9A-F]*)[^0-9A-F]*([0-9A-F]*).*", "0x${2} 0x${1}"),
        Format::Unicode2 => (r"([0-9A-Fx]*)[^0]*([0-9A-Fx]*).*", "${1} ${2}"),
        Format::Yasuoka => (r"([0-9A-F]*)[^0]*0-([0-9]*).*", "0x${2} 0x${1}"),
        Format::KanjiDatabase => (r"...(....) U\+([0-9A-F]*).*", "0x${1} 0x${2}"),
    };
    let pattern = re(pattern);
    let mut body: Vec<Line> = selected
        .into_iter()
        .map(|line| {
            let line = if format == Format::Glibc27 {
                let mut t = line;
                for (from, to) in [
                    (b'a', b'2'),
                    (b'b', b'3'),
                    (b'c', b'4'),
                    (b'd', b'5'),
                    (b'e', b'6'),
                    (b'f', b'7'),
                ] {
                    t = replace_all(&t, &[b'x', from], &[b'x', to]);
                }
                t
            } else {
                line
            };
            substitute(&line, &pattern, replacement)
        })
        .collect();
    match format {
        Format::Unicode => {
            body.sort();
            body.reverse();
        }
        Format::Unicode2 => {
            body = sort_numeric_field4(&run_awk(awk, &body));
        }
        _ => {
            body.sort();
            body = run_awk(awk, &body);
        }
    }
    out.extend(body);
    Ok(out)
}

fn replace_all(text: &[u8], from: &[u8], to: &[u8]) -> Line {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    out
}

pub fn glibc_map(
    top: &Path,
    gz: &str,
    address: &str,
    subst: Option<(&str, &str)>,
    format: Format,
    awk: Awk,
) -> io::Result<Vec<Line>> {
    mapconv(
        &top.join("admin/charsets/glibc").join(gz),
        Some(address),
        subst,
        format,
        awk,
    )
}

fn sed_first(lines: &[Line], pattern: &str, replacement: &str) -> Vec<Line> {
    let p = re(pattern);
    lines
        .iter()
        .map(|l| substitute(l, &p, replacement))
        .collect()
}

fn sed_range(lines: &[Line], start: &str, end: Option<&str>) -> Vec<Line> {
    let start = re(start);
    let end = end.map(re);
    let mut out = Vec::new();
    let mut active = false;
    for line in lines {
        if active {
            out.push(line.clone());
            if end.as_ref().is_some_and(|e| e.is_match(line)) {
                active = false;
            }
        } else if start.is_match(line) {
            out.push(line.clone());
            active = true;
        }
    }
    out
}

fn copy_text(from: &Path) -> io::Result<Vec<u8>> {
    let bytes = fs::read(from)?;
    Ok(replace_all(&bytes, b"\r\n", b"\n"))
}

struct Generator {
    top: PathBuf,
    charsets: PathBuf,
}

const GENERAL_ADDRESS: &str = "^<.*[ \t]/x";

impl Generator {
    fn glibc(
        &self,
        gz: &str,
        address: &str,
        subst: Option<(&str, &str)>,
        format: Format,
        awk: Awk,
    ) -> io::Result<Vec<Line>> {
        glibc_map(&self.top, gz, address, subst, format, awk)
    }

    fn mapfile(&self, name: &str) -> PathBuf {
        self.top.join("admin/charsets/mapfiles").join(name)
    }

    fn write(&self, name: &str, lines: &[Line]) -> io::Result<()> {
        fs::write(self.charsets.join(name), join(lines))
    }

    fn read_map(&self, name: &str) -> io::Result<Vec<Line>> {
        Ok(read_lines(&fs::read(self.charsets.join(name))?))
    }

    fn general(&self, map: &str, gz: &str) -> io::Result<()> {
        let lines = self.glibc(gz, GENERAL_ADDRESS, None, Format::Glibc1, Awk::Compact)?;
        self.write(map, &lines)
    }

    fn jisx2131_filter(&self) -> io::Result<Vec<Regex>> {
        let capture = re(r"^.*0x([0-9A-Z]*)$");
        let mut out = Vec::new();
        for line in read_source(&self.mapfile("JISX213A.map"))? {
            if line.first() == Some(&b'#') {
                continue;
            }
            if let Some(c) = capture.captures(&line) {
                out.push(re(&format!("0x0*{}$", s(&c[1]))));
            }
        }
        Ok(out)
    }

    fn run(&self) -> io::Result<()> {
        fs::create_dir_all(&self.charsets)?;
        for n in [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 15, 16] {
            self.general(&format!("8859-{n}.map"), &format!("ISO-8859-{n}.gz"))?;
        }
        for name in [
            "IBM037",
            "IBM038",
            "IBM256",
            "IBM273",
            "IBM274",
            "IBM275",
            "IBM277",
            "IBM278",
            "IBM280",
            "IBM281",
            "IBM284",
            "IBM285",
            "IBM290",
            "IBM297",
            "IBM420",
            "IBM423",
            "IBM424",
            "IBM437",
            "IBM500",
            "IBM850",
            "IBM851",
            "IBM852",
            "IBM855",
            "IBM856",
            "IBM857",
            "IBM860",
            "IBM861",
            "IBM862",
            "IBM863",
            "IBM864",
            "IBM865",
            "IBM866",
            "IBM868",
            "IBM869",
            "IBM870",
            "IBM871",
            "IBM874",
            "IBM875",
            "IBM880",
            "IBM891",
            "IBM903",
            "IBM904",
            "IBM905",
            "IBM918",
            "IBM1004",
            "IBM1026",
            "IBM1047",
            "CP737",
            "CP775",
            "CP1125",
            "CP1250",
            "CP1251",
            "CP1252",
            "CP1253",
            "CP1254",
            "CP1255",
            "CP1256",
            "CP1257",
            "CP1258",
            "CP10007",
            "KSC5636",
            "KOI-8",
            "KOI8-R",
            "KOI8-U",
            "KOI8-T",
            "TIS-620",
            "VISCII",
            "HP-ROMAN8",
            "NEXTSTEP",
            "MACINTOSH",
        ] {
            self.general(&format!("{name}.map"), &format!("{name}.gz"))?;
        }
        for (map, gz) in [
            ("KA-PS.map", "GEORGIAN-PS.gz"),
            ("KA-ACADEMY.map", "GEORGIAN-ACADEMY.gz"),
            ("EBCDICUK.map", "EBCDIC-UK.gz"),
            ("EBCDICUS.map", "EBCDIC-US.gz"),
        ] {
            self.general(map, gz)?;
        }
        for name in [
            "CP720.map",
            "CP858.map",
            "JISX213A.map",
            "MULE-ethiopic.map",
            "MULE-ipa.map",
            "MULE-is13194.map",
            "MULE-sisheng.map",
            "MULE-tibetan.map",
            "MULE-lviscii.map",
            "MULE-uviscii.map",
        ] {
            fs::write(self.charsets.join(name), copy_text(&self.mapfile(name))?)?;
        }
        let vscii = self.glibc(
            "TCVN5712-1.gz",
            "^<.*[ \t]/x[0-9a-f].[ \t]",
            None,
            Format::Glibc1,
            Awk::Compact,
        )?;
        self.write("VSCII.map", &vscii)?;
        let vscii2 = self.glibc(
            "TCVN5712-1.gz",
            "^<.*[ \t]/x[2-7a-f].[ \t]",
            None,
            Format::Glibc1,
            Awk::Compact,
        )?;
        self.write(
            "VSCII-2.map",
            &sed_first(&vscii2, "0x20-0x7F.*", "0x00-0x7F 0x0000"),
        )?;
        let mut alt: Vec<Line> = vec![
            b"# Modified from IBM866.map according to the chart at".to_vec(),
            b"# https://web.archive.org/web/20100131045151/http://www.cyrillic.com/ref/cyrillic/koi-8alt.html".to_vec(),
            b"# with guesses for the Unicodes of the glyphs.".to_vec(),
        ];
        let space = re(" .*");
        for line in self.read_map("IBM866.map")?.into_iter().skip(1) {
            let mut line = line;
            for (code, unicode) in [
                ("0xF2", "0x2019"),
                ("0xF3", "0x2018"),
                ("0xF4", "0x0301"),
                ("0xF5", "0x0300"),
                ("0xF6", "0x203A"),
                ("0xF7", "0x2039"),
                ("0xF8", "0x2191"),
                ("0xF9", "0x2193"),
                ("0xFA", "0x00B1"),
                ("0xFB", "0x00F7"),
            ] {
                if line.windows(code.len()).any(|w| w == code.as_bytes()) {
                    line = substitute(&line, &space, &format!(" {unicode}"));
                }
            }
            alt.push(line);
        }
        self.write("ALTERNATIVNYJ.map", &alt)?;
        let mik = mapconv(
            &self.mapfile("bulgarian-mik.txt"),
            None,
            None,
            Format::Czyborra,
            Awk::Compact,
        )?;
        self.write("MIK.map", &mik)?;
        let ptcp = mapconv(
            &self.mapfile("PTCP154"),
            Some("^0x"),
            None,
            Format::Iana,
            Awk::Compact,
        )?;
        self.write("PTCP154.map", &ptcp)?;
        for name in ["stdenc", "symbol"] {
            let lines = mapconv(
                &self.mapfile(&format!("{name}.txt")),
                Some("^[0-9A-Fa-f]"),
                None,
                Format::Unicode,
                Awk::Compact,
            )?;
            self.write(&format!("{name}.map"), &lines)?;
        }
        let cp949 = self.glibc(
            "CP949.gz",
            "^<.*[ \t]/x[89a-f]",
            None,
            Format::Glibc2,
            Awk::Compact,
        )?;
        self.write("CP949-2BYTE.map", &cp949)?;
        let gb2312 = self.glibc(
            "GB2312.gz",
            "^<.*[ \t]/x[a-f]",
            None,
            Format::Glibc27,
            Awk::Compact,
        )?;
        self.write("GB2312.map", &gb2312)?;
        let gbk = self.glibc(
            "GBK.gz",
            "^<.*[ \t]/x[89a-f]",
            None,
            Format::Glibc2,
            Awk::Compact,
        )?;
        self.write("GBK.map", &gbk)?;
        let gb2 = self.glibc(
            "GB18030.gz",
            "^<.*[ \t]/x../x..[ \t]",
            None,
            Format::Glibc2,
            Awk::Gb180302,
        )?;
        self.write("GB180302.map", &gb2)?;
        self.write("GB180304.map", &gb180304(&gb2))?;
        let mut jisx0201 = self.glibc(
            "JIS_X0201.gz",
            "^<.*[ \t]/x[0-9]",
            None,
            Format::Glibc1,
            Awk::Compact,
        )?;
        jisx0201.push(b"# Generated by hand".to_vec());
        jisx0201.push(b"0xA1-0xDF 0xFF61".to_vec());
        self.write("JISX0201.map", &jisx0201)?;
        let jisx0208 = self.glibc(
            "EUC-JP.gz",
            "^<.*[ \t]/x[a-f]",
            None,
            Format::Glibc27,
            Awk::None,
        )?;
        self.write("JISX0208.map", &sed_first(&jisx0208, "0x2015", "0x2014"))?;
        let jisx0212 = self.glibc(
            "EUC-JP.gz",
            "^<.*[ \t]/x8f",
            Some(("/x8f", "")),
            Format::Glibc27,
            Awk::Compact,
        )?;
        self.write("JISX0212.map", &jisx0212)?;
        let filter = self.jisx2131_filter()?;
        let jisx2131 = self.glibc(
            "EUC-JISX0213.gz",
            "^<.*[ \t]/x[a-f]",
            None,
            Format::Glibc27,
            Awk::None,
        )?;
        let jisx2131: Vec<Line> = jisx2131
            .into_iter()
            .filter(|l| !filter.iter().any(|f| f.is_match(l)))
            .collect();
        let jisx2131 = sed_first(
            &sed_first(&jisx2131, "0x2015", "0x2014"),
            "0x2299",
            "0x29BF",
        );
        self.write("JISX2131.map", &jisx2131)?;
        let jisx2132 = self.glibc(
            "EUC-JISX0213.gz",
            "^<.*[ \t]/x8f",
            Some(("/x8f", "")),
            Format::Glibc27,
            Awk::None,
        )?;
        self.write("JISX2132.map", &jisx2132)?;
        let cp932 = mapconv(
            &self.mapfile("CP932.TXT"),
            Some("^0x[89A-F][0-9A-F][0-9A-F]"),
            None,
            Format::Unicode2,
            Awk::Cp932,
        )?;
        self.write("CP932-2BYTE.map", &cp932)?;
        let lispint = self.top.join("lisp/international");
        fs::write(lispint.join("cp51932.el"), cp51932_el(&cp932))?;
        let eucjp = read_source(&self.top.join("admin/charsets/glibc/EUC-JP-MS.gz"))?;
        fs::write(lispint.join("eucjp-ms.el"), eucjp_ms_el(&eucjp))?;
        let jisc = mapconv(
            &self.mapfile("Uni2JIS"),
            Some("^[^#].*0-"),
            None,
            Format::Yasuoka,
            Awk::Kuten,
        )?;
        let jisc_filtered: Vec<Line> = jisc
            .iter()
            .map(|l| {
                if l.windows(6).any(|w| w == b"0x2140") {
                    substitute(l, &re("005C"), "FF3C")
                } else {
                    l.clone()
                }
            })
            .collect();
        let mut jisc = jisc_filtered;
        for extra in [
            "0x3442 0x3D4E",
            "0x374E 0x25874",
            "0x3764 0x28EF6",
            "0x513D 0x2F80F",
            "0x7045 0x9724",
        ] {
            jisc.push(extra.as_bytes().to_vec());
        }
        self.write("JISC6226.map", &jisc)?;
        let ksc = self.glibc(
            "EUC-KR.gz",
            "^<.*[ \t]/x[a-f]",
            None,
            Format::Glibc27,
            Awk::Compact,
        )?;
        self.write("KSC5601.map", &ksc)?;
        let big5_map = self.glibc(
            "BIG5.gz",
            "^<.*[ \t]/x[a-f]",
            None,
            Format::Glibc2,
            Awk::None,
        )?;
        self.write("BIG5.map", &big5_map)?;
        let mut big5_1 = vec![b"# Generated from BIG5.map".to_vec()];
        big5_1.extend(big5(&sed_range(&big5_map, "0xa140", Some("0xc8fe"))));
        self.write("BIG5-1.map", &big5_1)?;
        let mut big5_2 = vec![b"# Generated from BIG5.map".to_vec()];
        big5_2.extend(big5(&sed_range(&big5_map, "0xc940", None)));
        self.write("BIG5-2.map", &big5_2)?;
        let hkscs = self.glibc(
            "BIG5-HKSCS.gz",
            "^<.*[ \t]/x[89a-f]./",
            None,
            Format::Glibc2,
            Awk::Compact,
        )?;
        self.write("BIG5-HKSCS.map", &hkscs)?;
        let johab = self.glibc(
            "JOHAB.gz",
            "^<.*[ \t]/x[89a-f]",
            None,
            Format::Glibc2,
            Awk::Compact,
        )?;
        self.write("JOHAB.map", &johab)?;
        let cns1 = self.glibc(
            "EUC-TW.gz",
            "^<.*[ \t]/x[a-f]",
            None,
            Format::Glibc27,
            Awk::Compact,
        )?;
        self.write("CNS-1.map", &cns1)?;
        for n in 2..=7 {
            let lines = mapconv(
                &self.mapfile("cns2ucsdkw.txt"),
                Some(&format!("^C{n}")),
                None,
                Format::KanjiDatabase,
                Awk::Compact,
            )?;
            self.write(&format!("CNS-{n}.map"), &lines)?;
        }
        let cnsf = self.glibc(
            "EUC-TW.gz",
            "^<.*/x8e/xaf",
            Some(("/x8e/xaf", "")),
            Format::Glibc27,
            Awk::Compact,
        )?;
        self.write("CNS-F.map", &cnsf)?;
        fs::write(
            self.top.join("admin/charsets/charsets.stamp"),
            b"timestamp\n",
        )?;
        Ok(())
    }
}

pub fn generate(top: &Path) -> io::Result<()> {
    Generator {
        top: top.to_path_buf(),
        charsets: top.join("etc/charsets"),
    }
    .run()
}

pub fn up_to_date(top: &Path) -> bool {
    let stamp = top.join("admin/charsets/charsets.stamp");
    let Ok(stamp_time) = fs::metadata(&stamp).and_then(|m| m.modified()) else {
        return false;
    };
    let sources = [
        top.join("admin/charsets/glibc"),
        top.join("admin/charsets/mapfiles"),
    ];
    for dir in sources {
        let Ok(entries) = fs::read_dir(dir) else {
            return false;
        };
        for entry in entries.flatten() {
            if entry
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|t| t > stamp_time)
            {
                return false;
            }
        }
    }
    true
}
