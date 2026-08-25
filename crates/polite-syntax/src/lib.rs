//! Words in, sentence tree out.
//!
//! The lexer and parser are hand-written (spec 9.5), and the statement grammar comes from the
//! vocabulary table rather than from code (spec 4.1), so growing the language does not mean
//! growing this crate.

#![forbid(unsafe_code)]

pub mod ast;
pub mod hidden;
pub mod intern;
pub mod lex;
pub mod parse;

pub use ast::*;
pub use intern::{Interner, Sym};
pub use parse::{parse, Parsed};

#[cfg(test)]
mod tests {
    use super::*;
    use polite_vocab::Vocabulary;

    fn ok(src: &str) -> Ast {
        let v = Vocabulary::embedded();
        let p = parse(src, &v);
        assert!(
            p.problems.is_empty(),
            "unexpected problems: {:?}",
            p.problems.iter().map(|d| &d.title).collect::<Vec<_>>()
        );
        p.ast
    }

    fn problems(src: &str) -> Vec<String> {
        let v = Vocabulary::embedded();
        let p = parse(src, &v);
        p.problems.into_iter().map(|d| d.title).collect()
    }

    #[test]
    fn a_single_polite_request_parses() {
        let ast = ok("please show 1\n");
        assert_eq!(ast.block(ast.top).len(), 1);
    }

    #[test]
    fn every_courtesy_opener_means_the_same_thing() {
        for opener in [
            "please",
            "kindly",
            "would you",
            "would you please",
            "would you kindly",
            "if you would",
            "if you would please",
            "if you would be so kind",
            "if you would be so kind as to",
            "if it is not too much trouble",
        ] {
            let src = format!("{opener} show 1\n");
            let ast = ok(&src);
            assert_eq!(ast.block(ast.top).len(), 1, "opener {opener:?} did not work");
        }
    }

