mod common;

use common::{check, eval, on_big_stack};
use emacs_core::lisp::Lisp;

#[test]
fn special_forms() {
    check(&[
        ("(quote a)", "a"),
        ("(if nil 1 2 3)", "3"),
        ("(if t 1)", "1"),
        ("(if nil 1)", "nil"),
        ("(cond (nil 1) (2) (t 3))", "2"),
        ("(cond)", "nil"),
        ("(and)", "t"),
        ("(and 1 2)", "2"),
        ("(and 1 nil 2)", "nil"),
        ("(or nil 3)", "3"),
        ("(or)", "nil"),
        ("(progn 1 2 3)", "3"),
        ("(progn)", "nil"),
        ("(prog1 1 2 3)", "1"),
        ("(setq x 1 y 2)", "2"),
        ("(list x y)", "(1 2)"),
        ("(setq z)", "error: (wrong-number-of-arguments setq 1)"),
        ("(quote a b)", "error: (wrong-number-of-arguments quote 2)"),
    ]);
}

#[test]
fn dynamic_binding() {
    check(&[
        ("(let ((a 1) (b 2)) (+ a b))", "3"),
        ("(let* ((a 1) (b (1+ a))) b)", "2"),
        ("(let ((a 1)) (let ((a 2)) (symbol-value 'a)))", "2"),
        ("(let ((n 0)) (while (< n 5) (setq n (1+ n))) n)", "5"),
        ("(let (q) q)", "nil"),
        (
            "(let ((a 1 2)) a)",
            "error: (error \"`let' bindings can have only one value-form\" a 1 2)",
        ),
        ("(progn (setq g 1) (let ((g 2)) g))", "2"),
        ("g", "1"),
    ]);
}

#[test]
fn function_calls() {
    check(&[
        (
            "(funcall #'(lambda (a &optional b &rest c) (list a b c)) 1 2 3 4)",
            "(1 2 (3 4))",
        ),
        (
            "(funcall #'(lambda (a &optional b) (list a b)) 1)",
            "(1 nil)",
        ),
        ("(apply #'+ 1 2 '(3 4))", "10"),
        ("(apply '(+ 1 2))", "3"),
        ("(funcall 'list)", "nil"),
        ("(car)", "error: (wrong-number-of-arguments car 0)"),
        (
            "(funcall 'car)",
            "error: (wrong-number-of-arguments #<subr car> 0)",
        ),
        ("(car 1)", "error: (wrong-type-argument listp 1)"),
        ("(undefined-fn)", "error: (void-function undefined-fn)"),
        ("undefined-var", "error: (void-variable undefined-var)"),
        ("(funcall 'if t 1)", "error: (invalid-function #<subr if>)"),
        ("(defalias 'my-add #'(lambda (a b) (+ a b)))", "my-add"),
        ("(my-add 2 3)", "5"),
        ("(func-arity 'car)", "(1 . 1)"),
        ("(func-arity 'list)", "(0 . many)"),
        ("(func-arity 'if)", "(2 . unevalled)"),
        ("(func-arity #'(lambda (a &optional b) nil))", "(1 . 2)"),
        ("(functionp 'car)", "t"),
        ("(functionp 'if)", "nil"),
        ("(functionp #'(lambda ()))", "t"),
    ]);
}

#[test]
fn nonlocal_exits() {
    check(&[
        ("(catch 'tag (throw 'tag 42) 1)", "42"),
        ("(catch 'outer (catch 'inner (throw 'outer 1)) 2)", "1"),
        ("(throw 'nope 1)", "error: (no-catch nope 1)"),
        (
            "(condition-case err (car 1) (wrong-type-argument (list 'caught err)))",
            "(caught (wrong-type-argument listp 1))",
        ),
        (
            "(condition-case err (car 1) (error (car err)))",
            "wrong-type-argument",
        ),
        (
            "(condition-case nil (car 1) ((void-variable arith-error) 1) (error 2))",
            "2",
        ),
        ("(condition-case nil 5 (:success 'ok))", "ok"),
        ("(condition-case v (+ 1 2) (:success (* v 10)))", "30"),
        (
            "(let ((x 0)) (catch 'done (unwind-protect (throw 'done 1) (setq x 5))) x)",
            "5",
        ),
        ("(signal 'arith-error '(1 2))", "error: (arith-error 1 2)"),
        (
            "(condition-case e (signal 'quit nil) (quit 'quitted))",
            "quitted",
        ),
        (
            "(condition-case e (signal 'no-such-error nil) (error e))",
            "(error \"Invalid error symbol\" no-such-error)",
        ),
        (
            "(let ((log nil)) (condition-case nil (handler-bind-1 #'(lambda () (car 1)) '(error) #'(lambda (e) (setq log e))) (error log)))",
            "(wrong-type-argument listp 1)",
        ),
    ]);
}

#[test]
fn macros() {
    check(&[
        (
            "(defalias 'my-when (cons 'macro #'(lambda (c &rest body) (list 'if c (cons 'progn body)))))",
            "my-when",
        ),
        ("(my-when t 1 2)", "2"),
        ("(my-when nil 1 2)", "nil"),
        ("(macroexpand '(my-when a b))", "(if a (progn b))"),
        ("(macroexpand '(not-a-macro a))", "(not-a-macro a)"),
    ]);
}

