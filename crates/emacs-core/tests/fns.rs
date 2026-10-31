mod common;

use common::check;

#[test]
fn overriding_plist_environment() {
    check(&[
        ("overriding-plist-environment", "nil"),
        ("(special-variable-p 'overriding-plist-environment)", "t"),
        ("(put 'opl-sym 'opl-prop 2)", "2"),
        (
            "(funcall (eval '(function (lambda () (let ((overriding-plist-environment '((opl-sym opl-prop 1)))) (list (get 'opl-sym 'opl-prop) (get 'opl-sym 'other))))) t))",
            "(1 nil)",
        ),
        ("(get 'opl-sym 'opl-prop)", "2"),
    ]);
}

#[test]
fn buffer_hash() {
    check(&[
        ("(set-buffer (get-buffer-create \"bh\"))", "#<buffer bh>"),
        (
            "(buffer-hash)",
            "\"da39a3ee5e6b4b0d3255bfef95601890afd80709\"",
        ),
        (
            "(progn (insert \"abc\") (buffer-hash))",
            "\"a9993e364706816aba3e25717850c26c9cd0d89d\"",
        ),
        (
            "(progn (narrow-to-region 2 3) (prog1 (buffer-hash) (widen)))",
            "\"a9993e364706816aba3e25717850c26c9cd0d89d\"",
        ),
        (
            "(progn (erase-buffer) (insert \"\u{e9}\") (buffer-hash \"bh\"))",
            "\"bf15be717ac1b080b4f1c456692825891ff5073d\"",
        ),
        (
            "(progn (erase-buffer) (insert (make-string 100 ?a)) (goto-char 51) (insert \"b\") (buffer-hash (current-buffer)))",
            "\"e17f5c7370f1dd97c49c9d3cbc1abf22033df9e1\"",
        ),
        (
            "(progn (erase-buffer) (insert (make-string 100000 ?x)) (buffer-hash))",
            "\"f6ee99edde6199a3e982c46ef72bdd5cb5e41ddf\"",
        ),
        (
            "(buffer-hash \"no such buffer\")",
            "error: (error \"No buffer named no such buffer\")",
        ),
        ("(buffer-hash 1)", "error: (wrong-type-argument stringp 1)"),
    ]);
}

#[test]
fn base64_strings() {
    check(&[
        (
            "(base64-encode-string \"\u{fc}\")",
            "error: (error \"Multibyte character in data for base64 encoding\")",
        ),
        (
            "(base64url-encode-string \"\u{192}\")",
            "error: (error \"Multibyte character in data for base64 encoding\")",
        ),
        ("(base64-encode-string \"\\377a\")", "\"/2E=\""),
        (
            "(base64-encode-string (string-to-multibyte \"\\377a\"))",
            "\"/2E=\"",
        ),
        ("(length (base64-encode-string (make-string 60 ?a)))", "81"),
        ("(aref (base64-encode-string (make-string 60 ?a)) 76)", "10"),
        (
            "(length (base64-encode-string (make-string 60 ?a) t))",
            "80",
        ),
        (
            "(length (base64url-encode-string (make-string 60 ?a)))",
            "80",
        ),
        ("(base64url-encode-string \"f\" t)", "\"Zg\""),
        ("(base64url-encode-string \"f\")", "\"Zg==\""),
        (
            "(base64-decode-string \"Zg=\")",
            "error: (error \"Invalid base64 data\")",
        ),
        (
            "(base64-decode-string \"Zm9vYmE\")",
            "error: (error \"Invalid base64 data\")",
        ),
        ("(base64-decode-string \"Zm9vYmE\" t)", "\"fooba\""),
        ("(base64-decode-string \"Zm9\" t)", "\"fo\""),
        (
            "(base64-decode-string \"Zm9vYmFy=\")",
            "error: (error \"Invalid base64 data\")",
        ),
        (
            "(base64-decode-string \"Zg=Zg=\")",
            "error: (error \"Invalid base64 data\")",
        ),
        (
            "(base64-decode-string \"=Zg==\")",
            "error: (error \"Invalid base64 data\")",
        ),
        ("(base64-decode-string \"Zg==Zg==\")", "\"ff\""),
        ("(base64-decode-string \" Z\\tg\\n=\\r= \")", "\"f\""),
        ("(base64-decode-string \"Zg==\" nil t)", "\"f\""),
        ("(base64-decode-string \"Z!g\" nil t)", "\"f\""),
        (
            "(base64-decode-string \"Z!g\")",
            "error: (error \"Invalid base64 data\")",
        ),
        ("(base64-decode-string \"/w==\")", "\"\\377\""),
        (
            "(base64-decode-string \"_w==\")",
            "error: (error \"Invalid base64 data\")",
        ),
        ("(base64-decode-string \"_w==\" t)", "\"\\377\""),
    ]);
}

