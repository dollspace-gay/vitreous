use std::collections::HashMap;

use taffy::prelude::*;

use crate::tree::{
    LayoutInput, LayoutOutput, MeasureConstraint, NodeId, NodeLayout, Rect, layout_style_to_taffy,
};

/// Available space for layout computation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvailableSpace {
    pub width: f32,
    pub height: f32,
}

impl AvailableSpace {
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// Computes layout for a tree of nodes.
///
/// Builds a `TaffyTree` from the given `LayoutInput` nodes, runs flexbox
/// layout, and extracts resolved positions/sizes into a `LayoutOutput`.
///
/// The taffy tree is rebuilt from scratch each call (simple v1 approach).
pub fn compute_layout(
    nodes: &[LayoutInput],
    root_id: NodeId,
    available: AvailableSpace,
) -> LayoutOutput {
    if nodes.is_empty() {
        return LayoutOutput::new();
    }

    // Map our NodeId -> index into the nodes slice
    let node_index: HashMap<NodeId, usize> =
        nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();

    // Build taffy tree
    let mut tree = TaffyTree::<NodeId>::with_capacity(nodes.len());

    // First pass: create all taffy nodes (leaves first, then set children)
    let mut taffy_ids: HashMap<NodeId, taffy::NodeId> = HashMap::with_capacity(nodes.len());

    for input in nodes {
        let taffy_style = layout_style_to_taffy(&input.style);
        let taffy_node = tree
            .new_leaf_with_context(taffy_style, input.id)
            .expect("taffy node creation should not fail");
        taffy_ids.insert(input.id, taffy_node);
    }

    // Second pass: wire up children
    for input in nodes {
        if !input.children.is_empty() {
            let parent_taffy = taffy_ids[&input.id];
            let children_taffy: Vec<taffy::NodeId> = input
                .children
                .iter()
                .filter_map(|child_id| taffy_ids.get(child_id).copied())
                .collect();
            tree.set_children(parent_taffy, &children_taffy)
                .expect("setting children should not fail");
        }
    }

    // Run layout with measure function support
    let root_taffy = match taffy_ids.get(&root_id) {
        Some(&id) => id,
        None => return LayoutOutput::new(),
    };

    let space = taffy::Size {
        width: taffy::AvailableSpace::Definite(available.width),
        height: taffy::AvailableSpace::Definite(available.height),
    };

    tree.compute_layout_with_measure(
        root_taffy,
        space,
        |known_size: taffy::Size<Option<f32>>,
         _available: taffy::Size<taffy::AvailableSpace>,
         _node_taffy_id: taffy::NodeId,
         context: Option<&mut NodeId>,
         _style: &taffy::Style| {
            // If size is fully known, just return it
            if let (Some(w), Some(h)) = (known_size.width, known_size.height) {
                return taffy::Size {
                    width: w,
                    height: h,
                };
            }

            // Look up our node's measure function
            if let Some(our_id) = context
                && let Some(&idx) = node_index.get(our_id)
                && let Some(ref measure_fn) = nodes[idx].measure
            {
                let constraint = MeasureConstraint {
                    max_width: known_size.width.or(match _available.width {
                        taffy::AvailableSpace::Definite(w) => Some(w),
                        _ => None,
                    }),
                    max_height: known_size.height.or(match _available.height {
                        taffy::AvailableSpace::Definite(h) => Some(h),
                        _ => None,
                    }),
                };
                let size = measure_fn(constraint);
                return taffy::Size {
                    width: known_size.width.unwrap_or(size.width),
                    height: known_size.height.unwrap_or(size.height),
                };
            }

            // No measure function — return zero
            taffy::Size {
                width: known_size.width.unwrap_or(0.0),
                height: known_size.height.unwrap_or(0.0),
            }
        },
    )
    .expect("layout computation should not fail");

    // Extract results
    let mut output = LayoutOutput::new();
    collect_layouts(&tree, root_taffy, 0.0, 0.0, &mut output);
    output
}

/// Recursively collects layout for each node, accumulating absolute position.
fn collect_layouts(
    tree: &TaffyTree<NodeId>,
    taffy_node: taffy::NodeId,
    parent_x: f32,
    parent_y: f32,
    output: &mut LayoutOutput,
) {
    let layout = tree
        .layout(taffy_node)
        .expect("layout should be available after computation");

    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;

    if let Some(our_id) = tree.get_node_context(taffy_node) {
        let content_width = layout.content_size.width;
        let content_height = layout.content_size.height;

        output.nodes.insert(
            *our_id,
            NodeLayout {
                x: abs_x,
                y: abs_y,
                width: layout.size.width,
                height: layout.size.height,
                content_width,
                content_height,
                padding: Rect::new(
                    layout.padding.top,
                    layout.padding.right,
                    layout.padding.bottom,
                    layout.padding.left,
                ),
                border: Rect::new(
                    layout.border.top,
                    layout.border.right,
                    layout.border.bottom,
                    layout.border.left,
                ),
                margin: Rect::new(
                    layout.margin.top,
                    layout.margin.right,
                    layout.margin.bottom,
                    layout.margin.left,
                ),
            },
        );
    }

    // Recurse into children
    let child_count = tree.child_count(taffy_node);
    for i in 0..child_count {
        if let Ok(child) = tree.child_at_index(taffy_node, i) {
            collect_layouts(tree, child, abs_x, abs_y, output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::{
        AlignItems, Dimension, FlexDirection, JustifyContent, LayoutStyle, MeasureConstraint,
        NodeId, Overflow, Size,
    };

    fn px(v: f32) -> Dimension {
        Dimension::Px(v)
    }

    fn pct(v: f32) -> Dimension {
        Dimension::Percent(v)
    }

    // AC-1: Column layout with two children
    #[test]
    fn test_column_layout_two_children() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(200.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1), NodeId(2)]);

        let child0 = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                height: px(50.0),
                ..Default::default()
            },
        );

