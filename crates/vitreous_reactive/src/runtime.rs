use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

// ═══════════════════════════════════════════════════════════════════════════
// SlotMap — generational arena with O(1) insert/remove/access
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct SlotKey {
    pub index: u32,
    pub generation: u32,
}

pub(crate) struct SlotMap<V> {
    values: Vec<Option<V>>,
    generations: Vec<u32>,
    free_list: Vec<u32>,
}

impl<V> SlotMap<V> {
    pub fn new() -> Self {
        Self {
            values: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: V) -> SlotKey {
        if let Some(index) = self.free_list.pop() {
            let i = index as usize;
            self.values[i] = Some(value);
            SlotKey {
                index,
                generation: self.generations[i],
            }
        } else {
            let index = self.values.len() as u32;
            self.values.push(Some(value));
            self.generations.push(0);
            SlotKey {
                index,
                generation: 0,
            }
        }
    }

    pub fn get(&self, key: SlotKey) -> Option<&V> {
        let i = key.index as usize;
        if i < self.generations.len() && self.generations[i] == key.generation {
            self.values[i].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: SlotKey) -> Option<&mut V> {
        let i = key.index as usize;
        if i < self.generations.len() && self.generations[i] == key.generation {
            self.values[i].as_mut()
        } else {
            None
        }
    }

    pub fn remove(&mut self, key: SlotKey) -> Option<V> {
        let i = key.index as usize;
        if i < self.generations.len() && self.generations[i] == key.generation {
            self.generations[i] = self.generations[i].wrapping_add(1);
            self.free_list.push(key.index);
            self.values[i].take()
        } else {
            None
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// ID newtypes
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct SignalId(pub SlotKey);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct MemoId(pub SlotKey);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct EffectId(pub SlotKey);

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct ScopeId(pub SlotKey);

// ═══════════════════════════════════════════════════════════════════════════
// Observer & subscription tracking
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ObserverId {
    Effect(EffectId),
    Memo(MemoId),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum SubscriptionSource {
    Signal(SignalId),
    Memo(MemoId),
}

// ═══════════════════════════════════════════════════════════════════════════
// Type aliases for complex function pointer types
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) type EqFn = Rc<dyn Fn(&dyn Any, &dyn Any) -> bool>;
pub(crate) type SpawnFn = Rc<dyn Fn(Pin<Box<dyn Future<Output = ()> + 'static>>)>;

// ═══════════════════════════════════════════════════════════════════════════
// Slot types — type-erased storage for each reactive primitive
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) struct SignalSlot {
    pub value: Box<dyn Any>,
    pub subscribers: Vec<ObserverId>,
}

pub(crate) struct MemoSlot {
    pub value: Option<Box<dyn Any>>,
    pub compute: Rc<dyn Fn() -> Box<dyn Any>>,
    pub eq_fn: EqFn,
    pub stale: bool,
    pub changed_in_cycle: bool,
    pub sources: Vec<SubscriptionSource>,
    pub subscribers: Vec<ObserverId>,
    pub scope_id: Option<ScopeId>,
}

pub(crate) struct EffectSlot {
    pub func: Rc<dyn Fn()>,
    pub sources: Vec<SubscriptionSource>,
    pub scope_id: Option<ScopeId>,
}

pub(crate) struct ScopeSlot {
    pub cleanups: Vec<Box<dyn FnOnce()>>,
    pub parent: Option<ScopeId>,
    pub context: HashMap<TypeId, Box<dyn Any>>,
}

// ═══════════════════════════════════════════════════════════════════════════
// Runtime — the single thread-local reactive engine
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) struct Runtime {
    pub signals: SlotMap<SignalSlot>,
    pub memos: SlotMap<MemoSlot>,
    pub effects: SlotMap<EffectSlot>,
    pub scopes: SlotMap<ScopeSlot>,
    pub observer: Option<ObserverId>,
    pub tracking: Vec<SubscriptionSource>,
    pub batch_depth: u32,
    pub pending_effects: Vec<EffectId>,
    pub scope_stack: Vec<ScopeId>,
    pub executor: Option<SpawnFn>,
    pub recomputed_memos: Vec<MemoId>,
}

impl Runtime {
    fn new() -> Self {
        Self {
            signals: SlotMap::new(),
            memos: SlotMap::new(),
            effects: SlotMap::new(),
            scopes: SlotMap::new(),
            observer: None,
            tracking: Vec::new(),
            batch_depth: 0,
            pending_effects: Vec::new(),
            scope_stack: Vec::new(),
            executor: None,
            recomputed_memos: Vec::new(),
        }
    }
}

thread_local! {
    pub(crate) static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::new());
}

// ═══════════════════════════════════════════════════════════════════════════
// Tracking helpers — save/restore observer context across user code
// ═══════════════════════════════════════════════════════════════════════════

pub(crate) struct TrackingGuard {
    pub prev_observer: Option<ObserverId>,
    pub prev_tracking: Vec<SubscriptionSource>,
}

pub(crate) fn enter_tracking(observer: ObserverId) -> TrackingGuard {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let prev_observer = rt.observer.replace(observer);
        let prev_tracking = std::mem::take(&mut rt.tracking);
        TrackingGuard {
            prev_observer,
            prev_tracking,
        }
    })
}

pub(crate) fn exit_tracking(guard: TrackingGuard) -> Vec<SubscriptionSource> {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let sources = std::mem::take(&mut rt.tracking);
        rt.observer = guard.prev_observer;
        rt.tracking = guard.prev_tracking;
        sources
    })
}
