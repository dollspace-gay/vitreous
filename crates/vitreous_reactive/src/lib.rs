pub mod batch;
pub mod context;
pub mod effect;
pub mod graph;
pub mod memo;
pub mod resource;
pub mod runtime;
pub mod scope;
pub mod signal;

// ═══════════════════════════════════════════════════════════════════════════
// Public API re-exports
// ═══════════════════════════════════════════════════════════════════════════

pub use batch::batch;
pub use context::{provide_context, try_use_context, use_context};
pub use effect::{Effect, create_effect};
pub use memo::{Memo, create_memo};
pub use resource::{Resource, create_resource, set_executor};
pub use scope::{Scope, create_scope, run_in_scope};
pub use signal::{ReadSignal, Signal, create_signal};

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    // AC-1: create_signal(42) returns signal, .get() returns 42,
    //       .set(100) makes .get() return 100
    #[test]
    fn ac1_signal_get_set() {
        let s = create_signal(42);
        assert_eq!(s.get(), 42);
        s.set(100);
        assert_eq!(s.get(), 100);
    }

    // AC-2: create_memo recomputes when either dependency changes,
    //       returns cached value otherwise
    #[test]
    fn ac2_memo_recomputes_on_dependency_change() {
        let a = create_signal(1);
        let b = create_signal(2);
        let compute_count = Rc::new(Cell::new(0u32));
        let cc = compute_count.clone();
        let m = create_memo(move || {
            cc.set(cc.get() + 1);
            a.get() + b.get()
        });

        assert_eq!(m.get(), 3);
        let count_after_first = compute_count.get();

        // Reading again should use cache (no recompute)
        assert_eq!(m.get(), 3);
        assert_eq!(compute_count.get(), count_after_first);

        // Changing a should trigger recompute
        a.set(10);
        assert_eq!(m.get(), 12);
        assert!(compute_count.get() > count_after_first);

        // Changing b should trigger recompute
        let count_before_b = compute_count.get();
        b.set(20);
        assert_eq!(m.get(), 30);
        assert!(compute_count.get() > count_before_b);
    }

    // AC-3: Memo with clamping does NOT propagate to downstream effect
    //       when value unchanged
    #[test]
    fn ac3_memo_partial_eq_filtering() {
        let a = create_signal(100i32);
        let clamped = create_memo(move || a.get().min(10));
        let effect_count = Rc::new(Cell::new(0u32));
        let ec = effect_count.clone();

        create_effect(move || {
            clamped.get();
            ec.set(ec.get() + 1);
        });

        // Initial run
        assert_eq!(effect_count.get(), 1);

        // Change a from 100 to 200 — clamped stays 10 — effect should NOT fire
        a.set(200);
        assert_eq!(effect_count.get(), 1);

        // Change a to 5 — clamped changes to 5 — effect SHOULD fire
        a.set(5);
        assert_eq!(effect_count.get(), 2);
        assert_eq!(clamped.get(), 5);
    }

    // AC-4: Effect fires once initially, then once per batch of changes
    #[test]
    fn ac4_effect_fires_initially_and_on_change() {
        let s = create_signal(0);
        let values = Rc::new(RefCell::new(Vec::new()));
        let v = values.clone();

        create_effect(move || {
            v.borrow_mut().push(s.get());
        });

        assert_eq!(*values.borrow(), vec![0]);

        s.set(1);
        assert_eq!(*values.borrow(), vec![0, 1]);

        s.set(2);
        assert_eq!(*values.borrow(), vec![0, 1, 2]);
    }

    // AC-5: batch causes effect to fire exactly once with final value
    #[test]
    fn ac5_batch_coalesces_updates() {
        let s = create_signal(0);
        let values = Rc::new(RefCell::new(Vec::new()));
        let v = values.clone();

        create_effect(move || {
            v.borrow_mut().push(s.get());
        });

        // Initial run
        assert_eq!(*values.borrow(), vec![0]);

        batch(|| {
            s.set(1);
            s.set(2);
            s.set(3);
        });

        // Effect should fire exactly once with final value 3
        assert_eq!(*values.borrow(), vec![0, 3]);
    }

    // AC-6: Diamond dependency — d observes consistent b and c
    #[test]
    fn ac6_diamond_dependency_glitch_free() {
        let a = create_signal(1);
        let b = create_memo(move || a.get() * 2);
        let c = create_memo(move || a.get() * 3);
        let d = create_memo(move || b.get() + c.get());

        let observations = Rc::new(RefCell::new(Vec::new()));
        let obs = observations.clone();

        create_effect(move || {
            obs.borrow_mut().push((b.get(), c.get(), d.get()));
        });

        // Initial: a=1, b=2, c=3, d=5
        assert_eq!(observations.borrow()[0], (2, 3, 5));

        a.set(2);

        // After: a=2, b=4, c=6, d=10 — consistent, no intermediate values
        let obs = observations.borrow();
        assert_eq!(obs.len(), 2);
        assert_eq!(obs[1], (4, 6, 10));
    }

    // AC-7: Scope — effects created inside do not fire after scope is dropped
    #[test]
    fn ac7_scope_cleanup() {
        let s = create_signal(0);
        let effect_count = Rc::new(Cell::new(0u32));
        let ec = effect_count.clone();

        let scope = create_scope(|| {
            create_effect(move || {
                s.get();
                ec.set(ec.get() + 1);
            });
        });

        // Initial run inside scope
        assert_eq!(effect_count.get(), 1);

        s.set(1);
        assert_eq!(effect_count.get(), 2);

        // Drop the scope — effect should be cleaned up
        drop(scope);

        s.set(2);
        // Effect should NOT fire after scope drop
        assert_eq!(effect_count.get(), 2);
    }

    // AC-8: provide_context in outer scope, use_context in inner scope
    #[test]
    fn ac8_context_propagation() {
        let result = Rc::new(Cell::new(0u32));
        let r = result.clone();

        let _scope = create_scope(|| {
            provide_context(42u32);

            let _inner = create_scope(|| {
                let val = use_context::<u32>();
                r.set(val);
            });
        });

        assert_eq!(result.get(), 42);
    }

    // AC-9: try_use_context returns None when no provider exists
    #[test]
    fn ac9_try_use_context_none() {
        let result = Rc::new(RefCell::new(None::<String>));
        let r = result.clone();

        let _scope = create_scope(|| {
            let val = try_use_context::<String>();
            *r.borrow_mut() = val;
        });

        assert!(result.borrow().is_none());
    }

    // AC-10: Resource transitions through loading=true -> data=Some, loading=false
    #[test]
    fn ac10_resource_states() {
        // Track what the executor receives
        let task_cell: Rc<RefCell<Option<std::pin::Pin<Box<dyn Future<Output = ()>>>>>> =
            Rc::new(RefCell::new(None));
        let tc = task_cell.clone();

        set_executor(move |fut| {
            *tc.borrow_mut() = Some(fut);
        });

        let source = create_signal(1u32);
        let resource = create_resource(
            move || source.get(),
            |val| {
                Box::pin(async move { Ok(val * 10) })
                    as std::pin::Pin<
                        Box<dyn Future<Output = Result<u32, Box<dyn std::error::Error>>>>,
                    >
            },
        );

        // After creation: loading=true
        assert!(resource.loading());
        assert!(resource.data().is_none());

        // Drive the future to completion manually
        if let Some(fut) = task_cell.borrow_mut().take() {
            // Use a simple manual poll
            let waker = futures_noop_waker();
            let mut cx = std::task::Context::from_waker(&waker);
            let mut fut = fut;
            // For our simple async block, one poll should complete it
            let _ = std::pin::Pin::as_mut(&mut fut).poll(&mut cx);
        }

        // After completion: loading=false, data=Some(10)
        assert!(!resource.loading());
        assert_eq!(resource.data(), Some(10));
        assert!(resource.error().is_none());
    }

    // AC-11: get_untracked does NOT register dependency
    #[test]
    fn ac11_get_untracked() {
        let s = create_signal(1);
        let effect_count = Rc::new(Cell::new(0u32));
        let ec = effect_count.clone();

        create_effect(move || {
            s.get_untracked();
            ec.set(ec.get() + 1);
        });

        assert_eq!(effect_count.get(), 1);

        // Signal change should NOT re-trigger effect (only used get_untracked)
        s.set(2);
        assert_eq!(effect_count.get(), 1);
    }

    // AC-12: read_only returns ReadSignal that can get but not set
    #[test]
    fn ac12_read_only() {
        let s = create_signal(42);
        let ro = s.read_only();

        assert_eq!(ro.get(), 42);

        s.set(100);
        assert_eq!(ro.get(), 100);

        // ReadSignal has no .set() method — this is a compile-time guarantee.
        // We verify it compiles correctly; the absence of .set() is structural.
    }

    // AC-13: No Send/Sync bounds — Signal<Rc<RefCell<Vec<String>>>> compiles
    #[test]
    fn ac13_no_send_sync_bounds() {
        let data = Rc::new(RefCell::new(vec!["hello".to_string()]));
        let s = create_signal(data.clone());

        let got = s.get();
        assert_eq!(*got.borrow(), vec!["hello".to_string()]);
    }

    // AC-14: Property test — covered in integration tests with proptest
    // (Requires proptest dev-dependency; see tests/property_tests.rs)

    // AC-15: Benchmark — covered in benches/ with criterion
    // (Requires criterion dev-dependency; see benches/reactive_bench.rs)

    // ─── Additional correctness tests ────────────────────────────────────

    #[test]
    fn nested_memos() {
        let a = create_signal(1);
        let b = create_memo(move || a.get() * 2);
        let c = create_memo(move || b.get() + 10);

        assert_eq!(c.get(), 12);

        a.set(5);
        assert_eq!(c.get(), 20);
    }

    #[test]
    fn effect_with_multiple_signals() {
        let x = create_signal(1);
        let y = create_signal(2);
        let sums = Rc::new(RefCell::new(Vec::new()));
        let s = sums.clone();

        create_effect(move || {
            s.borrow_mut().push(x.get() + y.get());
        });

        assert_eq!(*sums.borrow(), vec![3]);

        x.set(10);
        assert_eq!(*sums.borrow(), vec![3, 12]);

        y.set(20);
        assert_eq!(*sums.borrow(), vec![3, 12, 30]);
    }

    #[test]
    fn batch_with_multiple_signals() {
        let a = create_signal(0);
        let b = create_signal(0);
        let values = Rc::new(RefCell::new(Vec::new()));
        let v = values.clone();

        create_effect(move || {
            v.borrow_mut().push((a.get(), b.get()));
        });

        assert_eq!(*values.borrow(), vec![(0, 0)]);

        batch(|| {
            a.set(1);
            b.set(2);
        });

        let vals = values.borrow();
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[1], (1, 2));
    }

    #[test]
    fn nested_scopes() {
        let s = create_signal(0);
        let inner_count = Rc::new(Cell::new(0u32));
        let ic = inner_count.clone();
        let outer_count = Rc::new(Cell::new(0u32));
        let oc = outer_count.clone();

        let outer = create_scope(|| {
            create_effect({
                let oc = oc.clone();
                move || {
                    s.get();
                    oc.set(oc.get() + 1);
                }
            });

            let inner = create_scope(|| {
                create_effect(move || {
                    s.get();
                    ic.set(ic.get() + 1);
                });
            });

            s.set(1);
            assert_eq!(inner_count.get(), 2);
            assert_eq!(outer_count.get(), 2);

            drop(inner);

            s.set(2);
            // Inner effect disposed, outer still active
            assert_eq!(inner_count.get(), 2);
            assert_eq!(outer_count.get(), 3);
        });

        drop(outer);

        s.set(3);
        // Both disposed
        assert_eq!(inner_count.get(), 2);
        assert_eq!(outer_count.get(), 3);
    }

    #[test]
    fn conditional_dependencies() {
        let flag = create_signal(true);
        let a = create_signal(1);
        let b = create_signal(2);
        let values = Rc::new(RefCell::new(Vec::new()));
        let v = values.clone();

        create_effect(move || {
            let val = if flag.get() { a.get() } else { b.get() };
            v.borrow_mut().push(val);
        });

        assert_eq!(*values.borrow(), vec![1]);

        // Changing b should NOT trigger (effect doesn't depend on b when flag=true)
        b.set(20);
        assert_eq!(*values.borrow(), vec![1]);

        // Switch flag — now depends on b
        flag.set(false);
        assert_eq!(values.borrow().last(), Some(&20));

        // Changing a should NOT trigger (effect now depends on b, not a)
        let len = values.borrow().len();
        a.set(100);
        assert_eq!(values.borrow().len(), len);
    }

    #[test]
    fn signal_update() {
        let s = create_signal(vec![1, 2, 3]);
        s.update(|v| v.push(4));
        assert_eq!(s.get(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn context_in_effect_rerun() {
        let s = create_signal(0);
        let values = Rc::new(RefCell::new(Vec::new()));
        let v = values.clone();

        let _scope = create_scope(|| {
            provide_context(100u32);

            create_effect(move || {
                let ctx = use_context::<u32>();
                let sig = s.get();
                v.borrow_mut().push((ctx, sig));
            });
        });

        assert_eq!(*values.borrow(), vec![(100, 0)]);

        s.set(1);
        assert_eq!(*values.borrow(), vec![(100, 0), (100, 1)]);
    }

    #[test]
    fn memo_chain_partial_eq_filtering() {
        let a = create_signal(5i32);
        let b = create_memo(move || a.get().min(10));
        let c = create_memo(move || b.get() * 2);
        let effect_count = Rc::new(Cell::new(0u32));
        let ec = effect_count.clone();

        create_effect(move || {
            c.get();
            ec.set(ec.get() + 1);
        });

        assert_eq!(effect_count.get(), 1);
        assert_eq!(c.get(), 10);

        // b doesn't change (5.min(10) == 3.min(10) is wrong, let's use correct values)
        // a=5 -> b=5 -> c=10
        // a=3 -> b=3 -> c=6 (changed!)
        a.set(3);
        assert_eq!(effect_count.get(), 2);
        assert_eq!(c.get(), 6);

        // a=100 -> b=10 -> c=20 (changed!)
        a.set(100);
        assert_eq!(effect_count.get(), 3);
        assert_eq!(c.get(), 20);

        // a=200 -> b=10 -> c=20 (NOT changed — b clamped)
        a.set(200);
        assert_eq!(effect_count.get(), 3);
        assert_eq!(c.get(), 20);
    }

    #[test]
    fn multiple_effects_same_memo() {
        let a = create_signal(1);
        let m = create_memo(move || a.get() * 2);

        let e1_count = Rc::new(Cell::new(0u32));
        let e2_count = Rc::new(Cell::new(0u32));
        let ec1 = e1_count.clone();
        let ec2 = e2_count.clone();

        create_effect(move || {
            m.get();
            ec1.set(ec1.get() + 1);
        });
        create_effect(move || {
            m.get();
            ec2.set(ec2.get() + 1);
        });

        assert_eq!(e1_count.get(), 1);
        assert_eq!(e2_count.get(), 1);

        a.set(2);
        assert_eq!(e1_count.get(), 2);
        assert_eq!(e2_count.get(), 2);
    }

    // ─── Noop waker for manual future polling in tests ───────────────────

    fn futures_noop_waker() -> std::task::Waker {
        use std::task::{RawWaker, RawWakerVTable};

        fn noop(_: *const ()) {}
        fn clone(p: *const ()) -> RawWaker {
            RawWaker::new(p, &VTABLE)
        }

        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);

        unsafe { std::task::Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
    }
}
