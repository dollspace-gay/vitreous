// ═══════════════════════════════════════════════════════════════════════════
// Kitchen Sink — exercises every public API in the vitreous framework
// ═══════════════════════════════════════════════════════════════════════════

use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use vitreous::{
    // ── Reactive ──────────────────────────────────────────────────────────
    Callback, Memo, ReadSignal, Resource, Signal,
    batch, create_effect, create_memo, create_resource, create_scope, create_signal,
    provide_context, set_executor, try_use_context, use_context,
    // ── Widgets ──────────────────────────────────────────────────────────
    AlignSelf, FlexDirection, ImageSource, Key, Node, NodeKind, TextContent,
    button, checkbox, container, divider, for_each, h_stack, image, overlay,
    provider, router, navigate, Route, scroll_view, select, show, show_else,
    slider, spacer, text, text_input, toggle, tooltip, use_route, v_stack,
    virtual_list, z_stack,
    // ── Style ────────────────────────────────────────────────────────────
    Animation, AnimatableProperty, AnimatableValue, AnimationDirection,
    AnimationIterations, Color, Corners, CursorIcon, Dimension, Easing, Edges,
    FontFamily, FontStyle, FontWeight, Keyframe, Overflow, Shadow, Style,
    TextAlign, TextOverflow, Theme, Transition, pct,
    // ── Events ───────────────────────────────────────────────────────────
    DragConfig, DropData, DropEvent, EventHandlers, KeyCode, KeyEvent,
    Modifiers, MouseButton, MouseEvent, ScrollEvent,
    // ── A11y ─────────────────────────────────────────────────────────────
    AccessibilityInfo, AccessibilityState, LivePoliteness, Role,
    // ── App ──────────────────────────────────────────────────────────────
    App, theme,
};
use vitreous_hot_reload::{HotReloadClient, ServerMessage, DEFAULT_PORT};

// ═══════════════════════════════════════════════════════════════════════════
// Main — App builder with every method
// ═══════════════════════════════════════════════════════════════════════════

fn main() {
    App::new()
        .title("Vitreous Kitchen Sink")
        .size(1200, 900)
        .min_size(800, 600)
        .max_size(1920, 1080)
        .resizable(true)
        .theme(Theme::dark())
        .run(root);
}

// ═══════════════════════════════════════════════════════════════════════════
// Root — router + nav bar
// ═══════════════════════════════════════════════════════════════════════════

fn root() -> Node {
    let t = theme();
    set_executor(|_fut| {});

    v_stack((
        nav_bar(),
        router(vec![
            Route::new("/", home_page),
            Route::new("/reactive", reactive_page),
            Route::new("/widgets", widgets_page),
            Route::new("/style", style_page),
            Route::new("/events", events_page),
            Route::new("/a11y", a11y_page),
            Route::new("/layout", layout_page),
            Route::new("/vlist", virtual_list_page),
        ]),
    ))
    .background(t.background)
}

fn nav_bar() -> Node {
    let t = theme();
    let route = use_route();
    h_stack((
        nav_btn("Home", "/", &route),
        nav_btn("Reactive", "/reactive", &route),
        nav_btn("Widgets", "/widgets", &route),
        nav_btn("Style", "/style", &route),
        nav_btn("Events", "/events", &route),
        nav_btn("A11y", "/a11y", &route),
        nav_btn("Layout", "/layout", &route),
        nav_btn("VList", "/vlist", &route),
        spacer(),
        hot_reload_indicator(),
    ))
    .gap(t.spacing_xs)
    .padding(t.spacing_sm)
    .background(t.surface)
    .role(Role::Toolbar)
    .label("Main navigation")
}

fn nav_btn(label: &str, path: &str, current: &str) -> Node {
    let t = theme();
    let is_active = current == path;
    let target = path.to_owned();
    button(label)
        .on_click(move || navigate(target.clone()))
        .padding(t.spacing_xs)
        .background(if is_active { t.primary } else { t.surface })
        .border_radius(t.radius_sm)
        .cursor(CursorIcon::Pointer)
        .key(path)
}

fn hot_reload_indicator() -> Node {
    let client = HotReloadClient::connect(
        &format!("ws://127.0.0.1:{DEFAULT_PORT}"),
        "kitchen-sink",
    );
    let connected = client.as_ref().ok().is_some_and(|c| c.is_connected());
    if let Ok(ref c) = client {
        for msg in c.drain() {
            match msg {
                ServerMessage::FileChanged(_)
                | ServerMessage::BuildStarted
                | ServerMessage::BuildComplete
                | ServerMessage::BuildFailed { .. }
                | ServerMessage::Shutdown => {}
            }
        }
    }
    text(if connected { "[HR: on]" } else { "[HR: off]" })
        .font_size(11.0)
        .foreground(if connected { Color::GREEN } else { Color::GRAY })
}

// ═══════════════════════════════════════════════════════════════════════════
// Home
// ═══════════════════════════════════════════════════════════════════════════

fn home_page() -> Node {
    let t = theme();
    v_stack((
        text("Vitreous Kitchen Sink").font_size(t.font_size_3xl).font_weight(FontWeight::Bold),
        text("Exercises every public API across all crates.").foreground(t.text_secondary),
        divider(),
        text("Use the nav bar to explore each feature area.").font_size(t.font_size_sm),
    ))
    .gap(t.spacing_md)
    .padding(t.spacing_xl)
}

