use std::cell::RefCell;
use std::collections::BTreeMap;

use proc_macro2::TokenStream;

use super::compile_err;
use crate::template_source::TemplateTransform;

/// Build-supplied static edits awaiting their attribute or start-tag end.
///
/// Codegen takes each edit at the one place it may apply. Anything left over
/// addressed something other than static markup and fails compilation.
pub(crate) struct Edits {
    pending: RefCell<BTreeMap<(usize, usize), String>>,
    transform: Option<TemplateTransform>,
}

impl Edits {
    pub(crate) fn new(transform: Option<&TemplateTransform>) -> Self {
        let pending = transform
            .map(|transform| {
                transform
                    .edits
                    .iter()
                    .map(|edit| ((edit.range.start, edit.range.end), edit.text.clone()))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            pending: RefCell::new(pending),
            transform: transform.cloned(),
        }
    }

    /// Inspect a replacement without consuming it. Context analysis must see
    /// effective markup before code generation places the edit.
    pub(crate) fn attribute_preview(&self, start: usize, end: usize) -> Option<String> {
        if start == end {
            return None;
        }
        self.pending.borrow().get(&(start, end)).cloned()
    }

    /// Replacement for a plain static attribute spanning exactly `start..end`.
    pub(crate) fn attribute(&self, start: usize, end: usize) -> Option<String> {
        if start == end {
            return None;
        }
        self.pending.borrow_mut().remove(&(start, end))
    }

    /// Markup inserted as the first content after a start tag ending at `offset`.
    pub(crate) fn insertion(&self, offset: usize) -> Option<String> {
        self.pending.borrow_mut().remove(&(offset, offset))
    }

    /// A compile error for the first edit codegen could not place, if any.
    pub(crate) fn unplaced(&self, template_path: &str) -> Option<TokenStream> {
        let pending = self.pending.borrow();
        let (&(start, _), _) = pending.iter().next()?;
        let transform = self.transform.as_ref()?;
        let (line, column) = transform.position(start);
        Some(compile_err(&format!(
            "{template_path}:{line}:{column}: hemplate: transform `{}` edits something other than a static attribute or start-tag end; rebuild, or report the integration that produced it",
            transform.file
        )))
    }
}
