/// Stable index for a lexical template scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScopeId(pub u32);

/// Where an attribute came from in the template syntax.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttributeOrigin {
    /// Plain HTML attribute such as `class="card"`.
    Static,
    /// Dynamic attribute such as `+class="self.classes"`.
    Dynamic,
    /// Hemplate directive attribute such as `h-for="item in items"`.
    Directive,
}

/// HTML form control shape visible at build time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ControlKind {
    /// Text input.
    Text,
    /// Number input with optional range metadata.
    Number {
        /// Static `min` attribute, when present.
        min: Option<String>,
        /// Static `max` attribute, when present.
        max: Option<String>,
        /// Static `step` attribute, when present.
        step: Option<String>,
    },
    /// Checkbox input.
    Checkbox,
    /// Radio input.
    Radio,
    /// Select control.
    Select {
        /// Whether the select has a `multiple` attribute.
        multiple: bool,
    },
    /// Textarea control.
    TextArea,
    /// File input.
    File,
    /// Hidden input.
    Hidden,
    /// Submit input.
    Submit,
    /// Recognized form control with a non-specialized tag or input type.
    Other {
        /// HTML tag name.
        tag: String,
        /// Static `type` attribute for input controls, when present.
        input_type: Option<String>,
    },
}

/// Lexical scope introduced by template control flow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScopeKind {
    /// Root document scope.
    Root,
    /// `h-for` scope.
    For {
        /// Binding pattern before `in`.
        pattern: String,
        /// Iterated expression after `in`.
        expr: String,
        /// Sibling `h-key` expression, when present.
        key_expr: Option<String>,
    },
    /// `h-if="let PAT = EXPR"` or `h-else-if="let PAT = EXPR"` scope.
    IfLet {
        /// Bound pattern.
        pattern: String,
        /// Matched expression.
        expr: String,
    },
    /// Non-wildcard `h-case` scope.
    Case {
        /// Matched case pattern.
        pattern: String,
    },
}

/// Scope entry in a surface document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceScope {
    /// Parent scope, or `None` for the root.
    pub parent: Option<ScopeId>,
    /// Scope kind and parsed directive metadata.
    pub kind: ScopeKind,
}

/// Attribute visible on a surface node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceAttribute {
    /// Attribute or directive name without parser delimiters.
    pub name: String,
    /// Static expression/value text, when the attribute has a value.
    pub value: Option<String>,
    /// Attribute syntax family.
    pub origin: AttributeOrigin,
}

/// Surface node shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceNodeKind {
    /// HTML element with a tag name.
    Element {
        /// Element tag name.
        tag: String,
    },
    /// Static text visible in the template.
    Text {
        /// Text with dynamic interpolation omitted.
        text: String,
    },
}

/// Build-time node extracted from a template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceNode {
    /// Stable index into `SurfaceDocument::nodes`.
    pub id: usize,
    /// Element or text payload.
    pub kind: SurfaceNodeKind,
    /// Attributes for element nodes; empty for text nodes.
    pub attrs: Vec<SurfaceAttribute>,
    /// Lexical scope containing this node.
    pub scope: ScopeId,
}

/// Named form control visible in a `<form>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceControl {
    /// Static `name` attribute.
    pub name: String,
    /// Control shape inferred from tag and static attributes.
    pub kind: ControlKind,
    /// Whether the control has a `required` attribute.
    pub required: bool,
}

/// Form and its named controls.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceForm {
    /// Node id for the `<form>` element.
    pub node: usize,
    /// Named controls contained by the form.
    pub controls: Vec<SurfaceControl>,
}

/// Generic build-time surface extracted from a template AST.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SurfaceDocument {
    /// Nodes in traversal order.
    pub nodes: Vec<SurfaceNode>,
    /// Scope table; `ScopeId(0)` is always the root.
    pub scopes: Vec<SurfaceScope>,
    /// Forms discovered in the template.
    pub forms: Vec<SurfaceForm>,
}