    #[test]
    fn rudeness_at_the_top_is_pointed_out_kindly() {
        let p = problems("show 1\n");
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("please"));
        assert!(polite_diag::find_blame_word(&p[0]).is_none());
    }

    /// Spec rule 3: one courtesy word covers everything inside the block it opened.
    #[test]
    fn inside_a_block_you_only_ask_once() {
        ok("please repeat 3 times:\n    show 1\n    show 2\nthanks\n");
    }

    #[test]
    fn being_extra_polite_inside_is_never_a_problem() {
        ok("please repeat 3 times:\n    please show 1\n    kindly show 2\nthanks\n");
    }

    /// Spec rule 2: indentation is purely cosmetic.
    #[test]
    fn badly_indented_code_still_parses() {
        let a = ok("please repeat 3 times:\n    show 1\nthanks\n");
        let b = ok("please repeat 3 times:\nshow 1\n            thanks\n");
        assert_eq!(a.stmts.len(), b.stmts.len());
    }

    /// Spec 4.1: six ways to loop, all landing on the same form.
    #[test]
    fn every_loop_phrasing_reaches_the_same_form() {
        use polite_vocab::Form;
        for (src, form) in [
            ("please repeat 3 times:\nshow 1\nthanks\n", Form::LoopCount),
            ("please do this 3 times:\nshow 1\nthanks\n", Form::LoopCount),
            (
                "please repeat while score is under 3:\nshow 1\nthanks\n",
                Form::LoopWhile,
            ),
            (
                "please keep going while score is under 3:\nshow 1\nthanks\n",
                Form::LoopWhile,
            ),
            (
                "please repeat until score is over 3:\nshow 1\nthanks\n",
                Form::LoopUntil,
            ),
            ("please keep going forever:\nshow 1\nthanks\n", Form::LoopForever),
            ("please repeat forever:\nshow 1\nthanks\n", Form::LoopForever),
            (
                "please for every n in names:\nshow n\nthanks\n",
                Form::LoopEach,
            ),
            (
                "please repeat for every n from 1 to 10:\nshow n\nthanks\n",
                Form::LoopRange,
            ),
            (
                "please count n from 1 to 10:\nshow n\nthanks\n",
                Form::LoopRange,
            ),
        ] {
            let ast = ok(src);
            let first = ast.block(ast.top)[0];
            match ast.stmt(first).kind {
                StmtKind::Form { form: f, .. } => assert_eq!(f, form, "for {src:?}"),
                other => panic!("expected a loop for {src:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn a_check_collects_all_its_arms() {
        let ast = ok(
            "please check if n is 1:\n    show 1\notherwise if n is 2:\n    show 2\notherwise:\n    show 3\nthanks\n",
        );
        let first = ast.block(ast.top)[0];
        match ast.stmt(first).kind {
            StmtKind::Check { arms } => assert_eq!(arms.len, 3),
            other => panic!("expected a check, got {other:?}"),
        }
    }

    #[test]
    fn an_action_becomes_a_word_you_can_use() {
        let ast = ok("please define greet with a name:\n    give back name\nthanks\n\nplease greet with \"you\"\n");
        assert_eq!(ast.actions.len(), 1);
        let last = *ast.block(ast.top).last().unwrap();
        assert!(matches!(ast.stmt(last).kind, StmtKind::Call { .. }));
    }

    #[test]
    fn an_action_may_be_called_before_it_is_defined() {
        ok("please play\n\nplease define play:\n    show 1\nthanks\n");
    }

    #[test]
    fn action_names_may_be_several_words() {
        let ast = ok("please define load the score:\n    give back 1\nthanks\n\nplease load the score\n");
        assert_eq!(ast.actions.len(), 1);
        assert_eq!(ast.name_of(ast.actions[0].name), "load the score");
    }

    /// Spec 3.10: a phrase never swallows a following `or`, so a fallback attaches to the whole
    /// phrase rather than to its last hole.
    #[test]
    fn a_fallback_attaches_outside_the_phrase() {
        let ast = ok("please remember first is item 1 of names or \"nobody\"\n");
        let first = ast.block(ast.top)[0];
        let args = match ast.stmt(first).kind {
            StmtKind::Form { args, .. } => args,
            other => panic!("expected remember, got {other:?}"),
        };
        let value = ast.arg_slice(args)[0];
        assert!(matches!(
            ast.expr(value).kind,
            ExprKind::Binary { op: BinOp::Or, .. }
        ));
    }

    /// Spec 3.10: inside an argument list, `and` separates arguments.
    #[test]
    fn and_separates_arguments_rather_than_joining_them() {
        let ast = ok(
            "please define greet with a name and a greeting:\n    give back name\nthanks\n\nplease greet with \"a\" and \"b\"\n",
        );
        let last = *ast.block(ast.top).last().unwrap();
        match ast.stmt(last).kind {
            StmtKind::Call { args, .. } => assert_eq!(args.len, 2),
            other => panic!("expected a call, got {other:?}"),
        }
    }

    #[test]
    fn text_pulls_values_in_through_braces() {
        let ast = ok("please show \"got it in {guesses} guesses\"\n");
        let first = ast.block(ast.top)[0];
        let args = match ast.stmt(first).kind {
            StmtKind::Form { args, .. } => args,
            other => panic!("expected show, got {other:?}"),
        };
        let value = ast.arg_slice(args)[0];
        match ast.expr(value).kind {
            ExprKind::Interp(range) => assert_eq!(range.len, 3),
            other => panic!("expected text with values in it, got {other:?}"),
        }
    }

    #[test]
    fn a_named_closer_that_does_not_match_is_reported() {
        let p = problems("please repeat 3 times:\n    show 1\nthank you for checking\n");
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("loop"));
    }

    #[test]
    fn a_matching_named_closer_is_accepted() {
        ok("please repeat 3 times:\n    show 1\nthank you for repeating\n");
        ok("please check if 1 is 1:\n    show 1\nthank you for checking\n");
    }

    #[test]
    fn an_unclosed_block_is_explained() {
        let p = problems("please repeat 3 times:\n    show 1\n");
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("never closed"));
    }

    #[test]
    fn a_misspelled_word_gets_a_suggestion() {
        let v = Vocabulary::embedded();
        let p = parse("please dispay 1\n", &v);
        assert_eq!(p.problems.len(), 1);
        let s = p.problems[0].suggestion.as_ref().expect("a suggestion");
        assert!(s.lead.contains("display"), "got {:?}", s.lead);
    }

    #[test]
    fn no_message_this_crate_produces_ever_blames() {
        let sources = [
            "show 1\n",
            "please dispay 1\n",
            "please repeat 3 times:\n    show 1\n",
            "please show \"unclosed\n",
            "please show 1 @ 2\n",
            "thanks\n",
            "please show\n",
            "please define:\n    show 1\nthanks\n",
            "please try to:\n    show 1\nthanks\n",
            "otherwise:\n    show 1\nthanks\n",
        ];
        let v = Vocabulary::embedded();
        for src in sources {
            for d in parse(src, &v).problems {
                let whole = format!(
                    "{} {} {}",
                    d.title,
                    d.because.clone().unwrap_or_default(),
                    d.suggestion.map(|s| s.lead).unwrap_or_default()
                );
                assert!(
                    polite_diag::find_blame_word(&whole).is_none(),
                    "a message blames the reader: {whole:?}"
                );
            }
        }
    }

    #[test]
    fn a_try_needs_its_other_half() {
        let p = problems("please try to:\n    show 1\nthanks\n");
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("does not work out"));
    }

    #[test]
    fn a_full_try_parses() {
        ok("please try to:\n    show the contents of \"x.txt\"\notherwise if it does not work out:\n    show \"oh dear\"\nthanks\n");
    }

    #[test]
    fn i_am_sure_is_understood() {
        let ast = ok("please remember c is the contents of \"x.txt\", I am sure\n");
        let first = ast.block(ast.top)[0];
        let args = match ast.stmt(first).kind {
            StmtKind::Form { args, .. } => args,
            other => panic!("expected remember, got {other:?}"),
        };
        assert!(matches!(
            ast.expr(ast.arg_slice(args)[0]).kind,
            ExprKind::Sure { .. }
        ));
    }

    #[test]
    fn infix_phrases_work_after_a_value() {
        ok("please check if names contains \"a\":\n    show 1\nthanks\n");
        ok("please check if 15 divides evenly by 3:\n    show 1\nthanks\n");
    }

    #[test]
    fn the_tree_stays_small() {
        // Spec 10.4 budget: about 2 KB per line. This is a coarse guard on the shape of the
        // arenas, not a benchmark.
        let mut src = String::new();
        for i in 0..500 {
            src.push_str(&format!("please show {i}\n"));
        }
        let ast = ok(&src);
        let per_line = ast.footprint_bytes() / 500;
        assert!(per_line < 2048, "{per_line} bytes per line is too much");
    }
}
