use crate::commands::RenderCommand;
use crate::damage::DamageRect;

/// Compares two command lists and returns damage rects for regions that changed.
///
/// Walks old and new lists in parallel. For each position:
/// - If both exist and differ → damage both old and new bounds
/// - If command was added (new has more) → damage the new bounds
/// - If command was removed (old has more) → damage the old bounds
///
/// Stack commands (PushClip, PopClip, PushOpacity, PopOpacity) are compared
/// but contribute no direct damage rects since they have no bounds.
pub fn diff_commands(old: &[RenderCommand], new: &[RenderCommand]) -> Vec<DamageRect> {
    let mut damage = Vec::new();
    let max_len = old.len().max(new.len());

    for i in 0..max_len {
        match (old.get(i), new.get(i)) {
            (Some(o), Some(n)) => {
                if o != n {
                    add_bounds_damage(&mut damage, o);
                    add_bounds_damage(&mut damage, n);
                }
            }
            (Some(o), None) => {
                // Command removed
                add_bounds_damage(&mut damage, o);
            }
            (None, Some(n)) => {
                // Command added
                add_bounds_damage(&mut damage, n);
            }
            (None, None) => unreachable!(),
        }
    }

    damage
}

fn add_bounds_damage(damage: &mut Vec<DamageRect>, cmd: &RenderCommand) {
    if let Some((x, y, w, h)) = cmd.bounds() {
        damage.push(DamageRect::new(x, y, w, h));
    }
}

/// Returns true if the two command lists are identical (no damage needed).
pub fn commands_equal(old: &[RenderCommand], new: &[RenderCommand]) -> bool {
    old.len() == new.len() && old.iter().zip(new.iter()).all(|(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vitreous_style::{Color, Corners};

    fn fill(x: f32, y: f32, w: f32, h: f32, color: Color) -> RenderCommand {
        RenderCommand::FillRect {
            x,
            y,
            width: w,
            height: h,
            color,
            border_radius: Corners::all(0.0),
        }
    }

    #[test]
    fn identical_commands_produce_no_damage() {
        let cmds = vec![fill(0.0, 0.0, 100.0, 50.0, Color::RED)];
        let damage = diff_commands(&cmds, &cmds);
        assert!(damage.is_empty());
        assert!(commands_equal(&cmds, &cmds));
    }

    #[test]
    fn color_change_produces_damage_at_rect() {
        let old = vec![fill(10.0, 20.0, 100.0, 50.0, Color::RED)];
        let new = vec![fill(10.0, 20.0, 100.0, 50.0, Color::BLUE)];
        let damage = diff_commands(&old, &new);
        // Both old and new bounds are damaged (they're the same rect)
        assert_eq!(damage.len(), 2);
        assert_eq!(damage[0].x, 10.0);
        assert_eq!(damage[0].width, 100.0);
        assert!(!commands_equal(&old, &new));
    }

    #[test]
    fn added_command_produces_damage() {
        let old = vec![fill(0.0, 0.0, 50.0, 50.0, Color::RED)];
        let new = vec![
            fill(0.0, 0.0, 50.0, 50.0, Color::RED),
            fill(100.0, 0.0, 50.0, 50.0, Color::GREEN),
        ];
        let damage = diff_commands(&old, &new);
        assert_eq!(damage.len(), 1);
        assert_eq!(damage[0].x, 100.0);
    }

    #[test]
    fn removed_command_produces_damage() {
        let old = vec![
            fill(0.0, 0.0, 50.0, 50.0, Color::RED),
            fill(100.0, 0.0, 50.0, 50.0, Color::GREEN),
        ];
        let new = vec![fill(0.0, 0.0, 50.0, 50.0, Color::RED)];
        let damage = diff_commands(&old, &new);
        assert_eq!(damage.len(), 1);
        assert_eq!(damage[0].x, 100.0);
    }

    #[test]
    fn position_change_damages_both_old_and_new() {
        let old = vec![fill(10.0, 10.0, 50.0, 50.0, Color::RED)];
        let new = vec![fill(20.0, 20.0, 50.0, 50.0, Color::RED)];
        let damage = diff_commands(&old, &new);
        assert_eq!(damage.len(), 2);
        assert_eq!(damage[0].x, 10.0); // old position
        assert_eq!(damage[1].x, 20.0); // new position
    }

    #[test]
    fn empty_lists_no_damage() {
        let damage = diff_commands(&[], &[]);
        assert!(damage.is_empty());
        assert!(commands_equal(&[], &[]));
    }

    #[test]
    fn push_pop_clip_change_no_bounds_damage() {
        let old = vec![RenderCommand::PushClip {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            border_radius: Corners::all(0.0),
        }];
        let new = vec![RenderCommand::PushClip {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
            border_radius: Corners::all(8.0),
        }];
        let damage = diff_commands(&old, &new);
        // PushClip does have bounds, so both old and new positions are damaged
        assert_eq!(damage.len(), 2);
    }

    #[test]
    fn opacity_change_no_bounds_damage() {
        let old = vec![RenderCommand::PushOpacity { opacity: 1.0 }];
        let new = vec![RenderCommand::PushOpacity { opacity: 0.5 }];
        let damage = diff_commands(&old, &new);
        // PushOpacity has no bounds
        assert!(damage.is_empty());
    }

    #[test]
    fn single_node_background_change_localized_damage() {
        // Simulates AC-6: changing one node's background color produces damage
        // covering only that node, not the entire window
        let bg = fill(0.0, 0.0, 800.0, 600.0, Color::WHITE);
        let node_old = fill(100.0, 100.0, 200.0, 100.0, Color::RED);
        let node_new = fill(100.0, 100.0, 200.0, 100.0, Color::BLUE);

        let old = vec![bg.clone(), node_old];
        let new = vec![bg, node_new];
        let damage = diff_commands(&old, &new);
        // Only the changed node has damage (2 entries: old bounds + new bounds)
        assert_eq!(damage.len(), 2);
        for d in &damage {
            assert_eq!(d.x, 100.0);
            assert_eq!(d.y, 100.0);
            assert_eq!(d.width, 200.0);
            assert_eq!(d.height, 100.0);
        }
    }
}
