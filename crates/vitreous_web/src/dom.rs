use std::collections::HashMap;

use vitreous_a11y::Role;
use vitreous_widgets::{ImageSource, Key, Node, NodeKind, TextContent};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::aria::{apply_aria, clear_aria_attributes};
use crate::events::{self, EventListeners};
use crate::styles::apply_styles;

/// An identifier for tracked DOM nodes, auto-incrementing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DomNodeId(usize);

/// A rendered DOM node with its associated event listener guard and children.
pub struct DomNode {
    pub id: DomNodeId,
    pub element: HtmlElement,
    pub key: Option<Key>,
    pub children: Vec<DomNode>,
    _listeners: EventListeners,
}

/// The DOM reconciler — maintains the mapping between vitreous nodes and DOM elements.
pub struct Reconciler {
    document: Document,
    next_id: usize,
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new()
    }
}

impl Reconciler {
    /// Create a new reconciler.
    pub fn new() -> Self {
        let window = web_sys::window().expect("no global window");
        let document = window.document().expect("no document");
        Self {
            document,
            next_id: 0,
        }
    }

    fn alloc_id(&mut self) -> DomNodeId {
        let id = DomNodeId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Create a DOM element tree from a vitreous `Node`.
    pub fn create_element(&mut self, node: Node) -> DomNode {
        let (tag, text_content) = element_tag_and_text(&node);

        let element: Element = self
            .document
            .create_element(tag)
            .expect("failed to create element");

        let html_element: HtmlElement = element
            .dyn_into::<HtmlElement>()
            .expect("not an HtmlElement");

        // Set text content if applicable
        if let Some(text) = text_content {
            html_element.set_text_content(Some(&text));
        }

        // Set image source
        if let NodeKind::Image(ref src) = node.kind
            && let Some(img) = html_element.dyn_ref::<web_sys::HtmlImageElement>()
        {
            match src {
                ImageSource::Url(url) => img.set_src(url),
                ImageSource::Path(path) => img.set_src(path),
                ImageSource::Bytes(data) => {
                    // Create a blob URL from the raw bytes
                    let array = js_sys::Uint8Array::new_with_length(data.len() as u32);
                    array.copy_from(data);
                    let parts = js_sys::Array::new();
                    parts.push(&array.buffer());
                    if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence(&parts.into())
                        && let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob)
                    {
                        img.set_src(&url);
                    }
                }
            }
        }

        // Handle text input elements
        if matches!(&node.kind, NodeKind::Text(_))
            && node.a11y.role == Role::TextInput
            && let Some(input) = html_element.dyn_ref::<web_sys::HtmlInputElement>()
            && let Some(label) = &node.a11y.label
        {
            input.set_value(label);
        }

        // Handle checkbox/switch input
        if matches!(node.a11y.role, Role::Checkbox | Role::Switch)
            && let Some(input) = html_element.dyn_ref::<web_sys::HtmlInputElement>()
        {
            input.set_type("checkbox");
            if let Some(vitreous_a11y::CheckedState::Checked) = node.a11y.state.checked {
                input.set_checked(true);
            }
        }

        // Handle slider input
        if node.a11y.role == Role::Slider
            && let Some(input) = html_element.dyn_ref::<web_sys::HtmlInputElement>()
        {
            input.set_type("range");
            if let Some(min) = node.a11y.state.value_min {
                input.set_min(&min.to_string());
            }
            if let Some(max) = node.a11y.state.value_max {
                input.set_max(&max.to_string());
            }
            if let Some(now) = node.a11y.state.value_now {
                input.set_value(&now.to_string());
            }
        }

        // Apply styles
        apply_styles(&html_element, &node);

        // Apply ARIA attributes
        apply_aria(&html_element.clone().into(), &node.a11y);

        // Attach event listeners (takes ownership)
        let listeners = events::attach_event_listeners_owned(&html_element, node.event_handlers);

        let key = node.key.clone();
        let id = self.alloc_id();

        // Recursively create children
        let mut children = Vec::with_capacity(node.children.len());
        for child_node in node.children {
            let child_dom = self.create_element(child_node);
            let _ = html_element.append_child(&child_dom.element);
            children.push(child_dom);
        }

        DomNode {
            id,
            element: html_element,
            key,
            children,
            _listeners: listeners,
        }
    }

    /// Reconcile an existing DOM tree with a new vitreous `Node` tree.
    ///
    /// Uses key-based diffing to minimize DOM operations:
    /// - Matching keys: update attributes/styles in place
    /// - New keys: create and insert
    /// - Missing keys: remove
    /// - Reordered keys: reorder with `insertBefore`
    pub fn reconcile(&mut self, existing: &mut DomNode, new_node: Node) {
        let (new_tag, text_content) = element_tag_and_text(&new_node);

        // Check if the element tag has changed — if so, replace entirely
        let old_tag = existing.element.tag_name().to_lowercase();
        if old_tag != new_tag {
            // Tag changed (e.g., div -> span, or span -> button): replace element
            let replacement = self.create_element(new_node);
            if let Some(parent) = existing.element.parent_element() {
                let _ = parent.replace_child(&replacement.element, &existing.element);
            }
            *existing = replacement;
            return;
        }

        // Update text content
        if let Some(text) = text_content {
            existing.element.set_text_content(Some(&text));
        }

        // Reapply styles
        let _ = existing.element.remove_attribute("style");
        apply_styles(&existing.element, &new_node);

        // Reapply ARIA attributes — clear stale ones first
        clear_aria_attributes(&existing.element);
        apply_aria(&existing.element.clone().into(), &new_node.a11y);

        // Reattach event listeners
        let new_listeners =
            events::attach_event_listeners_owned(&existing.element, new_node.event_handlers);
        existing._listeners = new_listeners;
        existing.key = new_node.key.clone();

        // Reconcile children
        self.reconcile_children(existing, new_node.children);
    }

