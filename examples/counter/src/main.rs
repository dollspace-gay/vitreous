use vitreous::{
    App, Dimension, FontWeight, Node, Theme, button, container, create_signal, h_stack, spacer,
    text, theme, v_stack,
};

fn root() -> Node {
    let t = theme();
    let count = create_signal(0i32);

    v_stack((
        // Title
        text("Counter")
            .font_size(t.font_size_2xl)
            .font_weight(FontWeight::Bold)
            .padding(t.spacing_md),
        // Count display — reactive text updates when signal changes
        text(move || format!("Count: {}", count.get()))
            .font_size(t.font_size_xl)
            .padding(t.spacing_lg),
        // Control buttons
        h_stack((
            button("- Decrement")
                .on_click(move || count.set(count.get() - 1))
                .padding(t.spacing_sm)
                .background(t.secondary)
                .border_radius(t.radius_md),
            spacer(),
            button("Reset")
                .on_click(move || count.set(0))
                .padding(t.spacing_sm)
                .background(t.warning)
                .border_radius(t.radius_md),
            spacer(),
            button("+ Increment")
                .on_click(move || count.set(count.get() + 1))
                .padding(t.spacing_sm)
                .background(t.primary)
                .border_radius(t.radius_md),
        ))
        .gap(t.spacing_sm)
        .padding(t.spacing_md),
        // Parity indicator — demonstrates reactive conditional styling
        container(text(move || {
            if count.get() % 2 == 0 {
                "Even".to_owned()
            } else {
                "Odd".to_owned()
            }
        }))
        .padding(t.spacing_sm)
        .background(t.surface)
        .border_radius(t.radius_sm)
        .margin(t.spacing_md),
    ))
    .padding(t.spacing_xl)
    .width(Dimension::Px(400.0))
    .background(t.background)
}

fn main() {
    App::new()
        .title("Vitreous Counter")
        .size(400, 300)
        .theme(Theme::light())
        .run(root);
}