// ═══════════════════════════════════════════════════════════════════════════
// Reactive — signals, memos, effects, batch, context, resource, scope
// ═══════════════════════════════════════════════════════════════════════════

fn reactive_page() -> Node {
    let t = theme();

    // ── Signals of various types ─────────────────────────────────────────
    let count = create_signal(0i32);
    let name = create_signal(String::from("World"));
    let flag = create_signal(false);

    // ── Signal methods ───────────────────────────────────────────────────
    let _initial = count.get();
    let _untracked = count.get_untracked();
    let read_only: ReadSignal<i32> = count.read_only();
    let _from_ro = read_only.get();
    let _from_ro_ut = read_only.get_untracked();
    count.update(|n| *n += 0);

    // ── Memo ─────────────────────────────────────────────────────────────
    let doubled: Memo<i32> = create_memo(move || count.get() * 2);
    let _mv = doubled.get();
    let _mu = doubled.get_untracked();
    let greeting = create_memo(move || format!("Hello, {}!", name.get()));

    // ── Effect ───────────────────────────────────────────────────────────
    let effect_log = create_signal(String::new());
    create_effect(move || {
        effect_log.set(format!("Effect saw count = {}", count.get()));
    });

    // ── Batch ────────────────────────────────────────────────────────────
    let batch_result = create_signal(0i32);
    batch(move || {
        count.set(count.get());
        batch_result.set(count.get());
    });

    // ── Context ──────────────────────────────────────────────────────────
    let _maybe_theme: Option<Theme> = try_use_context::<Theme>();

    // ── Scope ────────────────────────────────────────────────────────────
    let scope_result = create_signal(String::new());
    {
        let _scope = create_scope(move || {
            provide_context(42u32);
            let val = use_context::<u32>();
            scope_result.set(format!("Scope context: {val}"));
        });
    }

    // ── Resource ─────────────────────────────────────────────────────────
    let fetch_trigger = create_signal(());
    type Fut = Pin<Box<dyn Future<Output = Result<String, Box<dyn std::error::Error>>> + 'static>>;
    let resource: Resource<(), String> = create_resource(
        move || fetch_trigger.get(),
        |_| -> Fut { Box::pin(async { Ok("fetched".into()) }) },
    );
    let _loading = resource.loading();
    let _data = resource.data();
    let _error = resource.error();

    // ── Callback ─────────────────────────────────────────────────────────
    let cb = Callback::new(|x: i32| x * 3);
    let _cb_result = cb.call(7);

    // ── Signals used for UI that aren't otherwise exercised ──────────────
    let _ = flag.get(); // Signal<bool> exercised

    scroll_view(v_stack((
        section_title("Reactivity"),
        h_stack((
            button("- 1").on_click(move || count.set(count.get() - 1)),
            text(move || format!("count = {}", count.get())).font_size(t.font_size_lg),
            button("+ 1").on_click(move || count.set(count.get() + 1)),
        )).gap(t.spacing_sm),
        text(move || format!("doubled = {}", doubled.get())),
        text(move || greeting.get()),
        text(move || effect_log.get()).foreground(t.text_secondary),
        text(move || scope_result.get()),
        text(move || format!("batch_result = {}", batch_result.get())),
        text_input(name, move |val| name.set(val)),
        show_else(
            !resource.loading(),
            move || text(resource.data().unwrap_or_else(|| "No data".into())),
            || text("Loading..."),
        ),
        text(move || format!("read_only = {}", read_only.get())),
    )).gap(t.spacing_sm).padding(t.spacing_lg))
}

// ═══════════════════════════════════════════════════════════════════════════
// Widgets — every widget function and modifier
// ═══════════════════════════════════════════════════════════════════════════

fn widgets_page() -> Node {
    let t = theme();
    let check_val = create_signal(false);
    let toggle_val = create_signal(true);
    let slider_val = create_signal(50.0f64);
    let input_val = create_signal("edit me".to_owned());
    let select_idx = create_signal(1usize);
    let show_overlay = create_signal(false);

    scroll_view(v_stack((
        section_title("Widget Gallery"),
        widgets_primitives(t.clone(), check_val, toggle_val, slider_val, input_val, select_idx),
        divider(),
        widgets_containers(t.clone(), show_overlay, toggle_val, check_val),
        divider(),
        widgets_composition(t.clone()),
    )).gap(t.spacing_sm).padding(t.spacing_lg))
}

fn widgets_primitives(
    t: Theme,
    check_val: Signal<bool>,
    toggle_val: Signal<bool>,
    slider_val: Signal<f64>,
    input_val: Signal<String>,
    select_idx: Signal<usize>,
) -> Node {
    v_stack((
        // text: static (&str), static (String), dynamic (Fn)
        text("Static text (&str — IntoTextContent)"),
        text(String::from("Static text (String — IntoTextContent)")),
        text(move || format!("Dynamic text: toggle={}", toggle_val.get())),
        // button
        button("Click me")
            .on_click(|| {})
            .padding(t.spacing_sm)
            .background(t.primary)
            .border_radius(t.radius_md)
            .font_weight(FontWeight::SemiBold),
        // text_input
        text_input(input_val, move |v| input_val.set(v))
            .padding(t.spacing_sm)
            .border(1.0, t.border)
            .border_radius(t.radius_sm),
        // checkbox
        h_stack((
            checkbox(check_val).on_click(move || check_val.set(!check_val.get())),
            text(move || format!("Checked: {}", check_val.get())),
        )).gap(t.spacing_sm),
        // toggle
        h_stack((
            toggle(toggle_val).on_click(move || toggle_val.set(!toggle_val.get())),
            text(move || format!("On: {}", toggle_val.get())),
        )).gap(t.spacing_sm),
        // slider
        h_stack((
            slider(slider_val, 0.0, 100.0),
            text(move || format!("{:.0}", slider_val.get())),
        )).gap(t.spacing_sm),
        // select
        h_stack((
            select(vec!["Option A".into(), "Option B".into(), "Option C".into()], select_idx),
            text(move || format!("idx: {}", select_idx.get())),
        )).gap(t.spacing_sm),
        // image
        image(ImageSource::Path("placeholder.png".into()))
            .width(Dimension::Px(64.0))
            .height(Dimension::Px(64.0))
            .label("Placeholder image")
            .border_radius(t.radius_sm),
        // spacer + divider
        h_stack((text("Left"), spacer(), text("Right"))),
    ))
    .gap(t.spacing_sm)
}

