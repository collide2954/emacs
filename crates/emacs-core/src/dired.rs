use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::time::{SystemTime, UNIX_EPOCH};

use num_bigint::BigInt;

use crate::coding::f_coding_system_plist;
use crate::editfns::{f_user_login_name, f_user_uid};
use crate::error::LispResult;
use crate::fileio::{
    f_directory_file_name, f_expand_file_name, f_file_name_as_directory, normalize_slashes,
};
use crate::fns::{f_compare_strings, f_nreverse, f_sort, f_string_lessp, f_substring};
use crate::lisp::Lisp;
use crate::subr::Subr;
use crate::symbols as sym;
use crate::value::{MOST_POSITIVE_FIXNUM, Value};

const S_IFMT: u32 = 0o170000;
const S_IFDIR: u32 = 0o040000;
const S_IFREG: u32 = 0o100000;
const S_IFLNK: u32 = 0o120000;
#[cfg(windows)]
const S_IREAD: u32 = 0o400;
#[cfg(windows)]
const S_IWRITE: u32 = 0o200;
#[cfg(windows)]
const S_IEXEC: u32 = 0o100;

pub const SUBRS: &[Subr] = &[
    Subr::fixed("directory-files", 1, 5, f_directory_files),
    Subr::fixed(
        "directory-files-and-attributes",
        1,
        6,
        f_directory_files_and_attributes,
    ),
    Subr::fixed("file-name-completion", 2, 3, f_file_name_completion),
    Subr::fixed(
        "file-name-all-completions",
        2,
        2,
        f_file_name_all_completions,
    ),
    Subr::fixed("file-attributes", 1, 2, f_file_attributes),
    Subr::fixed("file-attributes-lessp", 2, 2, f_file_attributes_lessp),
    Subr::fixed("system-users", 0, 0, f_system_users),
    Subr::fixed("system-groups", 0, 0, f_system_groups),
];