#[test]
fn base64_regions() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"b64\")) (insert \"foobar\") (list (base64-encode-region 1 7) (buffer-string) (point)))",
            "(8 \"Zm9vYmFy\" 9)",
        ),
        (
            "(progn (erase-buffer) (insert \"xfooy\") (goto-char 3) (list (base64-encode-region 2 5) (buffer-string) (point)))",
            "(4 \"xZm9vy\" 2)",
        ),
        (
            "(progn (erase-buffer) (insert \"\u{fc}\") (list (condition-case e (base64-encode-region 1 2) (error e)) (buffer-string)))",
            "((error \"Multibyte character in data for base64 encoding\") \"\u{fc}\")",
        ),
        (
            "(progn (erase-buffer) (insert \"\\377\") (list (base64-encode-region 1 2) (buffer-string)))",
            "(4 \"/w==\")",
        ),
        (
            "(progn (erase-buffer) (insert \"fo\") (list (base64url-encode-region 1 3 t) (buffer-string)))",
            "(3 \"Zm8\")",
        ),
        (
            "(progn (erase-buffer) (insert \"fo\") (list (base64url-encode-region 1 3) (buffer-string)))",
            "(4 \"Zm8=\")",
        ),
        (
            "(progn (erase-buffer) (insert (make-string 60 ?a)) (base64-encode-region 1 61) (list (buffer-size) (char-after 77)))",
            "(81 10)",
        ),
        (
            "(progn (erase-buffer) (insert (make-string 60 ?a)) (base64-encode-region 1 61 t) (buffer-size))",
            "80",
        ),
        (
            "(progn (erase-buffer) (insert \"Zm9vYmFy\") (list (base64-decode-region 1 9) (buffer-string) (point)))",
            "(6 \"foobar\" 7)",
        ),
        (
            "(progn (erase-buffer) (insert \"/w==\") (list (base64-decode-region 1 5) (char-after 1) (multibyte-string-p (buffer-string)) (point)))",
            "(1 4194303 t 2)",
        ),
        (
            "(progn (erase-buffer) (insert \"Zg=\") (list (condition-case e (base64-decode-region 1 4) (error e)) (buffer-string)))",
            "((error \"Invalid base64 data\") \"Zg=\")",
        ),
        (
            "(progn (erase-buffer) (insert \"Z!m9v\") (list (base64-decode-region 1 6 nil t) (buffer-string)))",
            "(3 \"foo\")",
        ),
        (
            "(progn (erase-buffer) (insert \"Zm9v\") (goto-char 3) (base64-decode-region 1 5) (point))",
            "1",
        ),
        (
            "(progn (erase-buffer) (insert \"Zm8\") (list (base64-decode-region 1 4 t) (buffer-string)))",
            "(2 \"fo\")",
        ),
        (
            "(progn (erase-buffer) (insert \"xZm8y\") (goto-char 6) (list (base64-decode-region 2 5 t) (buffer-string) (point)))",
            "(2 \"xfoy\" 5)",
        ),
        (
            "(progn (erase-buffer) (set-buffer-multibyte nil) (insert \"/w==\") (base64-decode-region 1 5) (list (char-after 1) (char-before 2)))",
            "(255 255)",
        ),
    ]);
}

const HASH_INPUTS: [&str; 4] = [
    "\"\"",
    "\"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq\"",
    "\"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu\"",
    "(make-string 1000 ?a)",
];

const HASH_DIGESTS: [(&str, [&str; 4]); 10] = [
    (
        "md5",
        [
            "d41d8cd98f00b204e9800998ecf8427e",
            "8215ef0796a20bcaaae116d3876c664a",
            "03dd8807a93175fb062dfb55dc7d359c",
            "cabe45dcc9ae5b66ba86600cca6b8ba8",
        ],
    ),
    (
        "sha1",
        [
            "da39a3ee5e6b4b0d3255bfef95601890afd80709",
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            "a49b2446a02c645bf419f995b67091253a04a259",
            "291e9a6c66994949b57ba5e650361e98fc36b1ba",
        ],
    ),
    (
        "sha224",
        [
            "d14a028c2a3a2bc9476102bb288234c415a2b01f828ea62ac5b3e42f",
            "75388b16512776cc5dba5da1fd890150b0c6455cb4f58b1952522525",
            "c97ca9a559850ce97a04a96def6d99a9e0e0e2ab14e6b8df265fc0b3",
            "4e8f0ce90b64661a2b5e84be6d93a7d9b76871062f1814433d04a03d",
        ],
    ),
    (
        "sha256",
        [
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
            "cf5b16a778af8380036ce59e7b0492370b249b11e8f07a51afac45037afee9d1",
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3",
        ],
    ),
    (
        "sha384",
        [
            "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b",
            "3391fdddfc8dc7393707a65b1b4709397cf8b1d162af05abfe8f450de5f36bc6b0455a8520bc4e6f5fe95b1fe3c8452b",
            "09330c33f71147e83d192fc782cd1b4753111b173b3b05d22fa08086e3b0f712fcc7c71a557e2db966c3e9fa91746039",
            "f54480689c6b0b11d0303285d9a81b21a93bca6ba5a1b4472765dca4da45ee328082d469c650cd3b61b16d3266ab8ced",
        ],
    ),
    (
        "sha512",
        [
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
            "204a8fc6dda82f0a0ced7beb8e08a41657c16ef468b228a8279be331a703c33596fd15c13b1b07f9aa1d3bea57789ca031ad85c7a71dd70354ec631238ca3445",
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909",
            "67ba5535a46e3f86dbfbed8cbbaf0125c76ed549ff8b0b9e03e0c88cf90fa634fa7b12b47d77b694de488ace8d9a65967dc96df599727d3292a8d9d447709c97",
        ],
    ),
    (
        "sha3-224",
        [
            "6b4e03423667dbb73b6e15454f0eb1abd4597f9a1b078e3f5b5a6bc7",
            "8a24108b154ada21c9fd5574494479ba5c7e7ab76ef264ead0fcce33",
            "543e6868e1666c1a643630df77367ae5a62a85070a51c14cbf665cbc",
            "2461344b84416db8fe01c2a4966fea019590c231dd5724c1bfc26745",
        ],
    ),
    (
        "sha3-256",
        [
            "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a",
            "41c0dba2a9d6240849100376a8235e2c82e1b9998a999e21db32dd97496d3376",
            "916f6061fe879741ca6469b43971dfdb28b1a32dc36cb3254e812be27aad1d18",
            "8f3934e6f7a15698fe0f396b95d8c4440929a8fa6eae140171c068b4549fbf81",
        ],
    ),
    (
        "sha3-384",
        [
            "0c63a75b845e4f7d01107d852e4c2485c51a50aaaa94fc61995e71bbee983a2ac3713831264adb47fb6bd1e058d5f004",
            "991c665755eb3a4b6bbdfb75c78a492e8c56a22c5c4d7e429bfdbc32b9d4ad5aa04a1f076e62fea19eef51acd0657c22",
            "79407d3b5916b59c3e30b09822974791c313fb9ecc849e406f23592d04f625dc8c709b98b43b3852b337216179aa7fc7",
            "ccf4495ff20b4b33a1cc1917f9f0fe0fcb5e3d08e542cf4d4a90dd950b748e7e1cc07d2f3b36d62dd240724417cdd81b",
        ],
    ),
    (
        "sha3-512",
        [
            "a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26",
            "04a371e84ecfb5b8b77cb48610fca8182dd457ce6f326a0fd3d7ec2f1e91636dee691fbe0c985302ba1b0d8dc78c086346b533b49c030d99a27daf1139d6e75e",
            "afebb2ef542e6579c50cad06d2e578f9f8dd6881d7dc824d26360feebf18a4fa73e3261122948efcfd492e74e82e2189ed0fb440d187f382270cb455f21dd185",
            "ac7e95cc95aa7f24aaa95e040ca0c79b39cd9cc84a10abb84ddd8dd5e4b45cf96543aaa70d0ef99fbf8d2769639981ee1fd0b0276f4756b9d504d0b7de19b700",
        ],
    ),
];

