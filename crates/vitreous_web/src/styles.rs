use vitreous_style::{
    AnimatableProperty, Color, Corners, CursorIcon, Dimension, Easing, Edges, FontFamily,
    FontStyle, FontWeight, Overflow, Shadow, TextAlign, TextOverflow, Transition,
};
use vitreous_widgets::{FlexDirection, Node};

/// Format a `Color` as a CSS color string.
pub fn color_to_css(c: &Color) -> String {
    let r = (c.r * 255.0).round() as u8;
    let g = (c.g * 255.0).round() as u8;
    let b = (c.b * 255.0).round() as u8;
    if (c.a - 1.0).abs() < f32::EPSILON {
        format!("rgb({r}, {g}, {b})")
    } else {
        format!("rgba({r}, {g}, {b}, {:.2})", c.a)
    }
}

/// Format a `Dimension` as a CSS value.
fn dimension_to_css(d: &Dimension) -> Option<String> {
    match d {
        Dimension::Px(px) => Some(format!("{px}px")),
        Dimension::Percent(pct) => Some(format!("{pct}%")),
        Dimension::Auto => None,
    }
}

/// Format an `Easing` as a CSS timing function.
fn easing_to_css(e: &Easing) -> &'static str {
    match e {
        Easing::Linear => "linear",
        Easing::EaseIn => "ease-in",
        Easing::EaseOut => "ease-out",
        Easing::EaseInOut => "ease-in-out",
        // CubicBezier and Spring fall back to ease-in-out for CSS
        Easing::CubicBezier(..) | Easing::Spring { .. } => "ease-in-out",
    }
}

/// Format `CursorIcon` as a CSS cursor value.
fn cursor_to_css(c: &CursorIcon) -> &'static str {
    match c {
        CursorIcon::Default => "default",
        CursorIcon::Pointer => "pointer",
        CursorIcon::Text => "text",
        CursorIcon::Crosshair => "crosshair",
        CursorIcon::Move => "move",
        CursorIcon::NotAllowed => "not-allowed",
        CursorIcon::Grab => "grab",
        CursorIcon::Grabbing => "grabbing",
        CursorIcon::ColResize => "col-resize",
        CursorIcon::RowResize => "row-resize",
        CursorIcon::NResize => "n-resize",
        CursorIcon::EResize => "e-resize",
        CursorIcon::SResize => "s-resize",
        CursorIcon::WResize => "w-resize",
        CursorIcon::NeResize => "ne-resize",
        CursorIcon::NwResize => "nw-resize",
        CursorIcon::SeResize => "se-resize",
        CursorIcon::SwResize => "sw-resize",
        CursorIcon::EwResize => "ew-resize",
        CursorIcon::NsResize => "ns-resize",
        CursorIcon::NeswResize => "nesw-resize",
        CursorIcon::NwseResize => "nwse-resize",
        CursorIcon::Wait => "wait",
        CursorIcon::Progress => "progress",
        CursorIcon::Help => "help",
        CursorIcon::ZoomIn => "zoom-in",
        CursorIcon::ZoomOut => "zoom-out",
        CursorIcon::None => "none",
    }
}

/// Format a `FontWeight` as a CSS value.
fn font_weight_to_css(w: &FontWeight) -> &'static str {
    match w {
        FontWeight::Thin => "100",
        FontWeight::ExtraLight => "200",
        FontWeight::Light => "300",
        FontWeight::Regular => "400",
        FontWeight::Medium => "500",
        FontWeight::SemiBold => "600",
        FontWeight::Bold => "700",
        FontWeight::ExtraBold => "800",
        FontWeight::Black => "900",
    }
}

/// Format an `AnimatableProperty` as its CSS property name.
fn animatable_property_to_css(p: &AnimatableProperty) -> &'static str {
    match p {
        AnimatableProperty::Opacity => "opacity",
        AnimatableProperty::BackgroundColor => "background-color",
        AnimatableProperty::ForegroundColor => "color",
        AnimatableProperty::BorderColor => "border-color",
        AnimatableProperty::Width => "width",
        AnimatableProperty::Height => "height",
        AnimatableProperty::PaddingTop => "padding-top",
        AnimatableProperty::PaddingRight => "padding-right",
        AnimatableProperty::PaddingBottom => "padding-bottom",
        AnimatableProperty::PaddingLeft => "padding-left",
        AnimatableProperty::MarginTop => "margin-top",
        AnimatableProperty::MarginRight => "margin-right",
        AnimatableProperty::MarginBottom => "margin-bottom",
        AnimatableProperty::MarginLeft => "margin-left",
        AnimatableProperty::BorderRadius => "border-radius",
        AnimatableProperty::BorderWidth => "border-width",
        AnimatableProperty::FontSize => "font-size",
        AnimatableProperty::LetterSpacing => "letter-spacing",
        AnimatableProperty::LineHeight => "line-height",
        AnimatableProperty::Gap => "gap",
        AnimatableProperty::Transform => "transform",
        AnimatableProperty::BoxShadow => "box-shadow",
    }
}

