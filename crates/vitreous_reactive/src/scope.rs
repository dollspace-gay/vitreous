use std::collections::HashMap;
use std::marker::PhantomData;

use crate::runtime::{RUNTIME, ScopeId, ScopeSlot};

// ═══════════════════════════════════════════════════════════════════════════
// Scope — reactive ownership boundary
// ═══════════════════════════════════════════════════════════════════════════

/// A reactive ownership boundary. All signals, memos, and effects created
/// inside a scope are cleaned up when the scope is dropped.
///
/// Cleanup runs in reverse creation order: effects are disposed before the
/// signals they read, preventing stale reads during teardown.
pub struct Scope {
    id: ScopeId,
    _marker: PhantomData<*const ()>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        dispose_scope(self.id);
    }
}

/// Run all cleanup functions for a scope in reverse order, then remove the
/// scope from the runtime.
fn dispose_scope(id: ScopeId) {
    let cleanups = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        if let Some(mut slot) = rt.scopes.remove(id.0) {
            slot.cleanups.reverse();
            slot.cleanups
        } else {
            Vec::new()
        }
    });

    // Run cleanups outside the borrow — each cleanup does its own short borrow
    for cleanup in cleanups {
        cleanup();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// create_scope — free function constructor
// ═══════════════════════════════════════════════════════════════════════════

/// Run a closure with the given scope pushed onto the scope stack, so that
/// `use_context` and `provide_context` work. Unlike `create_scope`, this does
/// NOT create a new scope — it re-enters an existing one.
///
/// Used by the platform layer to give event handlers access to the reactive
/// context they were created in.
pub fn run_in_scope(scope: &Scope, f: impl FnOnce()) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().scope_stack.push(scope.id);
    });

    f();

    RUNTIME.with(|rt| {
        rt.borrow_mut().scope_stack.pop();
    });
}

/// Create a new reactive scope. The closure `f` runs synchronously within
/// the scope; any signals, memos, or effects created inside `f` are owned
/// by the returned `Scope` and will be disposed when it drops.
///
/// Scopes can be nested. Inner scopes can access context provided by outer
/// scopes via `use_context`.
pub fn create_scope(f: impl FnOnce()) -> Scope {
    let scope_id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let parent = rt.scope_stack.last().copied();
        let key = rt.scopes.insert(ScopeSlot {
            cleanups: Vec::new(),
            parent,
            context: HashMap::new(),
        });
        let scope_id = ScopeId(key);
        rt.scope_stack.push(scope_id);
        scope_id
    });

    // Run the closure within the scope context — no borrow held
    f();

    RUNTIME.with(|rt| {
        rt.borrow_mut().scope_stack.pop();
    });

    Scope {
        id: scope_id,
        _marker: PhantomData,
    }
}
