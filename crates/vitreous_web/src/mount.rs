use std::cell::RefCell;
use std::rc::Rc;

use vitreous_reactive::{Scope, create_effect, create_scope, set_executor};
use vitreous_widgets::Node;
use wasm_bindgen::JsCast;
use web_sys::HtmlElement;

use crate::dom::{DomNode, Reconciler};

/// A mounted vitreous application instance.
///
/// Holds the reactive scope and the rendered DOM tree. Dropping this
/// struct cleans up all event listeners and removes the rendered content.
pub struct WebApp {
    _scope: Scope,
    root_dom: Rc<RefCell<Option<DomNode>>>,
    container: HtmlElement,
}

impl WebApp {
    /// Mount a vitreous application into the DOM element with the given `id`.
    ///
    /// The `root` function is called to build the initial node tree and is
    /// re-called inside a reactive effect so that signal changes trigger
    /// automatic DOM reconciliation.
    ///
    /// # Panics
    ///
    /// Panics if no element with the given `id` exists in the document.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vitreous_web::mount::WebApp;
    /// use vitreous_widgets::text;
    ///
    /// WebApp::mount("app", || text("Hello, world!"));
    /// ```
    pub fn mount(element_id: &str, root: impl Fn() -> Node + 'static) -> Self {
        let window = web_sys::window().expect("no global window");
        let document = window.document().expect("no document");
        let container = document
            .get_element_by_id(element_id)
            .unwrap_or_else(|| panic!("no element with id \"{element_id}\""));
        let container: HtmlElement = container
            .dyn_into()
            .expect("mount target is not an HtmlElement");

        // Set up the WASM executor for async resources
        set_executor(|fut| {
            wasm_bindgen_futures::spawn_local(fut);
        });

        let reconciler = Rc::new(RefCell::new(Reconciler::new()));
        let root_dom: Rc<RefCell<Option<DomNode>>> = Rc::new(RefCell::new(None));
        let container_clone = container.clone();
        let root_dom_clone = root_dom.clone();

        let scope = create_scope(move || {
            let reconciler = reconciler.clone();
            let root_dom = root_dom_clone.clone();
            let container = container_clone.clone();

            create_effect(move || {
                let new_tree = root();
                let mut reconciler = reconciler.borrow_mut();
                let mut dom_ref = root_dom.borrow_mut();

                match dom_ref.as_mut() {
                    None => {
                        // Initial render — create the full DOM tree
                        let dom_node = reconciler.create_element(new_tree);
                        let _ = container.append_child(&dom_node.element);
                        *dom_ref = Some(dom_node);
                    }
                    Some(existing) => {
                        // Subsequent renders — reconcile
                        reconciler.reconcile(existing, new_tree);
                    }
                }
            });
        });

        Self {
            _scope: scope,
            root_dom,
            container,
        }
    }
}

impl Drop for WebApp {
    fn drop(&mut self) {
        // Remove all rendered content
        if let Some(dom_node) = self.root_dom.borrow_mut().take() {
            let _ = self.container.remove_child(&dom_node.element);
        }
    }
}

/// Convenience function to mount a vitreous app to a DOM element by ID.
///
/// This is the primary entry point for web applications.
///
/// # Example
///
/// ```ignore
/// use vitreous_web::mount;
/// use vitreous_widgets::text;
///
/// mount("app", || text("Hello, world!"));
/// ```
pub fn mount(element_id: &str, root: impl Fn() -> Node + 'static) -> WebApp {
    WebApp::mount(element_id, root)
}
