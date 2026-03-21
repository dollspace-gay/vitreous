use std::marker::PhantomData;
use std::rc::Rc;

use crate::graph::clean_observer_sources;
use crate::runtime::{EffectId, EffectSlot, ObserverId, RUNTIME, enter_tracking, exit_tracking};

// ═══════════════════════════════════════════════════════════════════════════
// Effect — reactive side-effect
// ═══════════════════════════════════════════════════════════════════════════

/// A handle to a reactive effect. The effect's function re-runs whenever its
/// tracked dependencies change.
///
/// Effects are stored in the runtime — dropping the handle does NOT dispose
/// the effect. Use `create_scope` to control effect lifetimes.
pub struct Effect {
    #[allow(dead_code)]
    pub(crate) id: EffectId,
    _marker: PhantomData<*const ()>,
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for Effect {}

// ═══════════════════════════════════════════════════════════════════════════
// run_effect — execute an effect with dependency tracking
// ═══════════════════════════════════════════════════════════════════════════

/// Run an effect: clean old subscriptions, enter tracking, execute the
/// function, then save the new dependency edges.
pub(crate) fn run_effect(id: EffectId) {
    let Some((func, scope_id)) = RUNTIME.with(|rt| {
        let rt = rt.borrow();
        rt.effects.get(id.0).map(|e| (e.func.clone(), e.scope_id))
    }) else {
        return; // Effect was disposed
    };

    // Clean old subscription edges
    clean_observer_sources(ObserverId::Effect(id));

    // Push the effect's scope so context lookups work during re-run
    if let Some(sid) = scope_id {
        RUNTIME.with(|rt| {
            rt.borrow_mut().scope_stack.push(sid);
        });
    }

    // Enter tracking scope
    let guard = enter_tracking(ObserverId::Effect(id));

    // Run the effect function — no borrow held
    func();

    // Exit tracking, collect new sources
    let new_sources = exit_tracking(guard);

    // Pop scope
    if scope_id.is_some() {
        RUNTIME.with(|rt| {
            rt.borrow_mut().scope_stack.pop();
        });
    }

    // Save new subscription sources
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        if let Some(effect) = rt.effects.get_mut(id.0) {
            effect.sources = new_sources;
        }
    });
}

// ═══════════════════════════════════════════════════════════════════════════
// create_effect — free function constructor
// ═══════════════════════════════════════════════════════════════════════════

/// Create a new reactive effect. The function `f` is called immediately to
/// establish initial dependencies, then re-runs whenever those dependencies
/// change.
///
/// If called inside a `create_scope`, the effect is owned by that scope
/// and will be disposed when the scope drops.
pub fn create_effect(f: impl Fn() + 'static) -> Effect {
    let func: Rc<dyn Fn()> = Rc::new(f);

    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let scope_id = rt.scope_stack.last().copied();
        let key = rt.effects.insert(EffectSlot {
            func: func.clone(),
            sources: Vec::new(),
            scope_id,
        });
        let effect_id = EffectId(key);

        // Register cleanup with current scope
        if let Some(scope_id) = scope_id {
            let eid = effect_id;
            if let Some(scope) = rt.scopes.get_mut(scope_id.0) {
                scope.cleanups.push(Box::new(move || {
                    crate::graph::dispose_effect(eid);
                }));
            }
        }

        effect_id
    });

    // Run immediately to establish initial dependencies
    run_effect(id);

    Effect {
        id,
        _marker: PhantomData,
    }
}