fn widgets_containers(
    t: Theme,
    show_overlay: Signal<bool>,
    toggle_val: Signal<bool>,
    check_val: Signal<bool>,
) -> Node {
    v_stack((
        // container
        container(text("Inside a container"))
            .padding(t.spacing_md).background(t.surface).border_radius(t.radius_md),
        // tooltip
        tooltip(button("Hover me"), text("Tooltip!")),
        // overlay
        button("Toggle Overlay")
            .on_click(move || show_overlay.set(!show_overlay.get())),
        show(show_overlay.get(), move || {
            overlay(v_stack((
                text("Modal").font_size(t.font_size_xl).font_weight(FontWeight::Bold),
                button("Close").on_click(move || show_overlay.set(false)),
            )).padding(t.spacing_xl).background(t.surface).border_radius(t.radius_lg).shadow(t.shadow_lg))
        }),
        // z_stack
        z_stack((
            container(spacer()).width(Dimension::Px(100.0)).height(Dimension::Px(60.0)).background(t.primary),
            text("Overlaid").foreground(Color::WHITE).padding(t.spacing_sm),
        )),
        // provider
        provider(42u32, (text("Context provided: 42u32"),)),
        // show / show_else
        show(check_val.get(), || text("Checkbox is checked")),
        show_else(toggle_val.get(), || text("Toggle ON"), || text("Toggle OFF")),
        // for_each
        for_each(
            vec!["Rust", "is", "great"],
            |item| *item,
            |item| text(*item).padding(4.0).background(Color::hex("#334")).border_radius(Corners::all(4.0)).key(*item),
        ),
    ))
    .gap(t.spacing_sm)
}

fn widgets_composition(t: Theme) -> Node {
    // Exercise: Key variants, Callback, apply, apply_if, disabled, focusable
    // Also exercise: NodeKind, FlexDirection, TextContent, Style, EventHandlers, ImageSource variants
    let _nk = NodeKind::Container;
    let _fd = FlexDirection::Row;
    let _tc = TextContent::Static("hello".into());
    let _s = Style::default();
    let _eh = EventHandlers::default();
    let _is_url = ImageSource::Url("https://example.com/img.png".into());
    let _is_bytes = ImageSource::Bytes(vec![0xFF, 0xD8]);

    v_stack((
        text("Key::Str").key("string-key"),
        text("Key::Int (u64)").key(42u64),
        text("Key::Int (usize)").key(99usize),
        text("Key::Int (i32)").key(7i32),
        {
            let _k: Key = "from".into();
            let _k2: Key = String::from("into").into();
            let cb = Callback::<(), String>::new(|()| "callback result".to_owned());
            text(cb.call(()))
        },
        text("Applied style")
            .apply(|n| n.padding(t.spacing_sm).background(t.info))
            .apply_if(true, |n| n.font_weight(FontWeight::Bold))
            .apply_if(false, |n| n.foreground(Color::RED)),
        button("Disabled").disabled(true).focusable(false),
        button("Focusable").focusable(true),
    ))
    .gap(t.spacing_sm)
}

// ═══════════════════════════════════════════════════════════════════════════
// Style — colors, dimensions, typography, animations
// ═══════════════════════════════════════════════════════════════════════════

fn style_page() -> Node {
    let t = theme();
    scroll_view(v_stack((
        section_title("Style & Theming"),
        style_colors(t.clone()),
        divider(),
        style_typography(t.clone()),
        divider(),
        style_animation(t.clone()),
    )).gap(t.spacing_sm).padding(t.spacing_lg))
}