#[test]
fn secure_hash_digests() {
    let mut cases = vec![(
        "(define-coding-system-alias 'raw-text 'no-conversion)".to_string(),
        "nil".to_string(),
    )];
    for (algorithm, digests) in HASH_DIGESTS {
        for (input, digest) in HASH_INPUTS.iter().zip(digests) {
            cases.push((
                format!("(secure-hash '{algorithm} {input})"),
                format!("\"{digest}\""),
            ));
        }
    }
    cases.push((
        "(secure-hash 'sha3-256 (make-string 136 ?a))".to_string(),
        "\"3fc5559f14db8e453a0a3091edbd2bc25e11528d81c66fa570a4efdcc2695ee1\"".to_string(),
    ));
    cases.push((
        "(secure-hash 'sha3-512 (make-string 72 ?a))".to_string(),
        "\"a8ae722a78e10cbbc413886c02eb5b369a03f6560084aff566bd597bb7ad8c1ccd86e81296852359bf2faddb5153c0a7445722987875e74287adac21adebe952\"".to_string(),
    ));
    let cases: Vec<(&str, &str)> = cases
        .iter()
        .map(|(s, e)| (s.as_str(), e.as_str()))
        .collect();
    check(&cases);
}

#[test]
fn secure_hash_objects() {
    check(&[
        (
            "(define-coding-system-alias 'raw-text 'no-conversion)",
            "nil",
        ),
        (
            "(secure-hash-algorithms)",
            "(md5 sha1 sha224 sha256 sha384 sha512 sha3-224 sha3-256 sha3-384 sha3-512)",
        ),
        ("(md5 \"foobar\")", "\"3858f62230ac3c915f300c664312c63f\""),
        (
            "(secure-hash 'md5 \"foobar\" 1 3)",
            "\"e47ca7a09cf6781e29634502345930a7\"",
        ),
        (
            "(equal (secure-hash 'sha1 \"foobar\" -3) (secure-hash 'sha1 \"bar\"))",
            "t",
        ),
        (
            "(secure-hash 'md5 \"foobar\" 3 10)",
            "error: (args-out-of-range \"foobar\" 3 10)",
        ),
        ("(length (secure-hash 'sha256 \"foobar\" nil nil t))", "32"),
        (
            "(multibyte-string-p (secure-hash 'md5 \"\" nil nil t))",
            "nil",
        ),
        ("(aref (secure-hash 'md5 \"\" nil nil t) 0)", "212"),
        (
            "(secure-hash 'foo \"x\")",
            "error: (error \"Invalid algorithm arg: foo\")",
        ),
        (
            "(secure-hash \"md5\" \"x\")",
            "error: (wrong-type-argument symbolp \"md5\")",
        ),
        (
            "(secure-hash 'md5 1)",
            "error: (error \"Invalid object argument\" 1)",
        ),
        (
            "(secure-hash 'md5 nil)",
            "error: (error \"Invalid object argument\" \"nil\")",
        ),
        (
            "(secure-hash 'md5 'iv-auto)",
            "error: (error \"Without a length, \u{2018}iv-auto\u{2019} can\u{2019}t be used; see Elisp manual\")",
        ),
        (
            "(let ((text-quoting-style 'grave)) (secure-hash 'md5 'iv-auto -1))",
            "error: (error \"Without a length, `iv-auto' can't be used; see Elisp manual\")",
        ),
        (
            "(let ((text-quoting-style 'straight)) (secure-hash 'md5 'iv-auto 'x))",
            "error: (error \"Without a length, 'iv-auto' can't be used; see Elisp manual\")",
        ),
        ("(length (secure-hash 'sha512 'iv-auto 100))", "128"),
        ("(length (secure-hash 'md5 'iv-auto 100 nil t))", "16"),
        (
            "(md5 \"\u{e9}\" nil nil 'no-such-cs)",
            "error: (coding-system-error no-such-cs)",
        ),
        (
            "(md5 \"\u{e9}\" nil nil 'no-such-cs t)",
            "\"66ddcd97cfdeabb2f6fb8a999b4bc76f\"",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"sh\")) (insert \"foobar\") (setq last-coding-system-used nil) (md5 (current-buffer)))",
            "\"3858f62230ac3c915f300c664312c63f\"",
        ),
        ("last-coding-system-used", "no-conversion"),
        (
            "(md5 (current-buffer) 2 4)",
            "\"e47ca7a09cf6781e29634502345930a7\"",
        ),
        (
            "(md5 (current-buffer) 4 2)",
            "\"e47ca7a09cf6781e29634502345930a7\"",
        ),
        (
            "(md5 (current-buffer) 0 4)",
            "error: (args-out-of-range 0 4)",
        ),
        (
            "(let ((coding-system-for-write 'bogus)) (md5 (current-buffer)))",
            "error: (coding-system-error bogus)",
        ),
        (
            "(let ((coding-system-for-write 'bogus)) (md5 (current-buffer) nil nil nil t))",
            "\"3858f62230ac3c915f300c664312c63f\"",
        ),
    ]);
}

