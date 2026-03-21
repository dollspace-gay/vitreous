use crate::runtime::{EffectId, MemoId, ObserverId, RUNTIME, SignalId, SubscriptionSource};

/// Remove an observer from all of its sources' subscriber lists, then clear
/// the observer's own source list. Called before re-running an effect/memo so
/// that stale edges are dropped and fresh edges are recorded during tracking.
pub(crate) fn clean_observer_sources(observer: ObserverId) {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();

        let sources = match observer {
            ObserverId::Effect(eid) => rt
                .effects
                .get(eid.0)
                .map(|e| e.sources.clone())
                .unwrap_or_default(),
            ObserverId::Memo(mid) => rt
                .memos
                .get(mid.0)
                .map(|m| m.sources.clone())
                .unwrap_or_default(),
        };

        for source in &sources {
            match source {
                SubscriptionSource::Signal(sid) => {
                    if let Some(sig) = rt.signals.get_mut(sid.0) {
                        sig.subscribers.retain(|s| *s != observer);
                    }
                }
                SubscriptionSource::Memo(mid) => {
                    if let Some(memo) = rt.memos.get_mut(mid.0) {
                        memo.subscribers.retain(|s| *s != observer);
                    }
                }
            }
        }

        match observer {
            ObserverId::Effect(eid) => {
                if let Some(e) = rt.effects.get_mut(eid.0) {
                    e.sources.clear();
                }
            }
            ObserverId::Memo(mid) => {
                if let Some(m) = rt.memos.get_mut(mid.0) {
                    m.sources.clear();
                }
            }
        }
    });
}

/// Push staleness transitively through the memo graph. Memos are marked stale;
/// effects are added to `pending_effects`. Staleness stops at memos that are
/// already stale (prevents infinite loops on diamonds).
pub(crate) fn mark_stale(subscribers: &[ObserverId]) {
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let mut stack: Vec<ObserverId> = subscribers.to_vec();

        while let Some(sub) = stack.pop() {
            match sub {
                ObserverId::Memo(mid) => {
                    if let Some(memo) = rt.memos.get_mut(mid.0)
                        && !memo.stale
                    {
                        memo.stale = true;
                        stack.extend(memo.subscribers.iter().copied());
                    }
                }
                ObserverId::Effect(eid) => {
                    if !rt.pending_effects.contains(&eid) {
                        rt.pending_effects.push(eid);
                    }
                }
            }
        }
    });
}

/// Notify all subscribers of a signal that just changed. Marks downstream
/// memos as stale, queues effects, and flushes if not inside a batch.
pub(crate) fn notify_signal_subscribers(signal_id: SignalId) {
    let subscribers = RUNTIME.with(|rt| {
        rt.borrow()
            .signals
            .get(signal_id.0)
            .map(|s| s.subscribers.clone())
            .unwrap_or_default()
    });

    if !subscribers.is_empty() {
        mark_stale(&subscribers);
    }

    let should_flush = RUNTIME.with(|rt| {
        let rt = rt.borrow();
        rt.batch_depth == 0 && !rt.pending_effects.is_empty()
    });

    if should_flush {
        crate::batch::flush_effects();
    }
}

/// Dispose a signal: remove it from the runtime slot map.
pub(crate) fn dispose_signal(id: SignalId) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().signals.remove(id.0);
    });
}

/// Dispose a memo: clean its source subscriptions, then remove from slot map.
pub(crate) fn dispose_memo(id: MemoId) {
    clean_observer_sources(ObserverId::Memo(id));
    RUNTIME.with(|rt| {
        rt.borrow_mut().memos.remove(id.0);
    });
}

/// Dispose an effect: clean its source subscriptions, then remove from slot map.
pub(crate) fn dispose_effect(id: EffectId) {
    clean_observer_sources(ObserverId::Effect(id));
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.effects.remove(id.0);
        rt.pending_effects.retain(|e| *e != id);
    });
}
