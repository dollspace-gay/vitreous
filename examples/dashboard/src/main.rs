use std::error::Error;
use std::future::Future;
use std::pin::Pin;

use vitreous::{
    App, Color, Dimension, FontWeight, Node, Resource, Theme, container, create_resource,
    create_signal, h_stack, set_executor, show_else, spacer, text, theme, v_stack,
};

// Simulated metric data returned by our "API"
#[derive(Clone)]
struct Metrics {
    total_users: u64,
    active_sessions: u64,
    revenue_cents: u64,
    error_rate: f64,
}

fn metric_card(label: &str, value: String, accent: Color) -> Node {
    let t = theme();
    v_stack((
        text(label)
            .font_size(t.font_size_sm)
            .font_weight(FontWeight::Regular),
        text(value)
            .font_size(t.font_size_2xl)
            .font_weight(FontWeight::Bold),
    ))
    .gap(t.spacing_xs)
    .padding(t.spacing_md)
    .background(t.surface)
    .border(2.0, accent)
    .border_radius(t.radius_md)
    .width(Dimension::Px(200.0))
}

fn loading_skeleton() -> Node {
    let t = theme();
    v_stack((
        text("Loading metrics...")
            .font_size(t.font_size_lg)
            .padding(t.spacing_md),
        // Placeholder cards
        h_stack((
            skeleton_card(),
            skeleton_card(),
            skeleton_card(),
            skeleton_card(),
        ))
        .gap(t.spacing_md)
        .padding(t.spacing_md),
    ))
}

fn skeleton_card() -> Node {
    let t = theme();
    container(spacer().height(Dimension::Px(60.0)))
        .width(Dimension::Px(200.0))
        .background(t.border)
        .border_radius(t.radius_md)
        .padding(t.spacing_md)
        .opacity(0.5)
}

fn root() -> Node {
    let t = theme();

    // Set up a no-op executor (real app would use tokio::task::spawn_local)
    set_executor(|_fut| {
        // In a real app: tokio::task::spawn_local(fut);
        // For this example, the Resource stays in loading state to demonstrate
        // the loading skeleton UI.
    });

    let fetch_trigger = create_signal(());

    type FetchResult = Pin<Box<dyn Future<Output = Result<Metrics, Box<dyn Error>>> + 'static>>;

    let metrics: Resource<(), Metrics> = create_resource(
        move || fetch_trigger.get(),
        |_| -> FetchResult {
            Box::pin(async {
                Ok(Metrics {
                    total_users: 12_847,
                    active_sessions: 1_523,
                    revenue_cents: 984_750,
                    error_rate: 0.23,
                })
            })
        },
    );

    v_stack((
        // Header
        h_stack((
            text("Dashboard")
                .font_size(t.font_size_3xl)
                .font_weight(FontWeight::Bold),
            spacer(),
            text("Last updated: just now").font_size(t.font_size_sm),
        ))
        .padding(t.spacing_lg),
        // Metrics section — show loading skeleton or real data
        show_else(
            !metrics.loading(),
            move || {
                let t = theme();
                match metrics.data() {
                    Some(m) => h_stack((
                        metric_card("Total Users", format!("{}", m.total_users), t.primary),
                        metric_card(
                            "Active Sessions",
                            format!("{}", m.active_sessions),
                            t.success,
                        ),
                        metric_card(
                            "Revenue",
                            format!("${:.2}", m.revenue_cents as f64 / 100.0),
                            t.info,
                        ),
                        metric_card("Error Rate", format!("{:.2}%", m.error_rate), t.error),
                    ))
                    .gap(t.spacing_md)
                    .padding(t.spacing_md),
                    None => text("No data available").padding(t.spacing_md),
                }
            },
            loading_skeleton,
        ),
        // Detail panels
        h_stack((
            // Activity panel
            v_stack((
                text("Recent Activity")
                    .font_size(t.font_size_lg)
                    .font_weight(FontWeight::Bold),
                activity_row("User signed up", "2m ago"),
                activity_row("Payment processed", "5m ago"),
                activity_row("Report generated", "12m ago"),
                activity_row("Alert resolved", "1h ago"),
            ))
            .gap(t.spacing_sm)
            .padding(t.spacing_md)
            .background(t.surface)
            .border_radius(t.radius_md)
            .flex_grow(1.0),
            // System status panel
            v_stack((
                text("System Status")
                    .font_size(t.font_size_lg)
                    .font_weight(FontWeight::Bold),
                status_row("API", "Operational", true),
                status_row("Database", "Operational", true),
                status_row("CDN", "Degraded", false),
                status_row("Workers", "Operational", true),
            ))
            .gap(t.spacing_sm)
            .padding(t.spacing_md)
            .background(t.surface)
            .border_radius(t.radius_md)
            .flex_grow(1.0),
        ))
        .gap(t.spacing_md)
        .padding(t.spacing_md),
    ))
    .background(t.background)
}

fn activity_row(event: &str, time: &str) -> Node {
    let t = theme();
    h_stack((
        text(event).flex_grow(1.0).font_size(t.font_size_sm),
        text(time).font_size(t.font_size_xs),
    ))
    .padding(t.spacing_xs)
}

fn status_row(service: &str, status: &str, healthy: bool) -> Node {
    let t = theme();
    h_stack((
        text(service).flex_grow(1.0).font_size(t.font_size_sm),
        text(status)
            .font_size(t.font_size_sm)
            .background(if healthy { t.success } else { t.warning })
            .padding(t.spacing_xs)
            .border_radius(t.radius_sm),
    ))
    .padding(t.spacing_xs)
}

fn main() {
    App::new()
        .title("Vitreous Dashboard")
        .size(900, 650)
        .theme(Theme::light())
        .run(root);
}
