use std::marker::PhantomData;

use std::any::Any;
use std::rc::Rc;

use crate::graph::clean_observer_sources;
use crate::runtime::{
    EqFn, MemoId, MemoSlot, ObserverId, RUNTIME, SubscriptionSource, enter_tracking, exit_tracking,
};

// ═══════════════════════════════════════════════════════════════════════════
// Memo<T> — lazy derived computation with PartialEq filtering
// ═══════════════════════════════════════════════════════════════════════════

/// A derived reactive value that caches its result and only recomputes when
/// dependencies change. Uses `PartialEq` to skip downstream propagation when
/// the recomputed value equals the previous one.
pub struct Memo<T: 'static> {
    pub(crate) id: MemoId,
    _marker: PhantomData<*const T>,
}

impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Memo<T> {}

impl<T: Clone + PartialEq + 'static> Memo<T> {
    /// Read the memo's current value, recomputing if stale. Registers a
    /// dependency on the current observer.
    pub fn get(&self) -> T {
        // Recompute if stale (pull phase). This must happen outside a borrow
        // because recomputation runs user code.
        let stale = RUNTIME.with(|rt| {
            rt.borrow()
                .memos
                .get(self.id.0)
                .map(|m| m.stale)
                .unwrap_or(false)
        });

        if stale {
            recompute_memo(self.id);
        }

        // Read cached value and register dependency
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let value = rt
                .memos
                .get(self.id.0)
                .and_then(|m| m.value.as_ref())
                .and_then(|v| v.downcast_ref::<T>())
                .cloned()
                .expect("Memo value not available — memo may have been disposed");

            if let Some(observer) = rt.observer {
                if let Some(memo) = rt.memos.get_mut(self.id.0)
                    && !memo.subscribers.contains(&observer)
                {
                    memo.subscribers.push(observer);
                }
                rt.tracking.push(SubscriptionSource::Memo(self.id));
            }

            value
        })
    }

    /// Read the memo's current value WITHOUT registering a dependency.
    /// Still triggers recomputation if stale.
    pub fn get_untracked(&self) -> T {
        let stale = RUNTIME.with(|rt| {
            rt.borrow()
                .memos
                .get(self.id.0)
                .map(|m| m.stale)
                .unwrap_or(false)
        });

        if stale {
            recompute_memo(self.id);
        }

        RUNTIME.with(|rt| {
            rt.borrow()
                .memos
                .get(self.id.0)
                .and_then(|m| m.value.as_ref())
                .and_then(|v| v.downcast_ref::<T>())
                .cloned()
                .expect("Memo value not available — memo may have been disposed")
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// recompute_memo — pull-based recomputation with PartialEq filtering
// ═══════════════════════════════════════════════════════════════════════════

/// Recompute a stale memo. Returns `true` if the value actually changed
/// (i.e. the new value is not equal to the old one per `eq_fn`).
///
/// This function:
/// 1. Cleans old subscription edges
/// 2. Enters a tracking scope for the memo
/// 3. Runs the compute function (which may pull other stale memos)
/// 4. Compares old/new values via the type-erased PartialEq
/// 5. Stores the result and records whether it changed
pub(crate) fn recompute_memo(memo_id: MemoId) -> bool {
    let Some((compute, eq_fn, scope_id)) = RUNTIME.with(|rt| {
        let rt = rt.borrow();
        rt.memos
            .get(memo_id.0)
            .map(|m| (m.compute.clone(), m.eq_fn.clone(), m.scope_id))
    }) else {
        return false;
    };

    // Clean old subscription edges
    clean_observer_sources(ObserverId::Memo(memo_id));

    // Push the memo's scope so context lookups work during recomputation
    if let Some(sid) = scope_id {
        RUNTIME.with(|rt| {
            rt.borrow_mut().scope_stack.push(sid);
        });
    }

    // Enter tracking scope for this memo
    let guard = enter_tracking(ObserverId::Memo(memo_id));

    // Run the compute function — no borrow held
    let new_value = compute();

    // Exit tracking, collect new sources
    let new_sources = exit_tracking(guard);

    // Pop scope
    if scope_id.is_some() {
        RUNTIME.with(|rt| {
            rt.borrow_mut().scope_stack.pop();
        });
    }

    // Compare with old value and update
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        if let Some(memo) = rt.memos.get_mut(memo_id.0) {
            memo.sources = new_sources;
            memo.stale = false;

            let changed = if let Some(ref old_value) = memo.value {
                !eq_fn(old_value.as_ref(), new_value.as_ref())
            } else {
                true // First computation — always "changed"
            };

            memo.value = Some(new_value);
            memo.changed_in_cycle = changed;
            rt.recomputed_memos.push(memo_id);

            changed
        } else {
            false
        }
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// create_memo — free function constructor
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new memo that lazily derives its value from other reactive sources.
///
/// The computation function `f` is called with automatic dependency tracking.
/// The result is cached and only recomputed when dependencies change. If the
/// recomputed value equals the previous one (via `PartialEq`), downstream
/// subscribers are not notified.
pub fn create_memo<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> Memo<T> {
    let compute: Rc<dyn Fn() -> Box<dyn Any>> = Rc::new(move || Box::new(f()) as Box<dyn Any>);

    let eq_fn: EqFn = Rc::new(
        |a, b| match (a.downcast_ref::<T>(), b.downcast_ref::<T>()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        },
    );

    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let scope_id = rt.scope_stack.last().copied();
        let key = rt.memos.insert(MemoSlot {
            value: None,
            compute,
            eq_fn,
            stale: true, // Needs initial computation
            changed_in_cycle: false,
            sources: Vec::new(),
            subscribers: Vec::new(),
            scope_id,
        });
        let memo_id = MemoId(key);

        // Register cleanup with current scope
        if let Some(scope_id) = scope_id {
            let mid = memo_id;
            if let Some(scope) = rt.scopes.get_mut(scope_id.0) {
                scope.cleanups.push(Box::new(move || {
                    crate::graph::dispose_memo(mid);
                }));
            }
        }

        memo_id
    });

    Memo {
        id,
        _marker: PhantomData,
    }
}
