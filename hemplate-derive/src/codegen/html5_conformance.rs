//! HTML5 differential gate with explicit, mechanically checked deviations.
//! See tests/html5lib-tests/SOURCE.md for provenance and interpretation.

use super::generate_body;
use hemplate_core::ast::build_ast;
use html5ever::interface::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::tendril::{StrTendril, TendrilSink};
use html5ever::{ns, parse_document, parse_fragment, Attribute, ParseOpts, QualName};
use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;
use std::rc::{Rc, Weak};
use std::sync::Arc;

// html5ever 0.40 and the latest published rcdom (0.39) use incompatible
// TreeSink traits. This small sink implements only the DOM operations needed
// for parser-only dumps, without adding a second HTML parser version.
type Handle = Rc<Node>;
struct Node {
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Handle>>,
    data: Data,
}
enum Data {
    Document,
    Element {
        name: QualName,
        attrs: RefCell<Vec<Attribute>>,
        template: Option<Handle>,
        math_integration: bool,
    },
    Text(RefCell<String>),
    Comment(String),
    Doctype(String, String, String),
    Pi(String, String),
}
fn node(data: Data) -> Handle {
    Rc::new(Node {
        parent: RefCell::new(Weak::new()),
        children: RefCell::new(Vec::new()),
        data,
    })
}
struct Dom {
    document: Handle,
}
impl Default for Dom {
    fn default() -> Self {
        Self {
            document: node(Data::Document),
        }
    }
}
fn detach(child: &Handle) {
    if let Some(parent) = child.parent.borrow_mut().upgrade() {
        parent
            .children
            .borrow_mut()
            .retain(|n| !Rc::ptr_eq(n, child));
    }
    *child.parent.borrow_mut() = Weak::new();
}
fn insert(parent: &Handle, before: Option<&Handle>, child: NodeOrText<Handle>) {
    // Detach before finding the index: adoption-agency moves may share a parent.
    if let NodeOrText::AppendNode(child) = &child {
        detach(child);
    }
    let mut children = parent.children.borrow_mut();
    let index = before
        .map(|sibling| {
            children
                .iter()
                .position(|n| Rc::ptr_eq(n, sibling))
                .unwrap()
        })
        .unwrap_or(children.len());
    let child = match child {
        NodeOrText::AppendText(text) => {
            if index > 0 {
                if let Data::Text(previous) = &children[index - 1].data {
                    previous.borrow_mut().push_str(&text);
                    return;
                }
            }
            node(Data::Text(RefCell::new(text.to_string())))
        }
        NodeOrText::AppendNode(child) => child,
    };
    *child.parent.borrow_mut() = Rc::downgrade(parent);
    children.insert(index, child);
}
impl TreeSink for Dom {
    type Handle = Handle;
    type Output = Self;
    type ElemName<'a> = &'a QualName;
    fn finish(self) -> Self {
        self
    }
    fn parse_error(&self, _: Cow<'static, str>) {}
    fn get_document(&self) -> Handle {
        self.document.clone()
    }
    fn elem_name<'a>(&'a self, target: &'a Handle) -> &'a QualName {
        match &target.data {
            Data::Element { name, .. } => name,
            _ => panic!("elem_name on non-element"),
        }
    }
    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> Handle {
        node(Data::Element {
            name,
            attrs: RefCell::new(attrs),
            template: flags.template.then(|| node(Data::Document)),
            math_integration: flags.mathml_annotation_xml_integration_point,
        })
    }
    fn create_comment(&self, text: StrTendril) -> Handle {
        node(Data::Comment(text.to_string()))
    }
    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Handle {
        node(Data::Pi(target.to_string(), data.to_string()))
    }
    fn append(&self, parent: &Handle, child: NodeOrText<Handle>) {
        insert(parent, None, child);
    }
    fn append_based_on_parent_node(
        &self,
        element: &Handle,
        previous: &Handle,
        child: NodeOrText<Handle>,
    ) {
        if element.parent.borrow().upgrade().is_some() {
            self.append_before_sibling(element, child);
        } else {
            self.append(previous, child);
        }
    }
    fn append_doctype_to_document(&self, name: StrTendril, public: StrTendril, system: StrTendril) {
        self.append(
            &self.document,
            NodeOrText::AppendNode(node(Data::Doctype(
                name.to_string(),
                public.to_string(),
                system.to_string(),
            ))),
        );
    }
    fn get_template_contents(&self, target: &Handle) -> Handle {
        match &target.data {
            Data::Element {
                template: Some(contents),
                ..
            } => contents.clone(),
            _ => panic!("get_template_contents on non-template"),
        }
    }
    fn same_node(&self, x: &Handle, y: &Handle) -> bool {
        Rc::ptr_eq(x, y)
    }
    // Quirks are tracked by html5ever's tree builder; dumps omit document mode.
    fn set_quirks_mode(&self, _: QuirksMode) {}
    fn append_before_sibling(&self, sibling: &Handle, child: NodeOrText<Handle>) {
        let parent = sibling
            .parent
            .borrow()
            .upgrade()
            .expect("sibling has parent");
        insert(&parent, Some(sibling), child);
    }
    fn add_attrs_if_missing(&self, target: &Handle, new: Vec<Attribute>) {
        let Data::Element { attrs, .. } = &target.data else {
            panic!("attributes on non-element")
        };
        let mut attrs = attrs.borrow_mut();
        for attr in new {
            if !attrs
                .iter()
                .any(|old| old.name.expanded() == attr.name.expanded())
            {
                attrs.push(attr);
            }
        }
    }
    fn remove_from_parent(&self, target: &Handle) {
        detach(target);
    }
    fn reparent_children(&self, from: &Handle, to: &Handle) {
        let children = std::mem::take(&mut *from.children.borrow_mut());
        for child in children {
            *child.parent.borrow_mut() = Weak::new();
            self.append(to, NodeOrText::AppendNode(child));
        }
    }
    // The TreeSink default for maybe_clone_an_option_into_selectedcontent is
    // intentionally not implemented by this parser-only sink. Its observable
    // omissions are recorded as SOURCE expectation mismatches, not attributed
    // to Hemplate (see the report's limitations).
    fn is_mathml_annotation_xml_integration_point(&self, target: &Handle) -> bool {
        matches!(
            &target.data,
            Data::Element {
                math_integration: true,
                ..
            }
        )
    }
}

#[derive(Debug)]
struct Case {
    source: String,
    context: Option<String>,
    scripting: Option<bool>,
    expected: String,
    errors: Vec<String>,
}
fn read_cases(input: &str) -> Vec<Case> {
    // Split only on a section marker at column zero, not on blank lines: data
    // and expected quoted text may themselves contain any number of newlines.
    let mut cases = Vec::new();
    if input.is_empty() {
        return cases;
    }
    let input = input
        .strip_prefix("#data\n")
        .expect("fixture starts with #data");
    for chunk in input.split("\n#data\n") {
        let (source, rest) = if let Some(rest) = chunk.strip_prefix("#errors\n") {
            ("", rest)
        } else {
            chunk.split_once("\n#errors\n").expect("fixture #errors")
        };
        let (metadata, expected) = rest.split_once("#document\n").expect("fixture #document");
        let errors = metadata
            .lines()
            .take_while(|line| !line.starts_with('#'))
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect();
        let mut context = None;
        let mut scripting = None;
        let mut lines = metadata.lines();
        while let Some(line) = lines.next() {
            match line {
                "#document-fragment" => {
                    context = Some(lines.next().expect("fragment context").into())
                }
                "#script-on" => scripting = Some(true),
                "#script-off" => scripting = Some(false),
                _ => {}
            }
        }
        cases.push(Case {
            source: source.into(),
            context,
            scripting,
            // Separators are outside quotes, so trimming LF here never strips
            // text-node newlines (the last text character is followed by '"').
            expected: expected.trim_end_matches('\n').into(),
            errors,
        });
    }
    cases
}
fn html_tree(source: &str, context: Option<&str>, scripting: bool) -> Dom {
    let mut opts = ParseOpts::default();
    opts.tree_builder.scripting_enabled = scripting;
    if let Some(context) = context {
        let (namespace, local) = match context.split_once(' ') {
            Some(("svg", local)) => (ns!(svg), local),
            Some(("math", local)) => (ns!(mathml), local),
            None => (ns!(html), context),
            _ => panic!("unknown fixture context: {context}"),
        };
        parse_fragment(
            Dom::default(),
            opts,
            QualName::new(None, namespace, local.into()),
            Vec::new(),
            scripting,
        )
        .one(source)
    } else {
        parse_document(Dom::default(), opts).one(source)
    }
}
fn qualified(name: &QualName, attribute: bool) -> String {
    let prefix = match name.ns {
        ns!(html) if !attribute => "",
        ns!() if attribute => "",
        ns!(svg) => "svg ",
        ns!(mathml) => "math ",
        ns!(xlink) => "xlink ",
        ns!(xml) => "xml ",
        ns!(xmlns) => "xmlns ",
        _ => panic!("unexpected namespace: {}", name.ns),
    };
    format!("{prefix}{}", name.local)
}
fn html_ws(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\x0c' | '\r' | ' ')
}
fn exact_text_tag(tag: &str) -> bool {
    matches!(
        tag,
        "pre"
            | "textarea"
            | "listing"
            | "plaintext"
            | "script"
            | "style"
            | "title"
            | "xmp"
            | "noscript"
            | "iframe"
            | "noembed"
            | "noframes"
    )
}
fn collapse(text: &str) -> String {
    let mut out = String::new();
    let mut in_whitespace = false;
    for c in text.chars() {
        if html_ws(c) {
            if !in_whitespace {
                out.push(' ');
            }
            in_whitespace = true;
        } else {
            out.push(c);
            in_whitespace = false;
        }
    }
    out
}
fn dump(dom: &Dom, context: Option<&str>, normalize: bool) -> String {
    let mut out = String::new();
    let document_children = dom.document.children.borrow();
    // html5ever returns fragment children under a synthetic HTML root. The
    // real context is omitted from the dump, but still owns its text rules.
    if let Some(context) = context {
        let exact = exact_text_tag(context.rsplit(' ').next().unwrap());
        assert_eq!(document_children.len(), 1, "fragment synthetic root");
        for child in document_children[0].children.borrow().iter() {
            dump_node(child, 0, normalize, exact, &mut out);
        }
    } else {
        for child in document_children.iter() {
            dump_node(child, 0, normalize, false, &mut out);
        }
    }
    out.pop(); // Remove only the dump's final LF, never data inside quotes.
    out
}
fn dump_node(node: &Handle, depth: usize, normalize: bool, exact: bool, out: &mut String) {
    let indent = format!("| {}", "  ".repeat(depth));
    let mut child_exact = exact;
    match &node.data {
        Data::Document => panic!("document inside dump"),
        Data::Element {
            name,
            attrs,
            template,
            ..
        } => {
            writeln!(out, "{indent}<{}>", qualified(name, false)).unwrap();
            let mut attrs: Vec<_> = attrs
                .borrow()
                .iter()
                .map(|a| (qualified(&a.name, true), a.value.to_string()))
                .collect();
            attrs.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            for (name, value) in attrs {
                writeln!(out, "{indent}  {name}=\"{value}\"").unwrap();
            }
            child_exact |= exact_text_tag(name.local.as_ref());
            if let Some(template) = template {
                writeln!(out, "{indent}  content").unwrap();
                for child in template.children.borrow().iter() {
                    dump_node(child, depth + 2, normalize, child_exact, out);
                }
            }
        }
        Data::Text(text) => {
            let text = text.borrow();
            if normalize && !exact {
                if text.chars().all(html_ws) {
                    return;
                }
                writeln!(out, "{indent}\"{}\"", collapse(&text)).unwrap();
            } else {
                writeln!(out, "{indent}\"{text}\"").unwrap();
            }
        }
        Data::Comment(text) => writeln!(out, "{indent}<!-- {text} -->").unwrap(),
        Data::Doctype(name, public, system) => {
            if public.is_empty() && system.is_empty() {
                writeln!(out, "{indent}<!DOCTYPE {name}>").unwrap();
            } else {
                writeln!(out, "{indent}<!DOCTYPE {name} \"{public}\" \"{system}\">").unwrap();
            }
        }
        Data::Pi(target, data) => writeln!(out, "{indent}<?{target} {data}>").unwrap(),
    }
    for child in node.children.borrow().iter() {
        dump_node(child, depth + 1, normalize, child_exact, out);
    }
}
fn first_compile_error(tokens: TokenStream) -> Option<String> {
    let tokens: Vec<_> = tokens.into_iter().collect();
    for (i, token) in tokens.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            if i >= 2
                && matches!(&tokens[i - 2], TokenTree::Ident(name) if name == "compile_error")
                && matches!(&tokens[i - 1], TokenTree::Punct(p) if p.as_char() == '!')
            {
                return Some(
                    syn::parse2::<syn::LitStr>(group.stream())
                        .map(|s| s.value())
                        .unwrap_or_else(|_| group.stream().to_string()),
                );
            }
            if let Some(error) = first_compile_error(group.stream()) {
                return Some(error);
            }
        }
    }
    None
}
fn static_output(tokens: TokenStream) -> Result<String, String> {
    if let Some(error) = first_compile_error(tokens.clone()) {
        return Err(format!("compile_error: {error}"));
    }
    let block: syn::Block = syn::parse2(quote::quote!({ #tokens }))
        .map_err(|error| format!("generated body is not Rust: {error}"))?;
    let mut output = String::new();
    for statement in block.stmts {
        // generate_body/merge_statics emit Static(s) as buf.write_str(s)?;.
        // Never collect literals recursively: doing so would render both sides
        // of conditionals or treat dynamic attribute fragments as static HTML.
        if let syn::Stmt::Expr(syn::Expr::Try(expr), Some(_)) = &statement {
            if let syn::Expr::MethodCall(call) = expr.expr.as_ref() {
                if call.method == "write_str"
                    && call.args.len() == 1
                    && call.attrs.is_empty()
                    && expr.attrs.is_empty()
                    && matches!(call.receiver.as_ref(), syn::Expr::Path(path) if path.path.is_ident("buf"))
                {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(text),
                        attrs,
                    }) = &call.args[0]
                    {
                        if attrs.is_empty() {
                            output.push_str(&text.value());
                            continue;
                        }
                    }
                }
            }
        }
        return Err(format!(
            "non-static statement: {}",
            statement.to_token_stream()
        ));
    }
    Ok(output)
}
fn render_case(case: &Case) -> Result<String, String> {
    // Raw-text fragments are not standalone documents: e.g. </plaintext> is
    // literal text only in its supplied context. Compile inside that context
    // and remove only the harness wrapper, retaining the original SOURCE/OUTPUT
    // comparison below. Ordinary fragments still compile as standalone markup.
    if let Some(tag @ ("style" | "plaintext")) = case.context.as_deref() {
        let open = format!("<{tag}>");
        let close = if tag == "plaintext" {
            String::new()
        } else {
            format!("</{tag}>")
        };
        let output = render(&format!("{open}{}{close}", case.source))?;
        return output
            .strip_prefix(&open)
            .and_then(|output| output.strip_suffix(&close))
            .map(str::to_owned)
            .ok_or_else(|| "fragment context wrapper was not preserved".into());
    }
    render(&case.source)
}

fn render(source: &str) -> Result<String, String> {
    let doc = build_ast(Arc::new(source.to_owned()))
        .map_err(|error| {
            if let hemplate_core::error::BuildError::Parse(error) = error {
                let before = &source[..error.byte_offset.min(source.len())];
                let line = before.matches('\n').count() + 1;
                let column = before.rsplit('\n').next().unwrap_or("").len() + 1;
                format!(
                    "html5-conformance.heml:{line}:{column}: hemplate: {}",
                    error
                        .message
                        .strip_prefix("hemplate: ")
                        .unwrap_or(&error.message)
                )
            } else {
                format!("AST: {error}")
            }
        })?
        .ok_or_else(|| "AST: no document extracted (build_ast returned None)".to_owned())?;
    static_output(generate_body(&doc, "html5-conformance.heml", None))
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Category {
    Identical,
    Whitespace,
    Different,
    Rejected,
}
impl Category {
    fn name(self) -> &'static str {
        match self {
            Self::Identical => "IDENTICAL",
            Self::Whitespace => "WHITESPACE",
            Self::Different => "DIFFERENT",
            Self::Rejected => "REJECTED",
        }
    }
}
const CATEGORIES: [Category; 4] = [
    Category::Identical,
    Category::Whitespace,
    Category::Different,
    Category::Rejected,
];
type Counts = [usize; 4];
fn accepted_source(source: &str, reason: &str) -> Option<String> {
    let tree = hemplate_parser::parse_template(source, None)?;
    let mut edits = Vec::new();
    let mut pending = vec![tree.root_node()];
    while let Some(node) = pending.pop() {
        if reason == "COMMENTS" && node.kind() == "comment" {
            edits.push((node.byte_range(), String::new()));
            continue;
        }
        if reason == "SELF_CLOSING" && node.kind() == "self_closing_element" {
            let name = node.named_child(0).unwrap();
            let tag = &source[name.byte_range()];
            if !super::is_html_void(tag) {
                let opening = &source[node.start_byte()..node.end_byte() - 2];
                edits.push((node.byte_range(), format!("{opening}></{tag}>")));
            }
        }
        pending.extend(node.named_children(&mut node.walk()));
    }
    edits.sort_by_key(|(range, _)| range.start);
    if edits.is_empty() {
        return None;
    }
    let mut adjusted = source.to_owned();
    for (range, text) in edits.into_iter().rev() {
        adjusted.replace_range(range, &text);
    }
    Some(adjusted)
}

fn trim_text_edges(node: &Handle, exact: bool) {
    let exact = exact
        || matches!(&node.data, Data::Element { name, .. } if exact_text_tag(name.local.as_ref()));
    if !exact {
        if let Data::Text(text) = &node.data {
            let trimmed = text.borrow().trim_matches(html_ws).to_owned();
            *text.borrow_mut() = trimmed;
        }
    }
    if let Data::Element {
        template: Some(contents),
        ..
    } = &node.data
    {
        trim_text_edges(contents, exact);
    }
    for child in node.children.borrow().iter() {
        trim_text_edges(child, exact);
    }
}

fn deviation_applies(case: &Case, output: &str, reason: &str) -> bool {
    let adjusted = match reason {
        "COMMENTS" | "SELF_CLOSING" => match accepted_source(&case.source, reason) {
            Some(source) => source,
            None => return false,
        },
        "WHITESPACE" => case.source.clone(),
        "noscript-as-markup" => return noscript_as_markup(case),
        _ => return false,
    };
    case.scripting
        .map_or_else(|| vec![false, true], |mode| vec![mode])
        .into_iter()
        .all(|scripting| {
            let source = html_tree(&adjusted, case.context.as_deref(), scripting);
            let rendered = html_tree(output, case.context.as_deref(), scripting);
            if reason == "WHITESPACE" {
                let exact = case.context.as_deref().is_some_and(exact_text_tag);
                trim_text_edges(&source.document, exact);
                trim_text_edges(&rendered.document, exact);
            }
            dump(&source, case.context.as_deref(), true)
                == dump(&rendered, case.context.as_deref(), true)
        })
}

fn requires_html5_repair(source: &str) -> bool {
    // These fixtures exercise browser-only selectedcontent omission/cloning,
    // adoption-agency reconstruction, or closing an outer font through select.
    // Do not generalize this to all select markup or all missing end tags.
    (source.contains("<select><button><selectedcontent></button>")
        && !source.contains("</selectedcontent>"))
        || source == "<font><select><option>a</option></font></select>"
}

fn noscript_as_markup(case: &Case) -> bool {
    // With scripting enabled this iframe opener is browser text. As ordinary
    // template markup it instead opens an unclosed iframe inside noscript.
    case.scripting == Some(true)
        && case.context.is_none()
        && case.source == "<!doctype html><noscript><iframe></noscript>X"
}

fn rejection_applies(case: &Case, reason: &str, rejection: &str) -> bool {
    if !has_position_diagnostic(rejection) {
        return false;
    }
    match reason {
        "requires-html5-repair" => case.errors.is_empty() && requires_html5_repair(&case.source),
        "noscript-as-markup" => noscript_as_markup(case),
        _ => false,
    }
}

fn read_accepted(path: &Path) -> BTreeMap<(String, usize), String> {
    let input = std::fs::read_to_string(path).expect("accepted.tsv must exist");
    let mut accepted = BTreeMap::new();
    for (line, row) in input.lines().enumerate() {
        if line == 0 {
            assert_eq!(row, "file\tindex\treason");
            continue;
        }
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 3, "accepted.tsv:{}", line + 1);
        let index = fields[1].parse().expect("accepted index is numeric");
        assert!(index > 0);
        assert!(
            matches!(
                fields[2],
                "COMMENTS"
                    | "SELF_CLOSING"
                    | "WHITESPACE"
                    | "requires-html5-repair"
                    | "noscript-as-markup"
            ),
            "unknown deviation reason: {}",
            fields[2]
        );
        assert!(
            accepted
                .insert((fields[0].to_owned(), index), fields[2].to_owned())
                .is_none(),
            "duplicate accepted entry: {row}"
        );
    }
    accepted
}

