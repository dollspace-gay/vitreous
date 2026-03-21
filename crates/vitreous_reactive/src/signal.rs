use std::marker::PhantomData;

use crate::graph::notify_signal_subscribers;
use crate::runtime::{RUNTIME, SignalId, SignalSlot, SubscriptionSource};

// ═══════════════════════════════════════════════════════════════════════════
// Signal<T> — owned reactive value
// ═══════════════════════════════════════════════════════════════════════════

/// A reactive value that notifies subscribers when changed.
///
/// `Signal<T>` is `Copy` and `!Send`/`!Sync` — it is a lightweight handle
/// into the thread-local runtime. The value is stored in the runtime, not
/// in the handle itself.
pub struct Signal<T: 'static> {
    pub(crate) id: SignalId,
    // *const T makes Signal !Send + !Sync and covariant in T
    _marker: PhantomData<*const T>,
}

impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for Signal<T> {}

impl<T: Clone + 'static> Signal<T> {
    /// Read the current value, registering a dependency on the current
    /// observer (effect or memo) if one is active.
    pub fn get(&self) -> T {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let value = rt
                .signals
                .get(self.id.0)
                .unwrap_or_else(|| panic!("Signal has been disposed"))
                .value
                .downcast_ref::<T>()
                .expect("Signal type mismatch")
                .clone();

            if let Some(observer) = rt.observer {
                if let Some(sig) = rt.signals.get_mut(self.id.0)
                    && !sig.subscribers.contains(&observer)
                {
                    sig.subscribers.push(observer);
                }
                rt.tracking.push(SubscriptionSource::Signal(self.id));
            }

            value
        })
    }

    /// Read the current value WITHOUT registering a dependency. The current
    /// observer will not re-run when this signal changes.
    pub fn get_untracked(&self) -> T {
        RUNTIME.with(|rt| {
            rt.borrow()
                .signals
                .get(self.id.0)
                .unwrap_or_else(|| panic!("Signal has been disposed"))
                .value
                .downcast_ref::<T>()
                .expect("Signal type mismatch")
                .clone()
        })
    }

    /// Replace the value and notify all subscribers.
    pub fn set(&self, value: T) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            if let Some(slot) = rt.signals.get_mut(self.id.0) {
                slot.value = Box::new(value);
            }
        });
        notify_signal_subscribers(self.id);
    }

    /// Mutate the value in place and notify all subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            if let Some(slot) = rt.signals.get_mut(self.id.0) {
                let value = slot
                    .value
                    .downcast_mut::<T>()
                    .expect("Signal type mismatch");
                f(value);
            }
        });
        notify_signal_subscribers(self.id);
    }

    /// Return a read-only view of this signal. The returned `ReadSignal`
    /// can `.get()` but has no `.set()` method.
    pub fn read_only(&self) -> ReadSignal<T> {
        ReadSignal {
            id: self.id,
            _marker: PhantomData,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ReadSignal<T> — read-only view of a signal
// ═══════════════════════════════════════════════════════════════════════════

/// A read-only handle to a `Signal<T>`. Can read but cannot write.
pub struct ReadSignal<T: 'static> {
    pub(crate) id: SignalId,
    _marker: PhantomData<*const T>,
}

impl<T: 'static> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for ReadSignal<T> {}

impl<T: Clone + 'static> ReadSignal<T> {
    /// Read the current value, registering a dependency on the current observer.
    pub fn get(&self) -> T {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let value = rt
                .signals
                .get(self.id.0)
                .unwrap_or_else(|| panic!("Signal has been disposed"))
                .value
                .downcast_ref::<T>()
                .expect("Signal type mismatch")
                .clone();

            if let Some(observer) = rt.observer {
                if let Some(sig) = rt.signals.get_mut(self.id.0)
                    && !sig.subscribers.contains(&observer)
                {
                    sig.subscribers.push(observer);
                }
                rt.tracking.push(SubscriptionSource::Signal(self.id));
            }

            value
        })
    }

    /// Read the current value WITHOUT registering a dependency.
    pub fn get_untracked(&self) -> T {
        RUNTIME.with(|rt| {
            rt.borrow()
                .signals
                .get(self.id.0)
                .unwrap_or_else(|| panic!("Signal has been disposed"))
                .value
                .downcast_ref::<T>()
                .expect("Signal type mismatch")
                .clone()
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// create_signal — free function constructor
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new reactive signal with the given initial value.
///
/// If called inside a `create_scope`, the signal is owned by that scope
/// and will be disposed when the scope drops.
pub fn create_signal<T: Clone + 'static>(value: T) -> Signal<T> {
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let key = rt.signals.insert(SignalSlot {
            value: Box::new(value),
            subscribers: Vec::new(),
        });
        let signal_id = SignalId(key);

        // Register cleanup with current scope
        if let Some(&scope_id) = rt.scope_stack.last() {
            let sid = signal_id;
            if let Some(scope) = rt.scopes.get_mut(scope_id.0) {
                scope.cleanups.push(Box::new(move || {
                    crate::graph::dispose_signal(sid);
                }));
            }
        }

        signal_id
    });

    Signal {
        id,
        _marker: PhantomData,
    }
}