#[test]
fn line_number_at_pos() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"lines\")) (insert \"one\\ntwo\\r\\nthree\\rfour\") nil)",
            "nil",
        ),
        ("(line-number-at-pos)", "3"),
        ("(line-number-at-pos 1)", "1"),
        ("(line-number-at-pos 4)", "1"),
        ("(line-number-at-pos 5)", "2"),
        ("(line-number-at-pos (point-max))", "3"),
        (
            "(line-number-at-pos 0)",
            "error: (args-out-of-range 0 1 20)",
        ),
        (
            "(line-number-at-pos 21)",
            "error: (args-out-of-range 21 1 20)",
        ),
        (
            "(line-number-at-pos 'x)",
            "error: (wrong-type-argument fixnump x)",
        ),
        (
            "(let ((selective-display t)) (line-number-at-pos (point-max)))",
            "5",
        ),
        (
            "(let ((selective-display 3)) (line-number-at-pos (point-max)))",
            "3",
        ),
        (
            "(progn (narrow-to-region 5 20) (line-number-at-pos 10))",
            "2",
        ),
        ("(line-number-at-pos 10 t)", "3"),
        ("(line-number-at-pos 2)", "1"),
        ("(line-number-at-pos 2 t)", "1"),
        ("(progn (widen) (line-number-at-pos (copy-marker 9)))", "2"),
    ]);
}

#[test]
fn sequences() {
    check(&[
        ("(length '(1 2 3))", "3"),
        ("(length \"héllo\")", "5"),
        ("(string-bytes \"héllo\")", "6"),
        ("(length [1 2])", "2"),
        ("(length '(1 . 2))", "error: (wrong-type-argument listp 2)"),
        ("(safe-length '(1 2 . 3))", "2"),
        ("(proper-list-p '(1 2))", "2"),
        ("(proper-list-p '(1 . 2))", "nil"),
        ("(length< '(1 2) 3)", "t"),
        ("(length= \"ab\" 2)", "t"),
        ("(nth 1 '(a b c))", "b"),
        ("(nth 5 '(a b c))", "nil"),
        ("(nthcdr 2 '(a b c))", "(c)"),
        ("(elt [a b] 1)", "b"),
        ("(reverse '(1 2 3))", "(3 2 1)"),
        ("(nreverse (list 1 2 3))", "(3 2 1)"),
        ("(reverse [1 2])", "[2 1]"),
        ("(reverse \"abc\")", "\"cba\""),
        ("(append '(1) '(2) 3)", "(1 2 . 3)"),
        ("(append [1 2] nil)", "(1 2)"),
        ("(append)", "nil"),
        ("(concat \"ab\" '(?c) [?d])", "\"abcd\""),
        ("(concat \"a\" \"é\")", "\"aé\""),
        ("(vconcat '(1) [2] \"a\")", "[1 2 97]"),
        ("(substring \"hello\" 1 3)", "\"el\""),
        ("(substring \"hello\" -3)", "\"llo\""),
        ("(substring [1 2 3] 1)", "[2 3]"),
        (
            "(substring \"hello\" 2 1)",
            "error: (args-out-of-range \"hello\" 2 1)",
        ),
        ("(copy-sequence '(1 2))", "(1 2)"),
        ("(take 2 '(1 2 3))", "(1 2)"),
        ("(take 0 '(1 2 3))", "nil"),
        ("(ntake 2 (list 1 2 3))", "(1 2)"),
        ("(nconc (list 1) nil (list 2 3))", "(1 2 3)"),
        ("(fillarray (make-vector 2 0) 7)", "[7 7]"),
    ]);
}

