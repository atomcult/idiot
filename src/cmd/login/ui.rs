use iocraft::prelude::*;

#[derive(Default, Props)]
struct FormFieldProps {
    label: String,
    value: Option<State<String>>,
    has_focus: bool,
}

#[component]
fn FormField(props: &FormFieldProps) -> impl Into<AnyElement<'static>> {
    let Some(mut value) = props.value else {
        panic!("value is required");
    };

    element! {
        View(
            border_style: if props.has_focus { BorderStyle::Round } else { BorderStyle::None },
            border_color: Color::Blue,
            padding_left: if props.has_focus { 0 } else { 1 },
            padding_right: if props.has_focus { 0 } else { 1 },
        ) {
            View(width: 15) {
                Text(content: format!("{}: ", props.label))
            }
            View(
                background_color: Color::DarkGrey,
                width: 30,
            ) {
                TextInput(
                    has_focus: props.has_focus,
                    value: value.to_string(),
                    on_change: move |new_value| value.set(new_value),
                )
            }
        }
    }
}

#[derive(Default, Props)]
struct FormProps<'a> {
    store_id: Option<&'a mut String>,
}

#[component]
fn Form<'a>(props: &mut FormProps<'a>, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();

    let store_id = hooks.use_state(|| "".to_string());
    let mut focus = hooks.use_state(|| 0);
    let mut should_submit = hooks.use_state(|| false);

    hooks.use_terminal_events(move |event| match event {
        TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind != KeyEventKind::Release => {
            match code {
                KeyCode::Enter => should_submit.set(true),
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => focus.set((focus + 1) % 2),
                _ => {}
            }
        }
        _ => {}
    });

    if should_submit.get() {
        if let Some(store_id_out) = props.store_id.as_mut() {
            **store_id_out = store_id.to_string();
        }
        system.exit();
        element!(View)
    } else {
        element! {
            View(
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                margin: 2,
            ) {
                View(
                    padding_bottom: if focus == 0 { 1 } else { 2 },
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                ) {
                    Text(content: "Press tab to cycle through fields.\nPress enter to submit.", color: Color::Grey, align: TextAlign::Center)
                }
                FormField(label: "Store ID", value: store_id, has_focus: focus == 0)
            }
        }
    }
}

pub fn spawn() {
    let mut store_id = String::new();
    smol::block_on(
        element! {
            Form(
                store_id: &mut store_id,
            )
        }
        .render_loop(),
    )
    .unwrap();
    if store_id.is_empty() {
        println!("No store ID entered.");
    } else {
        // FIXME: perform the login
        println!("Hello, {}!", store_id);
    }
}
