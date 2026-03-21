use std::any::TypeId;

use crate::runtime::RUNTIME;

// ═══════════════════════════════════════════════════════════════════════════
// Context — tree-scoped value injection
// ═══════════════════════════════════════════════════════════════════════════

/// Provide a context value of type `T` in the current scope. Inner scopes
/// (and effects re-running within those scopes) can retrieve it via
/// `use_context::<T>()`.
///
/// # Panics
///
/// Panics if called outside any scope.
pub fn provide_context<T: Clone + 'static>(value: T) {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let scope_id = rt
            .scope_stack
            .last()
            .copied()
            .expect("provide_context called outside a scope");
        let scope = rt
            .scopes
            .get_mut(scope_id.0)
            .expect("Current scope has been disposed");
        scope.context.insert(TypeId::of::<T>(), Box::new(value));
    });
}

/// Retrieve a context value of type `T` by walking up the scope tree from
/// the current scope.
///
/// # Panics
///
/// Panics if no provider for `T` is found in any ancestor scope, or if
/// called outside any scope.
pub fn use_context<T: Clone + 'static>() -> T {
    try_use_context::<T>().unwrap_or_else(|| {
        panic!(
            "use_context::<{}>() — no provider found in any ancestor scope",
            std::any::type_name::<T>()
        )
    })
}

/// Try to retrieve a context value of type `T` by walking up the scope tree.
/// Returns `None` if no provider exists.
pub fn try_use_context<T: Clone + 'static>() -> Option<T> {
    RUNTIME.with(|rt| {
        let rt = rt.borrow();
        let mut current = rt.scope_stack.last().copied();

        while let Some(scope_id) = current {
            if let Some(scope) = rt.scopes.get(scope_id.0) {
                if let Some(value) = scope.context.get(&TypeId::of::<T>()) {
                    return Some(
                        value
                            .downcast_ref::<T>()
                            .expect("Context type mismatch")
                            .clone(),
                    );
                }
                current = scope.parent;
            } else {
                break;
            }
        }

        None
    })
}
