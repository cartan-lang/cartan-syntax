/**
 * tree-sitter-cartan — a grammar for cartan documents (`.cart`).
 *
 * **This file is generated.** The standalone `cartan-syntax` crate
 * prints it from `src/grammar.rs`, the same grammar its concrete
 * syntax parser reads directly; edit that Rust source. The crate's
 * tests generate the parser from the same value and compare its
 * trees with the interpreter's on one-line fixtures, which is
 * evidence of agreement on those fixtures and no more.
 *
 * An item is a binding, a port, a function,
 * an update block, a use, or a bare expression standing alone — which is
 * the spelling of `main = expr` — and every composite expression reads in four
 * zones — `head(constitutive) : config { payload } with control`
 * — in that fixed order and any number of turns.
 *
 * A newline is a token here, and separates wherever a comma does,
 * at every depth. A line continues where it has not finished
 * speaking — after an operator, an opener or a keyword wanting an
 * expression, newlines are allowed — and a line opening with an
 * infix operator continues the line above. The external scanner,
 * `src/scanner.c`, reads both rules: it returns a newline only
 * where the parser can take a separator and the next line opens
 * with no continuation, and the newline is whitespace elsewhere.
 * A token that can begin an expression is no continuation, so
 * `[1\n -2\n 3]` keeps its three items and `[0..4\n ..8]` its two
 * axes. A word
 * operator opening a line — `with`, `but`, `and`, `or` — continues
 * it only with whitespace after the word, which is what keeps
 * `without = 2` on a fresh line a binding.
 */