fn style_colors(t: Theme) -> Node {
    // ── Color constructors ───────────────────────────────────────────────
    let c_rgb = Color::rgb(255, 100, 50);
    let c_rgba = Color::rgba(100, 200, 150, 0.5);
    let c_hex = Color::hex("#ff6600");
    let c_hsl = Color::hsl(200.0, 0.8, 0.5);
    let c_hsla = Color::hsla(120.0, 0.6, 0.4, 0.9);
    let c_f32 = Color::from_f32(0.5, 0.3, 0.8, 1.0);
    let c_alpha = c_rgb.with_alpha(0.5);
    let c_light = c_rgb.lighten(0.2);
    let c_dark = c_rgb.darken(0.2);
    let c_mix = Color::mix(Color::RED, Color::BLUE, 0.5);
    let _lum = Color::WHITE.relative_luminance();
    let _contrast = Color::contrast_ratio(Color::BLACK, Color::WHITE);

    // ── Theme constructors ───────────────────────────────────────────────
    let _light = Theme::light();
    let _dark = Theme::dark();
    let _sys = Theme::system();
    let _is_dark = t.is_dark;
    let _pv = t.primary_variant;
    let _sv = t.secondary_variant;
    let _tp = t.text_primary;
    let _ts = t.text_secondary;
    let _td = t.text_disabled;
    let _top = t.text_on_primary;
    let _tos = t.text_on_secondary;
    let _toe = t.text_on_error;
    let _dv = t.divider;

    // ── Dimension / Edges / Corners / Shadow ─────────────────────────────
    let _ = (Dimension::Px(1.0), Dimension::Percent(50.0), Dimension::Auto, pct(75.0));
    let _ = (Edges::all(8.0), Edges::symmetric(4.0, 8.0), Edges::new(1.0, 2.0, 3.0, 4.0));
    let _ = (Corners::all(12.0), Corners::new(4.0, 8.0, 12.0, 16.0));
    let _ = Shadow::new(2.0, 4.0, 8.0, 0.0, Color::BLACK.with_alpha(0.3));

    // ── Enum variants exercised ──────────────────────────────────────────
    let _ = (Overflow::Visible, Overflow::Hidden, Overflow::Scroll);
    let _ = (FontStyle::Normal, FontStyle::Italic);

    v_stack((
        text("Color constructors:").font_weight(FontWeight::Bold),
        h_stack((
            swatch("rgb", c_rgb), swatch("rgba", c_rgba), swatch("hex", c_hex),
            swatch("hsl", c_hsl), swatch("hsla", c_hsla), swatch("f32", c_f32),
        )).gap(t.spacing_xs),
        text("Color methods:").font_weight(FontWeight::Bold),
        h_stack((
            swatch("alpha", c_alpha), swatch("light", c_light),
            swatch("dark", c_dark), swatch("mix", c_mix),
        )).gap(t.spacing_xs),
        text("Named colors:").font_weight(FontWeight::Bold),
        h_stack((
            swatch("R", Color::RED), swatch("G", Color::GREEN), swatch("B", Color::BLUE),
            swatch("Y", Color::YELLOW), swatch("C", Color::CYAN), swatch("M", Color::MAGENTA),
            swatch("O", Color::ORANGE), swatch("P", Color::PURPLE), swatch("GR", Color::GRAY),
            swatch("LG", Color::LIGHT_GRAY), swatch("DG", Color::DARK_GRAY),
        )).gap(2.0),
        h_stack((
            swatch("PK", Color::PINK), swatch("BR", Color::BROWN),
            swatch("NV", Color::NAVY), swatch("TL", Color::TEAL), swatch("CO", Color::CORAL),
            swatch("W", Color::WHITE), swatch("BK", Color::BLACK), swatch("TR", Color::TRANSPARENT),
        )).gap(2.0),
    ))
    .gap(t.spacing_sm)
}

fn style_typography(t: Theme) -> Node {
    let weights = [
        FontWeight::Thin, FontWeight::ExtraLight, FontWeight::Light,
        FontWeight::Regular, FontWeight::Medium, FontWeight::SemiBold,
        FontWeight::Bold, FontWeight::ExtraBold, FontWeight::Black,
    ];
    let _numeric = FontWeight::Bold.numeric();

    v_stack((
        text("Font weights:").font_weight(FontWeight::Bold),
        for_each(
            weights.to_vec(),
            |w| w.numeric() as u64,
            |w| text(format!("{:?} ({})", w, w.numeric())).font_weight(*w),
        ),
        text("FontFamily: SansSerif").font_family(FontFamily::SansSerif),
        text("FontFamily: Serif").font_family(FontFamily::Serif),
        text("FontFamily: Monospace").font_family(FontFamily::Monospace),
        text("FontFamily: Named").font_family(FontFamily::Named("Fira Code".into())),
        text("TextAlign: Center").text_align(TextAlign::Center),
        text("TextAlign: End").text_align(TextAlign::End),
        text("TextAlign: Justify").text_align(TextAlign::Justify),
        text("TextOverflow: Ellipsis").text_overflow(TextOverflow::Ellipsis).width(Dimension::Px(120.0)),
        text("Line height 2.0").line_height(2.0),
    ))
    .gap(t.spacing_xs)
}

