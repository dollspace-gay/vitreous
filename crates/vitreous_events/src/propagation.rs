/// Tracks whether event propagation has been stopped during bubble-up dispatch.
///
/// Passed to event handlers; calling `stop_propagation()` prevents the event
/// from bubbling further up the ancestor chain.
#[derive(Debug, Default)]
pub struct PropagationContext {
    stopped: bool,
}

impl PropagationContext {
    /// Create a new propagation context (propagation active).
    pub fn new() -> Self {
        Self { stopped: false }
    }

    /// Stop the event from propagating to ancestor handlers.
    pub fn stop_propagation(&mut self) {
        self.stopped = true;
    }

    /// Returns `true` if propagation has been stopped.
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }
}
