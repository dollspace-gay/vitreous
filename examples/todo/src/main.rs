use vitreous::{
    App, Dimension, FontWeight, Node, Theme, button, checkbox, container, create_signal, divider,
    for_each, h_stack, scroll_view, text, text_input, theme, v_stack,
};

#[derive(Clone, PartialEq)]
struct TodoItem {
    id: u64,
    title: String,
    done: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Filter {
    All,
    Active,
    Completed,
}

fn root() -> Node {
    let t = theme();
    let next_id = create_signal(1u64);
    let input_text = create_signal(String::new());
    let todos = create_signal(Vec::<TodoItem>::new());
    let filter = create_signal(Filter::All);

    // --- Header ---
    let header = v_stack((
        text("Todo List")
            .font_size(t.font_size_2xl)
            .font_weight(FontWeight::Bold),
        // Input row
        h_stack((
            text_input(input_text, move |val| input_text.set(val))
                .flex_grow(1.0)
                .padding(t.spacing_sm)
                .border(1.0, t.border)
                .border_radius(t.radius_sm),
            button("Add")
                .on_click(move || {
                    let title = input_text.get();
                    if !title.is_empty() {
                        let id = next_id.get();
                        next_id.set(id + 1);
                        let mut list = todos.get();
                        list.push(TodoItem {
                            id,
                            title,
                            done: false,
                        });
                        todos.set(list);
                        input_text.set(String::new());
                    }
                })
                .padding(t.spacing_sm)
                .background(t.primary)
                .border_radius(t.radius_sm),
        ))
        .gap(t.spacing_sm),
    ))
    .gap(t.spacing_sm)
    .padding(t.spacing_md);

    // --- Filter bar ---
    let filter_bar = h_stack((
        filter_button("All", Filter::All, filter),
        filter_button("Active", Filter::Active, filter),
        filter_button("Completed", Filter::Completed, filter),
    ))
    .gap(t.spacing_sm)
    .padding_x(t.spacing_md);

    // --- Todo list ---
    let list = scroll_view(container({
        let current_filter = filter.get();
        let items: Vec<TodoItem> = todos
            .get()
            .into_iter()
            .filter(|item| match current_filter {
                Filter::All => true,
                Filter::Active => !item.done,
                Filter::Completed => item.done,
            })
            .collect();

        for_each(
            items,
            |item| item.id,
            move |item| {
                let item_id = item.id;
                let done_signal = create_signal(item.done);

                h_stack((
                    checkbox(done_signal).on_click(move || {
                        let mut list = todos.get();
                        if let Some(todo) = list.iter_mut().find(|t| t.id == item_id) {
                            todo.done = !todo.done;
                        }
                        todos.set(list);
                    }),
                    text(item.title.as_str())
                        .flex_grow(1.0)
                        .font_size(t.font_size_md)
                        .opacity(if item.done { 0.5 } else { 1.0 }),
                    // Show delete button on hover area
                    button("x")
                        .on_click(move || {
                            let list: Vec<TodoItem> = todos
                                .get()
                                .into_iter()
                                .filter(|t| t.id != item_id)
                                .collect();
                            todos.set(list);
                        })
                        .font_size(t.font_size_sm)
                        .padding(t.spacing_xs)
                        .background(t.error)
                        .border_radius(t.radius_sm),
                ))
                .gap(t.spacing_sm)
                .padding(t.spacing_sm)
                .border(1.0, t.border)
                .border_radius(t.radius_sm)
                .margin(t.spacing_xs)
            },
        )
    }))
    .flex_grow(1.0)
    .padding_x(t.spacing_md);

    // --- Status bar ---
    let status = {
        let items = todos.get();
        let active = items.iter().filter(|i| !i.done).count();
        let done = items.iter().filter(|i| i.done).count();
        h_stack((
            text(format!("{active} active")),
            spacer(),
            text(format!("{done} completed")),
        ))
        .padding(t.spacing_sm)
        .padding_x(t.spacing_md)
        .background(t.surface)
    };

    v_stack((header, divider(), filter_bar, list, divider(), status))
        .width(Dimension::Px(500.0))
        .height(Dimension::Px(600.0))
        .background(t.background)
}

fn filter_button(label: &str, target: Filter, current: vitreous::Signal<Filter>) -> Node {
    let t = theme();
    let is_active = current.get() == target;
    button(label)
        .on_click(move || current.set(target))
        .padding(t.spacing_xs)
        .background(if is_active { t.primary } else { t.surface })
        .border_radius(t.radius_sm)
}

fn spacer() -> Node {
    vitreous::spacer()
}

fn main() {
    App::new()
        .title("Vitreous Todo")
        .size(500, 600)
        .theme(Theme::light())
        .run(root);
}