fn style_animation(t: Theme) -> Node {
    // ── Easing variants ──────────────────────────────────────────────────
    let _ = (
        Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut,
        Easing::CubicBezier(0.42, 0.0, 0.58, 1.0),
        Easing::Spring { stiffness: 100.0, damping: 10.0, mass: 1.0 },
    );
    let _ = (AnimationIterations::Count(3), AnimationIterations::Infinite);
    let _ = (
        AnimationDirection::Normal, AnimationDirection::Reverse,
        AnimationDirection::Alternate, AnimationDirection::AlternateReverse,
    );

    // ── AnimatableProperty — every variant ───────────────────────────────
    let _ = [
        AnimatableProperty::Opacity, AnimatableProperty::BackgroundColor,
        AnimatableProperty::ForegroundColor, AnimatableProperty::BorderColor,
        AnimatableProperty::Width, AnimatableProperty::Height,
        AnimatableProperty::PaddingTop, AnimatableProperty::PaddingRight,
        AnimatableProperty::PaddingBottom, AnimatableProperty::PaddingLeft,
        AnimatableProperty::MarginTop, AnimatableProperty::MarginRight,
        AnimatableProperty::MarginBottom, AnimatableProperty::MarginLeft,
        AnimatableProperty::BorderRadius, AnimatableProperty::BorderWidth,
        AnimatableProperty::FontSize, AnimatableProperty::LetterSpacing,
        AnimatableProperty::LineHeight, AnimatableProperty::Gap,
        AnimatableProperty::Transform, AnimatableProperty::BoxShadow,
    ];
    let _ = (AnimatableValue::Float(1.0), AnimatableValue::Color(Color::RED));

    // ── Transition ───────────────────────────────────────────────────────
    let _tr = Transition::new(AnimatableProperty::Opacity, Duration::from_millis(300))
        .with_easing(Easing::EaseInOut)
        .with_delay(Duration::from_millis(50));

    // ── Keyframe + Animation ─────────────────────────────────────────────
    let kf = Keyframe {
        progress: 0.5,
        property: AnimatableProperty::Opacity,
        value: AnimatableValue::Float(0.5),
        easing: Easing::Linear,
    };
    let anim = Animation::new(vec![kf], Duration::from_secs(1))
        .with_iterations(AnimationIterations::Count(2))
        .with_direction(AnimationDirection::Alternate)
        .with_delay(Duration::from_millis(100))
        .with_easing(Easing::EaseOut);

    // ── CursorIcon — every variant ───────────────────────────────────────
    let _ = [
        CursorIcon::Default, CursorIcon::Pointer, CursorIcon::Text,
        CursorIcon::Crosshair, CursorIcon::Move, CursorIcon::NotAllowed,
        CursorIcon::Grab, CursorIcon::Grabbing, CursorIcon::ColResize,
        CursorIcon::RowResize, CursorIcon::NResize, CursorIcon::EResize,
        CursorIcon::SResize, CursorIcon::WResize, CursorIcon::NeResize,
        CursorIcon::NwResize, CursorIcon::SeResize, CursorIcon::SwResize,
        CursorIcon::EwResize, CursorIcon::NsResize, CursorIcon::NeswResize,
        CursorIcon::NwseResize, CursorIcon::Wait, CursorIcon::Progress,
        CursorIcon::Help, CursorIcon::ZoomIn, CursorIcon::ZoomOut,
        CursorIcon::None,
    ];

    v_stack((
        text("Transition on hover:").font_weight(FontWeight::Bold),
        container(text("Hover for transition"))
            .padding(t.spacing_sm).background(t.primary)
            .transition(AnimatableProperty::BackgroundColor, Duration::from_millis(300))
            .border_radius(t.radius_md),
        text("Animation:").font_weight(FontWeight::Bold),
        container(text("Animated"))
            .padding(t.spacing_sm).background(t.info)
            .border_radius(t.radius_md).animate(anim),
        text("Cursors (hover):").font_weight(FontWeight::Bold),
        h_stack((
            text("Pointer").cursor(CursorIcon::Pointer).padding(4.0).background(t.surface),
            text("Crosshair").cursor(CursorIcon::Crosshair).padding(4.0).background(t.surface),
            text("Grab").cursor(CursorIcon::Grab).padding(4.0).background(t.surface),
            text("Help").cursor(CursorIcon::Help).padding(4.0).background(t.surface),
        )).gap(t.spacing_xs),
    ))
    .gap(t.spacing_sm)
}

fn swatch(label: &str, color: Color) -> Node {
    v_stack((
        container(spacer())
            .width(Dimension::Px(28.0)).height(Dimension::Px(20.0))
            .background(color).border_radius(Corners::all(3.0)).border(1.0, Color::GRAY),
        text(label).font_size(8.0),
    )).gap(1.0)
}

// ═══════════════════════════════════════════════════════════════════════════
// Events — every event handler
// ═══════════════════════════════════════════════════════════════════════════

fn events_page() -> Node {
    let t = theme();
    let log = create_signal(String::from("(no events yet)"));

    scroll_view(v_stack((
        section_title("Event Handlers"),
        text(move || log.get())
            .font_family(FontFamily::Monospace).font_size(t.font_size_sm)
            .padding(t.spacing_sm).background(t.surface).border_radius(t.radius_sm)
            .live_region(LivePoliteness::Polite),
        divider(),
        events_mouse(t.clone(), log),
        events_keyboard(t.clone(), log),
        events_dragdrop(t.clone(), log),
        events_types_exercise(),
    )).gap(t.spacing_sm).padding(t.spacing_lg))
}

fn events_mouse(t: Theme, log: Signal<String>) -> Node {
    v_stack((
        button("Click / Double-click")
            .on_click(move || log.set("on_click".into()))
            .on_double_click(move || log.set("on_double_click".into()))
            .padding(t.spacing_sm).background(t.primary).border_radius(t.radius_sm),
        container(text("Mouse down / up / move"))
            .padding(t.spacing_md).background(t.surface).border_radius(t.radius_sm)
            .on_mouse_down(move |ev: MouseEvent| {
                let _ = (ev.button, ev.modifiers);
                log.set(format!("mouse_down ({:.0},{:.0}) {:?}", ev.x, ev.y, ev.button));
            })
            .on_mouse_up(move |ev: MouseEvent| {
                log.set(format!("mouse_up ({:.0},{:.0})", ev.x, ev.y));
            })
            .on_mouse_move(move |ev: MouseEvent| {
                let _ = (ev.global_x, ev.global_y);
                log.set(format!("mouse_move ({:.0},{:.0})", ev.x, ev.y));
            }),
        container(text("Hover enter / leave"))
            .padding(t.spacing_md).background(t.surface).border_radius(t.radius_sm)
            .on_mouse_enter(move || log.set("mouse_enter".into()))
            .on_mouse_leave(move || log.set("mouse_leave".into())),
        container(text("Scroll")).padding(t.spacing_md).background(t.surface).border_radius(t.radius_sm)
            .on_scroll(move |ev: ScrollEvent| {
                let _ = ev.modifiers;
                log.set(format!("scroll dx={:.1} dy={:.1}", ev.delta_x, ev.delta_y));
            }),
    ))
    .gap(t.spacing_sm)
}