fn counts_line(counts: &Counts) -> String {
    format!(
        "{} | {} | {} | {} | {}",
        counts.iter().sum::<usize>(),
        counts[0],
        counts[1],
        counts[2],
        counts[3]
    )
}
fn escaped_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\0', "\\0")
}
fn fence(out: &mut String, label: &str, value: &str) {
    // Data can contain backticks, Markdown and NUL. Keep blocks unambiguous,
    // displaying NUL visibly while preserving exact bytes in vendored fixtures.
    let ticks = "`".repeat(
        value
            .split(|c| c != '`')
            .map(str::len)
            .max()
            .unwrap_or(0)
            .max(2)
            + 1,
    );
    writeln!(
        out,
        "\n{label}:\n\n{ticks}text\n{}\n{ticks}",
        value.replace('\0', "␀")
    )
    .unwrap();
}
fn probable_cause(
    source: &str,
    output: &str,
    source_tree: &str,
    output_tree: &str,
    context: Option<&str>,
) -> &'static str {
    let lower = source.to_ascii_lowercase();
    if output.is_empty() && !source.is_empty() {
        "AST/code generation emitted no markup for nonempty SOURCE"
    } else if source_tree.contains("<!--") && !output_tree.contains("<!--") {
        "Comment omitted or parsed as ordinary template text"
    } else if source_tree.contains("<!DOCTYPE") != output_tree.contains("<!DOCTYPE") {
        "Doctype omitted or unrecognized (can change quirks-mode tree construction)"
    } else if lower.contains("<plaintext") || context == Some("plaintext") {
        "Plaintext tokenizer context not preserved"
    } else if ["pre", "textarea", "listing"]
        .iter()
        .any(|tag| lower.contains(&format!("<{tag}")) || context == Some(tag))
    {
        "Preformatted content or its parsing boundary changed (pre/textarea/listing)"
    } else if lower.contains("<title") || context == Some("title") {
        "Title RCDATA content or its parsing boundary changed"
    } else if [
        "script", "style", "xmp", "noscript", "iframe", "noembed", "noframes",
    ]
    .iter()
    .any(|tag| lower.contains(&format!("<{tag}")) || context == Some(tag))
    {
        "Raw-text content or its parsing boundary changed"
    } else if lower.contains("<svg")
        || lower.contains("<math")
        || context
            .is_some_and(|context| context.starts_with("svg ") || context.starts_with("math "))
    {
        "Foreign-content namespace, integration point or self-closing semantics changed"
    } else if lower.contains("<template") {
        "Template contents or insertion-mode boundary changed"
    } else if lower.contains("<table")
        || lower.contains("<tbody")
        || lower.contains("<tr")
        || lower.contains("<td")
    {
        "Table insertion/foster-parenting changed by reconstructed markup"
    } else if lower.contains("<select") || lower.contains("<option") {
        "Select/option insertion rules changed by reconstructed markup"
    } else if lower.contains("<frameset") || lower.contains("<frame") {
        "Frameset insertion rules changed by reconstructed markup"
    } else if lower.contains("<form") {
        "Form nesting/pointer rules changed by reconstructed markup"
    } else if lower.contains("<b>")
        || lower.contains("<i>")
        || lower.contains("<a ")
        || lower.contains("<nobr")
        || lower.contains("<font")
    {
        "Formatting-element reconstruction/adoption-agency behavior changed"
    } else if source.contains('&') && source != output {
        "Character-reference or surrounding text reconstruction changed"
    } else if (lower.contains("<!") && !lower.contains("<!doctype")) || lower.contains("<?") {
        "Malformed declaration/bogus-comment tokenization changed"
    } else if source.contains('\0') || source.contains('\r') || source.contains('\x0c') {
        "Input-control-character handling changed"
    } else {
        "HTML error recovery/implicit element boundaries changed by reconstructed markup"
    }
}
fn has_position_diagnostic(reason: &str) -> bool {
    let reason = reason.strip_prefix("compile_error: ").unwrap_or(reason);
    let mut parts = reason.splitn(4, ':');
    matches!((parts.next(), parts.next(), parts.next(), parts.next()),
        (Some("html5-conformance.heml"), Some(line), Some(column), Some(message))
        if line.parse::<usize>().is_ok_and(|n| n > 0)
            && column.parse::<usize>().is_ok_and(|n| n > 0)
            && message.trim_start().starts_with("hemplate:"))
}