        let child1 = LayoutInput::new(
            NodeId(2),
            LayoutStyle {
                height: px(30.0),
                ..Default::default()
            },
        );

        let output = compute_layout(
            &[root, child0, child1],
            NodeId(0),
            AvailableSpace::new(200.0, 1000.0),
        );

        let c0 = output.get(NodeId(1)).expect("child 0 layout");
        assert_eq!(c0.y, 0.0);
        assert_eq!(c0.height, 50.0);

        let c1 = output.get(NodeId(2)).expect("child 1 layout");
        assert_eq!(c1.y, 50.0);
        assert_eq!(c1.height, 30.0);
    }

    // AC-2: Row layout with flex_grow
    #[test]
    fn test_row_layout_flex_grow() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(300.0),
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1), NodeId(2)]);

        let child0 = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                flex_grow: 1.0,
                ..Default::default()
            },
        );

        let child1 = LayoutInput::new(
            NodeId(2),
            LayoutStyle {
                flex_grow: 2.0,
                ..Default::default()
            },
        );

        let output = compute_layout(
            &[root, child0, child1],
            NodeId(0),
            AvailableSpace::new(300.0, 1000.0),
        );

        let c0 = output.get(NodeId(1)).expect("child 0 layout");
        assert_eq!(c0.width, 100.0);

        let c1 = output.get(NodeId(2)).expect("child 1 layout");
        assert_eq!(c1.width, 200.0);
    }

    // AC-3: Text wrapping with MeasureFn
    #[test]
    fn test_text_wrapping_measure_fn() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(100.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1)]);

        let text_leaf = LayoutInput::new(NodeId(1), LayoutStyle::default()).with_measure(Box::new(
            |constraint: MeasureConstraint| {
                // Text that is 200px wide unconstrained, 20px per line
                let available_width = constraint.max_width.unwrap_or(f32::INFINITY);
                if available_width >= 200.0 {
                    Size::new(200.0, 20.0)
                } else {
                    // Wraps to 2 lines
                    let lines = (200.0 / available_width).ceil();
                    Size::new(available_width, lines * 20.0)
                }
            },
        ));

        let output = compute_layout(
            &[root, text_leaf],
            NodeId(0),
            AvailableSpace::new(100.0, 1000.0),
        );

        let text = output.get(NodeId(1)).expect("text node layout");
        assert_eq!(text.width, 100.0);
        assert_eq!(text.height, 40.0);
    }

    // AC-4: Percentage dimensions
    #[test]
    fn test_percentage_dimensions() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(400.0),
                height: px(300.0),
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1)]);

        let child = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                width: pct(50.0),
                height: pct(25.0),
                ..Default::default()
            },
        );

        let output = compute_layout(&[root, child], NodeId(0), AvailableSpace::new(400.0, 300.0));

        let c = output.get(NodeId(1)).expect("child layout");
        assert_eq!(c.width, 200.0);
        assert_eq!(c.height, 75.0);
    }

    // AC-5: JustifyContent::Center
    #[test]
    fn test_justify_content_center() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(300.0),
                height: px(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: Some(JustifyContent::Center),
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1)]);

        let child = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                width: px(100.0),
                height: px(50.0),
                ..Default::default()
            },
        );

        let output = compute_layout(&[root, child], NodeId(0), AvailableSpace::new(300.0, 100.0));

        let c = output.get(NodeId(1)).expect("child layout");
        assert_eq!(c.x, 100.0);
    }

    // AC-6: AlignItems::Center
    #[test]
    fn test_align_items_center() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(200.0),
                height: px(200.0),
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1)]);

        let child = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                width: px(100.0),
                height: px(50.0),
                ..Default::default()
            },
        );

        let output = compute_layout(&[root, child], NodeId(0), AvailableSpace::new(200.0, 200.0));

        let c = output.get(NodeId(1)).expect("child layout");
        assert_eq!(c.y, 75.0);
    }

    // AC-7: Gap between children
    #[test]
    fn test_gap_between_children() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(200.0),
                flex_direction: FlexDirection::Column,
                gap: Size::new(0.0, 8.0),
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1), NodeId(2), NodeId(3)]);

        let child0 = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                height: px(20.0),
                ..Default::default()
            },
        );
        let child1 = LayoutInput::new(
            NodeId(2),
            LayoutStyle {
                height: px(30.0),
                ..Default::default()
            },
        );
        let child2 = LayoutInput::new(
            NodeId(3),
            LayoutStyle {
                height: px(25.0),
                ..Default::default()
            },
        );

        let output = compute_layout(
            &[root, child0, child1, child2],
            NodeId(0),
            AvailableSpace::new(200.0, 1000.0),
        );

        let c0 = output.get(NodeId(1)).expect("child 0");
        let c1 = output.get(NodeId(2)).expect("child 1");
        let c2 = output.get(NodeId(3)).expect("child 2");

        assert_eq!(c0.y, 0.0);
        assert_eq!(c1.y, 20.0 + 8.0); // h0 + gap
        assert_eq!(c2.y, 20.0 + 8.0 + 30.0 + 8.0); // h0 + gap + h1 + gap
    }

    // AC-9: Scroll overflow content size
    #[test]
    fn test_scroll_overflow_content_size() {
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(200.0),
                height: px(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::Scroll,
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1), NodeId(2), NodeId(3)]);

        let child0 = LayoutInput::new(
            NodeId(1),
            LayoutStyle {
                height: px(100.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
        );
        let child1 = LayoutInput::new(
            NodeId(2),
            LayoutStyle {
                height: px(100.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
        );
        let child2 = LayoutInput::new(
            NodeId(3),
            LayoutStyle {
                height: px(100.0),
                flex_shrink: 0.0,
                ..Default::default()
            },
        );

        let output = compute_layout(
            &[root, child0, child1, child2],
            NodeId(0),
            AvailableSpace::new(200.0, 100.0),
        );

        let container = output.get(NodeId(0)).expect("root layout");
        assert_eq!(container.height, 100.0);
        assert_eq!(container.content_height, 300.0);
    }

    // AC-10: Property test — basic invariants
    #[test]
    fn test_layout_invariants_random_tree() {
        // Simplified property test: build a tree with random-ish flex properties,
        // verify non-negative sizes and that flex space is distributed
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(500.0),
                height: px(400.0),
                flex_direction: FlexDirection::Row,
                ..Default::default()
            },
        )
        .with_children(vec![NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);

        let children: Vec<LayoutInput> = (1..=4)
            .map(|i| {
                LayoutInput::new(
                    NodeId(i),
                    LayoutStyle {
                        flex_grow: i as f32,
                        ..Default::default()
                    },
                )
            })
            .collect();

        let mut all_nodes = vec![root];
        all_nodes.extend(children);

        let output = compute_layout(&all_nodes, NodeId(0), AvailableSpace::new(500.0, 400.0));

        let mut total_width = 0.0;
        for i in 1..=4 {
            let layout = output.get(NodeId(i)).expect("child layout");
            assert!(layout.width >= 0.0, "width must be non-negative");
            assert!(layout.height >= 0.0, "height must be non-negative");
            total_width += layout.width;
        }

        // Flex space should be fully distributed (within rounding tolerance)
        assert!(
            (total_width - 500.0).abs() < 1.0,
            "total width {total_width} should equal container width 500"
        );
    }

    // AC-11: Benchmark — layout of 1,000 nodes completes quickly
    #[test]
    fn test_layout_1000_nodes_performance() {
        let mut nodes = Vec::with_capacity(1001);

        // Root with 10 rows of 100 children each
        let root = LayoutInput::new(
            NodeId(0),
            LayoutStyle {
                width: px(1000.0),
                height: px(800.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        )
        .with_children((1..=10).map(|i| NodeId(i)).collect());
        nodes.push(root);

        // 10 row containers
        for row in 1..=10u32 {
            let row_children: Vec<NodeId> = (0..100)
                .map(|col| NodeId(100 + (row - 1) * 100 + col))
                .collect();
            let row_node = LayoutInput::new(
                NodeId(row),
                LayoutStyle {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    ..Default::default()
                },
            )
            .with_children(row_children);
            nodes.push(row_node);
        }

        // 1000 leaf children
        for i in 0..1000u32 {
            let leaf = LayoutInput::new(
                NodeId(100 + i),
                LayoutStyle {
                    flex_grow: 1.0,
                    ..Default::default()
                },
            );
            nodes.push(leaf);
        }

        let start = std::time::Instant::now();
        let output = compute_layout(&nodes, NodeId(0), AvailableSpace::new(1000.0, 800.0));
        let elapsed = start.elapsed();

        assert!(
            output.len() >= 1000,
            "should have at least 1000 nodes laid out"
        );
        assert!(
            elapsed.as_millis() < 100, // generous limit for CI; design says <1ms
            "layout took {}ms, expected < 100ms",
            elapsed.as_millis()
        );
    }
}