fn events_keyboard(t: Theme, log: Signal<String>) -> Node {
    v_stack((
        container(text("Focus and press keys"))
            .padding(t.spacing_md).background(t.surface).border_radius(t.radius_sm)
            .focusable(true)
            .on_key_down(move |ev: KeyEvent| {
                let _ = (ev.code, ev.repeat, ev.text.clone(), ev.modifiers);
                log.set(format!("key_down: {:?}", ev.key));
            })
            .on_key_up(move |ev: KeyEvent| {
                log.set(format!("key_up: {:?}", ev.key));
            }),
        container(text("Focus / blur"))
            .padding(t.spacing_md).background(t.surface).border_radius(t.radius_sm)
            .focusable(true)
            .on_focus(move || log.set("focus".into()))
            .on_blur(move || log.set("blur".into())),
    ))
    .gap(t.spacing_sm)
}

fn events_dragdrop(t: Theme, log: Signal<String>) -> Node {
    h_stack((
        container(text("Drag me"))
            .padding(t.spacing_md).background(t.warning).border_radius(t.radius_sm)
            .on_drag().cursor(CursorIcon::Grab),
        container(text("Drop here"))
            .padding(t.spacing_md).background(t.info).border_radius(t.radius_sm)
            .on_drop(move |ev: DropEvent| {
                let _ = (ev.x, ev.y);
                match &ev.data {
                    DropData::Text(t) => log.set(format!("drop text: {t}")),
                    DropData::Files(f) => log.set(format!("drop {} files", f.len())),
                    DropData::Custom(b) => log.set(format!("drop {} bytes", b.len())),
                }
            }),
    ))
    .gap(t.spacing_md)
}

fn events_types_exercise() -> Node {
    // compile-time exercise of event types
    let _ = Modifiers { shift: false, ctrl: false, alt: false, meta: false };
    let _ = Modifiers::none();
    let _ = (MouseButton::Left, MouseButton::Right, MouseButton::Middle, MouseButton::Back, MouseButton::Forward);
    let _ = KeyCode::KeyA;
    let _ = DragConfig { enabled: true };
    text("(event types exercised)").font_size(9.0).foreground(Color::GRAY)
}

// ═══════════════════════════════════════════════════════════════════════════
// Accessibility
// ═══════════════════════════════════════════════════════════════════════════

fn a11y_page() -> Node {
    let t = theme();

    let _info = AccessibilityInfo::default();
    let _state = AccessibilityState::default();
    let roles = [
        Role::Button, Role::Checkbox, Role::Dialog, Role::Grid, Role::GridCell,
        Role::Heading, Role::Image, Role::Link, Role::List, Role::ListItem,
        Role::Menu, Role::MenuItem, Role::ProgressBar, Role::RadioButton,
        Role::ScrollView, Role::Slider, Role::Switch, Role::Tab, Role::TabList,
        Role::TabPanel, Role::TextInput, Role::Text, Role::Toolbar, Role::Tooltip,
        Role::Tree, Role::TreeItem, Role::Window, Role::Group, Role::None,
    ];
    for r in &roles { let _ = r.is_default_focusable(); }
    let _ = (LivePoliteness::Off, LivePoliteness::Polite, LivePoliteness::Assertive);

    scroll_view(v_stack((
        section_title("Accessibility"),
        a11y_roles(t.clone()),
        divider(),
        a11y_live(t.clone()),
        divider(),
        container(text("Labeled + described"))
            .label("Important section")
            .description("Has label and description for screen readers")
            .focusable(true).role(Role::Group)
            .padding(t.spacing_md).background(t.surface).border(1.0, t.border).border_radius(t.radius_md),
    )).gap(t.spacing_sm).padding(t.spacing_lg))
}

fn a11y_roles(t: Theme) -> Node {
    v_stack((
        text("Semantic roles:").font_weight(FontWeight::Bold),
        button("Button").role(Role::Button).label("Action").description("Demo"),
        text("Heading").role(Role::Heading).label("Section heading"),
        container(text("Link")).role(Role::Link).label("Nav link"),
        container(text("List")).role(Role::List).label("Demo list"),
        container(text("Progress")).role(Role::ProgressBar).label("Loading"),
        container(text("Tab panel")).role(Role::TabPanel).label("Content"),
        container(text("Group")).role(Role::Group).label("Controls"),
    ))
    .gap(t.spacing_xs)
}

fn a11y_live(t: Theme) -> Node {
    v_stack((
        text("Live regions:").font_weight(FontWeight::Bold),
        text("Polite")
            .live_region(LivePoliteness::Polite)
            .padding(t.spacing_sm).background(t.surface).border_radius(t.radius_sm),
        text("Assertive")
            .live_region(LivePoliteness::Assertive)
            .padding(t.spacing_sm).background(t.error).border_radius(t.radius_sm),
    ))
    .gap(t.spacing_xs)
}