#[test]
fn membership() {
    check(&[
        ("(member \"b\" '(\"a\" \"b\"))", "(\"b\")"),
        ("(memq 'c '(a b))", "nil"),
        ("(memql 1.0 '(1 1.0))", "(1.0)"),
        ("(assq 'b '((a . 1) (b . 2)))", "(b . 2)"),
        ("(assoc \"b\" '((\"a\" . 1) (\"b\" . 2)))", "(\"b\" . 2)"),
        ("(assoc 2 '((1 . a) (3 . b)) #'<)", "(1 . a)"),
        ("(assoc 2 '((3 . a) (1 . b)) #'>)", "(3 . a)"),
        ("(rassq 2 '((a . 1) (b . 2)))", "(b . 2)"),
        ("(rassoc \"x\" '((a . \"x\")))", "(a . \"x\")"),
        ("(delq 'a (list 'a 'b 'a))", "(b)"),
        ("(delete 1 (list 1 2 1))", "(2)"),
        ("(delete \"a\" [\"a\" \"b\"])", "[\"b\"]"),
        ("(copy-alist '((a . 1)))", "((a . 1))"),
    ]);
}

#[test]
fn sorting() {
    check(&[
        ("(sort (list 3 1 2) #'<)", "(1 2 3)"),
        ("(sort '(3 1 2))", "(1 2 3)"),
        (
            "(sort [\"b\" \"a\"] :lessp #'string-lessp)",
            "[\"a\" \"b\"]",
        ),
        ("(sort '((2 . a) (1 . b)) :key #'car)", "((1 . b) (2 . a))"),
        ("(sort (list 1 3 2) :reverse t)", "(3 2 1)"),
        (
            "(sort '((1 . a) (0 . b) (1 . c)) :key #'car)",
            "((0 . b) (1 . a) (1 . c))",
        ),
        ("(value< 'a 'b)", "t"),
        ("(value< '(1 2) '(1 3))", "t"),
        ("(value< 1 \"a\")", "error: (type-mismatch 1 \"a\")"),
    ]);
}

#[test]
fn sxhash_values() {
    check(&[
        ("(sxhash-equal \"abc\")", "8059383"),
        ("(sxhash-equal 5)", "4"),
        ("(sxhash-eq 1)", "0"),
        ("(sxhash-equal '(1 2))", "22"),
        ("(sxhash-equal [1 2])", "662"),
        (
            "(progn (set-buffer (get-buffer-create \"sx\")) (insert \"abc\") (= (sxhash-equal (point-marker)) (sxhash-equal (point-marker))))",
            "t",
        ),
        ("(= (sxhash-eql 1.5) (sxhash-equal 1.5))", "t"),
        (
            "(= (sxhash-eql (expt 2 70)) (sxhash-equal (expt 2 70)))",
            "t",
        ),
        (
            "(= (sxhash-equal-including-properties (propertize \"a\" 'x 1)) (sxhash-equal \"a\"))",
            "nil",
        ),
        (
            "(= (sxhash-equal-including-properties \"a\") (sxhash-equal \"a\"))",
            "t",
        ),
    ]);
}

#[test]
fn equal_including_properties() {
    check(&[
        (
            "(equal-including-properties (concat (propertize \"abc\" 'a 1) (propertize \"de\" 'a 1)) (propertize \"abcde\" 'a 1))",
            "t",
        ),
        (
            "(equal-including-properties (propertize \"ab\" 'a 1 'b 2) (propertize \"ab\" 'b 2 'a 1))",
            "t",
        ),
        (
            "(equal-including-properties (propertize \"ab\" 'a \"x\") (propertize \"ab\" 'a \"x\"))",
            "t",
        ),
        (
            "(equal-including-properties (propertize \"ab\" 'a 1) \"ab\")",
            "nil",
        ),
        (
            "(equal-including-properties (concat \"a\" (propertize \"b\" 'a 1)) (propertize \"ab\" 'a 1))",
            "nil",
        ),
    ]);
}

#[test]
fn buffer_line_statistics() {
    check(&[
        (
            "(progn (set-buffer (get-buffer-create \"bls\")) (insert \"123\\n12345\\n123\") (buffer-line-statistics))",
            "(3 5 3.6666666666666665)",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"bls2\")) (buffer-line-statistics))",
            "(0 0 0.0)",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"bls3\")) (insert \"\\n\\n\\n\") (buffer-line-statistics (current-buffer)))",
            "(3 0 0.0)",
        ),
        (
            "(buffer-line-statistics \"no such\")",
            "error: (error \"No buffer named no such\")",
        ),
    ]);
}