/// Format a `Transition` as a CSS transition shorthand fragment.
fn transition_to_css(t: &Transition) -> String {
    let prop = animatable_property_to_css(&t.property);
    let dur_ms = t.duration.as_millis();
    let easing = easing_to_css(&t.easing);
    let delay_ms = t.delay.as_millis();
    if delay_ms > 0 {
        format!("{prop} {dur_ms}ms {easing} {delay_ms}ms")
    } else {
        format!("{prop} {dur_ms}ms {easing}")
    }
}

/// Format a `Shadow` as a CSS box-shadow value.
fn shadow_to_css(s: &Shadow) -> String {
    format!(
        "{}px {}px {}px {}px {}",
        s.offset_x,
        s.offset_y,
        s.blur_radius,
        s.spread_radius,
        color_to_css(&s.color)
    )
}

/// Apply all style properties and flex layout from a `Node` to a
/// `web_sys::HtmlElement`'s inline style.
pub fn apply_styles(element: &web_sys::HtmlElement, node: &Node) {
    let style = &node.style;
    let css = element.style();

    // Display: always flex for layout
    let _ = css.set_property("display", "flex");

    // Flex direction
    let dir = match node.flex_direction {
        FlexDirection::Row => "row",
        FlexDirection::Column => "column",
    };
    let _ = css.set_property("flex-direction", dir);

    // Flex item properties
    if node.flex_grow != 0.0 {
        let _ = css.set_property("flex-grow", &node.flex_grow.to_string());
    }
    if (node.flex_shrink - 1.0).abs() > f32::EPSILON {
        let _ = css.set_property("flex-shrink", &node.flex_shrink.to_string());
    }
    if let Some(basis) = dimension_to_css(&node.flex_basis) {
        let _ = css.set_property("flex-basis", &basis);
    }
    if let Some(align) = &node.align_self {
        let val = match align {
            vitreous_widgets::AlignSelf::Start => "start",
            vitreous_widgets::AlignSelf::End => "end",
            vitreous_widgets::AlignSelf::FlexStart => "flex-start",
            vitreous_widgets::AlignSelf::FlexEnd => "flex-end",
            vitreous_widgets::AlignSelf::Center => "center",
            vitreous_widgets::AlignSelf::Baseline => "baseline",
            vitreous_widgets::AlignSelf::Stretch => "stretch",
        };
        let _ = css.set_property("align-self", val);
    }
    if node.gap > 0.0 {
        let _ = css.set_property("gap", &format!("{}px", node.gap));
    }
    if let Some(ratio) = node.aspect_ratio {
        let _ = css.set_property("aspect-ratio", &ratio.to_string());
    }

    // Dimensions
    if let Some(w) = dimension_to_css(&style.width) {
        let _ = css.set_property("width", &w);
    }
    if let Some(h) = dimension_to_css(&style.height) {
        let _ = css.set_property("height", &h);
    }
    if let Some(v) = dimension_to_css(&style.min_width) {
        let _ = css.set_property("min-width", &v);
    }
    if let Some(v) = dimension_to_css(&style.min_height) {
        let _ = css.set_property("min-height", &v);
    }
    if let Some(v) = dimension_to_css(&style.max_width) {
        let _ = css.set_property("max-width", &v);
    }
    if let Some(v) = dimension_to_css(&style.max_height) {
        let _ = css.set_property("max-height", &v);
    }

    // Padding
    apply_edges(&css, "padding", &style.padding);

    // Margin
    apply_edges(&css, "margin", &style.margin);

    // Background
    if let Some(bg) = &style.background {
        let _ = css.set_property("background-color", &color_to_css(bg));
    }

    // Foreground (text color)
    if let Some(fg) = &style.foreground {
        let _ = css.set_property("color", &color_to_css(fg));
    }

    // Border
    if let Some(bc) = &style.border_color {
        let bw = &style.border_width;
        if bw.top > 0.0 || bw.right > 0.0 || bw.bottom > 0.0 || bw.left > 0.0 {
            let _ = css.set_property("border-style", "solid");
            let _ = css.set_property(
                "border-width",
                &format!("{}px {}px {}px {}px", bw.top, bw.right, bw.bottom, bw.left),
            );
            let _ = css.set_property("border-color", &color_to_css(bc));
        }
    }

    // Border radius
    apply_corners(&css, &style.border_radius);

    // Opacity
    if (style.opacity - 1.0).abs() > f32::EPSILON {
        let _ = css.set_property("opacity", &style.opacity.to_string());
    }

    // Shadow
    if let Some(shadow) = &style.shadow {
        let _ = css.set_property("box-shadow", &shadow_to_css(shadow));
    }

    // Font size
    if let Some(fs) = style.font_size {
        let _ = css.set_property("font-size", &format!("{fs}px"));
    }

    // Font weight
    if let Some(fw) = &style.font_weight {
        let _ = css.set_property("font-weight", font_weight_to_css(fw));
    }

    // Font family
    if let Some(ff) = &style.font_family {
        let val = match ff {
            FontFamily::SansSerif => "sans-serif",
            FontFamily::Serif => "serif",
            FontFamily::Monospace => "monospace",
            FontFamily::Named(name) => name.as_str(),
        };
        let _ = css.set_property("font-family", val);
    }

    // Font style
    if let Some(fs) = &style.font_style {
        let val = match fs {
            FontStyle::Normal => "normal",
            FontStyle::Italic => "italic",
        };
        let _ = css.set_property("font-style", val);
    }

    // Text align
    if let Some(ta) = &style.text_align {
        let val = match ta {
            TextAlign::Start => "start",
            TextAlign::Center => "center",
            TextAlign::End => "end",
            TextAlign::Justify => "justify",
        };
        let _ = css.set_property("text-align", val);
    }

    // Text overflow
    if let Some(to) = &style.text_overflow {
        match to {
            TextOverflow::Clip => {
                let _ = css.set_property("text-overflow", "clip");
            }
            TextOverflow::Ellipsis => {
                let _ = css.set_property("text-overflow", "ellipsis");
                let _ = css.set_property("white-space", "nowrap");
            }
        }
    }

    // Line height
    if let Some(lh) = style.line_height {
        let _ = css.set_property("line-height", &lh.to_string());
    }

    // Letter spacing
    if let Some(ls) = style.letter_spacing {
        let _ = css.set_property("letter-spacing", &format!("{ls}px"));
    }

    // Cursor
    if let Some(cursor) = &style.cursor {
        let _ = css.set_property("cursor", cursor_to_css(cursor));
    }

    // Overflow
    match style.overflow {
        Overflow::Visible => {}
        Overflow::Hidden => {
            let _ = css.set_property("overflow", "hidden");
        }
        Overflow::Scroll => {
            let _ = css.set_property("overflow", "auto");
        }
    }

    // Clip content
    if style.clip_content {
        let _ = css.set_property("overflow", "hidden");
    }

    // Transitions
    if !style.transitions.is_empty() {
        let val: String = style
            .transitions
            .iter()
            .map(transition_to_css)
            .collect::<Vec<_>>()
            .join(", ");
        let _ = css.set_property("transition", &val);
    }

    // Box sizing for consistent layout
    let _ = css.set_property("box-sizing", "border-box");
}