#[test]
fn lexical_closures() {
    check(&[
        ("(funcall (eval '(let ((x 1)) #'(lambda () x)) t))", "1"),
        (
            "(eval '(let ((x 1)) #'(lambda () x)) t)",
            "#[nil (x) ((x . 1) t)]",
        ),
        (
            "(let ((f (eval '(let ((n 0)) #'(lambda () (setq n (1+ n)))) t))) (funcall f) (funcall f))",
            "2",
        ),
        ("(eval '(let ((x 1)) (let ((x 2)) x)) t)", "2"),
        ("(eval '(let* ((x 1) (y x)) y) t)", "1"),
        (
            "(eval '(condition-case e (car 1) (error (cdr e))) t)",
            "(listp 1)",
        ),
        (
            "(funcall (eval '(let ((f nil)) (setq f #'(lambda (n) (if (= n 0) 0 (1+ (funcall f (1- n)))))) f) t) 100)",
            "100",
        ),
    ]);
}

#[test]
fn variables() {
    check(&[
        ("(defvar my-var 10)", "my-var"),
        ("my-var", "10"),
        ("(defvar my-var 20)", "my-var"),
        ("my-var", "10"),
        ("(special-variable-p 'my-var)", "t"),
        ("(defconst my-c 3)", "my-c"),
        ("my-c", "3"),
        ("(setq nil 1)", "error: (setting-constant nil)"),
        ("(set :kw 1)", "error: (setting-constant :kw)"),
        ("(set :kw :kw)", ":kw"),
        (
            "(progn (defvar v1 1) (let ((v1 2)) (default-value 'v1)))",
            "2",
        ),
        (
            "(progn (defvar base-v 5) (defvaralias 'alias-v 'base-v) alias-v)",
            "5",
        ),
        ("(progn (setq alias-v 7) base-v)", "7"),
        (
            "(progn (defvar watched 1) (setq wlog nil) (add-variable-watcher 'watched #'(lambda (s n o w) (setq wlog (list s n o)))) (setq watched 2) wlog)",
            "(watched 2 set)",
        ),
        ("(makunbound 'my-var)", "my-var"),
        ("(boundp 'my-var)", "nil"),
    ]);
}

#[test]
fn hooks() {
    check(&[
        (
            "(progn (setq my-hook (list #'(lambda () (setq ran t)))) (run-hooks 'my-hook) ran)",
            "t",
        ),
        (
            "(progn (setq h2 (list #'(lambda (x) nil) #'(lambda (x) (* x 2)))) (run-hook-with-args-until-success 'h2 21))",
            "42",
        ),
    ]);
}

#[test]
fn deep_recursion_is_reported() {
    check(&[
        ("(defalias 'inf #'(lambda () (inf)))", "inf"),
        (
            "(condition-case e (inf) (excessive-lisp-nesting 'deep))",
            "deep",
        ),
        ("(+ 1 1)", "2"),
    ]);
}

#[test]
fn caught_errors_stay_rooted() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        let flow = lisp
            .eval_string("(signal 'error (list (concat \"a\" \"b\")))")
            .expect_err("signal");
        let condition = lisp.flow_condition(&flow);
        lisp.garbage_collect();
        assert!(lisp.heap.is_live(condition));
        assert_eq!(lisp.prin1_to_string(condition), "(error \"ab\")");
        let live = lisp
            .condition_case_internal(
                emacs_core::symbols::ERROR,
                |l| {
                    let s = l.string("cd");
                    Err(l.signal_list(emacs_core::symbols::ERROR, &[s]))
                },
                |l, err| {
                    l.garbage_collect();
                    let live = l.heap.is_live(err) && l.heap.is_live(l.xcdr(err));
                    Ok(l.bool(live))
                },
            )
            .expect("handled");
        assert_eq!(lisp.prin1_to_string(live), "t");
    });
}

#[test]
fn nonlocal_exits_survive_collection() {
    on_big_stack(|| {
        let mut lisp = Lisp::new();
        lisp.gc_stress = true;
        for (source, expected) in [
            (
                "(condition-case e (unwind-protect (signal 'error (list (concat \"a\" \"b\"))) (make-list 100 (concat \"x\" \"y\"))) (error (car (cdr e))))",
                "\"ab\"",
            ),
            (
                "(catch 'tag (unwind-protect (throw 'tag (concat \"c\" \"d\")) (condition-case nil (signal 'error (list (concat \"e\" \"f\"))) (error nil))))",
                "\"cd\"",
            ),
            (
                "(condition-case e (unwind-protect (signal 'error (list (concat \"g\" \"h\"))) (condition-case nil (signal 'error (list 1)) (error (make-list 10 1)))) (error (car (cdr e))))",
                "\"gh\"",
            ),
        ] {
            assert_eq!(eval(&mut lisp, source), expected, "evaluating {source}");
        }
        let level = lisp.flow_roots.len();
        for _ in 0..50 {
            eval(
                &mut lisp,
                "(condition-case nil (signal 'error (list 1)) (error nil))",
            );
            eval(&mut lisp, "(catch 'x (throw 'x 1))");
        }
        assert_eq!(lisp.flow_roots.len(), level);
    });
}

#[test]
fn collection_during_evaluation_keeps_live_data() {
    check(&[
        (
            "(let ((i 0) (l nil)) (while (< i 2000) (setq l (cons (make-string 10 ?a) l)) (setq i (1+ i))) (garbage-collect) (length l))",
            "2000",
        ),
        (
            "(let ((v (make-vector 3 nil))) (aset v 0 (list 1 2)) (garbage-collect) (aref v 0))",
            "(1 2)",
        ),
    ]);
}