// ═══════════════════════════════════════════════════════════════════════════
// Layout — flex, alignment, sizing, overflow
// ═══════════════════════════════════════════════════════════════════════════

fn layout_page() -> Node {
    let t = theme();
    scroll_view(v_stack((
        section_title("Layout"),
        layout_flex(t.clone()),
        divider(),
        layout_sizing(t.clone()),
        divider(),
        layout_spacing(t.clone()),
        divider(),
        layout_visual(t.clone()),
    )).gap(t.spacing_sm).padding(t.spacing_lg))
}

fn layout_flex(t: Theme) -> Node {
    v_stack((
        text("flex_grow:").font_weight(FontWeight::Bold),
        h_stack((
            text("grow=1").flex_grow(1.0).background(t.primary).padding(4.0),
            text("grow=2").flex_grow(2.0).background(t.secondary).padding(4.0),
            text("grow=0").flex_grow(0.0).background(t.info).padding(4.0),
        )).gap(4.0),
        text("flex_shrink + flex_basis:").font_weight(FontWeight::Bold),
        h_stack((
            text("shrink=1 basis=200").flex_shrink(1.0).flex_basis(Dimension::Px(200.0)).background(t.primary).padding(4.0),
            text("shrink=0 basis=100").flex_shrink(0.0).flex_basis(Dimension::Px(100.0)).background(t.secondary).padding(4.0),
        )).gap(4.0),
        text("align_self:").font_weight(FontWeight::Bold),
        h_stack((
            text("Start").align_self(AlignSelf::Start).background(t.surface).padding(4.0),
            text("Center").align_self(AlignSelf::Center).background(t.surface).padding(4.0),
            text("End").align_self(AlignSelf::End).background(t.surface).padding(4.0),
            text("Stretch").align_self(AlignSelf::Stretch).background(t.surface).padding(4.0),
            text("FlxStart").align_self(AlignSelf::FlexStart).background(t.surface).padding(4.0),
            text("FlxEnd").align_self(AlignSelf::FlexEnd).background(t.surface).padding(4.0),
            text("Base").align_self(AlignSelf::Baseline).background(t.surface).padding(4.0),
        )).gap(4.0).height(Dimension::Px(80.0)),
    ))
    .gap(t.spacing_sm)
}

fn layout_sizing(t: Theme) -> Node {
    v_stack((
        text("Dimension: Px, Percent, Auto:").font_weight(FontWeight::Bold),
        h_stack((
            container(text("200px")).width(Dimension::Px(200.0)).height(Dimension::Px(40.0)).background(t.primary),
            container(text("50%")).width(pct(50.0)).height(Dimension::Px(40.0)).background(t.secondary),
            container(text("Auto")).width(Dimension::Auto).background(t.info).padding(4.0),
        )).gap(4.0),
        text("min/max:").font_weight(FontWeight::Bold),
        container(text("min_w=100 max_w=300"))
            .min_width(Dimension::Px(100.0)).max_width(Dimension::Px(300.0))
            .min_height(Dimension::Px(30.0)).max_height(Dimension::Px(60.0))
            .background(t.surface).padding(4.0),
        text("aspect_ratio(16/9):").font_weight(FontWeight::Bold),
        container(spacer())
            .width(Dimension::Px(160.0)).aspect_ratio(16.0 / 9.0)
            .background(t.primary).border_radius(t.radius_sm),
    ))
    .gap(t.spacing_sm)
}

fn layout_spacing(t: Theme) -> Node {
    v_stack((
        text("Padding & margin:").font_weight(FontWeight::Bold),
        container(text("padding(16)")).padding(16.0).background(t.surface),
        container(text("padding_x(24)")).padding_x(24.0).background(t.surface),
        container(text("padding_y(12)")).padding_y(12.0).background(t.surface),
        container(text("margin(8)")).margin(8.0).background(t.surface),
        text("gap=24:").font_weight(FontWeight::Bold),
        h_stack((
            text("A").background(t.primary).padding(4.0),
            text("B").background(t.secondary).padding(4.0),
            text("C").background(t.info).padding(4.0),
        )).gap(24.0),
    ))
    .gap(t.spacing_sm)
}

fn layout_visual(t: Theme) -> Node {
    v_stack((
        text("clip:").font_weight(FontWeight::Bold),
        container(text("This long text should be clipped").width(Dimension::Px(400.0)))
            .width(Dimension::Px(150.0)).height(Dimension::Px(30.0))
            .clip().background(t.surface).border(1.0, t.border),
        text("Shadows:").font_weight(FontWeight::Bold),
        h_stack((
            container(text("sm")).padding(t.spacing_md).background(t.surface).shadow(t.shadow_sm).border_radius(t.radius_md),
            container(text("md")).padding(t.spacing_md).background(t.surface).shadow(t.shadow_md).border_radius(t.radius_md),
            container(text("lg")).padding(t.spacing_md).background(t.surface).shadow(t.shadow_lg).border_radius(t.radius_md),
        )).gap(t.spacing_lg),
        text("Borders:").font_weight(FontWeight::Bold),
        h_stack((
            container(text("1px")).padding(8.0).border(1.0, t.border).border_radius(Corners::all(0.0)),
            container(text("2px round")).padding(8.0).border(2.0, t.primary).border_radius(t.radius_md),
            container(text("mixed")).padding(8.0).border(1.0, t.error).border_radius(Corners::new(0.0, 16.0, 0.0, 16.0)),
        )).gap(t.spacing_sm),
        text("Opacity:").font_weight(FontWeight::Bold),
        h_stack((
            text("100%").background(t.primary).padding(8.0).opacity(1.0),
            text("75%").background(t.primary).padding(8.0).opacity(0.75),
            text("50%").background(t.primary).padding(8.0).opacity(0.5),
            text("25%").background(t.primary).padding(8.0).opacity(0.25),
        )).gap(t.spacing_sm),
    ))
    .gap(t.spacing_sm)
}