#[test]
fn circular_and_dotted_lists() {
    check(&[
        (
            "(progn (fset 'cyc (function (lambda (x) (let ((l (list x x x))) (setcdr (cdr (cdr l)) l) l)))) (fset 'err (function (lambda (f) (condition-case e (funcall f) (error (car e)))))) 'cyc)",
            "cyc",
        ),
        (
            "(mapcar 'err (list (function (lambda () (memq 2 (cyc 1)))) (function (lambda () (member \"a\" (cyc \"b\")))) (function (lambda () (memql 2.0 (cyc 1.0)))) (function (lambda () (assq 2 (cyc '(1 . 2))))) (function (lambda () (assoc \"a\" (cyc '(\"b\" . 2))))) (function (lambda () (rassq 3 (cyc '(1 . 2))))) (function (lambda () (rassoc \"a\" (cyc '(1 . \"b\"))))) (function (lambda () (delq 2 (cyc 1)))) (function (lambda () (delete \"a\" (cyc \"b\")))) (function (lambda () (reverse (cyc 1)))) (function (lambda () (nconc (cyc 1) 'tail))) (function (lambda () (plist-put (cyc 1) 2 3))) (function (lambda () (plist-member (cyc 1) 2))) (function (lambda () (length (cyc 1))))))",
            "(circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list circular-list)",
        ),
        (
            "(list (consp (memql 1.0 (cyc 1.0))) (plist-get (cyc 1) 2) (proper-list-p (cyc 1)) (>= (safe-length (cyc 1)) 3) (length< (cyc 1) 5))",
            "(t nil nil t nil)",
        ),
        (
            "(progn (setplist 'cyc-plist (cyc 'a)) (get 'cyc-plist 'b))",
            "nil",
        ),
        ("(delq 1 '(1 . 2))", "error: (wrong-type-argument listp 2)"),
        ("(reverse '(1 . 2))", "error: (wrong-type-argument listp 2)"),
        (
            "(rassq 2 '((0 . 1) . x))",
            "error: (wrong-type-argument listp ((0 . 1) . x))",
        ),
        (
            "(assoc 2 '((1) . x) (function equal))",
            "error: (wrong-type-argument listp ((1) . x))",
        ),
        ("(nconc 1 '(2))", "error: (wrong-type-argument consp 1)"),
        (
            "(plist-member '(a 1 . b) 'c)",
            "error: (wrong-type-argument plistp (a 1 . b))",
        ),
        (
            "(plist-put '(a 1 b) 'c 2)",
            "error: (wrong-type-argument plistp (a 1 b))",
        ),
        (
            "(memql (expt 2 70) (list 1 (expt 2 70)))",
            "(1180591620717411303424)",
        ),
        (
            "(memq 3 '(1 . 2))",
            "error: (wrong-type-argument listp (1 . 2))",
        ),
        ("(length= '(1 2) 2)", "t"),
        ("(length> '(1 2 3) 2)", "t"),
        ("(proper-list-p '(1 2))", "2"),
        ("(proper-list-p '(1 . 2))", "nil"),
        ("(safe-length 'a)", "0"),
    ]);
}

#[test]
fn take_and_nthcdr() {
    check(&[
        ("(ntake 1 'a)", "error: (wrong-type-argument listp a)"),
        (
            "(ntake 2 '(a . b))",
            "error: (wrong-type-argument listp (a . b))",
        ),
        ("(ntake (- most-negative-fixnum 1) (list 1 2))", "nil"),
        ("(ntake (1+ most-positive-fixnum) (list 1 2))", "(1 2)"),
        ("(ntake 'x nil)", "error: (wrong-type-argument integerp x)"),
        ("(ntake 0 'a)", "nil"),
        ("(ntake 1 (list 1 2))", "(1)"),
        (
            "(nthcdr 2 '(1 . 2))",
            "error: (wrong-type-argument listp (1 . 2))",
        ),
        ("(nthcdr 5 '(1 2))", "nil"),
        ("(nthcdr -1 '(1))", "(1)"),
        ("(nthcdr (expt 2 70) '(1 2))", "nil"),
        (
            "(let ((l (list 1 2 3))) (setcdr (cdr (cdr l)) l) (car (nthcdr 1000 l)))",
            "2",
        ),
        (
            "(let ((l (list 1 2 3))) (setcdr (cdr (cdr l)) l) (list (car (nthcdr (expt 2 70) l)) (car (nthcdr (1+ (expt 2 70)) l))))",
            "(2 3)",
        ),
        ("(nthcdr 200 (make-list 150 1))", "nil"),
    ]);
}

#[test]
fn random_numbers() {
    check(&[
        ("(random -1)", "error: (args-out-of-range -1)"),
        ("(random 0)", "error: (args-out-of-range 0)"),
        (
            "(random (- (expt 2 70)))",
            "error: (args-out-of-range -1180591620717411303424)",
        ),
        ("(random 1)", "0"),
        ("(equal (random \"seed\") (random \"seed\"))", "t"),
        (
            "(let ((n (random (expt 2 70)))) (and (integerp n) (<= 0 n) (< n (expt 2 70))))",
            "t",
        ),
        (
            "(let ((n (random))) (and (integerp n) (<= most-negative-fixnum n most-positive-fixnum)))",
            "t",
        ),
        (
            "(progn (get-buffer-create \" rn\") (string-match \"\\\\` rn-[0-9]+\\\\'\" (generate-new-buffer-name \" rn\")))",
            "0",
        ),
    ]);
}

#[cfg(windows)]
#[test]
fn w32_random_generator() {
    let mut r = emacs_core::sysdep::Random::default();
    r.srandom(1);
    assert_eq!(r.random(), -1_415_428_051);
}

#[cfg(unix)]
#[test]
fn glibc_random_generator() {
    let mut r = emacs_core::sysdep::Random::default();
    r.srandom(1);
    assert_eq!(r.random(), 1_804_289_383);
    assert_eq!(r.random(), 846_930_886);
}