fn rejection_group(reason: &str) -> String {
    if let Some(message) = reason.strip_prefix("AST: ") {
        // BuildError includes coordinates; retain the semantic first message.
        message
            .split_once(": ")
            .map_or(message, |(_, text)| text)
            .to_owned()
    } else if let Some(message) = reason.strip_prefix("compile_error: html5-conformance.heml:") {
        format!(
            "compile_error: {}",
            message.splitn(3, ':').nth(2).unwrap_or(message).trim()
        )
    } else if reason.starts_with("non-static statement:") {
        "non-static generated statement".into()
    } else {
        reason.into()
    }
}
fn grouped(
    out: &mut String,
    title: &str,
    groups: &BTreeMap<String, (usize, Vec<String>)>,
    limit: usize,
) {
    writeln!(
        out,
        "\n## {title}\n\nCount | Probable cause / reason | Representative cases\n---: | --- | ---"
    )
    .unwrap();
    let mut groups: Vec<_> = groups.iter().collect();
    groups.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.0.cmp(b.0)));
    for (reason, (count, examples)) in groups.into_iter().take(limit) {
        writeln!(
            out,
            "{count} | {} | {}",
            escaped_cell(reason).replace('|', "\\|"),
            examples.join(", ")
        )
        .unwrap();
    }
}
fn add_group(groups: &mut BTreeMap<String, (usize, Vec<String>)>, reason: String, label: &str) {
    let (count, examples) = groups.entry(reason).or_default();
    *count += 1;
    if examples.len() < 3 {
        examples.push(label.into());
    }
}

