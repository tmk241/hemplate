use hemplate_fmt::format_heml;

fn fmt(source: &str) -> String {
    format_heml(source, "test.heml").unwrap()
}

#[test]
fn canonical_layout_and_attribute_order() {
    assert_eq!(
        fmt("<div><p>hello</p><p>world</p></div>"),
        "<div>\n  <p>hello</p>\n  <p>world</p>\n</div>\n"
    );
    assert_eq!(
        fmt("<div>\n       <p>hello</p>\n</div>\n"),
        "<div>\n  <p>hello</p>\n</div>\n"
    );
    assert_eq!(
        fmt("<p>Hello   <b>{+ self.name +}</b>!</p>"),
        "<p>Hello <b>{+ self.name +}</b>!</p>\n"
    );
    assert_eq!(
        fmt("<p>&amp; <!--  keep  -->&lt;</p>"),
        "<p>&amp; <!--  keep  -->&lt;</p>\n"
    );
    assert_eq!(
        fmt("<p>a\u{2028}b\u{2029}c\u{a0}d</p>"),
        "<p>a\u{2028}b\u{2029}c\u{a0}d</p>\n"
    );
    // A soft space between blocks is rendered, unlike a newline: keep it.
    assert_eq!(
        fmt("<div><p>a</p>  <p>b</p></div>"),
        "<div><p>a</p> <p>b</p></div>\n"
    );
    assert_eq!(
        fmt("<input  id = 'query' disabled +checked=\"self.checked\" h-if='true'>"),
        "<input  id = 'query' disabled +checked=\"self.checked\" h-if='true'>\n"
    );
    let long = format!(
        "<input id='{}' disabled +checked=\"self.x\" h-if='true'>",
        "x".repeat(75)
    );
    assert_eq!(fmt(&long), format!("{long}\n"));
    for width in [100, 101] {
        let source = format!("<input id='{}'>", "x".repeat(width - 13));
        assert_eq!(source.chars().count(), width);
        assert_eq!(fmt(&source), format!("{source}\n"));
    }
}

#[test]
fn opaque_content_is_byte_identical() {
    for source in [
        "<pre>  x\n    y\r\n</pre>",
        "<textarea>\n  {+ self.x +}  \n</textarea>",
        "<script  type='module'>let x =  1;\r\n  alert(x);</script>",
        "<style>p { white-space:  pre; }\n</style>",
        "<!--  comment\n    with spaces  -->",
        "<p>{+ { /* +} */ let x = \"+}\";\n   x } +}{+= self.raw =+}</p>",
        "<p +class='if true { \"a  b\" } else { \"x\" }'>x</p>",
        "<svg><text>  x\n  y </text></svg>",
        "\u{feff}\u{feff}<p>x</p>",
    ] {
        let expected = if source.contains('\u{feff}') {
            source.to_owned()
        } else {
            format!("{source}\n")
        };
        assert_eq!(fmt(source), expected);
        assert_eq!(fmt(&fmt(source)), fmt(source));
    }
}

#[test]
fn malformed_source_returns_unchanged_input_and_file_position() {
    let source = "<div>\n  <p title=\"unterminated";
    let error = format_heml(source, "templates/broken.heml").unwrap_err();
    assert_eq!(error.source, source);
    assert_eq!(error.file.to_str(), Some("templates/broken.heml"));
    assert!(error.line > 0 && error.column > 0);
    assert!(error.to_string().starts_with(&format!(
        "templates/broken.heml:{}:{}:",
        error.line, error.column
    )));
}

#[test]
fn representative_layouts_are_idempotent_and_parseable() {
    for source in [
        "", "   \n", "plain   text", "<div><br>hello<hr></div>",
        "<p>&amp; &lt; &nbsp;</p>",
        "<template h-match='self.x'><span h-case='true'>y</span><span h-case='_'>n</span></template>",
        "<!doctype HTML><html><head></head><body><!-- x --><p>hi</p></body></html>",
        "<div>text <section><p>nested</p></section> text</div>",
        "<p><span>a</span>\n <span>b</span></p>",
        "<img  src='x' />",
        "<p>before\n{+ self.x +}   after</p>",
    ] {
        let first = fmt(source);
        assert_eq!(fmt(&first), first, "{source}");
        assert!(!hemplate_parser::parse_template(&first, None).unwrap().root_node().has_error());
    }
}

#[test]
fn raw_text_exemptions_follow_the_scanners_namespace_decision() {
    for wrapper in [
        "<svg>MARK</svg>",
        "<math>MARK</math>",
        "<math><foreignObject>MARK</foreignObject></math>",
        "<math><title>MARK</title></math>",
        "<math><mtext><mglyph>MARK</mglyph></mtext></math>",
        "<math><annotation-xml encoding='application/xml'>MARK</annotation-xml></math>",
        "<svg><foreignObject><svg>MARK</svg></foreignObject></svg>",
    ] {
        for tag in [
            "xmp",
            "iframe",
            "noembed",
            "noframes",
            "plaintext",
            "script",
            "style",
            "title",
            "textarea",
        ] {
            let source = wrapper.replace("MARK", &format!("<{tag}>{{+ broken</{tag}>"));
            let error = format_heml(&source, "foreign.heml").unwrap_err();
            assert_eq!(error.source, source);
            assert_eq!(
                (error.line, error.column),
                (1, source.find("{+").unwrap() + 1)
            );
        }
    }
    for source in [
        "<svg><foreignObject><xmp>{+ broken</xmp></foreignObject></svg>",
        "<svg><desc><iframe>{+ broken</iframe></desc></svg>",
        "<svg><title><noembed>{+ broken</noembed></title></svg>",
        "<math><mtext><noframes>{+ broken</noframes></mtext></math>",
        "<math><annotation-xml encoding='text/html'><xmp>{+ broken</xmp></annotation-xml></math>",
        "<svg><div><xmp>{+ broken</xmp></div></svg>",
    ] {
        assert_eq!(fmt(source), format!("{source}\n"));
    }
}