#[test]
fn value_ordering() {
    check(&[
        (
            "(list (value< (position-symbol 'a 1) (position-symbol 'b 2)) (value< (position-symbol 'a 1) (position-symbol 'a 2)))",
            "(t nil)",
        ),
        (
            "(value< (position-symbol 'a 1) 'b)",
            "error: (type-mismatch #<symbol a at 1> b)",
        ),
        (
            "(let ((symbols-with-pos-enabled t)) (list (value< (position-symbol 'a 1) 'b) (value< 'a (position-symbol 'b 1))))",
            "(t t)",
        ),
        (
            "(progn (set-buffer (get-buffer-create \"vo1\")) (insert \"abcdef\") (let ((other (get-buffer-create \"vo2\"))) (list (value< (copy-marker 2) (copy-marker 3)) (value< (make-marker) (copy-marker 3)) (value< (current-buffer) other) (value< other (current-buffer)) (value< (set-marker (make-marker) 1 other) (copy-marker 1)))))",
            "(t t t nil nil)",
        ),
        (
            "(let ((a (list 1 2 3)) (b (list 1 2 3))) (setcdr (cdr (cdr a)) a) (setcdr (cdr (cdr b)) b) (car (condition-case e (value< a b) (error e))))",
            "circular-list",
        ),
        ("(value< (make-hash-table) (make-hash-table))", "nil"),
        (
            "(value< [1] (record 'a))",
            "error: (type-mismatch [1] #s(a))",
        ),
        (
            "(list (value< 1 1.5) (value< 2.0 1) (value< 0.0e+NaN 1))",
            "(t nil nil)",
        ),
        (
            "(list (value< (expt 2 70) 1) (value< 1 (expt 2 70)) (value< 1.0 (expt 2 70)))",
            "(nil t t)",
        ),
        ("(value< '(1 . 2) '(1 . 3))", "t"),
        ("(value< '(1 2) 'a)", "error: (type-mismatch (1 2) a)"),
        ("(value< '(1) nil)", "nil"),
        ("(value< nil '(1))", "t"),
        (
            "(list (string-lessp \"\\200\" (string-to-multibyte \"\\200a\")) (string-lessp (string-to-multibyte \"\\200a\") \"\\200\") (value< \"\\200\" \"\\200\"))",
            "(t nil nil)",
        ),
    ]);
}

#[test]
fn equality() {
    check(&[
        ("(equal '(1 (2 \"x\")) '(1 (2 \"x\")))", "t"),
        ("(eq \"a\" \"a\")", "nil"),
        ("(eql 1.0 1.0)", "t"),
        ("(equal 0.0 -0.0)", "nil"),
        ("(eql 2.0 2)", "nil"),
        ("(equal [1 (2)] [1 (2)])", "t"),
        ("(equal (expt 2 70) (expt 2 70))", "t"),
        ("(eql (expt 2 70) (expt 2 70))", "t"),
        ("(eq (expt 2 70) (expt 2 70))", "nil"),
    ]);
}

#[test]
fn plists_and_mapping() {
    check(&[
        ("(plist-get '(a 1 b 2) 'b)", "2"),
        ("(plist-get '(\"a\" 1) \"a\" #'equal)", "1"),
        ("(plist-put (list 'a 1) 'b 2)", "(a 1 b 2)"),
        ("(plist-member '(a nil) 'a)", "(a nil)"),
        ("(mapcar #'1+ '(1 2 3))", "(2 3 4)"),
        ("(mapcar #'identity \"ab\")", "(97 98)"),
        ("(mapconcat #'identity '(\"a\" \"b\") \"-\")", "\"a-b\""),
        ("(mapconcat #'identity '(\"a\" \"b\"))", "\"ab\""),
        ("(mapcan #'list '(1 2))", "(1 2)"),
        ("(mapc #'ignore '(1 2))", "error: (void-function ignore)"),
    ]);
}

#[test]
fn hash_tables() {
    check(&[
        (
            "(let ((h (make-hash-table :test 'equal))) (puthash \"a\" 1 h) (gethash \"a\" h))",
            "1",
        ),
        (
            "(let ((h (make-hash-table))) (puthash 'a 1 h) (puthash 'b 2 h) (remhash 'a h) (hash-table-count h))",
            "1",
        ),
        (
            "(let ((h (make-hash-table))) (puthash 1 2 h) h)",
            "#s(hash-table data (1 2))",
        ),
        (
            "(let ((h (make-hash-table :test 'eq)) (r nil)) (puthash 'a 1 h) (puthash 'b 2 h) (maphash #'(lambda (k v) (setq r (cons k r))) h) r)",
            "(b a)",
        ),
        ("(gethash 'missing (make-hash-table) 'default)", "default"),
        ("(hash-table-test (make-hash-table :test 'equal))", "equal"),
        (
            "(make-hash-table :test 'bogus)",
            "error: (error \"Invalid hash table test\" bogus)",
        ),
        (
            "(read \"#s(hash-table test equal data (\\\"k\\\" 1))\")",
            "#s(hash-table test equal data (\"k\" 1))",
        ),
        (
            "(progn (define-hash-table-test 'ci #'(lambda (a b) (string-equal (downcase a) (downcase b))) #'(lambda (k) (sxhash-equal (downcase k)))) (let ((h (make-hash-table :test 'ci))) (puthash \"A\" 1 h) (gethash \"a\" h)))",
            "1",
        ),
    ]);
}

