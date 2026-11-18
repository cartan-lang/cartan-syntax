use cartan_syntax::ast::{self, ItemKind};
use cartan_syntax::selection::{select, SelectionError, Selector, SelectorError};

#[test]
fn selectors_parse_bindings_update_blocks_and_the_update_alias() {
    assert_eq!(Selector::parse("update"), Ok(Selector::Update(0)));
    assert_eq!(Selector::parse("update:12"), Ok(Selector::Update(12)));
    assert_eq!(
        Selector::parse("energy"),
        Ok(Selector::Binding("energy".into()))
    );
    assert_eq!(Selector::default(), Selector::Binding("main".into()));
    assert_eq!(Selector::Update(0).to_string(), "update:0");
    for written in [
        "",
        "update:",
        "update:x",
        "update:-1",
        "update: 1",
        "a=b",
        "a b",
        "x:1",
    ] {
        assert_eq!(
            Selector::parse(written),
            Err(SelectorError::Malformed {
                written: written.into()
            }),
            "{written:?}"
        );
    }
}

#[test]
fn selection_resolves_one_root_item_by_written_name_or_whole_update_block() {
    let text = "param n = 4\nstate u = 0\n[a, b] = [1, 2]\nf(x) = x\nmain = n\n\
                step {\n  u <- u + 1\n  v <- u\n}\nstep { u <- 0 }\n";
    let document = ast::parse_document(text).unwrap();
    let source = |selector: &str| {
        let item = select(&document, &Selector::parse(selector).unwrap()).unwrap();
        &text[item.span.start..item.span.end]
    };
    assert_eq!(source("main"), "main = n");
    assert_eq!(source("n"), "param n = 4");
    assert_eq!(source("b"), "[a, b] = [1, 2]");
    assert_eq!(source("f"), "f(x) = x");
    assert_eq!(source("update"), "step {\n  u <- u + 1\n  v <- u\n}");
    assert_eq!(source("update:1"), "step { u <- 0 }");
    let first = select(&document, &Selector::Update(0)).unwrap();
    let ItemKind::Step(block) = &first.kind else {
        panic!("expected an update block");
    };
    assert_eq!(
        &text[block.body_span.start..block.body_span.end],
        "{\n  u <- u + 1\n  v <- u\n}"
    );
    let [u, v] = block.writes.as_slice() else {
        panic!("expected two writes");
    };
    assert_eq!(u.target.text, "u");
    assert_eq!(v.target.text, "v");
    assert_eq!(&text[u.value.span.start..u.value.span.end], "u + 1");
    assert_eq!(&text[v.span.start..v.span.end], "v <- u");

    assert_eq!(
        select(&document, &Selector::Update(2)),
        Err(SelectionError::UnknownUpdate { index: 2, count: 2 })
    );
    let Err(SelectionError::UnknownBinding { name, available }) =
        select(&document, &Selector::Binding("u2".into()))
    else {
        panic!()
    };
    assert_eq!(name, "u2");
    assert_eq!(available, ["n", "u", "a", "b", "f", "main"]);

    let text = "main = 1\nmain = 2\n";
    let document = ast::parse_document(text).unwrap();
    let Err(SelectionError::AmbiguousBinding { candidates, .. }) =
        select(&document, &Selector::default())
    else {
        panic!()
    };
    assert_eq!(candidates.len(), 2);
    assert_eq!(&text[candidates[1].start..candidates[1].end], "main = 2");
}
