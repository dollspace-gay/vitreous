use crate::effect::run_effect;
use crate::memo::recompute_memo;
use crate::runtime::{EffectId, RUNTIME, SubscriptionSource};

// ═══════════════════════════════════════════════════════════════════════════
// batch — group signal updates into a single effect flush
// ═══════════════════════════════════════════════════════════════════════════

/// Group multiple signal updates so that effects only flush once, after all
/// updates complete. Batches can be nested; effects flush when the outermost
/// batch exits.
pub fn batch(f: impl FnOnce()) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().batch_depth += 1;
    });

    f();

    let should_flush = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.batch_depth -= 1;
        rt.batch_depth == 0 && !rt.pending_effects.is_empty()
    });

    if should_flush {
        flush_effects();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// flush_effects — run pending effects with PartialEq pre-check
// ═══════════════════════════════════════════════════════════════════════════

/// Drain and execute all pending effects. Before running each effect, stale
/// memo sources are recomputed; if none of them actually changed (and the
/// effect has no direct signal sources), the effect is skipped.
///
/// Effects that trigger further signal changes accumulate new pending effects,
/// which are processed in subsequent loop iterations.
pub(crate) fn flush_effects() {
    // Prevent re-entrant flush: signal.set() inside effects won't flush
    // because batch_depth > 0.
    RUNTIME.with(|rt| {
        rt.borrow_mut().batch_depth += 1;
    });

    loop {
        let effect_ids = RUNTIME.with(|rt| std::mem::take(&mut rt.borrow_mut().pending_effects));

        if effect_ids.is_empty() {
            break;
        }

        // Deduplicate while preserving order
        let mut seen = Vec::new();
        let effect_ids: Vec<_> = effect_ids
            .into_iter()
            .filter(|e| {
                if seen.contains(e) {
                    false
                } else {
                    seen.push(*e);
                    true
                }
            })
            .collect();

        for eid in effect_ids {
            if should_run_effect(eid) {
                run_effect(eid);
            }
        }
    }

    // Clear changed_in_cycle flags for all memos recomputed during this flush
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let recomputed = std::mem::take(&mut rt.recomputed_memos);
        for mid in recomputed {
            if let Some(memo) = rt.memos.get_mut(mid.0) {
                memo.changed_in_cycle = false;
            }
        }
        rt.batch_depth -= 1;
    });
}

/// Determine whether a queued effect should actually run by checking if any
/// of its sources changed.
///
/// - Signal sources: always considered "changed" (we don't have PartialEq on T).
/// - Memo sources: if stale, recompute and check `changed_in_cycle`. If already
///   clean, check `changed_in_cycle` (set by a prior effect's pre-check in the
///   same flush cycle).
fn should_run_effect(eid: EffectId) -> bool {
    let sources = RUNTIME.with(|rt| {
        rt.borrow()
            .effects
            .get(eid.0)
            .map(|e| e.sources.clone())
            .unwrap_or_default()
    });

    // Empty sources means the effect was just created or has no deps.
    // The initial run is handled by create_effect directly, so if we reach
    // here with empty sources, the effect was queued but has nothing to check.
    // This can happen if an effect's deps were cleaned but it was still queued.
    if sources.is_empty() {
        return true;
    }

    for source in &sources {
        match source {
            SubscriptionSource::Signal(_) => {
                // A direct signal dependency was set — always run.
                return true;
            }
            SubscriptionSource::Memo(mid) => {
                let (stale, changed) = RUNTIME.with(|rt| {
                    let rt = rt.borrow();
                    rt.memos
                        .get(mid.0)
                        .map(|m| (m.stale, m.changed_in_cycle))
                        .unwrap_or((false, false))
                });

                if stale {
                    // Recompute to find out if value actually changed
                    let changed = recompute_memo(*mid);
                    if changed {
                        return true;
                    }
                } else if changed {
                    // Already recomputed by a prior effect's pre-check in
                    // this flush cycle, and it DID change.
                    return true;
                }
            }
        }
    }

    false
}