    /// Reconcile the children of a DOM node using key-based diffing.
    fn reconcile_children(&mut self, parent: &mut DomNode, new_children: Vec<Node>) {
        let parent_element = &parent.element;

        // Build a map of existing children by key
        let mut keyed_old: HashMap<Key, usize> = HashMap::new();
        for (i, child) in parent.children.iter().enumerate() {
            if let Some(key) = &child.key {
                keyed_old.insert(key.clone(), i);
            }
        }

        // Track which old children were reused
        let mut reused: Vec<bool> = vec![false; parent.children.len()];
        let mut new_dom_children: Vec<DomNode> = Vec::with_capacity(new_children.len());

        for new_child in new_children {
            let matched_idx = new_child
                .key
                .as_ref()
                .and_then(|k| keyed_old.get(k).copied());

            if let Some(idx) = matched_idx {
                // Reuse existing element — reconcile in place
                reused[idx] = true;
                // We need to take ownership temporarily for reconciliation.
                // Since we can't easily remove from Vec while iterating,
                // we'll use a placeholder approach.
                let mut existing = std::mem::replace(
                    &mut parent.children[idx],
                    placeholder_dom_node(&self.document),
                );
                self.reconcile(&mut existing, new_child);
                new_dom_children.push(existing);
            } else {
                // New node — create fresh element
                let dom_node = self.create_element(new_child);
                new_dom_children.push(dom_node);
            }
        }

        // Remove old children that weren't reused
        for (i, was_reused) in reused.iter().enumerate() {
            if !was_reused {
                parent.children[i].element.remove();
            }
        }

        // Clear parent's child nodes and re-append in new order
        // This is simpler than computing minimal moves and correct for
        // the typical case sizes we handle.
        while let Some(child) = parent_element.first_child() {
            let _ = parent_element.remove_child(&child);
        }

        for child in &new_dom_children {
            let _ = parent_element.append_child(&child.element);
        }

        parent.children = new_dom_children;
    }
}

/// Determine the HTML tag and optional text content for a vitreous `Node`.
fn element_tag_and_text(node: &Node) -> (&'static str, Option<String>) {
    match &node.kind {
        NodeKind::Container => {
            // ScrollView -> div with overflow:auto (handled by styles)
            // Container -> div
            ("div", None)
        }
        NodeKind::Text(tc) => {
            let text = match tc {
                TextContent::Static(s) => s.clone(),
                TextContent::Dynamic(f) => f(),
            };

            // Button role -> <button>
            if node.a11y.role == Role::Button {
                return ("button", Some(text));
            }

            // TextInput role -> <input>
            if node.a11y.role == Role::TextInput {
                return ("input", None);
            }

            // Default text -> <span>
            ("span", Some(text))
        }
        NodeKind::Image(_) => ("img", None),
        NodeKind::Canvas(_) => ("canvas", None),
        NodeKind::NativeEmbed(_) => ("div", None),
        NodeKind::Component(_) => {
            // Components render to their output — for now treat as div
            ("div", None)
        }
    }
}

/// Create a minimal placeholder DomNode for use during reconciliation swaps.
fn placeholder_dom_node(document: &Document) -> DomNode {
    let element = document
        .create_element("div")
        .expect("create placeholder")
        .dyn_into::<HtmlElement>()
        .expect("placeholder HtmlElement");

    let target = element.clone().into();
    DomNode {
        id: DomNodeId(usize::MAX),
        element,
        key: None,
        children: Vec::new(),
        _listeners: EventListeners::empty(target),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn element_tag_container() {
        let node = Node::new(NodeKind::Container);
        let (tag, text) = element_tag_and_text(&node);
        assert_eq!(tag, "div");
        assert!(text.is_none());
    }

    #[test]
    fn element_tag_text() {
        let node = Node::new(NodeKind::Text(TextContent::Static("hello".into())));
        let (tag, text) = element_tag_and_text(&node);
        assert_eq!(tag, "span");
        assert_eq!(text, Some("hello".into()));
    }

    #[test]
    fn element_tag_button() {
        let mut node = Node::new(NodeKind::Text(TextContent::Static("Click".into())));
        node.a11y.role = Role::Button;
        let (tag, text) = element_tag_and_text(&node);
        assert_eq!(tag, "button");
        assert_eq!(text, Some("Click".into()));
    }

    #[test]
    fn element_tag_text_input() {
        let mut node = Node::new(NodeKind::Text(TextContent::Static(String::new())));
        node.a11y.role = Role::TextInput;
        let (tag, text) = element_tag_and_text(&node);
        assert_eq!(tag, "input");
        assert!(text.is_none());
    }

    #[test]
    fn element_tag_image() {
        let node = Node::new(NodeKind::Image(ImageSource::Url("test.png".into())));
        let (tag, _) = element_tag_and_text(&node);
        assert_eq!(tag, "img");
    }

    #[test]
    fn dom_node_id_equality() {
        assert_eq!(DomNodeId(0), DomNodeId(0));
        assert_ne!(DomNodeId(0), DomNodeId(1));
    }
}
