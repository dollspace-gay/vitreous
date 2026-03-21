# Feature: Implement vitreous_reactive — signal-based reactivity system

## Summary

Implement the core reactive primitives library: `Signal<T>`, `Memo<T>`, `Effect`, `Context<T>`, `Scope`, `Resource<S,T>`, and the thread-local `Runtime` with automatic dependency tracking, batched effect execution, and glitch-free propagation. This crate has zero external dependencies and is the foundation all UI reactivity builds on.

## Requirements

- REQ-1: `Signal<T>` — owned reactive value with `get()`, `get_untracked()`, `set()`, `update()`, and `read_only()` methods
- REQ-2: `Memo<T>` — lazy derived computation that auto-tracks dependencies, caches result, and uses `PartialEq` to skip downstream propagation when value unchanged
- REQ-3: `Effect` — side-effect that re-runs when dependencies change, batched to run after all synchronous signal updates complete
- REQ-4: `Context<T>` — tree-scoped value injection via `provide_context()`, `use_context()`, `try_use_context()`
- REQ-5: `Scope` — reactive ownership boundary; all signals/memos/effects created within a scope are cleaned up when the scope drops
- REQ-6: `Resource<S, T>` — async data source that refetches when source signal changes, tracks loading/error/data states reactively, cancels previous fetch on re-trigger
- REQ-7: Thread-local `Runtime` with slot-map storage, automatic dependency tracking via observer stack, and batch depth counter
- REQ-8: `batch()` function groups multiple signal updates into one effect flush; all event handlers implicitly batch
- REQ-9: Push-pull hybrid propagation: staleness pushed eagerly on signal write, values pulled lazily on memo read
- REQ-10: Diamond dependency problem handled glitch-free — memos in diamond graphs never observe inconsistent intermediate states
- REQ-11: `create_signal`, `create_memo`, `create_effect`, `create_resource`, `create_scope`, `provide_context`, `use_context`, `try_use_context`, `batch` are free functions (not methods)

## Acceptance Criteria

- [ ] AC-1: `create_signal(42)` returns signal, `.get()` returns 42, `.set(100)` makes `.get()` return 100 (REQ-1)
- [ ] AC-2: `create_memo(move || a.get() + b.get())` recomputes when either `a` or `b` changes, returns cached value otherwise (REQ-2)
- [ ] AC-3: Memo with clamping function (`a.get().min(10)`) does NOT propagate to downstream effect when input changes from 100 to 200 (both clamp to 10) (REQ-2)
- [ ] AC-4: Effect created with `create_effect` fires once initially, then once per batch of dependency changes (REQ-3)
- [ ] AC-5: `batch(|| { s.set(1); s.set(2); s.set(3); })` causes downstream effect to fire exactly once with final value 3 (REQ-8)
- [ ] AC-6: Diamond dependency (a -> b, a -> c, b+c -> d): when `a` changes, `d` observes consistent `b` and `c`, never intermediate values (REQ-10)
- [ ] AC-7: `create_scope` returns `Scope`; effects created inside do not fire after scope is dropped (REQ-5)
- [ ] AC-8: `provide_context(42u32)` in outer scope, `use_context::<u32>()` in inner scope returns 42 (REQ-4)
- [ ] AC-9: `try_use_context::<String>()` returns `None` when no provider exists (REQ-4)
- [ ] AC-10: `create_resource` with mock async fetcher transitions through loading=true -> data=Some, loading=false (REQ-6)
- [ ] AC-11: `Signal::get_untracked()` reads value without registering as dependency — effect using only `get_untracked` does not re-run on signal change (REQ-1)
- [ ] AC-12: `Signal::read_only()` returns `ReadSignal<T>` that can `.get()` but has no `.set()` method (REQ-1)
- [ ] AC-13: No `Send` or `Sync` bounds on signal values — `Signal<Rc<RefCell<Vec<String>>>>` compiles (REQ-7)
- [ ] AC-14: Property test with random signal/memo/effect graphs: no glitches, all effects fire exactly once per batch, all cleanup runs on scope drop (REQ-9, REQ-10)
- [ ] AC-15: Benchmark: signal get/set < 50ns, memo recomputation (single dep) < 100ns (REQ-9)