pub fn init_vars(lisp: &mut Lisp) {
    lisp.defvar(sym::COMPLETION_IGNORED_EXTENSIONS, Value::NIL);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DirentType {
    Unknown,
    Dir,
    Lnk,
    Other,
}

struct Dirent {
    name: String,
    kind: DirentType,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Dir,
    Lnk,
    Reg,
}

struct Stat {
    kind: FileKind,
    mode: u32,
    nlink: u64,
    uid: i64,
    gid: i64,
    uname: Option<String>,
    gname: Option<String>,
    atime: (i64, i64),
    mtime: (i64, i64),
    ctime: (i64, i64),
    size: u64,
    ino: u64,
    dev: u64,
}

fn dirent_type(file_type: Option<std::fs::FileType>) -> DirentType {
    if cfg!(windows) {
        return DirentType::Unknown;
    }
    match file_type {
        Some(t) if t.is_dir() => DirentType::Dir,
        Some(t) if t.is_symlink() => DirentType::Lnk,
        Some(_) => DirentType::Other,
        None => DirentType::Unknown,
    }
}

fn read_directory(path: &Path) -> Result<Vec<Dirent>, (&'static str, std::io::Error)> {
    let iter = std::fs::read_dir(path).map_err(|e| ("Opening directory", e))?;
    let mut out = Vec::new();
    if !cfg!(windows) || path.parent().is_some() {
        for name in [".", ".."] {
            out.push(Dirent {
                name: name.to_string(),
                kind: dirent_type(std::fs::metadata(path).ok().map(|m| m.file_type())),
            });
        }
    }
    for entry in iter {
        let entry = entry.map_err(|e| ("Reading directory", e))?;
        out.push(Dirent {
            name: entry.file_name().to_string_lossy().into_owned(),
            kind: dirent_type(entry.file_type().ok()),
        });
    }
    Ok(out)
}

fn is_any_sep(byte: u8) -> bool {
    byte == b'/' || (cfg!(windows) && (byte == b'\\' || byte == b':'))
}

#[cfg(windows)]
fn timespec(t: std::io::Result<SystemTime>) -> Option<(i64, i64)> {
    let t = t.ok()?;
    Some(match t.duration_since(UNIX_EPOCH) {
        Ok(d) => (d.as_secs() as i64, d.subsec_nanos() as i64),
        Err(e) => {
            let d = e.duration();
            let (mut s, mut ns) = (-(d.as_secs() as i64), -(d.subsec_nanos() as i64));
            if ns < 0 {
                s -= 1;
                ns += 1_000_000_000;
            }
            (s, ns)
        }
    })
}

#[cfg(windows)]
fn is_exec(path: &Path) -> bool {
    path.extension().is_some_and(|e| {
        matches!(
            e.to_string_lossy().to_ascii_lowercase().as_str(),
            "exe" | "com" | "bat" | "cmd"
        )
    })
}

#[cfg(windows)]
fn lstat(path: &Path, uid: i64, uname: &str) -> std::io::Result<Stat> {
    const FILETIME_ZERO: i64 = -11_644_473_600;
    let convert = |t: std::io::Result<SystemTime>| match timespec(t) {
        Some((s, ns)) if s != FILETIME_ZERO => (s, ns),
        _ => (0, 0),
    };
    let meta = std::fs::symlink_metadata(path)?;
    let is_link = meta.file_type().is_symlink();
    let kind = if is_link {
        FileKind::Lnk
    } else if meta.is_dir() {
        FileKind::Dir
    } else {
        FileKind::Reg
    };
    let mut mode = match kind {
        FileKind::Lnk => S_IFLNK,
        FileKind::Dir => S_IFDIR,
        FileKind::Reg => S_IFREG,
    };
    let permission = if is_link {
        S_IREAD | S_IWRITE | S_IEXEC
    } else {
        let mut p = if meta.permissions().readonly() {
            S_IREAD
        } else {
            S_IREAD | S_IWRITE
        };
        if meta.is_dir() || is_exec(path) {
            p |= S_IEXEC;
        }
        p
    };
    mode |= permission | (permission >> 3) | (permission >> 6);
    let mtime = convert(meta.modified());
    let mut atime = convert(meta.accessed());
    if atime.0 == 0 {
        atime = mtime;
    }
    let mut ctime = convert(meta.created());
    if ctime.0 == 0 {
        ctime = mtime;
    }
    Ok(Stat {
        kind,
        mode,
        nlink: 1,
        uid,
        gid: uid,
        uname: Some(uname.to_string()),
        gname: Some("None".to_string()),
        atime,
        mtime,
        ctime,
        size: meta.len(),
        ino: 0,
        dev: 0,
    })
}

#[cfg(unix)]
fn lstat(path: &Path, _uid: i64, _uname: &str) -> std::io::Result<Stat> {
    use std::os::unix::fs::MetadataExt;
    let m = std::fs::symlink_metadata(path)?;
    let kind = if m.file_type().is_symlink() {
        FileKind::Lnk
    } else if m.is_dir() {
        FileKind::Dir
    } else {
        FileKind::Reg
    };
    Ok(Stat {
        kind,
        mode: m.mode(),
        nlink: m.nlink(),
        uid: m.uid() as i64,
        gid: m.gid() as i64,
        uname: None,
        gname: None,
        atime: (m.atime(), m.atime_nsec()),
        mtime: (m.mtime(), m.mtime_nsec()),
        ctime: (m.ctime(), m.ctime_nsec()),
        size: m.size(),
        ino: m.ino(),
        dev: m.dev(),
    })
}

fn ftypelet(bits: u32) -> char {
    match bits & S_IFMT {
        S_IFREG => '-',
        S_IFDIR => 'd',
        0o060000 => 'b',
        0o020000 => 'c',
        S_IFLNK => 'l',
        0o010000 => 'p',
        0o140000 => 's',
        _ => '?',
    }
}

fn filemodestring(bits: u32) -> String {
    let flag = |bit: u32, c: char| if bits & bit != 0 { c } else { '-' };
    let exec =
        |special: u32, x: u32, lower: char, upper: char| match (bits & special != 0, bits & x != 0)
        {
            (true, true) => lower,
            (true, false) => upper,
            (false, true) => 'x',
            (false, false) => '-',
        };
    [
        ftypelet(bits),
        flag(0o400, 'r'),
        flag(0o200, 'w'),
        exec(0o4000, 0o100, 's', 'S'),
        flag(0o040, 'r'),
        flag(0o020, 'w'),
        exec(0o2000, 0o010, 's', 'S'),
        flag(0o004, 'r'),
        flag(0o002, 'w'),
        exec(0o1000, 0o001, 't', 'T'),
    ]
    .iter()
    .collect()
}

impl Lisp {
    fn unsigned_integer(&mut self, n: u64) -> Value {
        match i64::try_from(n) {
            Ok(v) => self.integer(v),
            Err(_) => self.bignum(BigInt::from(n)),
        }
    }

    pub(crate) fn file_metadata_errno(
        &mut self,
        action: &str,
        file: Value,
        err: &std::io::Error,
    ) -> LispResult {
        match err.kind() {
            std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory => Ok(Value::NIL),
            _ => Err(self.file_error(action, err, file)),
        }
    }

    fn file_attributes(&mut self, path: &Path, filename: Value, id_format: Value) -> LispResult {
        let uid = f_user_uid(self, &[])?.as_fixnum().unwrap_or(0);
        let login = f_user_login_name(self, &[Value::NIL])?;
        let login = if login.is_string() {
            self.string_to_rust(login)
        } else {
            String::new()
        };
        let st = match lstat(path, uid, &login) {
            Ok(st) => st,
            Err(e) => return self.file_metadata_errno("Getting attributes", filename, &e),
        };
        let file_type = match st.kind {
            FileKind::Lnk => match std::fs::read_link(path) {
                Ok(target) => {
                    let target = normalize_slashes(&target.to_string_lossy());
                    self.string(&target)
                }
                Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => Value::NIL,
                Err(e) => self.file_metadata_errno("Reading symbolic link", filename, &e)?,
            },
            FileKind::Dir => sym::T,
            FileKind::Reg => Value::NIL,
        };
        if st.kind == FileKind::Lnk && file_type.is_nil() {
            return Ok(Value::NIL);
        }
        let (uname, gname) = if id_format.is_nil() || id_format == sym::INTEGER {
            (None, None)
        } else {
            (st.uname, st.gname)
        };
        let uid = match uname {
            Some(name) => self.string(&name),
            None => self.integer(st.uid),
        };
        let gid = match gname {
            Some(name) => self.string(&name),
            None => self.integer(st.gid),
        };
        let atime = self.make_lisp_time(st.atime.0, st.atime.1);
        let mtime = self.make_lisp_time(st.mtime.0, st.mtime.1);
        let ctime = self.make_lisp_time(st.ctime.0, st.ctime.1);
        let size = self.unsigned_integer(st.size);
        let modes = self.string(&filemodestring(st.mode));
        let ino = self.unsigned_integer(st.ino);
        let dev = self.unsigned_integer(st.dev);
        Ok(self.list(&[
            file_type,
            Value::fixnum(st.nlink as i64),
            uid,
            gid,
            atime,
            mtime,
            ctime,
            size,
            modes,
            sym::T,
            ino,
            dev,
        ]))
    }

    #[allow(clippy::too_many_arguments)]
    fn directory_files_internal(
        &mut self,
        directory: Value,
        full: Value,
        match_: Value,
        nosort: Value,
        attrs: bool,
        id_format: Value,
        return_count: Value,
    ) -> LispResult {
        let mut last = MOST_POSITIVE_FIXNUM;
        if !return_count.is_nil() {
            last = match return_count.as_fixnum() {
                Some(n) if n >= 0 => n,
                _ => return Err(self.wrong_type(sym::WHOLENUMP, return_count)),
            };
        }
        if !match_.is_nil() {
            self.check_string(match_)?;
        }
        let base = self.stack.len();
        self.stack.extend([directory, match_, id_format]);
        let r = self
            .directory_files_internal_1(directory, full, match_, nosort, attrs, id_format, last);
        self.stack.truncate(base);
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn directory_files_internal_1(
        &mut self,
        directory: Value,
        full: Value,
        match_: Value,
        nosort: Value,
        attrs: bool,
        id_format: Value,
        last: i64,
    ) -> LispResult {
        let dirfilename = f_directory_file_name(self, &[directory])?;
        self.stack.push(dirfilename);
        let dirpath = PathBuf::from(self.string_to_rust(dirfilename));
        let entries = match read_directory(&dirpath) {
            Ok(entries) => entries,
            Err((action, e)) => {
                let file = if action == "Opening directory" {
                    dirfilename
                } else {
                    directory
                };
                return Err(self.file_error(action, &e, file));
            }
        };
        let dir_text = self.string_to_rust(directory);
        let needsep = self
            .string_bytes(directory)
            .last()
            .is_none_or(|&b| !is_any_sep(b));
        let case_table = if cfg!(windows) {
            self.ascii_canon_table
        } else {
            Value::NIL
        };
        let mut list = Value::NIL;
        let mut ind = 0;
        for dirent in entries {
            let name = self.string(&dirent.name);
            self.stack.push(name);
            if !match_.is_nil() && self.fast_string_match(match_, name, case_table)? < 0 {
                continue;
            }
            let mut fileattrs = Value::NIL;
            if attrs {
                fileattrs = self.file_attributes(&dirpath.join(&dirent.name), name, id_format)?;
                if fileattrs.is_nil() {
                    continue;
                }
                self.stack.push(fileattrs);
            }
            let finalname = if full.is_nil() {
                name
            } else {
                let sep = if needsep { "/" } else { "" };
                self.string(&format!("{dir_text}{sep}{}", dirent.name))
            };
            if ind == last {
                break;
            }
            ind += 1;
            let elt = if attrs {
                self.cons(finalname, fileattrs)
            } else {
                finalname
            };
            list = self.cons(elt, list);
            self.stack.push(list);
        }
        if nosort.is_nil() {
            let ordered = f_nreverse(self, &[list])?;
            self.stack.push(ordered);
            let predicate = if attrs {
                sym::FILE_ATTRIBUTES_LESSP
            } else {
                sym::STRING_LESSP
            };
            list = f_sort(self, &[ordered, predicate])?;
        }
        Ok(list)
    }

    fn scmp(&self, s1: &[u8], s2: &[u8], len: usize) -> i64 {
        let ignore_case = self.var_true(sym::COMPLETION_IGNORE_CASE);
        let mut l = len;
        let mut i = 0;
        while l > 0 {
            let (a, b) = (
                s1.get(i).copied().unwrap_or(0) as u32,
                s2.get(i).copied().unwrap_or(0) as u32,
            );
            let same = if ignore_case {
                self.buffer_downcase(a) == self.buffer_downcase(b)
            } else {
                a == b
            };
            if !same {
                break;
            }
            l -= 1;
            i += 1;
        }
        if l == 0 { -1 } else { (len - l) as i64 }
    }

    fn compare_strings_t(&mut self, args: [Value; 7]) -> LispResult<bool> {
        Ok(f_compare_strings(self, &args)? == sym::T)
    }

    fn file_name_completion(
        &mut self,
        file: Value,
        dirname: Value,
        all_flag: bool,
        predicate: Value,
    ) -> LispResult {
        self.check_string(file)?;
        let count = self.specpdl.len();
        let base = self.stack.len();
        self.stack.extend([file, dirname, predicate]);
        let r = self.file_name_completion_1(file, dirname, all_flag, predicate);
        self.stack.truncate(base);
        self.unbind_to_value(count, r)
    }

    fn file_name_completion_1(
        &mut self,
        file: Value,
        dirname: Value,
        all_flag: bool,
        predicate: Value,
    ) -> LispResult {
        let ignore_case = |l: &Lisp| l.var_true(sym::COMPLETION_IGNORE_CASE);
        let mut bestmatchsize: i64 = 0;
        let mut matchcount = 0;
        let mut bestmatch = Value::NIL;
        let mut includeall = true;
        let mut check_decoded = false;
        self.specbind(sym::DEFAULT_DIRECTORY, dirname)?;
        let encoded_file = self.string_bytes(file).to_vec();
        let encoded_dir = f_directory_file_name(self, &[dirname])?;
        let mut file_encoding = self.find_symbol_value(sym::FILE_NAME_CODING_SYSTEM);
        if file_encoding.is_nil() {
            file_encoding = self.find_symbol_value(sym::DEFAULT_FILE_NAME_CODING_SYSTEM);
        }
        if !file_encoding.is_nil() {
            let plist = f_coding_system_plist(self, &[file_encoding])?;
            if !self
                .plist_get_eq(plist, sym::DECOMPOSED_CHARACTERS)
                .is_nil()
            {
                check_decoded = true;
            }
        }
        let dirpath = PathBuf::from(self.string_to_rust(encoded_dir));
        let entries = match read_directory(&dirpath) {
            Ok(entries) => entries,
            Err((action, e)) => return Err(self.file_error(action, &e, dirname)),
        };
        let zero = Value::fixnum(0);
        let enc_file_len = encoded_file.len();
        let file_chars = self.string_chars(file) as i64;
        let file_len = Value::fixnum(file_chars);
        for dirent in entries {
            let d_name = dirent.name.as_bytes();
            let len = d_name.len();
            let mut canexclude = false;
            if len < enc_file_len
                || (!ignore_case(self) && self.scmp(d_name, &encoded_file, enc_file_len) >= 0)
            {
                continue;
            }
            let mut name = self.string(&dirent.name);
            self.stack.push(name);
            let name_blen = self.string_bytes(name).len() as i64;
            let name_len = self.string_chars(name) as i64;
            if ignore_case(self)
                && !self.compare_strings_t([name, zero, file_len, file, zero, file_len, sym::T])?
            {
                continue;
            }
            let directoryp = match dirent.kind {
                DirentType::Dir => true,
                DirentType::Lnk | DirentType::Unknown => {
                    std::fs::metadata(dirpath.join(&dirent.name)).is_ok_and(|m| m.is_dir())
                }
                DirentType::Other => false,
            };
            if !all_flag {
                let mut tem = Value::NIL;
                if directoryp {
                    if dirent.name == "." || dirent.name == ".." {
                        canexclude = true;
                    } else if len > enc_file_len {
                        tem = self.find_symbol_value(sym::COMPLETION_IGNORED_EXTENSIONS);
                        while tem.is_cons() {
                            let elt = self.xcar(tem);
                            if !elt.is_string() {
                                tem = self.xcdr(tem);
                                continue;
                            }
                            let p1 = self.string_bytes(elt).to_vec();
                            let elt_len = p1.len() as i64 - 1;
                            if elt_len <= 0 || p1[elt_len as usize] != b'/' {
                                tem = self.xcdr(tem);
                                continue;
                            }
                            let skip = name_blen - elt_len;
                            if skip < 0 {
                                tem = self.xcdr(tem);
                                continue;
                            }
                            if !ignore_case(self) {
                                let tail = self.string_bytes(name)[skip as usize..].to_vec();
                                if self.scmp(&tail, &p1, elt_len as usize) >= 0 {
                                    tem = self.xcdr(tem);
                                    continue;
                                }
                            }
                            if ignore_case(self) {
                                let elt_len = self.string_chars(elt) as i64 - 1;
                                let skip = name_len - elt_len;
                                if skip < 0
                                    || !self.compare_strings_t([
                                        name,
                                        Value::fixnum(skip),
                                        Value::NIL,
                                        elt,
                                        zero,
                                        Value::fixnum(elt_len),
                                        sym::T,
                                    ])?
                                {
                                    tem = self.xcdr(tem);
                                    continue;
                                }
                            }
                            break;
                        }
                    }
                } else if len > enc_file_len {
                    tem = self.find_symbol_value(sym::COMPLETION_IGNORED_EXTENSIONS);
                    while tem.is_cons() {
                        let elt = self.xcar(tem);
                        if !elt.is_string() {
                            tem = self.xcdr(tem);
                            continue;
                        }
                        let p1 = self.string_bytes(elt).to_vec();
                        let elt_len = p1.len() as i64;
                        let skip = len as i64 - elt_len;
                        if skip < 0 {
                            tem = self.xcdr(tem);
                            continue;
                        }
                        if !ignore_case(self) {
                            let tail = self
                                .string_bytes(name)
                                .get(skip as usize..)
                                .unwrap_or(&[])
                                .to_vec();
                            if self.scmp(&tail, &p1, elt_len as usize) >= 0 {
                                tem = self.xcdr(tem);
                                continue;
                            }
                        }
                        if ignore_case(self) {
                            let elt_len = self.string_chars(elt) as i64;
                            let skip = name_len - elt_len;
                            if skip < 0
                                || !self.compare_strings_t([
                                    name,
                                    Value::fixnum(skip),
                                    Value::NIL,
                                    elt,
                                    zero,
                                    Value::fixnum(elt_len),
                                    sym::T,
                                ])?
                            {
                                tem = self.xcdr(tem);
                                continue;
                            }
                        }
                        break;
                    }
                }
                if tem.is_cons() {
                    canexclude = true;
                }
                if !includeall && canexclude {
                    continue;
                }
                if includeall && !canexclude {
                    includeall = false;
                    bestmatch = Value::NIL;
                    bestmatchsize = 0;
                    matchcount = 0;
                }
            }
            let table = if ignore_case(self) {
                self.ascii_canon_table
            } else {
                Value::NIL
            };
            let mut regexps = self.find_symbol_value(sym::COMPLETION_REGEXP_LIST);
            while regexps.is_cons() {
                let re = self.xcar(regexps);
                if self.fast_string_match(re, name, table)? < 0 {
                    break;
                }
                regexps = self.xcdr(regexps);
            }
            if regexps.is_cons() {
                continue;
            }
            if directoryp {
                name = f_file_name_as_directory(self, &[name])?;
                self.stack.push(name);
            }
            if !predicate.is_nil() && self.funcall(predicate, &[name])?.is_nil() {
                continue;
            }
            if check_decoded && file_chars <= self.string_chars(name) as i64 {
                let compare = Value::fixnum(file_chars);
                let fold = self.bool(ignore_case(self));
                if !self.compare_strings_t([name, zero, compare, file, zero, compare, fold])? {
                    continue;
                }
            }
            if matchcount <= 1 {
                matchcount += 1;
            }
            if all_flag {
                bestmatch = self.cons(name, bestmatch);
                self.stack.push(bestmatch);
            } else if bestmatch.is_nil() {
                bestmatch = name;
                bestmatchsize = self.string_chars(name) as i64;
            } else {
                let compare = bestmatchsize.min(self.string_chars(name) as i64);
                let fold = self.bool(ignore_case(self));
                let cmp = f_compare_strings(
                    self,
                    &[
                        bestmatch,
                        zero,
                        Value::fixnum(compare),
                        name,
                        zero,
                        Value::fixnum(compare),
                        fold,
                    ],
                )?;
                let matchsize = if cmp == sym::T {
                    compare
                } else {
                    cmp.as_fixnum().unwrap_or(0).abs() - 1
                };
                if ignore_case(self) {
                    let name_chars = self.string_chars(name) as i64;
                    let best_chars = self.string_chars(bestmatch) as i64;
                    let directoryp = i64::from(directoryp);
                    if (matchsize == name_chars && matchsize + directoryp < best_chars)
                        || ((matchsize == name_chars) == (matchsize + directoryp == best_chars)
                            && self.compare_strings_t([
                                name,
                                zero,
                                file_len,
                                file,
                                zero,
                                Value::NIL,
                                Value::NIL,
                            ])?
                            && !self.compare_strings_t([
                                bestmatch,
                                zero,
                                file_len,
                                file,
                                zero,
                                Value::NIL,
                                Value::NIL,
                            ])?)
                    {
                        bestmatch = name;
                    }
                }
                bestmatchsize = matchsize;
                if matchsize <= file_chars
                    && !includeall
                    && (!ignore_case(self) || matchsize == 0)
                    && matchcount > 1
                {
                    break;
                }
            }
        }
        if all_flag || bestmatch.is_nil() {
            return Ok(bestmatch);
        }
        if matchcount == 1 && self.equal(bestmatch, file)? {
            return Ok(sym::T);
        }
        f_substring(
            self,
            &[bestmatch, Value::fixnum(0), Value::fixnum(bestmatchsize)],
        )
    }
}

fn f_directory_files(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (full, match_, nosort, count) = (args[1], args[2], args[3], args[4]);
    let directory = f_expand_file_name(lisp, &[args[0], Value::NIL])?;
    if let Some(handler) = lisp.find_file_name_handler(directory, sym::DIRECTORY_FILES)? {
        return lisp.funcall(
            handler,
            &[sym::DIRECTORY_FILES, directory, full, match_, nosort, count],
        );
    }
    lisp.directory_files_internal(directory, full, match_, nosort, false, Value::NIL, count)
}

fn f_directory_files_and_attributes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (full, match_, nosort, id_format, count) = (args[1], args[2], args[3], args[4], args[5]);
    let directory = f_expand_file_name(lisp, &[args[0], Value::NIL])?;
    if let Some(handler) =
        lisp.find_file_name_handler(directory, sym::DIRECTORY_FILES_AND_ATTRIBUTES)?
    {
        return lisp.funcall(
            handler,
            &[
                sym::DIRECTORY_FILES_AND_ATTRIBUTES,
                directory,
                full,
                match_,
                nosort,
                id_format,
                count,
            ],
        );
    }
    lisp.directory_files_internal(directory, full, match_, nosort, true, id_format, count)
}

fn f_file_name_completion(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (file, predicate) = (args[0], args[2]);
    let directory = f_expand_file_name(lisp, &[args[1], Value::NIL])?;
    for name in [directory, file] {
        if let Some(handler) = lisp.find_file_name_handler(name, sym::FILE_NAME_COMPLETION)? {
            return lisp.funcall(
                handler,
                &[sym::FILE_NAME_COMPLETION, file, directory, predicate],
            );
        }
    }
    lisp.file_name_completion(file, directory, false, predicate)
}

fn f_file_name_all_completions(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let file = args[0];
    let directory = f_expand_file_name(lisp, &[args[1], Value::NIL])?;
    for name in [directory, file] {
        if let Some(handler) = lisp.find_file_name_handler(name, sym::FILE_NAME_ALL_COMPLETIONS)? {
            return lisp.funcall(handler, &[sym::FILE_NAME_ALL_COMPLETIONS, file, directory]);
        }
    }
    lisp.file_name_completion(file, directory, true, Value::NIL)
}

fn f_file_attributes(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let (filename, id_format) = (args[0], args[1]);
    let filename = lisp.condition_case_internal(
        sym::T,
        |l| f_expand_file_name(l, &[filename, Value::NIL]),
        |_, err| Ok(err),
    )?;
    if !filename.is_string() {
        return Ok(Value::NIL);
    }
    if let Some(handler) = lisp.find_file_name_handler(filename, sym::FILE_ATTRIBUTES)? {
        return if id_format.is_nil() {
            lisp.funcall(handler, &[sym::FILE_ATTRIBUTES, filename])
        } else {
            lisp.funcall(handler, &[sym::FILE_ATTRIBUTES, filename, id_format])
        };
    }
    let path = PathBuf::from(lisp.string_to_rust(filename));
    lisp.file_attributes(&path, filename, id_format)
}

fn f_file_attributes_lessp(lisp: &mut Lisp, args: &[Value]) -> LispResult {
    let a = lisp.car(args[0])?;
    let b = lisp.car(args[1])?;
    f_string_lessp(lisp, &[a, b])
}

fn f_system_users(lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    let name = lisp.find_symbol_value(sym::USER_REAL_LOGIN_NAME);
    Ok(lisp.list(&[name]))
}

fn f_system_groups(_lisp: &mut Lisp, _args: &[Value]) -> LispResult {
    Ok(Value::NIL)
}
