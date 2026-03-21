use std::cell::Cell;
use std::error::Error;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::rc::Rc;

use crate::effect::create_effect;
use crate::runtime::RUNTIME;
use crate::signal::{Signal, create_signal};

// ═══════════════════════════════════════════════════════════════════════════
// Resource<S, T> — async data source with reactive state tracking
// ═══════════════════════════════════════════════════════════════════════════

/// An async data source that refetches when its source signal changes.
///
/// Tracks loading, data, and error states reactively. Previous fetches are
/// cancelled (via generation counter) when a new fetch is triggered.
pub struct Resource<S: 'static, T: 'static> {
    loading_signal: Signal<bool>,
    data_signal: Signal<Option<T>>,
    error_signal: Signal<Option<String>>,
    _marker: PhantomData<*const S>,
}

impl<S: 'static, T: 'static> Clone for Resource<S, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<S: 'static, T: 'static> Copy for Resource<S, T> {}

impl<S: 'static, T: Clone + 'static> Resource<S, T> {
    /// Whether a fetch is currently in-flight. Tracks reactively.
    pub fn loading(&self) -> bool {
        self.loading_signal.get()
    }

    /// The most recent successful fetch result. Tracks reactively.
    pub fn data(&self) -> Option<T> {
        self.data_signal.get()
    }

    /// The most recent fetch error message. Tracks reactively.
    pub fn error(&self) -> Option<String> {
        self.error_signal.get()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// create_resource — free function constructor
// ═══════════════════════════════════════════════════════════════════════════

/// Create an async resource that fetches data whenever `source` changes.
///
/// - `source`: a reactive function returning the fetch key (tracked).
/// - `fetcher`: given the source value, returns a future that resolves to
///   the data or an error.
///
/// The resource creates an internal effect that watches `source`. When the
/// source value changes, the previous fetch is logically cancelled (via a
/// generation counter) and a new one is spawned through the executor set
/// by `set_executor`.
///
/// # Panics
///
/// The spawned future will silently drop results if no executor is set.
pub fn create_resource<S, T>(
    source: impl Fn() -> S + 'static,
    fetcher: impl Fn(S) -> Pin<Box<dyn Future<Output = Result<T, Box<dyn Error>>> + 'static>> + 'static,
) -> Resource<S, T>
where
    S: Clone + PartialEq + 'static,
    T: Clone + 'static,
{
    let loading = create_signal(true);
    let data = create_signal(None::<T>);
    let error = create_signal(None::<String>);
    let generation = Rc::new(Cell::new(0u64));
    let fetcher = Rc::new(fetcher);

    create_effect({
        let generation = generation.clone();
        let fetcher = fetcher.clone();
        move || {
            // Reading source() establishes the reactive dependency
            let source_value = source();

            loading.set(true);
            error.set(None);

            let fetch_gen = generation.get() + 1;
            generation.set(fetch_gen);

            let fut = fetcher(source_value);
            let generation = generation.clone();

            let task = Box::pin(async move {
                match fut.await {
                    Ok(value) => {
                        if generation.get() == fetch_gen {
                            data.set(Some(value));
                            loading.set(false);
                        }
                    }
                    Err(e) => {
                        if generation.get() == fetch_gen {
                            error.set(Some(e.to_string()));
                            loading.set(false);
                        }
                    }
                }
            }) as Pin<Box<dyn Future<Output = ()> + 'static>>;

            // Spawn via the pluggable executor
            RUNTIME.with(|rt| {
                let rt = rt.borrow();
                if let Some(ref executor) = rt.executor {
                    executor(task);
                }
            });
        }
    });

    Resource {
        loading_signal: loading,
        data_signal: data,
        error_signal: error,
        _marker: PhantomData,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// set_executor — configure the async runtime for Resource
// ═══════════════════════════════════════════════════════════════════════════

/// Set the executor used by `Resource` to spawn async fetches.
///
/// Call once during app initialization:
/// - Desktop: `set_executor(|fut| tokio::task::spawn_local(fut))`
/// - Web: `set_executor(|fut| wasm_bindgen_futures::spawn_local(fut))`
pub fn set_executor(executor: impl Fn(Pin<Box<dyn Future<Output = ()> + 'static>>) + 'static) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().executor = Some(std::rc::Rc::new(executor));
    });
}