## Architecture

### File Structure

```
crates/vitreous_reactive/src/
├── lib.rs          # Public API re-exports: create_signal, create_memo, etc.
├── signal.rs       # Signal<T>, ReadSignal<T>, SignalId
├── memo.rs         # Memo<T>, MemoId
├── effect.rs       # Effect, EffectId, create_effect
├── resource.rs     # Resource<S,T>, create_resource
├── context.rs      # Context<T>, provide_context, use_context, try_use_context
├── runtime.rs      # Runtime struct (thread-local), slot maps, observer stack
├── batch.rs        # batch() function, batch depth counter, effect flush
├── graph.rs        # Dependency edge tracking, subscriber notification
└── scope.rs        # Scope struct, create_scope, cleanup on Drop
```

### Runtime Internals

```rust
pub(crate) struct Runtime {
    signals: SlotMap<SignalId, SignalSlot>,     // Signal storage
    memos: SlotMap<MemoId, MemoSlot>,           // Memo storage
    effects: SlotMap<EffectId, EffectSlot>,     // Effect storage
    observer: Option<ObserverId>,               // Current tracking context
    batch_depth: u32,                           // Nested batch counter
    pending_effects: Vec<EffectId>,             // Queue for deferred effects
    context_stack: Vec<HashMap<TypeId, Box<dyn Any>>>, // Context scopes
    scope_stack: Vec<ScopeId>,                  // Active scope tracking
}
```

The runtime is accessed via `thread_local!` — one per thread, compatible with single-threaded WASM.

### Dependency Tracking Algorithm

**On `Signal::get()`**: If `observer` is `Some(id)`, record `(signal_id, observer_id)` as a dependency edge.

**On `Signal::set()`**: Store new value. Walk all subscriber memos — mark stale (don't recompute). Walk all subscriber effects — add to `pending_effects`. If `batch_depth == 0`, flush effects.

**On `Memo::get()`**: If stale, recompute (re-registers dependencies). Compare via `PartialEq`. If unchanged, mark clean, do NOT propagate. If changed, store new value, propagate staleness downstream.

### Scope Cleanup

`Scope` holds `Vec<CleanupFn>`. When a signal, memo, or effect is created inside a scope, a cleanup closure is registered. On `Scope::drop()`, all cleanup closures run in reverse order, removing entries from the runtime's slot maps.

### Trait Bounds

- `Signal<T>`: requires `T: Clone + 'static` for `get()`/`set()`, no `Send`/`Sync`
- `Memo<T>`: requires `T: Clone + PartialEq + 'static`
- `Effect`: closure is `Fn() + 'static`
- `Resource<S, T>`: `S: Clone + PartialEq + 'static`, `T: Clone + 'static`, fetcher returns `Future<Output = Result<T, Box<dyn Error>>> + 'static`

### Async Executor Integration

`Resource` spawns futures via a pluggable executor trait:
- Desktop: `tokio::spawn` (or `tokio::spawn_local` if on the UI thread)
- Web: `wasm_bindgen_futures::spawn_local`

The executor is set once during app initialization. Resource does not depend on tokio or wasm-bindgen directly — it accepts a `Box<dyn Fn(Pin<Box<dyn Future<Output = ()> + 'static>>)>`.

## Open Questions

None — the reactive system is fully specified.

## Out of Scope

- UI-aware dirty tracking (widget rebuilds) — that's `vitreous_widgets` (Phase 3)
- Async runtime setup — that's `vitreous_platform` (Phase 5) and `vitreous_web` (Phase 4B)
- Serialization of reactive state
- Cross-thread signal sharing (signals are `!Send` by design)
