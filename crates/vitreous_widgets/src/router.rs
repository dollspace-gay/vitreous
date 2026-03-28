use std::cell::RefCell;

use vitreous_reactive::{Signal, create_unscoped_signal, provide_context, use_context};

use crate::node::{Node, NodeKind};

// ---------------------------------------------------------------------------
// Route — a path pattern mapped to a component function
// ---------------------------------------------------------------------------

pub struct Route {
    pub path: String,
    pub component: Box<dyn Fn() -> Node>,
}

impl Route {
    pub fn new(path: impl Into<String>, component: impl Fn() -> Node + 'static) -> Self {
        Self {
            path: path.into(),
            component: Box::new(component),
        }
    }
}

// ---------------------------------------------------------------------------
// RouterState — injected via context
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct RouterState {
    current_path: Signal<String>,
    params: Signal<Vec<(String, String)>>,
}

// ---------------------------------------------------------------------------
// Path matching
// ---------------------------------------------------------------------------

/// Match a URL path against a route pattern with `:param` segments.
/// Returns `Some(params)` on match, `None` otherwise.
fn match_path(pattern: &str, path: &str) -> Option<Vec<(String, String)>> {
    let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if pattern_segments.len() != path_segments.len() {
        return None;
    }

    let mut params = Vec::new();
    for (pat, actual) in pattern_segments.iter().zip(path_segments.iter()) {
        if let Some(param_name) = pat.strip_prefix(':') {
            params.push((param_name.to_owned(), (*actual).to_owned()));
        } else if pat != actual {
            return None;
        }
    }

    Some(params)
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// Persist router signals across scope rebuilds. The reactive scope is
// recreated every frame by build_widget_tree, which would destroy the
// current_path signal and reset navigation to "/". Thread-local storage
// keeps the signals alive so route changes persist.
thread_local! {
    static PERSISTENT_STATE: RefCell<Option<RouterState>> = const { RefCell::new(None) };
}

/// Create a router that matches the current path against the given routes
/// and renders the matching component.
///
/// The router injects a `RouterState` context so that `navigate()`,
/// `use_route()`, and `use_param()` work in descendant widgets.
pub fn router(routes: Vec<Route>) -> Node {
    let state = PERSISTENT_STATE.with(|cell| {
        let mut opt = cell.borrow_mut();
        if let Some(ref s) = *opt {
            s.clone()
        } else {
            let s = RouterState {
                current_path: create_unscoped_signal("/".to_owned()),
                params: create_unscoped_signal(Vec::<(String, String)>::new()),
            };
            *opt = Some(s.clone());
            s
        }
    });
    provide_context(state.clone());

    render_route(&state, &routes)
}

/// Navigate to a new path. Call from within a component rendered by `router()`.
pub fn navigate(path: impl Into<String>) {
    let state: RouterState = use_context();
    let new_path = path.into();
    state.current_path.set(new_path);
}

/// Get the current route path. Reads from the persistent router state,
/// so it works both inside and outside the `router()` subtree.
pub fn use_route() -> String {
    PERSISTENT_STATE.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|s| s.current_path.get())
            .unwrap_or_else(|| "/".to_owned())
    })
}

/// Get a route parameter by name. Call from within a component rendered by
/// `router()`. Returns the value of the `:param` segment that matched.
pub fn use_param(name: &str) -> Option<String> {
    let state: RouterState = use_context();
    let params = state.params.get();
    params
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.clone())
}

fn render_route(state: &RouterState, routes: &[Route]) -> Node {
    let current = state.current_path.get();

    for route in routes {
        if let Some(matched_params) = match_path(&route.path, &current) {
            state.params.set(matched_params);
            return (route.component)();
        }
    }

    // No route matched — return empty container
    Node::new(NodeKind::Container)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::text;
    use vitreous_reactive::create_signal;

    #[test]
    fn match_path_exact() {
        let result = match_path("/", "/");
        assert_eq!(result, Some(vec![]));
    }

    #[test]
    fn match_path_static_segments() {
        let result = match_path("/users", "/users");
        assert_eq!(result, Some(vec![]));
    }

    #[test]
    fn match_path_with_param() {
        let result = match_path("/users/:id", "/users/42");
        assert_eq!(result, Some(vec![("id".to_owned(), "42".to_owned())]));
    }

    #[test]
    fn match_path_multiple_params() {
        let result = match_path("/org/:org_id/users/:user_id", "/org/acme/users/42");
        assert_eq!(
            result,
            Some(vec![
                ("org_id".to_owned(), "acme".to_owned()),
                ("user_id".to_owned(), "42".to_owned()),
            ])
        );
    }

    #[test]
    fn match_path_no_match() {
        let result = match_path("/users", "/posts");
        assert_eq!(result, None);
    }

    #[test]
    fn match_path_length_mismatch() {
        let result = match_path("/users/:id", "/users");
        assert_eq!(result, None);
    }

    #[test]
    fn router_renders_matching_route() {
        use vitreous_reactive::create_scope;

        let _scope = create_scope(|| {
            let node = router(vec![
                Route::new("/", || text("home")),
                Route::new("/about", || text("about")),
            ]);

            // Default path is "/", so the home route should match
            match &node.kind {
                NodeKind::Text(tc) => match tc {
                    crate::node::TextContent::Static(s) => assert_eq!(s, "home"),
                    _ => panic!("expected static text"),
                },
                _ => panic!("expected text node"),
            }
        });
    }

    #[test]
    fn router_no_match_returns_empty() {
        let current_path = create_signal("/nonexistent".to_owned());
        let params = create_signal(Vec::<(String, String)>::new());

        let state = RouterState {
            current_path,
            params,
        };

        let routes = vec![Route::new("/home", || text("home"))];
        let node = render_route(&state, &routes);

        match &node.kind {
            NodeKind::Container => assert!(node.children.is_empty()),
            _ => panic!("expected empty container"),
        }
    }

    #[test]
    fn router_param_extraction() {
        let current_path = create_signal("/users/42".to_owned());
        let params = create_signal(Vec::<(String, String)>::new());

        let state = RouterState {
            current_path,
            params: params.clone(),
        };

        let routes = vec![Route::new("/users/:id", || text("user page"))];
        let _node = render_route(&state, &routes);

        let extracted = params.get();
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0], ("id".to_owned(), "42".to_owned()));
    }
}
