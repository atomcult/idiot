use core::ops::Deref;

use anyhow::Result;
use iocraft::prelude::*;

use crate::store::login;

#[derive(Default, Props, Clone)]
struct FormFieldProps {
    label: String,
    value: Option<State<String>>,
    has_focus: bool,
    is_censored: bool,
}

#[component]
fn FormField(props: &FormFieldProps) -> impl Into<AnyElement<'static>> {
    let Some(mut value) = props.value else {
        panic!("value is required");
    };

    let closure_props = (*props).clone();

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
                    value: if props.is_censored {
                            value.to_string().chars().map(|_| '*').collect::<String>()
                        } else {
                            value.to_string()
                        },
                    on_change: move |new_value: String| {
                        if !closure_props.is_censored {
                            value.set(new_value);
                            return
                        }

                        let mut old_value: String = value.read().deref().into();
                        if new_value.len() <= old_value.len() {
                            old_value.truncate(new_value.len());
                        } else {
                            old_value.push_str(&new_value[old_value.len()..]);
                        }
                        value.set(old_value);
                    },
                )
            }
        }
    }
}

#[derive(Default, Props)]
struct FormProps<'a> {
    user: Option<&'a mut String>,
    pass: Option<&'a mut String>,
}

#[component]
fn Form<'a>(props: &mut FormProps<'a>, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();

    let user = hooks.use_state(|| "".to_string());
    let pass = hooks.use_state(|| "".to_string());

    let mut focus = hooks.use_state(|| 0);
    let mut should_submit = hooks.use_state(|| false);

    hooks.use_terminal_events(move |event| match event {
        TerminalEvent::Key(KeyEvent { code, kind, .. }) if kind == KeyEventKind::Press => {
            match code {
                KeyCode::Enter => should_submit.set(true),
                KeyCode::Tab | KeyCode::Up | KeyCode::Down => focus.set((focus + 1) % 2),
                _ => {}
            }
        }
        _ => {}
    });

    if should_submit.get() {
        if let Some(user_out) = props.user.as_mut() {
            **user_out = user.to_string();
        }
        if let Some(pass_out) = props.pass.as_mut() {
            **pass_out = pass.to_string();
        }
        system.exit();
        return element!(View);
    }

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
            FormField(label: "Username", value: user, has_focus: focus == 0, is_censored: false)
            FormField(label: "Password", value: pass, has_focus: focus == 1, is_censored: true)
        }
    }
}

pub async fn run() -> Result<()> {
    let mut user = String::new();
    let mut pass = String::new();

    element! {
        Form(
            user: &mut user,
            pass: &mut pass,
        )
    }
    .render_loop()
    .await?;

    if user.is_empty() {
        println!("No username entered.");
        return Ok(());
    }

    if pass.is_empty() {
        println!("No password entered.");
        return Ok(());
    }

    // FIXME: try login again if we need TOTP token
    let macaroon = login::login(user, pass).await?;
    println!("macaroon: {}", macaroon);

    Ok(())
}