module.exports = grammar({
  name: "cartan",

  extras: ($) => [/[ \t\r]+/, $.comment, /\n/],

  externals: ($) => [$._newline, $._no_operand, $._bare_operand, $._error_sentinel],

  word: ($) => $.identifier,

  conflicts: ($) => [
    [$.texture_expression, $._expression],
    [$.range_from_header, $.header_open_range],
    [$.range_to_header, $.header_open_range],
    [$._config_atom, $._expression],
    [$._expression, $._expression_key],
    [$._expression, $._pattern],
    [$._expression, $.map_sub_pattern],
    [$._expression, $._pattern, $.map_sub_pattern],
    [$.parameter, $.constructor_pattern],
    [$._expression, $.component_key],
    [$._expression, $.component_key, $.position_pair],
    [$.fiber_type, $._pattern],
    [$.fiber_type, $._expression],
    [$.fiber_type, $._expression, $._pattern],
    [$._expression, $._pattern, $.map_entry],
    [$._expression, $._pattern, $.map_sub_pattern, $.map_entry],
    [$.parenthesized_expression, $.computed_key],
    [$.variant_argument, $._pattern],
    [$._expression, $.write],
    [$.range_to, $.open_range],
    [$._expression, $.map_entry],
    [$.fiber_type, $.signature_type],
    [$.variant, $.fiber_type],
  ],

  rules: {
    // items separated by newlines alone; an item that does not parse becomes an error node that ends at a line's end after its diagnostic, where its brackets are closed or the next line starts no further in than it, and the next item is read
    source_file: ($) =>
      optional(
        seq($._item_line, repeat(seq(repeat1($._newline), $._item_line))),
      ),

    comment: (_) => token(seq("#", /[^\n]*/)),

    _item_line: ($) => $._item,

    _item: ($) =>
      choice(
        $.doc_declaration,
        $.use_declaration,
        $.namespace_definition,
        $.port_declaration,
        $.fiber_declaration,
        $.struct_declaration,
        $.step_block,
        $.require_statement,
        $.bare_expression,
        $.pattern_binding,
        $.function_definition,
        $.binding,
      ),

    // `"""prose"""` standing alone — a doc string. It
    // is a declaration rather than a binding: no name, no node in the
    // graph, no part in evaluation. Standing first it is the document's
    // prose; standing above a binding, a function, a fiber or a struct
    // it is that declaration's prose. Which one it is is positional, so
    // the grammar accepts the literal as an item anywhere and the source
    // tree places it, refusing one standing above anything else.
    //
    // A triple string is an expression too, so a document's first line
    // could be read as the bare item `main = """prose"""`
    // ; the precedence here is the statement that a
    // triple string standing alone as an item is the prose, and the
    // interpreter, reading the alternatives in order, states the same
    // by taking this one first.
    doc_declaration: ($) => prec(1, field("text", $.triple_string)),

    // `use "fig.cart" as fig`, `use "fig.cart"::{panel}`, `use std::*`,
    // `use std::{slider}`, `use std::slider` and `use "hydro.cart" as
    // ideal with (pressure: ideal_gas)` — the one line that brings
    // another document's names here.
    //
    // **The head is a quoted path or a bare namespace name**, and the
    // absence of quotes is the whole of the difference: a path names a
    // file, a bare name a namespace the language knows, `std` among
    // them. **What follows either head is the same**: `as` binds the
    // namespace under a name of the caller's choosing, `::*`, `::{a,
    // b}` and `::a` import names into this document's own scope, and
    // a `with (…)` clause states the argument each of an instantiated
    // namespace's holes takes. Where `as` is absent a file's stem is
    // the namespace's name.
    //
    // That a line states at most one of `as` and an import list, and
    // which combinations a namespace head admits, a later analysis stage states.
    use_declaration: ($) =>
      seq(
        "use",
        choice(field("path", $.string), field("namespace", $.identifier)),
        optional(seq("::", field("imports", $.import_list))),
        optional(seq("as", field("alias", $.identifier))),
        optional(field("holes", $.with_clause)),
      ),

    // `::*`, `::{lighter, slider}` and `::slider` — what a `use` line
    // takes into this document's own scope. The star
    // imports every public name of the namespace and the braced list
    // the names it writes; a bare name is exactly the one-name list,
    // so `use std::slider` and `use std::{slider}` are one line. An
    // imported name that collides with a binding of this document is
    // refused at the `use` line, by name.
    import_list: ($) =>
      choice(
        "*",
        seq(
          "{",
          repeat(choice(",", $._newline)),
          field("name", $.identifier),
          repeat(
            seq(repeat1(choice(",", $._newline)), field("name", $.identifier)),
          ),
          repeat(choice(",", $._newline)),
          "}",
        ),
        field("name", $.identifier),
      ),

    // `with (pressure: ideal_gas)` — the arguments an instantiated
    // namespace's holes take: one binding per hole the
    // head declares, the hole's name and the expression this document
    // hands it. `with` is the control zone's word in expression
    // position, and in a `use` line, where no expression stands, it is
    // this clause and nothing else. That every hole is bound exactly
    // once and that each argument fits the shape its hole declares,
    // a later analysis stage states.
    with_clause: ($) =>
      seq(
        "with",
        "(",
        repeat(choice(",", $._newline)),
        $.hole_binding,
        repeat(seq(repeat1(choice(",", $._newline)), $.hole_binding)),
        repeat(choice(",", $._newline)),
        ")",
      ),

    hole_binding: ($) =>
      seq(field("name", $.identifier), ":", field("value", $._expression)),

    // `namespace hydro(pressure(rho: Real): Real) { … }` — a unit
    // written inside a document: a body of items whose
    // holes the instantiating `use` line fills. A document holding
    // no `namespace` item is one named by its file stem, so every
    // `use` line in the corpus keeps its reading.
    //
    // **It holds values, functions and fibers alone.** A port, an
    // update block and a `use` line are the document's, not its: a
    // durable port declared inside one instantiated twice would
    // have one declaring line reachable through two names, and no
    // atomic multi-file rename makes those two writes one (A7,
    // A3). One that needs a port takes it as a hole.
    namespace_definition: ($) =>
      seq(
        "namespace",
        field("name", $.identifier),
        field("holes", $.hole_list),
        "{",
        repeat(choice(",", $._newline)),
        optional(
          seq(
            $._namespace_item,
            repeat(seq(repeat1(choice(",", $._newline)), $._namespace_item)),
            repeat(choice(",", $._newline)),
          ),
        ),
        "}",
      ),

    hole_list: ($) =>
      seq(
        "(",
        repeat(choice(",", $._newline)),
        optional(
          seq(
            $.hole,
            repeat(seq(repeat1(choice(",", $._newline)), $.hole)),
            repeat(choice(",", $._newline)),
          ),
        ),
        ")",
      ),

    // `gamma: Real` and `pressure(rho: Real): Real` — one hole of a
    // namespace head: the name the body reads, and
    // the shape whatever fills it must take. A plain name states the
    // fiber its value takes, and a parenthesized parameter list states
    // a function's shape, the fiber after the parens being the one the
    // result takes. The parameter list is the declaration's own
    // , so a hole's parameters are written the way a
    // function's are; that a hole's parameter declares no default, the
    // lowering states.
    //
    // A hole a value fills may state a default, `gamma: Real = 5.0 /
    // 3.0`, and a `use` line leaving that hole unbound takes it. That a
    // function-shaped hole states none — a default there would be a
    // function value, and the function space is first-order — the
    // lowering states.
    hole: ($) =>
      seq(
        field("name", $.identifier),
        optional(field("parameters", $.parameter_list)),
        ":",
        field("type", $.type_annotation),
        optional(seq("=", field("default", $._expression))),
      ),

    _namespace_item: ($) =>
      choice(
        $.doc_declaration,
        $.fiber_declaration,
        $.pattern_binding,
        $.function_definition,
        $.binding,
      ),

    // `param name = expr` / `state name = expr` — every port holds an
    // initial (axiom A2). A `param` belongs to the text and a `state`
    // to the run. A port states an optional fiber after `:`,
    // `state x: Vector<Real, 2>|None = vec(0, 0)`, and the initial is
    // held to it.
    //
    // **A parenthetical after the initial states when the state is
    // locked**: `state res: Int = 100 (locked when
    // physics != none)`. It belongs to a `state`: a `param` is fixed
    // over a run, so a lock has nothing to guard there, and source
    // normalization refuses the clause written on one by name.
    port_declaration: ($) =>
      seq(
        field("kind", choice("param", "state")),
        field("name", $.identifier),
        optional(seq(":", field("type", $.type_annotation))),
        "=",
        field("value", $._expression),
        optional(field("lock", $.lock_clause)),
      ),

    // `(locked when physics != none)` — the parenthetical a `state`
    // writes after its initial to state the condition under which it
    // is locked. The condition is an ordinary Bool
    // expression of the document, elaborated into the graph like any
    // other and read live, so a lock states itself at the declaration
    // rather than being inferred from what a run did.
    //
    // **Inside its parens the clause breaks where it likes**, as any
    // bracketed run does. The `(` stands on the initial's line: a line
    // opening with `(` begins an item, since a paren can begin an
    // expression and so never continues the line above.
    lock_clause: ($) =>
      seq("(", "locked", "when", field("condition", $._expression), ")"),

    // `fiber Cons = (D: Real, S: Vector<Real, 2>, tau: Real)` and
    // `fiber Shape = (A: Real | B: Vector<Real, 3>)` — a fiber
    // declaration: a product's components separated by `,`
    // , a sum's variants separated by `|`
    // , each a name and a fiber type inside the named
    // parens the value form already uses — a variant written as a
    // bare name taking no payload, so `fiber Mode = (Binary |
    // Single)` is an enumeration. A product states at least one
    // component and a sum at least two variants, and no name twice,
    // which a later analysis stage states. `fiber Cell<k> = (at: [Int; k],
    // v: Vector<Real, k>)` names a width parameter after the name,
    // which the component fibers read.
    //
    // **A fiber's field kinds determine its algebra**, which a later
    // analysis stage states; the declaration writes no algebra word.
    fiber_declaration: ($) =>
      seq(
        "fiber",
        field("name", $.identifier),
        optional(seq("<", field("param", $.identifier), ">")),
        "=",
        choice(
          seq(
            "(",
            field("variant", $.variant),
            repeat1(seq("|", field("variant", $.variant))),
            ")",
          ),
          seq(
            "(",
            repeat(choice(",", $._newline)),
            optional(
              seq(
                field("component", $.component),
                repeat(
                  seq(
                    repeat1(choice(",", $._newline)),
                    field("component", $.component),
                  ),
                ),
                repeat(choice(",", $._newline)),
              ),
            ),
            ")",
          ),
        ),
      ),

    // `struct State = { iter: Int, time: Real, cons: Array<Conserved, 2> }`
    // — a name for a struct's entries, standing wherever a type
    // annotation stands. It is a declaration and no
    // node: a document that states one shape at several binding sites
    // writes the braces once.
    //
    // **It is an alias and no constructor.** Braces build a struct and
    // parens build a fiber, so a struct alias has no call form and
    // `State(…)` refuses, naming the brace literal that builds one.
    struct_declaration: ($) =>
      seq(
        "struct",
        field("name", $.identifier),
        "=",
        field("entries", $.struct_type),
      ),

    // `{ iter: Int, cons: Array<Conserved, 2> }` — a struct's entries
    // written where a type stands: one entry per key, in the order the
    // text writes them, each a bare name and the kind it holds. It is
    // the Map literal read backwards, as the brace pattern is
    // , and it nests.
    //
    // **Parens describe a fiber's components and braces a struct's
    // entries.** A fiber is plain old data at a site and a struct is a
    // record of whole values, so the bracket tells the two apart and a
    // struct stands in no field; that a struct is refused wherever a
    // fiber is required, a later analysis stage states.
    struct_type: ($) =>
      seq(
        "{",
        repeat(choice(",", $._newline)),
        optional(
          seq(
            field("entry", $.struct_entry),
            repeat(
              seq(
                repeat1(choice(",", $._newline)),
                field("entry", $.struct_entry),
              ),
            ),
            repeat(choice(",", $._newline)),
          ),
        ),
        "}",
      ),

    // `iter: Int` — one entry of a struct: the key, and the kind the
    // entry holds, which may take the absence alternative,
    // `name: Real|None`: the entry is always there and its value may be
    // absent. That a struct states at least one entry, and no key
    // twice, and that the alternative is `None`, a later analysis stage
    // states.
    struct_entry: ($) =>
      seq(
        field("name", $.identifier),
        ":",
        field("type", $.fiber_type),
        optional(seq("|", field("alternative", $.fiber_type))),
      ),

    component: ($) =>
      seq(field("name", $.identifier), ":", field("type", $.fiber_type)),

    // `A: Real` and `Binary` — one variant of a sum fiber
    // : its name, and the fiber its payload takes where
    // it takes one. **A variant written as a bare name takes no
    // payload**, so `(Binary | Single)` is an enumeration and
    // `(Point | Segment: Real)` mixes the two forms.
    variant: ($) =>
      seq(
        field("name", $.identifier),
        optional(seq(":", field("type", $.fiber_type))),
      ),

    // A fiber type: a name with optional `<…>` arguments, types or
    // widths — `Real`, `Vector<Real, 2>`, `Mat<3, 3>`, a static list
    // kind `[Int; 2]` (`static_list_type`), a type taking the
    // absence alternative, `List<Real|None>`, where a later analysis stage admits
    // an entry that may be absent — or a product's
    // components or a sum's variants stated in the open, either of
    // which stands wherever an alias does.
    // **The name is qualified where a call head's
    // is**, `orbit::Binary` naming a used document's
    // fiber. The lookahead before the components is the
    // interpreter's alone, as `described_head`'s is: a `component`
    // states its own sentence for a missing `:`, and the parenthesized
    // description is one of three readings of a paren, so the sentence
    // waits until the colon says the reading is this one.
    fiber_type: ($) =>
      choice(
        seq(
          "(",
          field("variant", $.variant),
          repeat1(seq("|", field("variant", $.variant))),
          ")",
        ),
        seq(
          "(",
          repeat(choice(",", $._newline)),
          optional(
            seq(
              field("component", $.component),
              repeat(
                seq(
                  repeat1(choice(",", $._newline)),
                  field("component", $.component),
                ),
              ),
              repeat(choice(",", $._newline)),
            ),
          ),
          ")",
        ),
        field("entries", $.struct_type),
        field("list", $.static_list_type),
        seq(
          field("name", choice($.qualified_identifier, $.identifier)),
          optional(
            seq(
              "<",
              repeat(choice(",", $._newline)),
              optional(
                seq(
                  choice($.type_annotation, $.number),
                  repeat(
                    seq(
                      repeat1(choice(",", $._newline)),
                      choice($.type_annotation, $.number),
                    ),
                  ),
                  repeat(choice(",", $._newline)),
                ),
              ),
              ">",
            ),
          ),
        ),
      ),

    // `[Int; 2]` and `[Int, Real]` — a static list kind:
    // the repeated form states one entry kind and a width — a number, a
    // declaration's width parameter, or `_` for any — and the concrete
    // form one kind per position, at least one, which is sugar for the
    // repeated form where its entries agree. The brackets are positional
    // as a List literal's are, and the length is a fact of the kind.
    static_list_type: ($) =>
      choice(
        seq(
          "[",
          field("item", $.fiber_type),
          ";",
          field("width", choice($.number, $.identifier)),
          "]",
        ),
        seq(
          "[",
          field("item", $.fiber_type),
          repeat(seq(",", field("item", $.fiber_type))),
          "]",
        ),
      ),

    // `name = expr`, and `name: Real = expr` where the binding states
    // the fiber its value takes.
    binding: ($) =>
      seq(
        field("name", $.identifier),
        optional(seq(":", field("type", $.type_annotation))),
        "=",
        field("value", $._expression),
      ),

    // `require(a and b)` and `require(n > 0, f"n = {n}")` — a stated
    // requirement standing alone, as a document's item or a body's.
    // It binds no name and is no body's result. The optional message
    // is a quoted, triple-quoted or tagged literal. Whether and when
    // the condition is checked, a later analysis stage states.
    require_statement: ($) =>
      seq(
        "require",
        "(",
        field("condition", $._expression),
        optional(
          seq(
            ",",
            field(
              "message",
              choice($.template_string, $.triple_string, $.string),
            ),
          ),
        ),
        ")",
      ),

    // `plot`, `3.0 * x`, `canvas2d { … } with nav(xr)` standing alone
    // as an item — **the spelling of `main = expr`**.
    // Any expression stands here, over as many
    // continuation lines as it takes, and the item ends where the
    // line does. A later analysis stage supplies the name `main` and the item
    // reaches the elaborator as the binding it spells, so a document
    // writing both refuses where two `main` bindings refuse. A
    // namespace body holds no such item: it declares no endpoint.
    //
    // **The lookahead is what leaves every declaration the form it
    // is**: an identifier followed by `=`, `:`, `:=` or `<-`, and an
    // identifier followed by a parenthesized parameter list and then
    // `=` or `:`, are the binding, the local, the write and the
    // function definition, and a parenthesized or braced pattern
    // followed by `=` is the top-level pattern; this item refuses
    // them all before reading a word. What is left is exactly the
    // bare expression: an identifier that runs on into a call's
    // arguments, a `with`, a brace, a `.`, an operator or the line's
    // end. A head whose parens the writer has not closed on the line
    // stands out too, since the declaration forms read the same text
    // and their own miss reports it as text run out. The lookahead
    // is the interpreter's, which tree-sitter, being GLR, prints
    // without: there the declaration's own `=` is what decides, and
    // the two readings run together until it — `parameter_list`
    // taking the immediate `(` is what carries the definition's
    // reading that far, and `doc_declaration`'s precedence is what
    // keeps a triple string standing alone the prose it is.
    bare_expression: ($) => field("value", $._expression),

    // `(D: D) = U`, `[i, j] = s` and `{ rho, vel } = m` — a top-level
    // pattern item: each name the pattern binds is a
    // document name holding that component, exported through a `use`
    // alias as any other document name is, and a field of products
    // binds its component fields. The parenthesized patterns and the
    // braced one open where no other item does. The constructor form
    // `Cons(D, S, tau) = U` is written the way a function definition
    // is, and elaboration tells the two apart by what the head names.
    // A bare run, `a, b = t`, is `[a, b] = t` (T:78); it stands first,
    // so `[a, b], c = t` is one run rather than a bracket missing its
    // `=`, and a top-level comma separates no items.
    pattern_binding: ($) =>
      seq(
        field(
          "pattern",
          choice(
            $.bare_list_pattern,
            $.named_pattern,
            $.list_pattern,
            $.map_pattern,
          ),
        ),
        "=",
        field("value", $._expression),
      ),

    // `name(params) : { k: default, … } = expr` — a transparent function
    // (A9). The declaration mirrors the call: the Map gives the
    // configuration names and their defaults.
    //
    // The head is spelled exactly as a call is, and an item stands on
    // a bare call too, so the two readings run together
    // until the `=` decides. What carries both that far is
    // `parameter_list` taking the immediate `(` the lexer reads after
    // a head as well as the plain one.
    function_definition: ($) =>
      seq(
        field("name", $.identifier),
        field("parameters", $.parameter_list),
        optional(seq(":", field("config", $.map))),
        "=",
        field("body", $._expression),
      ),

    parameter_list: ($) =>
      seq(
        choice(token.immediate("("), "("),
        repeat(choice(",", $._newline)),
        optional(
          seq(
            $.parameter,
            repeat(seq(repeat1(choice(",", $._newline)), $.parameter)),
            repeat(choice(",", $._newline)),
          ),
        ),
        ")",
      ),

    // `xr: Interval`, `at: Real|None`, `color: Color = hex("#56b6c2")`,
    // `Cons(d, s, _)` — a parameter binds a pattern and states an
    // optional type and an optional default, independently of each
    // other (wishlist item 71). The annotation and the default stand on
    // the whole slot. That only a trailing run declares defaults, that a
    // default stands on a bare name, and that `None` is the one
    // alternative an annotation takes, a later analysis stage states. A
    // positional tuple stands only with the fiber it reads stated after
    // it, `(rho, _, pre): Primitive`.
    parameter: ($) =>
      choice(
        seq(
          field("name", $.site_pattern),
          ":",
          field("type", $.type_annotation),
          optional(seq("=", field("default", $._expression))),
        ),
        seq(
          field("name", $._pattern),
          optional(
            seq(
              choice(token.immediate(":"), ":"),
              field("type", $.type_annotation),
            ),
          ),
          optional(seq("=", field("default", $._expression))),
        ),
      ),

    // A type, or a function signature where a type stands. The
    // signature stands first, since a fiber description also opens
    // with a paren and takes `()`. A trailing `->` after a bare type
    // is read so that the missing parentheses are named rather than
    // left to the token after it; where a signature is admitted,
    // source normalization states.
    type_annotation: ($) =>
      choice(
        $.signature_type,
        seq(
          $.fiber_type,
          optional(seq("|", field("alternative", $.fiber_type))),
          optional(
            seq(field("arrow", "->"), field("result", $.type_annotation)),
          ),
        ),
      ),

    // `(Real, Real) -> Real` — the signature of a function a parameter
    // receives: its argument types in parens, any number of them, and
    // its result type. It describes a function named at the call, not
    // a function value.
    signature_type: ($) =>
      seq(
        "(",
        repeat(choice(",", $._newline)),
        optional(
          seq(
            field("argument", $.type_annotation),
            repeat(
              seq(
                repeat1(choice(",", $._newline)),
                field("argument", $.type_annotation),
              ),
            ),
            repeat(choice(",", $._newline)),
          ),
        ),
        ")",
        field("arrow", "->"),
        field("result", $.type_annotation),
      ),

    // `step { p <- e, … }` declares an unconditional update.
    step_block: ($) => seq("step", field("body", $.body)),

    // `texture(|p| color)` or `texture(|p| { local := value
    //  color })` — a
    //                  deferred per-pixel body. `p` is the canvas data-coordinate
    //                  `Vector<Real, 2>`, and the body must return one rank-zero `Color`.
    //                  The binder is local to the body; this form does not make a
    //                  general function value
    texture_expression: ($) =>
      seq(
        "texture",
        token.immediate("("),
        "|",
        field("parameter", $.identifier),
        "|",
        field(
          "body",
          choice(prec.dynamic(2, $.body), prec.dynamic(1, $._expression)),
        ),
        ")",
      ),

    // the expression, in one table: the primary forms, then every
    // operator with its precedence — tightest last
    _expression: ($) =>
      choice(
        $.configurable_expression,
        $.texture_expression,
        $.call_expression,
        $.qualified_identifier,
        $.boolean,
        $.none,
        $.template_string,
        $.identifier,
        $.number,
        $.triple_string,
        $.string,
        $.list,
        $.product,
        $.parenthesized_expression,
        $.map,
        $.body,
        $.tap_expression,
        $.for_expression,
        $.each_expression,
        $.fold_expression,
        $.conditional_expression,
        $.match_expression,
        $.full_range,
        $.unary_expression,
        $.periodic_range,
        $.range_to,
        $.range_expression,
        $.range_from,
        $.periodic_expression,
        $.binary_expression,
        $.with_expression,
        $.site_update_expression,
        $.but_expression,
        $.absent_expression,
        $.index_expression,
        $.component_expression,
        $.config_expression,
        $.configure_dot_expression,
        $.payload_expression,
      ),

    // `not`, above the comparisons and below `and`
    // unary `+`, standing wherever unary `-` does, so `+0.5` and `-0.5`
    // read alike. A line opening with `+` still continues the line above,
    // the continuation token taking it as the binary operator.
    unary_expression: ($) =>
      choice(
        prec.right(
          4,
          seq(field("operator", "not"), field("operand", $._expression)),
        ),
        prec.right(
          9,
          seq(field("operator", "-"), field("operand", $._expression)),
        ),
        prec.right(
          9,
          seq(field("operator", "+"), field("operand", $._expression)),
        ),
      ),

    // `..a..b` — a periodic axis. The bounded range after the leading
    // `..` takes the expression context's precedence and exclusions. For
    // example, `..0..n + 1` wraps `0..(n + 1)` and `for i in ..0..n { i }`
    // leaves the brace for the comprehension body. `a..b periodic` is
    // the equivalent postfix spelling.
    periodic_range: ($) =>
      prec.right(
        2,
        seq(field("operator", ".."), field("axis", $.range_expression)),
      ),

    // `..e` / `..=e` — a range with no start, an ordinary value as any
    // range is: `r = ..e`, `f(..=e)`. Its end reads at the range's own
    // precedence, so `..n - 1` is `..(n - 1)`. Where a bounded range follows
    // the `..`, the whole is the periodic axis `..a..b` instead.
    range_to: ($) =>
      prec.right(
        1,
        seq(field("operator", choice("..=", "..")), field("end", $._expression)),
      ),

    // `a..b` / `a..=b` — the loosest operator there is, Rust's own
    // precedence, so `0..n - 1` reads `0..(n - 1)`. It groups left here,
    // and a later analysis stage refuses `a..b..c`: a range states two endpoints.
    // An end reads no brace, which is a header's body after `for i in 3..`.
    // The end stands on the operator's line: `0..` is complete as the open
    // range, so a newline after the operator ends it.
    range_expression: ($) =>
      prec.left(
        1,
        seq(
          field("left", $._expression),
          field("operator", choice("..=", "..")),
          field("right", $._expression),
        ),
      ),

    // `s..` — a range with no end, where no end follows the operator
    // : the range above takes one where it can. A newline after the
    // operator ends it, so `r = 0..` is complete.
    range_from: ($) =>
      prec.left(
        1,
        seq(
          field("start", $._expression),
          field("operator", choice("..=", "..")),
          $._no_operand,
        ),
      ),

    // `0..n periodic` — the other spelling of the periodic axis `..0..n`,
    // still accepted. It stands on a range and nowhere else, so it reads at
    // the range's own precedence and a later analysis stage refuses it on
    // anything but a range: the word states an axis of a box, and a box's
    // axis is a range.
    periodic_expression: ($) =>
      prec.left(1, seq(field("axis", $._expression), "periodic")),

    // the comparisons group left here, and a later analysis stage refuses a chain
    // — `a < b < c` is two comparisons and says so
    // `x ?? y` — the presence default: `x` where it is
    // present, `y` where it is absent. It groups right, so defaults chain,
    // and stands between the comparisons and arithmetic, so `m ?? 0 > 3` is
    // `(m ?? 0) > 3` and `m ?? n + 1` is `m ?? (n + 1)`. The lexer's longest
    // match takes it over the postfix absence mark `?`.
    // `a <> b` — the join: the string two strings make, the list two
    // lists make end to end, the union two Maps make. It stands at `+`'s
    // precedence and groups left, and it is a continuation operator, so a
    // line opening with `<>` attaches to the line above it. The lexer's
    // longest match takes it over `<` followed by `>`, which is no legal
    // sequence.
    // `-` carries no leading newlines: a line opening with `-` is a fresh
    // signed item, so `[1\n -2\n 3]` keeps its three items
    // `@` is the action operator: infix at the
    // multiplicative level, left-associative, so `g @ a @ b` is the scalar
    // and `(R1 * R2) @ v` composes then acts
    binary_expression: ($) =>
      choice(
        prec.left(
          2,
          seq(
            field("left", $._expression),
            field("operator", "or"),
            field("right", $._expression),
          ),
        ),
        prec.left(
          3,
          seq(
            field("left", $._expression),
            field("operator", "and"),
            field("right", $._expression),
          ),
        ),
        prec.left(
          5,
          seq(
            field("left", $._expression),
            field(
              "operator",
              choice(
                alias(token(/==/), "=="),
                alias(token(/!=/), "!="),
                alias(token(/<=/), "<="),
                alias(token(/>=/), ">="),
                alias(token(/</), "<"),
                alias(token(/>/), ">"),
              ),
            ),
            field("right", $._expression),
          ),
        ),
        prec.right(
          6,
          seq(
            field("left", $._expression),
            field("operator", alias(token(/\?\?/), "??")),
            field("right", $._expression),
          ),
        ),
        prec.left(
          7,
          seq(
            field("left", $._expression),
            field("operator", alias(token(/\+/), "+")),
            field("right", $._expression),
          ),
        ),
        prec.left(
          7,
          seq(
            field("left", $._expression),
            field("operator", alias(token(/<>/), "<>")),
            field("right", $._expression),
          ),
        ),
        prec.left(
          7,
          seq(
            field("left", $._expression),
            field("operator", "-"),
            field("right", $._expression),
          ),
        ),
        prec.left(
          8,
          seq(
            field("left", $._expression),
            field(
              "operator",
              choice(
                alias(token(/\*/), "*"),
                alias(token(/\/\//), "//"),
                alias(token(/\//), "/"),
                alias(token(/@/), "@"),
              ),
            ),
            field("right", $._expression),
          ),
        ),
        prec.right(
          10,
          seq(
            field("left", $._expression),
            field("operator", alias(token(/\^/), "^")),
            field("right", $._expression),
          ),
        ),
      ),

    // `view with control` — attachment, chainable, never a child. The
    // control takes the postfix zones of its own — `with scrub(g) : cfg { … }`
    // — and not a further `with`, so the chain stays flat.
    with_expression: ($) =>
      prec.left(
        11,
        seq(
          field("target", $._expression),
          "with",
          field("control", $._expression),
        ),
      ),

    // `base but key: value` — the site update: the
    // field on the left with the sites the key states holding the value and
    // every other site kept. The key is an expression — a range or a box,
    // `space(x)`, a mask — closed by its colon, so it reads no
    // configuration zone; the chain groups left at `with`'s precedence, and a
    // line opening with `but` continues the one above. It stands before the
    // component update, which the interpreter reads when no colon closes a
    // key: `cam but (d: 3.0)` names a component, `a but (n - 1): 0.0` a site.
    site_update_expression: ($) =>
      prec.dynamic(
        1,
        prec.left(
          11,
          seq(
            field("target", $._expression),
            "but",
            field("key", $._expression_key),
            ":",
            field("value", $._expression),
          ),
        ),
      ),

    // `base but override` — the product-fiber component update
    // : the value on the left with the components the
    // override names replaced, at `with`'s precedence and grouping left.
    but_expression: ($) =>
      prec.left(
        11,
        seq(
          field("target", $._expression),
          "but",
          field("override", $._expression),
        ),
      ),

    // `f(x?, y)` — the argument mark: a postfix `?` says
    // the slot it stands on licenses absence. It rides the postfix chain,
    // so a call result marks too — `points(vec(x, y)?)`.
    absent_expression: ($) =>
      prec.left(13, seq(field("value", $._expression), "?")),

    index_expression: ($) =>
      prec.left(
        13,
        seq(field("target", $._expression), field("index", $.subscript)),
      ),

    // `S of u`, `0 of v`, `a of c`, `(i, j) of m` — the component read
    // . It stands above every arithmetic and
    // comparison operator and above unary minus, so `0 of v + 1` adds one
    // to the component and `-0 of v` negates it, and its operand is a
    // postfix-level expression, so `0 of f(x)` and `0 of v[s]` read as
    // written. It groups right, so `0 of S of u` reads the product's
    // component and then that value's.
    component_expression: ($) =>
      prec.right(
        12,
        seq(field("key", $.component_key), "of", field("value", $._expression)),
      ),

    // `e : map` — configuration, ascribed against a registry signature.
    // It chains, and a later key wins. A link of the chain
    // takes a dynamic precedence, so `e : a : b[i]` is two links with `b[i]`
    // the second atom rather than one link whose atom is `(a : b)[i]`.
    config_expression: ($) =>
      choice(
        prec.dynamic(
          1,
          prec.left(
            13,
            seq(
              field("target", choice($.config_expression)),
              field("operator", alias(token(/:/), ":")),
              field("config", $._config_atom),
            ),
          ),
        ),
        prec.left(
          13,
          seq(
            field("target", $._expression),
            field("operator", alias(token(/:/), ":")),
            field("config", $._config_atom),
          ),
        ),
      ),

    // `value.{key: expression}` — literal settings closing; links chain left
    configure_dot_expression: ($) =>
      prec.left(
        13,
        seq(
          field("target", $._expression),
          field("operator", alias(token(/\./), ".")),
          field("config", $.map),
        ),
      ),

    // `head(…) { items }` — call sugar for a trailing List argument; a
    // brace whose first item is a `port <-` write is the writes a control
    // head takes, which a later analysis stage decides
    // `head [a, b]` — a payload of one row, exactly `head { [a, b] }`
    // . The bracket opens on the head's line, as the
    // brace does, so `head` and a `[` on the next line stay two items.
    // The index takes its `[` immediately after its target, so `xs[i]`
    // reads the element and `xs [i]` is this zone.
    payload_expression: ($) =>
      choice(
        prec.left(
          13,
          seq(field("target", $._expression), field("body", $.body)),
        ),
        prec.left(
          13,
          seq(field("target", $._expression), field("body", $.list)),
        ),
      ),

    // `_expression` without `payload_expression` and `config_expression` and `call_refusal` and `site_update_expression` and `but_expression`:
    // the rule a reference through `without` in the grammar's source
    // reads — a loop header's expression, whose brace and colon belong
    // to the header. Every node is aliased to its name in `_expression`, so
    // the tree is the same tree; the precedences are lifted to
    // `2p + 1` so the header's reading wins wherever the two could
    // take the same text.
    _expression_key: ($) =>
      choice(
        $.configurable_expression,
        $.texture_expression,
        $.call_expression,
        $.qualified_identifier,
        $.boolean,
        $.none,
        $.template_string,
        $.identifier,
        $.number,
        $.triple_string,
        $.string,
        $.list,
        $.product,
        $.parenthesized_expression,
        $.map,
        $.body,
        $.tap_expression,
        $.for_expression,
        $.each_expression,
        $.fold_expression,
        $.conditional_expression,
        $.match_expression,
        $.full_range,
        alias($.unary_expression_key, $.unary_expression),
        alias($.periodic_range_key, $.periodic_range),
        alias($.range_to_key, $.range_to),
        alias($.range_expression_key, $.range_expression),
        alias($.range_from_key, $.range_from),
        alias($.periodic_expression_key, $.periodic_expression),
        alias($.binary_expression_key, $.binary_expression),
        alias($.with_expression_key, $.with_expression),
        alias($.absent_expression_key, $.absent_expression),
        alias($.index_expression_key, $.index_expression),
        alias($.component_expression_key, $.component_expression),
        alias($.configure_dot_expression_key, $.configure_dot_expression),
      ),

    unary_expression_key: ($) =>
      choice(
        prec.right(
          9,
          seq(field("operator", "not"), field("operand", $._expression_key)),
        ),
        prec.right(
          19,
          seq(field("operator", "-"), field("operand", $._expression_key)),
        ),
        prec.right(
          19,
          seq(field("operator", "+"), field("operand", $._expression_key)),
        ),
      ),

    periodic_range_key: ($) =>
      prec.right(
        5,
        seq(
          field("operator", ".."),
          field("axis", alias($.range_expression_key, $.range_expression)),
        ),
      ),

    range_to_key: ($) =>
      prec.right(
        3,
        seq(
          field("operator", choice("..=", "..")),
          field("end", $._expression_key),
        ),
      ),

    range_expression_key: ($) =>
      prec.left(
        3,
        seq(
          field("left", $._expression_key),
          field("operator", choice("..=", "..")),
          field("right", $._expression_key),
        ),
      ),

    range_from_key: ($) =>
      prec.left(
        3,
        seq(
          field("start", $._expression_key),
          field("operator", choice("..=", "..")),
          $._no_operand,
        ),
      ),

    periodic_expression_key: ($) =>
      prec.left(3, seq(field("axis", $._expression_key), "periodic")),

    binary_expression_key: ($) =>
      choice(
        prec.left(
          5,
          seq(
            field("left", $._expression_key),
            field("operator", "or"),
            field("right", $._expression_key),
          ),
        ),
        prec.left(
          7,
          seq(
            field("left", $._expression_key),
            field("operator", "and"),
            field("right", $._expression_key),
          ),
        ),
        prec.left(
          11,
          seq(
            field("left", $._expression_key),
            field(
              "operator",
              choice(
                alias(token(/==/), "=="),
                alias(token(/!=/), "!="),
                alias(token(/<=/), "<="),
                alias(token(/>=/), ">="),
                alias(token(/</), "<"),
                alias(token(/>/), ">"),
              ),
            ),
            field("right", $._expression_key),
          ),
        ),
        prec.right(
          13,
          seq(
            field("left", $._expression_key),
            field("operator", alias(token(/\?\?/), "??")),
            field("right", $._expression_key),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_key),
            field("operator", alias(token(/\+/), "+")),
            field("right", $._expression_key),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_key),
            field("operator", alias(token(/<>/), "<>")),
            field("right", $._expression_key),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_key),
            field("operator", "-"),
            field("right", $._expression_key),
          ),
        ),
        prec.left(
          17,
          seq(
            field("left", $._expression_key),
            field(
              "operator",
              choice(
                alias(token(/\*/), "*"),
                alias(token(/\/\//), "//"),
                alias(token(/\//), "/"),
                alias(token(/@/), "@"),
              ),
            ),
            field("right", $._expression_key),
          ),
        ),
        prec.right(
          21,
          seq(
            field("left", $._expression_key),
            field("operator", alias(token(/\^/), "^")),
            field("right", $._expression_key),
          ),
        ),
      ),

    with_expression_key: ($) =>
      prec.left(
        23,
        seq(
          field("target", $._expression_key),
          "with",
          field("control", $._expression_key),
        ),
      ),

    absent_expression_key: ($) =>
      prec.left(27, seq(field("value", $._expression_key), "?")),

    index_expression_key: ($) =>
      prec.left(
        27,
        seq(field("target", $._expression_key), field("index", $.subscript)),
      ),

    component_expression_key: ($) =>
      prec.right(
        25,
        seq(
          field("key", $.component_key),
          "of",
          field("value", $._expression_key),
        ),
      ),

    configure_dot_expression_key: ($) =>
      prec.left(
        27,
        seq(
          field("target", $._expression_key),
          field("operator", alias(token(/\./), ".")),
          field("config", $.map),
        ),
      ),

    // `_expression` without `payload_expression` and `config_expression` and `call_refusal`:
    // the rule a reference through `without` in the grammar's source
    // reads — a loop header's expression, whose brace and colon belong
    // to the header. Every node is aliased to its name in `_expression`, so
    // the tree is the same tree; the precedences are lifted to
    // `2p + 1` so the header's reading wins wherever the two could
    // take the same text.
    _expression_header: ($) =>
      prec(
        1,
        choice(
          $.configurable_expression,
          $.texture_expression,
          $.call_expression,
          $.qualified_identifier,
          $.boolean,
          $.none,
          $.template_string,
          $.identifier,
          $.number,
          $.triple_string,
          $.string,
          $.list,
          $.product,
          $.parenthesized_expression,
          $.map,
          $.body,
          $.tap_expression,
          $.for_expression,
          $.each_expression,
          $.fold_expression,
          $.conditional_expression,
          $.match_expression,
          $.full_range,
          alias($.unary_expression_header, $.unary_expression),
          alias($.periodic_range_header, $.periodic_range),
          alias($.range_to_header, $.range_to),
          alias($.range_expression_header, $.range_expression),
          alias($.range_from_header, $.range_from),
          alias($.periodic_expression_header, $.periodic_expression),
          alias($.binary_expression_header, $.binary_expression),
          alias($.with_expression_header, $.with_expression),
          alias($.site_update_expression_header, $.site_update_expression),
          alias($.but_expression_header, $.but_expression),
          alias($.absent_expression_header, $.absent_expression),
          alias($.index_expression_header, $.index_expression),
          alias($.component_expression_header, $.component_expression),
          alias($.configure_dot_expression_header, $.configure_dot_expression),
        ),
      ),

    unary_expression_header: ($) =>
      choice(
        prec.right(
          9,
          seq(field("operator", "not"), field("operand", $._expression_header)),
        ),
        prec.right(
          19,
          seq(field("operator", "-"), field("operand", $._expression_header)),
        ),
        prec.right(
          19,
          seq(field("operator", "+"), field("operand", $._expression_header)),
        ),
      ),

    periodic_range_header: ($) =>
      prec.right(
        5,
        seq(
          field("operator", ".."),
          field("axis", alias($.range_expression_header, $.range_expression)),
        ),
      ),

    range_to_header: ($) =>
      prec.right(
        3,
        seq(
          field("operator", choice("..=", "..")),
          field("end", $._expression_header),
        ),
      ),

    range_expression_header: ($) =>
      prec.left(
        3,
        seq(
          field("left", $._expression_header),
          field("operator", choice("..=", "..")),
          field("right", $._expression_header),
        ),
      ),

    range_from_header: ($) =>
      prec.left(
        3,
        seq(
          field("start", $._expression_header),
          field("operator", choice("..=", "..")),
          $._no_operand,
        ),
      ),

    periodic_expression_header: ($) =>
      prec.left(3, seq(field("axis", $._expression_header), "periodic")),

    binary_expression_header: ($) =>
      choice(
        prec.left(
          5,
          seq(
            field("left", $._expression_header),
            field("operator", "or"),
            field("right", $._expression_header),
          ),
        ),
        prec.left(
          7,
          seq(
            field("left", $._expression_header),
            field("operator", "and"),
            field("right", $._expression_header),
          ),
        ),
        prec.left(
          11,
          seq(
            field("left", $._expression_header),
            field(
              "operator",
              choice(
                alias(token(/==/), "=="),
                alias(token(/!=/), "!="),
                alias(token(/<=/), "<="),
                alias(token(/>=/), ">="),
                alias(token(/</), "<"),
                alias(token(/>/), ">"),
              ),
            ),
            field("right", $._expression_header),
          ),
        ),
        prec.right(
          13,
          seq(
            field("left", $._expression_header),
            field("operator", alias(token(/\?\?/), "??")),
            field("right", $._expression_header),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_header),
            field("operator", alias(token(/\+/), "+")),
            field("right", $._expression_header),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_header),
            field("operator", alias(token(/<>/), "<>")),
            field("right", $._expression_header),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_header),
            field("operator", "-"),
            field("right", $._expression_header),
          ),
        ),
        prec.left(
          17,
          seq(
            field("left", $._expression_header),
            field(
              "operator",
              choice(
                alias(token(/\*/), "*"),
                alias(token(/\/\//), "//"),
                alias(token(/\//), "/"),
                alias(token(/@/), "@"),
              ),
            ),
            field("right", $._expression_header),
          ),
        ),
        prec.right(
          21,
          seq(
            field("left", $._expression_header),
            field("operator", alias(token(/\^/), "^")),
            field("right", $._expression_header),
          ),
        ),
      ),

    with_expression_header: ($) =>
      prec.left(
        23,
        seq(
          field("target", $._expression_header),
          "with",
          field("control", $._expression_header),
        ),
      ),

    site_update_expression_header: ($) =>
      prec.dynamic(
        1,
        prec.left(
          23,
          seq(
            field("target", $._expression_header),
            "but",
            field("key", $._expression_key),
            ":",
            field("value", $._expression_header),
          ),
        ),
      ),

    but_expression_header: ($) =>
      prec.left(
        23,
        seq(
          field("target", $._expression_header),
          "but",
          field("override", $._expression_header),
        ),
      ),

    absent_expression_header: ($) =>
      prec.left(27, seq(field("value", $._expression_header), "?")),

    index_expression_header: ($) =>
      prec.left(
        27,
        seq(field("target", $._expression_header), field("index", $.subscript)),
      ),

    component_expression_header: ($) =>
      prec.right(
        25,
        seq(
          field("key", $.component_key),
          "of",
          field("value", $._expression_header),
        ),
      ),

    configure_dot_expression_header: ($) =>
      prec.left(
        27,
        seq(
          field("target", $._expression_header),
          field("operator", alias(token(/\./), ".")),
          field("config", $.map),
        ),
      ),

    // `_expression` without `config_expression`:
    // the rule a reference through `without` in the grammar's source
    // reads — a loop header's expression, whose brace and colon belong
    // to the header. Every node is aliased to its name in `_expression`, so
    // the tree is the same tree; the precedences are lifted to
    // `2p + 1` so the header's reading wins wherever the two could
    // take the same text.
    _expression_hole: ($) =>
      prec(
        1,
        choice(
          $.configurable_expression,
          $.texture_expression,
          $.call_expression,
          $.qualified_identifier,
          $.boolean,
          $.none,
          $.template_string,
          $.identifier,
          $.number,
          $.triple_string,
          $.string,
          $.list,
          $.product,
          $.parenthesized_expression,
          $.map,
          $.body,
          $.tap_expression,
          $.for_expression,
          $.each_expression,
          $.fold_expression,
          $.conditional_expression,
          $.match_expression,
          $.full_range,
          alias($.unary_expression_hole, $.unary_expression),
          alias($.periodic_range_hole, $.periodic_range),
          alias($.range_to_hole, $.range_to),
          alias($.range_expression_hole, $.range_expression),
          alias($.range_from_hole, $.range_from),
          alias($.periodic_expression_hole, $.periodic_expression),
          alias($.binary_expression_hole, $.binary_expression),
          alias($.with_expression_hole, $.with_expression),
          alias($.site_update_expression_hole, $.site_update_expression),
          alias($.but_expression_hole, $.but_expression),
          alias($.absent_expression_hole, $.absent_expression),
          alias($.index_expression_hole, $.index_expression),
          alias($.component_expression_hole, $.component_expression),
          alias($.configure_dot_expression_hole, $.configure_dot_expression),
          alias($.payload_expression_hole, $.payload_expression),
        ),
      ),

    unary_expression_hole: ($) =>
      choice(
        prec.right(
          9,
          seq(field("operator", "not"), field("operand", $._expression_hole)),
        ),
        prec.right(
          19,
          seq(field("operator", "-"), field("operand", $._expression_hole)),
        ),
        prec.right(
          19,
          seq(field("operator", "+"), field("operand", $._expression_hole)),
        ),
      ),

    periodic_range_hole: ($) =>
      prec.right(
        5,
        seq(
          field("operator", ".."),
          field("axis", alias($.range_expression_hole, $.range_expression)),
        ),
      ),

    range_to_hole: ($) =>
      prec.right(
        3,
        seq(
          field("operator", choice("..=", "..")),
          field("end", $._expression_hole),
        ),
      ),

    range_expression_hole: ($) =>
      prec.left(
        3,
        seq(
          field("left", $._expression_hole),
          field("operator", choice("..=", "..")),
          field("right", $._expression_hole),
        ),
      ),

    range_from_hole: ($) =>
      prec.left(
        3,
        seq(
          field("start", $._expression_hole),
          field("operator", choice("..=", "..")),
          $._no_operand,
        ),
      ),

    periodic_expression_hole: ($) =>
      prec.left(3, seq(field("axis", $._expression_hole), "periodic")),

    binary_expression_hole: ($) =>
      choice(
        prec.left(
          5,
          seq(
            field("left", $._expression_hole),
            field("operator", "or"),
            field("right", $._expression_hole),
          ),
        ),
        prec.left(
          7,
          seq(
            field("left", $._expression_hole),
            field("operator", "and"),
            field("right", $._expression_hole),
          ),
        ),
        prec.left(
          11,
          seq(
            field("left", $._expression_hole),
            field(
              "operator",
              choice(
                alias(token(/==/), "=="),
                alias(token(/!=/), "!="),
                alias(token(/<=/), "<="),
                alias(token(/>=/), ">="),
                alias(token(/</), "<"),
                alias(token(/>/), ">"),
              ),
            ),
            field("right", $._expression_hole),
          ),
        ),
        prec.right(
          13,
          seq(
            field("left", $._expression_hole),
            field("operator", alias(token(/\?\?/), "??")),
            field("right", $._expression_hole),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_hole),
            field("operator", alias(token(/\+/), "+")),
            field("right", $._expression_hole),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_hole),
            field("operator", alias(token(/<>/), "<>")),
            field("right", $._expression_hole),
          ),
        ),
        prec.left(
          15,
          seq(
            field("left", $._expression_hole),
            field("operator", "-"),
            field("right", $._expression_hole),
          ),
        ),
        prec.left(
          17,
          seq(
            field("left", $._expression_hole),
            field(
              "operator",
              choice(
                alias(token(/\*/), "*"),
                alias(token(/\/\//), "//"),
                alias(token(/\//), "/"),
                alias(token(/@/), "@"),
              ),
            ),
            field("right", $._expression_hole),
          ),
        ),
        prec.right(
          21,
          seq(
            field("left", $._expression_hole),
            field("operator", alias(token(/\^/), "^")),
            field("right", $._expression_hole),
          ),
        ),
      ),

    with_expression_hole: ($) =>
      prec.left(
        23,
        seq(
          field("target", $._expression_hole),
          "with",
          field("control", $._expression_hole),
        ),
      ),

    site_update_expression_hole: ($) =>
      prec.dynamic(
        1,
        prec.left(
          23,
          seq(
            field("target", $._expression_hole),
            "but",
            field("key", $._expression_key),
            ":",
            field("value", $._expression_hole),
          ),
        ),
      ),

    but_expression_hole: ($) =>
      prec.left(
        23,
        seq(
          field("target", $._expression_hole),
          "but",
          field("override", $._expression_hole),
        ),
      ),

    absent_expression_hole: ($) =>
      prec.left(27, seq(field("value", $._expression_hole), "?")),

    index_expression_hole: ($) =>
      prec.left(
        27,
        seq(field("target", $._expression_hole), field("index", $.subscript)),
      ),

    component_expression_hole: ($) =>
      prec.right(
        25,
        seq(
          field("key", $.component_key),
          "of",
          field("value", $._expression_hole),
        ),
      ),

    configure_dot_expression_hole: ($) =>
      prec.left(
        27,
        seq(
          field("target", $._expression_hole),
          field("operator", alias(token(/\./), ".")),
          field("config", $.map),
        ),
      ),

    payload_expression_hole: ($) =>
      choice(
        prec.left(
          27,
          seq(field("target", $._expression_hole), field("body", $.body)),
        ),
        prec.left(
          27,
          seq(field("target", $._expression_hole), field("body", $.list)),
        ),
      ),

    // only named functions can be called, and a used document's name is
    // reached through its namespace. Primes ride the head —
    // `f'(x)`, the univariate derivative at the prime count's order
    // — and a primed head is a call, so the parens
    // follow it. **The paren follows the head immediately**, as the
    // subscript's bracket follows its target: `f(x)` is a call and
    // `f (x)` is the name `f` beside a parenthesized value, which is
    // what lets a header's collection stand beside a bare body and a
    // Map walk's collection beside its entry. A
    // parenthesized fiber description heads a call too,
    // `(D: Real, E: Real)(1, 0)`, which is the anonymous spelling of a
    // declared product's constructor:
    // in head position the parens can hold nothing else, since neither
    // a product value nor a grouping takes arguments.
    call_expression: ($) =>
      prec(
        2,
        choice(
          prec.dynamic(
            3,
            seq(
              field("function", choice($.qualified_identifier, $.identifier)),
              field("variants", $.variant_arguments),
            ),
          ),
          prec.dynamic(
            3,
            seq(
              field("described", $.described_head),
              field("variants", $.variant_arguments),
            ),
          ),
          seq(
            field("function", choice($.qualified_identifier, $.identifier)),
            field("primes", $.primes),
            field("arguments", $.argument_list),
          ),
          seq(
            field("function", choice($.qualified_identifier, $.identifier)),
            field("arguments", $.argument_list),
          ),
          seq(
            field("function", "periodic"),
            field("arguments", $.argument_list),
          ),
          seq(field("function", "locked"), field("arguments", $.argument_list)),
          prec.dynamic(
            2,
            seq(
              field("described", $.described_head),
              field("arguments", $.argument_list),
            ),
          ),
        ),
      ),

    // `(D: Real, E: Real)` standing where a constructor's name stands —
    // in `(D: Real, E: Real)(1, 0)` and in the pattern
    // `(D: Real, E: Real)(D, E) := u`. The description names the
    // same fiber as its alias, written where no `fiber`
    // item declares one, and each component states the fiber that
    // component takes. The lookahead is the
    // interpreter's alone — a `component` states its own sentence for a
    // missing `:`, and that sentence belongs to the declaration rather
    // than to a paren the parser is still choosing between three
    // readings of; tree-sitter, which is GLR, prints without it. A
    // sum's variants stand here too, `(A: Real | B: Real)(A: 1.0)`
    // .
    described_head: ($) =>
      choice(
        seq(
          "(",
          field("variant", $.variant),
          repeat1(seq("|", field("variant", $.variant))),
          ")",
        ),
        seq(
          "(",
          repeat(choice(",", $._newline)),
          field("component", $.component),
          repeat(
            seq(
              repeat1(choice(",", $._newline)),
              field("component", $.component),
            ),
          ),
          repeat(choice(",", $._newline)),
          ")",
        ),
      ),

    // `(A: 1.0)` — the argument a sum fiber's constructor takes
    // : the variant it names and the payload it holds.
    // The rule takes a run of them so that a call naming none or
    // several is refused at elaboration, against the roster the
    // declaration states, rather than by the parser.
    variant_arguments: ($) =>
      seq(
        token.immediate("("),
        repeat(choice(",", $._newline)),
        $.variant_argument,
        repeat(seq(repeat1(choice(",", $._newline)), $.variant_argument)),
        repeat(choice(",", $._newline)),
        ")",
      ),

    // `A: 1.0` — the variant a sum fiber's constructor names and the
    // payload it holds. **A variant argument's colon is
    // tight**: it follows the name immediately, and `name : value`
    // with whitespace before the colon is a configuration zone on the
    // expression `name`, read by the positional
    // argument list. That is what tells the configured payload in
    // `f(block : { pad: 1 } { 1 }, "t")` from a constructor in both
    // readers (D:20).
    variant_argument: ($) =>
      seq(
        field("name", $.identifier),
        token.immediate(":"),
        field("value", $._expression),
      ),

    primes: (_) => token.immediate(/'+/),

    // the constitutive zone is positional; configuration rides after
    // ':'. A slot may open with a run of `:` marks — the call-site
    // derivative's slot, its colon count the order;
    // the dynamic precedence keeps `f(a : style)` an ascription on `a`
    // rather than `a` beside a marked second slot. A name inside the
    // parens is a configuration key in the wrong zone, and says so
    // . The `(` is immediate, so a call's parens
    // follow its head with no space between them.
    argument_list: ($) =>
      seq(
        token.immediate("("),
        repeat(choice(",", $._newline)),
        optional(
          seq(
            choice(
              prec.dynamic(-1, seq(field("mark", $.mark), $._expression)),
              $._expression,
            ),
            repeat(
              seq(
                repeat1(choice(",", $._newline)),
                choice(
                  prec.dynamic(-1, seq(field("mark", $.mark), $._expression)),
                  $._expression,
                ),
              ),
            ),
            repeat(choice(",", $._newline)),
          ),
        ),
        ")",
      ),

    // the rest of an argument list from a value through its `)`, each
    // further argument a value, optionally written after a misplaced
    // `key =`
    _argument_rest: ($) =>
      seq(
        $._expression,
        repeat(
          seq(
            repeat1(choice(",", $._newline)),
            optional(seq($.identifier, "=")),
            $._expression,
          ),
        ),
        repeat(choice(",", $._newline)),
        ")",
      ),

    mark: (_) => /:+/,

    // immediate `[`: `wave[i]` indexes, and a `[` after a space or on a
    // fresh line opens a row. Axis selectors are comma-separated.
    subscript: ($) =>
      seq(token.immediate("["), $.selector, repeat(seq(",", $.selector)), "]"),

    // One axis selector: an ordinary expression — an
    // Int global coordinate or a range restriction — or an open
    // range, where the target's own box bounds the missing end:
    // `a[4..]`, `a[..10]`, `a[..=10]`, `a[..]`.
    selector: ($) => choice($.open_range, $._expression),

    // `..` — the range with neither end, an ordinary value: `r = ..`,
    // `f(..)`. It stands where no end follows; `..e` takes one.
    full_range: ($) => seq(field("operator", ".."), $._no_operand),

    // An axis open at one end or both, as a List or box entry or a
    // subscript's selector, where the target's own box bounds the
    // missing end. Elsewhere the same ranges are ordinary values
    // through `range_to`, `range_from` and `full_range`. `..=` with no
    // end has nothing to reach, which a later analysis stage says.
    open_range: ($) =>
      choice(
        prec.dynamic(
          1,
          prec.right(
            1,
            seq(
              field("operator", choice("..=", "..")),
              choice(field("end", $._expression), $._no_operand),
            ),
          ),
        ),
        prec(
          -1,
          seq(
            field("start", $._expression),
            field("operator", choice("..=", "..")),
            $._no_operand,
          ),
        ),
      ),

    // The open axis a `for` header's collection states alone,
    // `for i in .. { … }`, `for i in 3.. { … }`: its ends are the
    // header's own expressions, and a missing end reads no brace,
    // which is the header's body.
    header_open_range: ($) =>
      choice(
        prec.dynamic(
          1,
          prec.right(
            3,
            seq(
              field("operator", choice("..=", "..")),
              choice(field("end", $._expression_header), $._no_operand),
            ),
          ),
        ),
        prec(
          3,
          seq(
            field("start", $._expression_header),
            field("operator", choice("..=", "..")),
            $._no_operand,
          ),
        ),
      ),

    // `:base :over { body }` — ordered settings atoms and a value body
    configurable_expression: ($) =>
      seq(
        repeat1(
          seq(
            field("operator", alias(token(/:/), ":")),
            field("config", $._config_atom),
          ),
        ),
        field("body", $.body),
      ),

    // a name, and what a name reaches: an index picks a style out of a
    // list of them and a call asks a function for one.
    // The chain stops before a brace, which belongs to the head, and
    // the call and the index bind to the atom rather than to the
    // ascription that holds it: `el : styles[i]` picks a style, where
    // `(el : styles)[i]` would index a dressed element.
    _config_atom: ($) =>
      choice(
        $.map,
        prec.dynamic(1, $.index_expression),
        prec.dynamic(1, $.call_expression),
        $.qualified_identifier,
        $.identifier,
        $.parenthesized_expression,
      ),

    // `fold u := u0 for x in seq while G : { max_iter } { body }` and
    // `reduce s := s0 …` — the bounded recurrence and the header that
    // answers its destination. One rule,
    // because it is one header: the keyword is the whole of the
    // difference. The dynamic precedence gives the brace to the body.
    fold_expression: ($) =>
      prec.dynamic(
        1,
        seq(
          field("keyword", choice("fold", "reduce")),
          field("accumulator", $.identifier),
          ":=",
          field("seed", $._expression_header),
          optional(
            seq(
              "for",
              field("element", $.binder_list),
              "in",
              field("driver", choice($.header_open_range, $._expression_header)),
            ),
          ),
          optional(seq("while", field("guard", $._expression_header))),
          repeat(
            seq(
              field("operator", alias(token(/:/), ":")),
              field("config", $._config_atom),
            ),
          ),
          field("body", $.body),
        ),
      ),

    // `if c { a } else { b }` — the conditional, a member of the header
    // family beside `match` and `for`. The condition stands in the
    // header's own zone, so the brace after it always opens the
    // consequence and is never a payload on the condition. **Each branch
    // is a braced body**, holding `:=` locals and requirements beside
    // its one result, and its brace stands on the line the condition
    // or the `else` ends. `else if` chains without further braces,
    // `if a { x } else if b { y } else { z }`. Without the `else` clause
    // a later analysis stage supplies `none` for the alternative.
    //
    // The braces make the form whole, as a `match` is: it is a primary
    // expression, so `if c { a } else { b } + 1` adds one to the
    // conditional, and no `else` is left for an inner `if` to take.
    // `else` is a continuation word, so it may open a line of its
    // own after the consequence's closing brace.
    conditional_expression: ($) =>
      seq(
        "if",
        field("condition", $._expression_header),
        field("consequence", $.body),
        optional(
          seq(
            "else",
            field("alternative", choice($.body, $.conditional_expression)),
          ),
        ),
      ),

    // `match s { A(x) { x }, B(v) { norm(v) } }` — the sum fiber's
    // reader, the header family's member beside `if` and `for`.
    // The scrutinee stands in the header's own zone,
    // so the brace after it holds the arms rather than a payload on
    // the scrutinee, and the arms separate as body items do, one per
    // line or by commas. A trailing `else` arm stands where no
    // variant's arm did; that every variant has an arm or an `else`
    // stands, elaboration states against the declared roster.
    match_expression: ($) =>
      prec.dynamic(
        1,
        seq(
          "match",
          field("scrutinee", $._expression_header),
          "{",
          repeat(choice(",", $._newline)),
          optional(
            seq(
              choice(seq($.match_arm, optional($.else_arm)), $.else_arm),
              repeat(
                seq(
                  repeat1(choice(",", $._newline)),
                  choice(seq($.match_arm, optional($.else_arm)), $.else_arm),
                ),
              ),
              repeat(choice(",", $._newline)),
            ),
          ),
          "}",
        ),
      ),

    // `A(x) { x }`, `B(X: x, Y: y) { x + y }` and `Binary { 2 }` —
    // one arm of a `match`: the variant's
    // name, one sub-pattern for its payload where it takes one,
    // and the braced body the arm takes, an ordinary body with its
    // locals and one result. Since a variant holds
    // exactly one payload, the parens may hold the items of the
    // payload's named-parens pattern directly, `B(X: x, Y: y)`
    // standing for `B((X: x, Y: y))`; the colons tell that spelling
    // from the positional one, and both are admitted. **A variant
    // that takes no payload is read by its bare name**, and that an
    // arm's parens agree with the roster, elaboration states.
    match_arm: ($) =>
      seq(
        field("variant", $.identifier),
        optional(
          seq(
            "(",
            choice(
              seq(
                repeat(choice(",", $._newline)),
                $.named_sub_pattern,
                repeat(
                  seq(repeat1(choice(",", $._newline)), $.named_sub_pattern),
                ),
                repeat(choice(",", $._newline)),
              ),
              field("pattern", $._pattern),
            ),
            ")",
          ),
        ),
        field("body", $.body),
      ),

    // `else { e }` — the arm a `match` takes where no variant's arm
    // stood. One of these stands in place of the arms the
    // roster is missing. Its `else` is the continuation word, so an
    // `else` arm opening a line follows the arm above it whether or not
    // a comma ends that arm, and on one line it may follow an arm's
    // closing brace directly, as a conditional's `else` does.
    else_arm: ($) => seq("else", field("body", $.body)),

    // `for [i, j] in space { body }` and `for I in space body` — the
    // comprehension, a member of the header family.
    // The braced body has the higher dynamic precedence,
    // which is how the grammar states the loop-header zone rule: a
    // brace after the space is the comprehension's body, never a
    // payload on the space. The brace is optional, as it is on a
    // function definition — a bare body may not open with `(`, `[` or
    // an operator, since juxtaposition is whitespace-insensitive here
    // and the space would take them.
    for_expression: ($) =>
      seq(
        "for",
        field("binders", $.binder_list),
        "in",
        field("space", choice($.header_open_range, $._expression_header)),
        choice(
          prec.dynamic(2, field("body", $.body)),
          prec.dynamic(1, seq($._bare_operand, field("body", $._expression))),
        ),
      ),

    // `each x in xs { body }` and `each x in xs body` — the List
    // comprehension, `for`'s sibling: it walks a
    // List, a Map, a range or a box of any rank and builds a `List` of
    // the body's results
    // where `for` builds a located field. The header shape is `for`'s
    // clause for clause.
    each_expression: ($) =>
      seq(
        "each",
        field("binders", $.binder_list),
        "in",
        field("collection", choice($.header_open_range, $._expression_header)),
        choice(
          prec.dynamic(2, field("body", $.body)),
          prec.dynamic(1, seq($._bare_operand, field("body", $._expression))),
        ),
      ),

    // a header's binder is one pattern: a
    // name takes the element whole, a bracket destructures — the site of
    // a box one name per axis, or a lockstep's collections one pattern
    // apiece, which a later analysis stage tells apart by the collection — and a
    // constructor pattern reads components. `(i, j)` is no pattern here,
    // and says so naming the bracket. Two or more patterns written
    // with commas and no bracket are the bracket's pattern
    // (`bare_list_pattern`): `for i, j in [.., ..]` is `for [i, j] in`.
    binder_list: ($) => choice($.bare_list_pattern, $._pattern),

    // A pattern — what stands wherever a name is bound: a
    // local, a `for` binder, a parameter, a control binder and a
    // top-level item. A name binds the value whole, `_` binds nothing,
    // `[i, j]` reads a static list's positions,
    // `Cons(d, s, tau)` reads a declared product's components in the
    // declared order, `(at: p)` reads components by name
    // , and `{ rho, vel }` reads a Map's entries by
    // key. Every sub-pattern position takes any pattern, so the form
    // nests. The named parens begin the way an expression does, and
    // the token after the first name decides; the braced form begins
    // the way a Map literal does, and the `:=` after it decides. A
    // positional tuple, `(rho, _, pre)`, stands only at a parameter or
    // a local stating the fiber it reads.
    _pattern: ($) =>
      choice(
        $.constructor_pattern,
        $.named_pattern,
        $.list_pattern,
        $.map_pattern,
        $.identifier,
      ),

    // `[i, j]`, `[[i, j], t]` — a static list destructured, one
    // sub-pattern per position: the List literal read
    // backwards. The length is the right side's kind's, so a list of no
    // known length refuses at check time. A position is a pattern again,
    // and a parenthesized one is none: a bracket nests in a bracket.
    list_pattern: ($) =>
      seq(
        "[",
        repeat(choice(",", $._newline)),
        $._pattern,
        repeat(seq(repeat1(choice(",", $._newline)), $._pattern)),
        repeat(choice(",", $._newline)),
        "]",
      ),

    // `i, j` and `a, [b, c]` — a static list destructured with no
    // bracket, exactly `[i, j]`, wherever a whole binder stands: a
    // comprehension's header, a local before its `:=` and a top-level
    // pattern item before its `=`. It takes two
    // positions or more, so one name stays a name. Inside a brace a
    // run of names followed by `:=` is this pattern rather than an item
    // and a local, `{ a, b := t … }` binding both; a position is a
    // pattern again and nests with a bracket. Tree-sitter reads the
    // brace's other reading, an item and then a local, beside it, and
    // the dynamic precedence keeps this one, as the interpreter's
    // order does.
    bare_list_pattern: ($) =>
      prec.dynamic(1, seq($._pattern, repeat1(seq(",", $._pattern)))),

    // `Cons(d, vec(sx, sy), _)` — the components of `vec` or of a
    // declared product fiber, positional in the declared order, one
    // sub-pattern each. It mirrors the constructor literal
    // , and the arity is checked against the
    // declaration at elaboration. A fiber description heads it too,
    // `(D: Real, E: Real)(D, E) := u`, which states the fiber the
    // value takes and reads its components. The paren
    // follows the head immediately, as the constructor literal's does.
    constructor_pattern: ($) =>
      choice(
        seq(
          field("head", $.identifier),
          token.immediate("("),
          repeat(choice(",", $._newline)),
          $._pattern,
          repeat(seq(repeat1(choice(",", $._newline)), $._pattern)),
          repeat(choice(",", $._newline)),
          ")",
        ),
        prec.dynamic(
          2,
          seq(
            field("described", $.described_head),
            token.immediate("("),
            repeat(choice(",", $._newline)),
            $._pattern,
            repeat(seq(repeat1(choice(",", $._newline)), $._pattern)),
            repeat(choice(",", $._newline)),
            ")",
          ),
        ),
      ),

    // `(at: p, grab: q)`, `(0: x, 2: z)` — components by key, any subset
    // in any order. It mirrors the value form a readout writes for an
    // unnamed product, it is the form the gesture
    // products take, and an integer key reads a `vec` or a `Complex`
    // by position.
    named_pattern: ($) =>
      seq(
        "(",
        repeat(choice(",", $._newline)),
        $.named_sub_pattern,
        repeat(seq(repeat1(choice(",", $._newline)), $.named_sub_pattern)),
        repeat(choice(",", $._newline)),
        ")",
      ),

    named_sub_pattern: ($) =>
      seq(
        field("name", choice($.identifier, $.number)),
        ":",
        field("pattern", $._pattern),
      ),

    // `{ rho, vel }`, `{ rho: r }`, `{ pos: (x, y) }` — a Map's entries
    // by key: the Map literal read backwards, with the binder
    // where the literal puts the value. A bare name is the key and the
    // binder at once, renaming puts a sub-pattern after the colon as a
    // named-parens pattern does, and the sub-pattern nests. A key
    // stands twice refuses, and each key is the subscript that reads
    // it.
    map_pattern: ($) =>
      seq(
        "{",
        repeat(choice(",", $._newline)),
        $.map_sub_pattern,
        repeat(seq(repeat1(choice(",", $._newline)), $.map_sub_pattern)),
        repeat(choice(",", $._newline)),
        "}",
      ),

    map_sub_pattern: ($) =>
      choice(
        seq(
          field("key", $.identifier),
          alias(token(/:/), ":"),
          field("pattern", $._pattern),
        ),
        field("key", $.identifier),
      ),

    // `(rho, _, pre): Primitive` — a positional tuple, one sub-pattern
    // per component of the fiber the binding site states after it, in
    // declared order, a nested tuple reading a
    // component's own components, `(d, (sx, _), _): Cons`. A site is a
    // static list and destructures with a bracket, `[i, j]`
    // .
    site_pattern: ($) =>
      seq(
        "(",
        choice($.site_pattern, $._pattern),
        repeat(seq(",", choice($.site_pattern, $._pattern))),
        ")",
      ),

    // The key of a component read: a name for a
    // declared product fiber's component and for a `Color`'s channel,
    // an integer for a `vec` or a `Complex`, and a
    // parenthesized pair for a `Mat` or a `Metric`. Nothing else
    // stands before `of`, so the key is read at elaboration rather
    // than computed.
    component_key: ($) =>
      choice(
        $.identifier,
        $.number,
        seq("(", $.number, repeat1(seq(",", $.number)), ")"),
      ),

    // `(0, 1)` — a `Mat`'s or a `Metric`'s entry named by its row and
    // column, where a product's component key stands.
    position_pair: ($) => seq("(", $.number, repeat1(seq(",", $.number)), ")"),

    // `(D: 1.0, S: vec(0.3, 0.4), tau: 2.5)` — a product fiber's value
    // stated component by component, and the override
    // a `but` takes, one component included. A component
    // key and its colon tell it from grouping parens, which hold a bare
    // expression. An integer key names
    // a component by position, so `v but (0: e)` updates a `vec`
    // .
    product: ($) =>
      seq(
        "(",
        repeat(choice(",", $._newline)),
        $.product_component,
        repeat(seq(repeat1(choice(",", $._newline)), $.product_component)),
        repeat(choice(",", $._newline)),
        ")",
      ),

    // the component wins over reading the key as an expression with a
    // configuration after it. A parenthesized run of
    // positions in key position is a component's position written out,
    // `m but ((0, 1): 1.0)` one entry of a `Mat`; a key is never a site,
    // the sites of a field being updated outside the parens,
    // `u but [0..1, 1..2]: 1.0`.
    product_component: ($) =>
      prec(
        14,
        seq(
          field("name", choice($.identifier, $.number, $.position_pair)),
          ":",
          field("value", $._expression),
        ),
      ),

    // `(3, 4)` and `(3,)` refuse: parens group one expression, and a
    // site is the static list of its Ints, `[3, 4]`
    parenthesized_expression: ($) =>
      seq(
        "(",
        repeat(choice(",", $._newline)),
        $._expression,
        repeat(choice(",", $._newline)),
        ")",
      ),

    // a List literal, and the box literal a header walks: an entry may
    // be an open range, as in `[.., 0..n]`
    list: ($) =>
      seq(
        "[",
        repeat(choice(",", $._newline)),
        optional(
          seq(
            choice($.open_range, $._expression),
            repeat(
              seq(
                repeat1(choice(",", $._newline)),
                choice($.open_range, $._expression),
              ),
            ),
            repeat(choice(",", $._newline)),
          ),
        ),
        "]",
      ),

    // `tap (g)? (: config)* { p <- e, … }` — the gesture-triggered writes,
    // which are the whole of the control. The config zone stands between
    // the head and its writes, as it does on a `fold`. The optional
    // binder is the form the rest of the control roster takes
    // : `g` is bound to the gesture, and the body
    // reads it by dot. `scrub(g)`, `hover(g)` and `wheel(g)` reach the
    // same reading through `call_expression` under a juxtaposed writes
    // brace, since those heads are ordinary names where `tap` is a
    // keyword.
    tap_expression: ($) =>
      seq(
        "tap",
        optional(seq("(", field("binder", $._pattern), ")")),
        repeat(
          seq(
            field("operator", alias(token(/:/), ":")),
            field("config", $._config_atom),
          ),
        ),
        field("body", $.body),
      ),

    // `{ name: v, expression => w }` — a brace literal with static
    // names and computed keys in source order. An empty brace is
    // a literal rather than a body. The older `(e): v` and embedded
    // walk forms remain CST-only; normalization rejects them.
    map: ($) =>
      prec(
        1,
        seq(
          "{",
          repeat(choice(",", $._newline)),
          optional(
            seq(
              choice($.map_walk, $.map_entry),
              repeat(
                seq(
                  repeat1(choice(",", $._newline)),
                  choice($.map_walk, $.map_entry),
                ),
              ),
              repeat(choice(",", $._newline)),
            ),
          ),
          "}",
        ),
      ),

    // `name: v` or `expression => v` — one literal entry. A name, a colon and a value
    // is also a body item configuring that name, `{ w: style }`, and
    // the entry's dynamic precedence settles the brace toward the Map
    // . It stands on the entry rather than on the brace so
    // that it grows with the entries, one configuration link being
    // dynamic itself.
    map_entry: ($) =>
      prec.dynamic(
        2,
        choice(
          seq(
            field("key", choice($.identifier, $.computed_key)),
            field("separator", alias(token(/:/), ":")),
            field("value", $._expression),
          ),
          seq(
            field("key", $._expression),
            field("separator", alias(token(/=>/), "=>")),
            field("value", $._expression),
          ),
        ),
      ),

    // `(expr)` — a Map entry's key computed rather than named, since
    // a bare identifier in a Map literal is the key's own name
    // . The expression returns a Str.
    computed_key: ($) =>
      seq(
        "(",
        repeat(choice(",", $._newline)),
        $._expression,
        repeat(choice(",", $._newline)),
        ")",
      ),

    // `each k in ks (k): f(k)` — a comprehension among a Map
    // literal's entries: the header is `each`'s, and
    // the body is the one entry the walk writes per element, whose
    // key is parenthesized. **The body is that entry, braced or
    // bare**: `each k in ks (k): v` and `each k in ks { (k): v }`
    // are the one walk, as a List's splice takes both spellings.
    // The braceless form reads because a call's paren follows its
    // head immediately, so the collection ends at the space before
    // the key. The walk is one item of the literal, so its entries
    // land in walk order among the others. The dynamic precedence
    // outranks the braced comprehension body's, so a brace holding a
    // walk reads as the Map it is rather than as a body whose one
    // item is an `each`.
    map_walk: ($) =>
      prec.dynamic(
        3,
        seq(
          "each",
          field("binders", $.binder_list),
          "in",
          field("collection", $._expression_header),
          field("body", choice($.map, $.map_walk_entry)),
        ),
      ),

    // `(k): v` standing as a Map literal's walk body with no brace
    // around it. It is the entry `map_entry` spells,
    // held to the computed key: a bare name is the key's own name,
    // which every element would then share, and the walk's brace-free
    // body is the one place where a bare name here would also open a
    // comprehension's bare body dressed by a configuration zone. The
    // braced spelling takes `map_entry` through `map`, and the two
    // lower as one entry.
    map_walk_entry: ($) =>
      seq(
        field("key", $.computed_key),
        alias(token(/:/), ":"),
        field("value", $._expression),
      ),

    // A braced body can hold `:=` locals, expressions and `<-`
    // writes. The enclosing construct determines which items it accepts.
    // A durable local refuses here, on A7's ground: a
    // body's line stands for many instances, and a durable write
    // rewrites one declaring line.
    body: ($) =>
      seq(
        "{",
        repeat(choice(",", $._newline)),
        optional(
          seq(
            choice(
              $.require_statement,
              $.local,
              $.state_local,
              $.write,
              $._expression,
            ),
            repeat(
              seq(
                repeat1(choice(",", $._newline)),
                choice(
                  $.require_statement,
                  $.local,
                  $.state_local,
                  $.write,
                  $._expression,
                ),
              ),
            ),
            repeat(choice(",", $._newline)),
          ),
        ),
        "}",
      ),

    // `x := e` and `Cons(d, s, _) := u` — a local, whose left side is a
    // pattern. The parenthesized patterns begin the way an
    // expression does, and the `:=` is what tells a local from a bare
    // item. A local states an optional fiber after `:`, `a: Real := 3`,
    // and the value it binds is held to it; the
    // lowering states that the annotation stands on a bare name, since
    // a pattern already states what the value is. The annotation's
    // colon is the one a Map entry and a configuration zone take, so
    // a brace opening `{ c: Real := 2` shifts one token for all
    // three readings and keeps them alive to the `:=` that settles
    // them. A positional tuple stands only with the fiber it reads
    // stated after it, `(rho, _, pre): Primitive := p`
    // .
    local: ($) =>
      seq(
        choice(
          seq(
            field("pattern", $.site_pattern),
            alias(token(/:/), ":"),
            field("type", $.type_annotation),
          ),
          field("pattern", $.bare_list_pattern),
          seq(
            field("pattern", $._pattern),
            optional(
              seq(alias(token(/:/), ":"), field("type", $.type_annotation)),
            ),
          ),
        ),
        ":=",
        field("value", $._expression),
      ),

    // `state name := init` — a local transient port, declared inside a
    // block or a function body. It states an
    // optional fiber after `:` as every other binding site does
    // .
    state_local: ($) =>
      seq(
        "state",
        field("name", $.identifier),
        optional(seq(":", field("type", $.type_annotation))),
        ":=",
        field("value", $._expression),
      ),

    // `p <- e` — this port takes that expression's value, one batch.
    // **A write names a whole State**: `u[i, j] <- e` is no write, and
    // the subscript is read, its `[` against the name as an index's is,
    // so that source normalization refuses it by name rather than the
    // line failing to parse.
    write: ($) =>
      prec.dynamic(
        1,
        seq(
          field("target", $.identifier),
          optional(field("key", $.subscript)),
          "<-",
          field("value", $._expression),
        ),
      ),

    // `fig::panel` — one qualified identifier reaching into a
    // namespace: a used document's, or one the language
    // knows. `::` is the namespace separator everywhere, and it is the
    // only one: a dot between two names is refused with this spelling,
    // and a component is read by its key. The
    // lowering joins the segments into one name and elaboration reads
    // it by what the head names.
    qualified_identifier: ($) =>
      seq(
        field("namespace", $.identifier),
        repeat1(seq("::", field("name", $.identifier))),
      ),

    // `f"e = {e:.3} deg"` and `f"""…"""` — the tagged literal, whose
    // holes hold expressions. The `f` stands against the
    // opening quote and tags it, so an untagged literal keeps every
    // meaning it has, braces included. A chunk is the text between the
    // holes; `{{` and `}}` spell a brace; an escape is read as the
    // quoted literal's is, and the triple-quoted form takes its holes
    // with the raw form's other rules. A hole holds any expression of
    // the language, resolved in the enclosing scope as any
    // subexpression is, and states an optional format after `:`.
    //
    // The literal desugars to the join before any node exists, so the
    // graph holds what the operator spelling holds.
    template_string: ($) =>
      choice(
        seq(
          alias(token(/f"""/), "f\"\"\""),
          repeat(
            choice($.raw_chunk, $.raw_quote, $.brace_escape, $.substitution),
          ),
          token.immediate("\"\"\""),
        ),
        seq(
          alias(token(/f"/), "f\""),
          repeat(choice($.chunk, $.escape, $.substitution)),
          token.immediate("\""),
        ),
      ),

    // the text between a quoted tagged literal's holes: everything but
    // the delimiter, the braces a hole and its escape are spelled
    // with, a backslash and a newline
    chunk: (_) => token.immediate(/[^"{}\\\n]+/),

    // the same, inside `f"""…"""`: a backslash and a newline stand
    // for themselves, as in the untagged raw literal, and so does a
    // quotation mark where three do not close the literal; the text
    // reaches the value as written
    raw_chunk: (_) => token.immediate(/(([^"{}]|"[^"{}]|""[^"{}]))+/),

    // `"` or `""` inside `f"""…"""` where a brace follows — raw text
    // that a raw chunk, whose quotes take a following character, cannot
    // end with. Three quotation marks close the literal instead.
    raw_quote: (_) => token.immediate(/""|"/),

    // `{{` and `}}` inside `f"""…"""` — a brace written as two, since a
    // single brace opens a hole; the raw form has no backslash escape.
    brace_escape: (_) => token.immediate(/\{\{|\}\}/),

    // `\n`, `\"`, `{{` and `}}` — one character written as two inside
    // `f"…"`. The backslash escapes are the quoted literal's
    // and the doubled braces are the tagged literal's own,
    // since a single brace opens a hole.
    escape: (_) => token.immediate(/\\.|\{\{|\}\}/),

    // `{e}` and `{e:.3}` — one hole of a tagged literal: an expression,
    // and the format its value is written in. The configuration zone
    // is left out of the hole's expression, since the colon inside a
    // hole opens the format: a configured value is bound above the
    // literal and named here.
    substitution: ($) =>
      seq(
        token.immediate("{"),
        field("expr", $._expression_hole),
        optional(field("spec", $.format_spec)),
        "}",
      ),

    // `:.3`, `:sci`, `:eng` and `:.3sci` — the format a hole's value is
    // written in: `fmt`'s two keys and nothing more, the digits after
    // the point and the notation. It stands immediately after the
    // expression, with no space before the colon, which is what tells
    // it from the configuration zone's colon. Which texts are formats,
    // a later analysis stage states, so a spelling that is none is named in a
    // sentence rather than left to the closing brace.
    format_spec: (_) => token.immediate(/:[._A-Za-z0-9]+/),

    identifier: (_) => /[_A-Za-z][_A-Za-z0-9]*/,

    // the fractional part takes at least one digit, so the dots of
    // `0..10` belong to the range and not to the number; `1.0` is the
    // spelling of one as a Real, and `1.` is not a literal
    number: (_) =>
      token(
        seq(
          /[0-9][_0-9]*/,
          optional(/\.[0-9][_0-9]*/),
          optional(/[eE][+\-]?[0-9]+/),
        ),
      ),

    string: (_) => token(seq("\"", repeat(choice(/[^"\\\n]/, /\\./)), "\"")),

    // `"""…"""` — the multi-line literal. Three quotes win
    // over two by the lexer's longest match, so `""` keeps its meaning.
    // The contents are raw, and the content token states exactly the
    // scan: everything up to the first `"""`. Its own node carries the
    // prose, which is what `injections.scm` renders as markdown.
    triple_string: ($) =>
      seq(
        "\"\"\"",
        optional(field("text", $.triple_string_content)),
        token.immediate("\"\"\""),
      ),

    triple_string_content: (_) => token.immediate(/(([^"]|"[^"]|""[^"]))+/),

    boolean: (_) => choice("true", "false"),

    // `none` — the None unit kind's sole value: the
    // literal a hover writes at the pointer's leaving, and the one a
    // durable absent port carries in its text.
    none: (_) => "none",
  },
});