// ═══════════════════════════════════════════════════════════════════════════
// Virtual list
// ═══════════════════════════════════════════════════════════════════════════

fn virtual_list_page() -> Node {
    let t = theme();
    let scroll_offset = create_signal(0.0f32);
    let item_count = 10_000usize;
    let item_height = 32.0f32;
    let viewport_height = 500.0f32;

    v_stack((
        section_title("Virtual List"),
        text(format!("{item_count} items, only visible rows rendered")),
        container(
            virtual_list(item_count, item_height, viewport_height, scroll_offset.get(), |idx| {
                let bg = if idx % 2 == 0 { Color::hex("#222") } else { Color::hex("#2a2a2a") };
                text(format!("Row {idx}"))
                    .font_size(14.0).font_family(FontFamily::Monospace)
                    .padding(6.0).background(bg).key(idx as u64)
            }),
        )
        .height(Dimension::Px(viewport_height))
        .border(1.0, t.border).border_radius(t.radius_sm)
        .on_scroll(move |ev: ScrollEvent| {
            let max = (item_count as f32 * item_height - viewport_height).max(0.0);
            scroll_offset.set((scroll_offset.get() + ev.delta_y as f32).clamp(0.0, max));
        }),
    ))
    .gap(t.spacing_sm)
    .padding(t.spacing_lg)
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn section_title(title: &str) -> Node {
    let t = theme();
    text(title).font_size(t.font_size_2xl).font_weight(FontWeight::Bold).foreground(t.text_primary)
}

// ═══════════════════════════════════════════════════════════════════════════
// Tests — compile-time verification of all type constructors
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use vitreous_hot_reload::{ChangeKind, ClientMessage, FileChange, FileEvent};

    #[test]
    fn protocol_types() {
        let _sm = ServerMessage::FileChanged(FileChange {
            path: "src/main.rs".into(), kind: ChangeKind::Source, event: FileEvent::Modified,
        });
        let _ = (ServerMessage::BuildStarted, ServerMessage::BuildComplete,
                 ServerMessage::BuildFailed { errors: "e".into() }, ServerMessage::Shutdown);
        let _ = (ClientMessage::Hello { app_name: "t".into() }, ClientMessage::RequestBuild);
        let _ = (ChangeKind::Style, ChangeKind::Asset);
        let _ = (FileEvent::Created, FileEvent::Removed);
        let _ = DEFAULT_PORT;
    }

    #[test]
    fn style_types() {
        let _ = Style::default();
        let _ = Shadow::new(0.0, 0.0, 0.0, 0.0, Color::BLACK);
        let _ = Transition::new(AnimatableProperty::Opacity, Duration::from_millis(100));
        let _ = Keyframe { progress: 0.0, property: AnimatableProperty::Width,
                           value: AnimatableValue::Float(0.0), easing: Easing::Linear };
        let _ = Animation::new(vec![], Duration::from_secs(1));
    }

    #[test]
    fn event_types() {
        let _ = Modifiers::none();
        let _ = DragConfig { enabled: true };
        let _ = (DropData::Text("".into()), DropData::Files(vec![]), DropData::Custom(vec![]));
    }

    #[test]
    fn a11y_types() {
        let _ = AccessibilityInfo::default();
        let _ = AccessibilityState::default();
        assert!(Role::Button.is_default_focusable());
        assert!(!Role::Text.is_default_focusable());
    }

    #[test]
    fn color_methods() {
        let c = Color::rgb(100, 150, 200);
        let _ = (c.relative_luminance(), Color::contrast_ratio(c, Color::WHITE));
        let _ = (Color::mix(c, Color::RED, 0.5), c.lighten(0.1), c.darken(0.1), c.with_alpha(0.5));
    }

    #[test]
    fn dimension_variants() {
        let _ = (Dimension::Px(42.0), Dimension::Percent(50.0), Dimension::Auto, pct(75.0));
    }

    #[test]
    fn key_from_impls() {
        let _: Key = "hello".into();
        let _: Key = String::from("world").into();
        let _: Key = 42u64.into();
        let _: Key = 99usize.into();
        let _: Key = 7i32.into();
    }

    #[test]
    fn image_source_variants() {
        let _ = ImageSource::Path("foo.png".into());
        let _ = ImageSource::Url("https://example.com/img.png".into());
        let _ = ImageSource::Bytes(vec![0xFF, 0xD8]);
    }

    #[test]
    fn callback_works() {
        let cb = Callback::new(|x: i32| x + 1);
        assert_eq!(cb.call(5), 6);
    }

    #[test]
    fn easing_variants() {
        let _ = (Easing::Linear, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut);
        let _ = Easing::CubicBezier(0.0, 0.0, 1.0, 1.0);
        let _ = Easing::Spring { stiffness: 100.0, damping: 10.0, mass: 1.0 };
    }

    #[test]
    fn theme_constructors() {
        let l = Theme::light();
        let d = Theme::dark();
        let _ = Theme::system();
        assert!(!l.is_dark);
        assert!(d.is_dark);
    }
}