fn apply_edges(css: &web_sys::CssStyleDeclaration, prop: &str, edges: &Edges) {
    if edges.top != 0.0 || edges.right != 0.0 || edges.bottom != 0.0 || edges.left != 0.0 {
        let _ = css.set_property(
            prop,
            &format!(
                "{}px {}px {}px {}px",
                edges.top, edges.right, edges.bottom, edges.left
            ),
        );
    }
}

fn apply_corners(css: &web_sys::CssStyleDeclaration, corners: &Corners) {
    if corners.top_left != 0.0
        || corners.top_right != 0.0
        || corners.bottom_right != 0.0
        || corners.bottom_left != 0.0
    {
        let _ = css.set_property(
            "border-radius",
            &format!(
                "{}px {}px {}px {}px",
                corners.top_left, corners.top_right, corners.bottom_right, corners.bottom_left
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_opaque() {
        let c = Color::rgb(255, 0, 0);
        assert_eq!(color_to_css(&c), "rgb(255, 0, 0)");
    }

    #[test]
    fn color_with_alpha() {
        let c = Color::rgba(255, 0, 0, 0.50);
        assert_eq!(color_to_css(&c), "rgba(255, 0, 0, 0.50)");
    }

    #[test]
    fn dimension_px() {
        assert_eq!(dimension_to_css(&Dimension::Px(16.0)), Some("16px".into()));
    }

    #[test]
    fn dimension_percent() {
        assert_eq!(
            dimension_to_css(&Dimension::Percent(50.0)),
            Some("50%".into())
        );
    }

    #[test]
    fn dimension_auto() {
        assert_eq!(dimension_to_css(&Dimension::Auto), None);
    }

    #[test]
    fn shadow_css() {
        let s = Shadow::new(2.0, 4.0, 8.0, 0.0, Color::BLACK);
        assert_eq!(shadow_to_css(&s), "2px 4px 8px 0px rgb(0, 0, 0)");
    }

    #[test]
    fn transition_css() {
        let t = Transition::new(
            AnimatableProperty::Opacity,
            core::time::Duration::from_millis(300),
        );
        assert_eq!(transition_to_css(&t), "opacity 300ms ease-in-out");
    }

    #[test]
    fn transition_with_delay() {
        let t = Transition::new(
            AnimatableProperty::BackgroundColor,
            core::time::Duration::from_millis(200),
        )
        .with_delay(core::time::Duration::from_millis(100));
        assert_eq!(
            transition_to_css(&t),
            "background-color 200ms ease-in-out 100ms"
        );
    }
}