#[test]
fn html5_conformance_measurement() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let directory = root.join("tests/html5lib-tests/tree-construction");
    let mut files: Vec<_> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| {
            panic!(
                "missing/unreadable conformance data {}: {error}",
                directory.display()
            )
        })
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "dat"))
        .collect();
    files.sort();
    let mut accepted = read_accepted(&root.join("tests/html5lib-tests/accepted.tsv"));
    let mut accepted_counts = BTreeMap::<String, usize>::new();
    let mut gate_failures = Vec::new();
    let mut per_file = BTreeMap::new();
    let mut totals = [0; 4];
    let mut tsv = "file\tindex\tcategory\treason\n".to_owned();
    let mut differences = String::new();
    let mut mismatches = String::new();
    let mut different_groups = BTreeMap::new();
    let mut rejected_groups = BTreeMap::new();
    let mut mismatch_cases = 0;
    let mut mismatch_modes = 0;
    let mut evaluated_modes = 0;
    let mut invalid_rejections = Vec::new();
    let mut evidence = "file\tindex\thtml5lib_errors\tcategory\treason\n".to_owned();
    for file in files {
        let name = file.file_name().unwrap().to_str().unwrap();
        let mut counts = [0; 4];
        for (index, case) in read_cases(&std::fs::read_to_string(&file).unwrap())
            .into_iter()
            .enumerate()
        {
            let index = index + 1;
            let label = format!("{name}:{index}");
            let rendered = render_case(&case);
            let mut category = Category::Identical;
            let mut mode_details = String::new();
            let mut cause = None;
            let mut case_mismatch = false;
            let modes = case
                .scripting
                .map_or_else(|| vec![false, true], |mode| vec![mode]);
            for scripting in modes {
                evaluated_modes += 1;
                let source_dom = html_tree(&case.source, case.context.as_deref(), scripting);
                let source_tree = dump(&source_dom, case.context.as_deref(), false);
                if source_tree != case.expected {
                    mismatch_modes += 1;
                    case_mismatch = true;
                    writeln!(
                        mismatches,
                        "\n### {label}, scripting={scripting}, context={:?}",
                        case.context
                    )
                    .unwrap();
                    fence(&mut mismatches, "SOURCE", &case.source);
                    fence(&mut mismatches, "Expected #document", &case.expected);
                    fence(&mut mismatches, "html5ever SOURCE tree", &source_tree);
                }
                if let Ok(output) = &rendered {
                    let output_dom = html_tree(output, case.context.as_deref(), scripting);
                    let output_tree = dump(&output_dom, case.context.as_deref(), false);
                    let mode_category = if source_tree == output_tree {
                        Category::Identical
                    } else if dump(&source_dom, case.context.as_deref(), true)
                        == dump(&output_dom, case.context.as_deref(), true)
                    {
                        Category::Whitespace
                    } else {
                        Category::Different
                    };
                    category = category.max(mode_category);
                    if mode_category == Category::Different {
                        cause.get_or_insert_with(|| {
                            probable_cause(
                                &case.source,
                                output,
                                &source_tree,
                                &output_tree,
                                case.context.as_deref(),
                            )
                        });
                        writeln!(mode_details, "\nScripting: {scripting}").unwrap();
                        fence(&mut mode_details, "SOURCE tree", &source_tree);
                        fence(&mut mode_details, "OUTPUT tree", &output_tree);
                    }
                }
            }
            if case_mismatch {
                mismatch_cases += 1;
            }
            let accepted_reason = accepted.remove(&(name.to_owned(), index));
            let accepted_rejection = rendered
                .as_ref()
                .err()
                .zip(accepted_reason.as_deref())
                .is_some_and(|(rejection, reason)| rejection_applies(&case, reason, rejection));
            if let Err(reason) = &rendered {
                if !has_position_diagnostic(reason) {
                    gate_failures.push(format!(
                        "{label}: rejection lacks template position: {reason}"
                    ));
                }
            }
            match (&rendered, accepted_reason) {
                (Err(_), Some(reason)) if accepted_rejection => {
                    *accepted_counts.entry(reason).or_default() += 1;
                }
                (Ok(output), Some(reason))
                    if category == Category::Different
                        && deviation_applies(&case, output, &reason) =>
                {
                    *accepted_counts.entry(reason).or_default() += 1;
                }
                (_, Some(reason)) => gate_failures.push(format!(
                    "{label}: stale/inapplicable accepted reason {reason}"
                )),
                (Ok(_), None) if category == Category::Different => {
                    gate_failures.push(format!("{label}: unaccepted DIFFERENT"))
                }
                _ => {}
            }
            let reason = match rendered {
                Err(reason) => {
                    category = Category::Rejected;
                    add_group(&mut rejected_groups, rejection_group(&reason), &label);
                    reason
                }
                Ok(output) if category == Category::Different => {
                    let cause = cause.unwrap();
                    add_group(&mut different_groups, cause.into(), &label);
                    writeln!(differences, "\n### {label}\n\nProbable cause: {cause}\n\nFragment context: {:?}. SOURCE expectation mismatch: {case_mismatch}.", case.context).unwrap();
                    fence(&mut differences, "SOURCE", &case.source);
                    fence(&mut differences, "Hemplate OUTPUT", &output);
                    differences.push_str(&mode_details);
                    cause.into()
                }
                Ok(_) if category == Category::Whitespace => {
                    "Only normalized HTML text whitespace differs".into()
                }
                Ok(_) => String::new(),
            };
            let invalid_hemplate_syntax =
                case.source.contains("{+") && reason.contains("interpolation");
            if category == Category::Rejected
                && case.errors.is_empty()
                && !invalid_hemplate_syntax
                && !accepted_rejection
            {
                invalid_rejections.push(format!("{label}: {reason}"));
            }
            writeln!(
                evidence,
                "{name}\t{index}\t{}\t{}\t{}",
                escaped_cell(&case.errors.join("; ")),
                category.name(),
                escaped_cell(&reason)
            )
            .unwrap();
            counts[category as usize] += 1;
            totals[category as usize] += 1;
            writeln!(
                tsv,
                "{name}\t{index}\t{}\t{}",
                category.name(),
                escaped_cell(&reason)
            )
            .unwrap();
        }
        per_file.insert(name.to_owned(), counts);
    }
    let total: usize = totals.iter().sum();
    assert!(total > 0, "no conformance cases evaluated");
    let mut report = format!("# Hemplate HTML5 conformance measurement\n\nEnforcing gate: new deviations, stale acceptances and unpositioned rejections fail.\n\n{total} fixture cases, {} files, {evaluated_modes} parser modes.\n\nSOURCE expectation mismatches: **{mismatch_cases} cases / {mismatch_modes} modes**.\nThese are html5ever/fixture/harness issues, not Hemplate deviations.\n\nSee `tests/html5lib-tests/SOURCE.md` for the pinned upstream SHA, reproduction,\nnormalization and scripting rules. Indices are 1-based; each TSV row is one\nfixture, with the worst category across its scripting modes. NUL is displayed\nas ␀ in Markdown, and escaped as \\0 in TSV. Other TSV controls are backslash-escaped.\n\n## Counts\n\nFile | Total | IDENTICAL | WHITESPACE | DIFFERENT | REJECTED\n--- | ---: | ---: | ---: | ---: | ---:\n**TOTAL** | {}\n", per_file.len(), counts_line(&totals));
    for (file, counts) in &per_file {
        writeln!(report, "{file} | {}", counts_line(counts)).unwrap();
    }
    writeln!(
        report,
        "\n## Accepted deviations\n\nReason | Count\n--- | ---:"
    )
    .unwrap();
    for (reason, count) in &accepted_counts {
        writeln!(report, "{reason} | {count}").unwrap();
    }
    grouped(
        &mut report,
        "Top 15 DIFFERENT probable causes (heuristics, not diagnoses)",
        &different_groups,
        15,
    );
    grouped(
        &mut report,
        "REJECTED reasons (coordinates grouped away)",
        &rejected_groups,
        usize::MAX,
    );
    report.push_str("\n## Measurement limitations\n\nThe minimal TreeSink does not implement option-to-selectedcontent cloning;\nwebkit02.dat:45–48 expose that known sink limitation in SOURCE expectation\nchecks. The select-fragment expectation mismatch is separate and remains\nunattributed. No scripts execute. Tree dumps omit document quirks-mode metadata,\nalthough html5ever uses that mode during tree construction. Probable-cause\ngroups are first-match triage heuristics, not confirmed defect diagnoses.\n\nREJECTED includes Ok(None) from build_ast: no document exists to generate,\nand Hemplate's file loader rejects that result too. Conversely, an extracted\ndocument with no generated statements really has empty static OUTPUT; it is\nmeasured as such, not relabeled as a rejection.\n");
    report.push_str("\n## DIFFERENT cases\n");
    report.push_str(&differences);
    report.push_str("\n## SOURCE expectation mismatches (not Hemplate findings)\n");
    report.push_str(&mismatches);
    let destination = root.join("target/html5-conformance");
    std::fs::create_dir_all(&destination).unwrap();
    std::fs::write(destination.join("report.md"), report).unwrap();
    std::fs::write(destination.join("cases.tsv"), tsv).unwrap();
    std::fs::write(destination.join("errors.tsv"), evidence).unwrap();
    eprintln!(
        "HTML5 measurement: {total} cases in {} files",
        per_file.len()
    );
    for category in CATEGORIES {
        eprintln!("  {}: {}", category.name(), totals[category as usize]);
    }
    eprintln!("  SOURCE expectation mismatches: {mismatch_cases} cases / {mismatch_modes} modes");
    eprintln!("  Reports: {}", destination.display());
    for (file, index) in accepted.keys() {
        gate_failures.push(format!("{file}:{index}: accepted case does not exist"));
    }
    assert!(
        gate_failures.is_empty(),
        "HTML5 gate failures:\n{}",
        gate_failures.join("\n")
    );
    assert!(
        invalid_rejections.is_empty(),
        "html5lib cases without parse errors must compile:\n{}",
        invalid_rejections.join("\n")
    );
}