#[test]
fn strings() {
    check(&[
        ("(string-equal \"abc\" 'abc)", "t"),
        ("(string-lessp \"abc\" \"abd\")", "t"),
        ("(compare-strings \"abc\" nil nil \"abd\" nil nil)", "-3"),
        ("(compare-strings \"ABC\" nil nil \"abc\" nil nil t)", "t"),
        ("(string-version-lessp \"foo2\" \"foo10\")", "t"),
        ("(string-search \"lo\" \"hello\")", "3"),
        ("(string-search \"zz\" \"hello\")", "nil"),
        ("(string-distance \"kitten\" \"sitting\")", "3"),
        ("(base64-encode-string \"hello\")", "\"aGVsbG8=\""),
        ("(base64-decode-string \"aGVsbG8=\")", "\"hello\""),
        ("(base64url-encode-string \"hi?\" t)", "\"aGk_\""),
        ("(upcase \"abc\")", "\"ABC\""),
        ("(upcase ?a)", "65"),
        ("(capitalize \"hello world\")", "\"Hello World\""),
        ("(downcase \"ÀB\")", "\"àb\""),
        ("(string ?a ?b)", "\"ab\""),
        ("(string-width \"日本\")", "2"),
        (
            "(progn (set-char-table-range char-width-table '(#x4E00 . #x9FFF) 2) (string-width \"日本\"))",
            "4",
        ),
        ("(char-width ?a)", "1"),
        ("(char-width ?\\t)", "8"),
        ("(char-width ?\\n)", "0"),
        ("(char-width ?\\C-a)", "2"),
        ("(char-width #x80)", "4"),
        ("(multibyte-string-p \"é\")", "t"),
        ("(multibyte-string-p \"e\")", "nil"),
        ("(string-to-multibyte \"\\351\")", "\"\\351\""),
        ("(string-as-unibyte \"é\")", "\"\\303\\251\""),
        ("(string-make-unibyte \"é\")", "\"\\351\""),
    ]);
}

#[test]
fn char_tables() {
    check(&[
        (
            "(let ((ct (make-char-table 'test))) (set-char-table-range ct '(?a . ?z) 'lower) (list (aref ct ?m) (aref ct ?A)))",
            "(lower nil)",
        ),
        (
            "(let ((ct (make-char-table 'test 0))) (aset ct ?x 1) (char-table-range ct ?x))",
            "1",
        ),
        (
            "(let ((ct (make-char-table 'test)) (r nil)) (set-char-table-range ct '(?a . ?c) 1) (map-char-table #'(lambda (k v) (setq r (cons k r))) ct) r)",
            "((97 . 99))",
        ),
        (
            "(let* ((p (make-char-table 'test)) (c (make-char-table 'test))) (aset p ?q 'from-parent) (set-char-table-parent c p) (aref c ?q))",
            "from-parent",
        ),
        (
            "(let ((ct (make-char-table 'test))) (set-char-table-range ct '(#x10000 . #x10010) 'astral) (aref ct #x10005))",
            "astral",
        ),
    ]);
}

#[test]
fn features() {
    check(&[
        ("(progn (provide 'my-feature) (featurep 'my-feature))", "t"),
        ("(featurep 'not-a-feature)", "nil"),
        ("(require 'my-feature)", "my-feature"),
    ]);
}

#[test]
fn printing() {
    check(&[
        ("(prin1-to-string 'a)", "\"a\""),
        ("(prin1-to-string \"x\" t)", "\"x\""),
        (
            "(let ((print-length 2)) (prin1-to-string '(1 2 3)))",
            "\"(1 2 ...)\"",
        ),
        (
            "(let ((print-level 1)) (prin1-to-string '(1 (2))))",
            "\"(1 ...)\"",
        ),
        (
            "(let ((print-circle t) (x (list 1))) (prin1-to-string (list x x)))",
            "\"(#1=(1) #1#)\"",
        ),
        (
            "(error-message-string '(wrong-type-argument listp 1))",
            "\"Wrong type argument: listp, 1\"",
        ),
        ("(error-message-string '(error \"Foo\" 1))", "\"Foo: 1\""),
        (
            "(error-message-string '(end-of-file))",
            "\"End of file during parsing\"",
        ),
        ("(prin1-to-string (make-symbol \"g\"))", "\"g\""),
        (
            "(let ((print-gensym t)) (prin1-to-string (make-symbol \"g\")))",
            "\"#:g\"",
        ),
        ("(prin1-to-string '(quote x))", "\"'x\""),
        ("(prin1-to-string 1.0e+INF)", "\"1.0e+INF\""),
        (
            "(let ((float-output-format \"%.3f\")) (prin1-to-string 1.5))",
            "\"1.500\"",
        ),
    ]);
}

#[test]
fn equal_cycles() {
    check(&[
        (
            "(let ((a (list 1 2)) (b (list 1 2))) (setcdr (cdr a) a) (setcdr (cdr b) b) (equal a b))",
            "t",
        ),
        (
            "(let ((a (list nil)) (b (list nil))) (setcar a a) (setcar b b) (equal a b))",
            "t",
        ),
        (
            "(let ((a (list 1))) (setcdr a a) (equal a '(1 1 1)))",
            "nil",
        ),
        (
            "(let ((a (list 1 2)) (b (list 1 3))) (setcdr (cdr a) a) (setcdr (cdr b) b) (equal a b))",
            "nil",
        ),
    ]);
}