#[test]
fn html5_conformance_static_extraction_is_conservative() {
    assert_eq!(
        static_output(quote::quote! { buf.write_str("a")?; buf.write_str("b\n")?; }).unwrap(),
        "ab\n"
    );
    assert!(
        static_output(quote::quote! { if true { buf.write_str("not unconditional")?; } })
            .unwrap_err()
            .starts_with("non-static statement:")
    );
    assert!(static_output(quote::quote! { buf.write_str(value)?; }).is_err());
    assert!(static_output(quote::quote! { buf.write_str(concat!("a", "b"))?; }).is_err());
    assert_eq!(
        static_output(quote::quote! { if true { compile_error!("first diagnostic"); } })
            .unwrap_err(),
        "compile_error: first diagnostic"
    );
    assert_eq!(render("<p>hello</p>").unwrap(), "<p>hello</p>");
    assert!(has_position_diagnostic(
        &render("FOO<!-- BAR -- <QUX> -- MUX --!>BAZ").unwrap_err()
    ));
    assert!(render("<p>{+ self.value +}</p>")
        .unwrap_err()
        .starts_with("non-static statement:"));
}

#[test]
fn html5_conformance_dump_and_normalization_checks() {
    let tree = html_tree(
        "<!doctype html><template><svg viewBox='0'><a xlink:href='x'/></svg></template>",
        None,
        false,
    );
    assert_eq!(dump(&tree, None, false), "| <!DOCTYPE html>\n| <html>\n|   <head>\n|     <template>\n|       content\n|         <svg svg>\n|           viewBox=\"0\"\n|           <svg a>\n|             xlink href=\"x\"\n|   <body>");
    let fragment = html_tree("<td>A<td>B", Some("tr"), false);
    assert_eq!(
        dump(&fragment, Some("tr"), false),
        "| <td>\n|   \"A\"\n| <td>\n|   \"B\""
    );
    let normalized = |source: &str| dump(&html_tree(source, None, false), None, true);
    assert_eq!(
        normalized("<p>a \n b</p> \n <div>x</div>"),
        normalized("<p>a b</p><div>x</div>")
    );
    assert_ne!(normalized("<p>a&nbsp;b</p>"), normalized("<p>a b</p>"));
    assert_ne!(normalized("<p>a </p>"), normalized("<p>a</p>"));
    assert_ne!(normalized("<!--  x -->"), normalized("<!-- x -->"));
    assert_ne!(
        normalized("<div title='a  b'>"),
        normalized("<div title='a b'>")
    );
    for tag in [
        "pre",
        "textarea",
        "listing",
        "plaintext",
        "script",
        "style",
        "title",
        "xmp",
        "noscript",
        "iframe",
        "noembed",
        "noframes",
    ] {
        assert_ne!(
            normalized(&format!("<!doctype html><body><{tag}>a  b</{tag}>")),
            normalized(&format!("<!doctype html><body><{tag}>a b</{tag}>")),
            "{tag}"
        );
    }
    for context in [
        "pre",
        "textarea",
        "listing",
        "plaintext",
        "script",
        "style",
        "title",
        "xmp",
        "noscript",
        "iframe",
        "noembed",
        "noframes",
    ] {
        let fragment =
            |text: &str| dump(&html_tree(text, Some(context), true), Some(context), true);
        assert_ne!(fragment("a  b"), fragment("a b"), "fragment {context}");
        assert_ne!(
            fragment("  "),
            fragment(" "),
            "whitespace-only fragment {context}"
        );
    }
    let cases = read_cases("#data\na\n\nb\n#errors\n#document-fragment\nsvg svg\n#script-off\n#document\n| \"a\n\nb\"\n\n#data\n#errors\n#document\n\n");
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[0].source, "a\n\nb");
    assert_eq!(cases[0].context.as_deref(), Some("svg svg"));
    assert_eq!(cases[0].scripting, Some(false));
    assert_eq!(cases[0].expected, "| \"a\n\nb\"");
    assert_eq!(cases[1].source, "");
}

#[test]
fn html5_conformance_noscript_acceptance_requires_the_exact_markup_rejection() {
    let mut case = read_cases(concat!(
        "#data\n<!doctype html><noscript><iframe></noscript>X\n",
        "#errors\n#script-on\n#document\n"
    ))
    .pop()
    .unwrap();
    let rejection = render_case(&case).unwrap_err();
    assert!(rejection_applies(&case, "noscript-as-markup", &rejection));
    assert!(!rejection_applies(&case, "COMMENTS", &rejection));
    assert!(!rejection_applies(
        &case,
        "noscript-as-markup",
        "unpositioned"
    ));
    case.scripting = Some(false);
    assert!(!rejection_applies(&case, "noscript-as-markup", &rejection));
    case.scripting = Some(true);
    case.source = "<noscript><b>ordinary markup</b></noscript>".into();
    assert!(!rejection_applies(&case, "noscript-as-markup", &rejection));
}
