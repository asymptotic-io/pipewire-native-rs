// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::Arc;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use pipewire::{keys, proxy::HasProxy};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

mod pw;

#[derive(Eq, PartialEq)]
enum Pane {
    Types,
    Objects,
    Details,
}

impl Pane {
    fn next(&self) -> Self {
        match self {
            Self::Types => Self::Objects,
            Self::Objects => Self::Details,
            Self::Details => Self::Details,
        }
    }

    fn prev(&self) -> Self {
        match self {
            Self::Details => Self::Objects,
            Self::Objects => Self::Types,
            Self::Types => Self::Types,
        }
    }
}

struct UiState {
    pane: Pane,
    position: i32,
    last_position: i32,
}

fn init_ui() -> Result<DefaultTerminal> {
    color_eyre::install()?;
    Ok(ratatui::init())
}

fn main() -> Result<()> {
    let mut terminal = init_ui()?;
    let pw_state = pw::State::new("pw-browse")?;

    let pw_state_ = pw_state.clone();

    let handle = pw_state.run();

    let mut ui_state = UiState {
        pane: Pane::Types,
        position: 0,
        last_position: 0,
    };

    // And use this main thread as the UI event loop
    loop {
        terminal.draw(|frame| {
            draw(frame, &ui_state, &pw_state_);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Right,
                    ..
                }) => {
                    if ui_state.pane != Pane::Details {
                        ui_state.pane = ui_state.pane.next();
                        ui_state.last_position = ui_state.position;
                        ui_state.position = 0;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Left,
                    ..
                }) => {
                    if ui_state.pane != Pane::Types {
                        ui_state.pane = ui_state.pane.prev();
                        ui_state.position = ui_state.last_position;
                        ui_state.last_position = 0;
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    ..
                }) => ui_state.position = ui_state.position.wrapping_add(1),
                Event::Key(KeyEvent {
                    code: KeyCode::Up, ..
                }) => ui_state.position = ui_state.position.wrapping_sub(1),
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }) => break,
                _ => (),
            }
        }
    }

    pw_state.quit();
    let _ = handle.join();

    ratatui::restore();

    Ok(())
}

fn draw(frame: &mut Frame, ui_state: &UiState, pw_state: &Arc<pw::State>) {
    let selection_style = Style::default().add_modifier(Modifier::BOLD);
    let block_padding = Padding::uniform(1);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Percentage(10),
            Constraint::Percentage(20),
            Constraint::Percentage(70),
        ])
        .split(frame.area());

    let mut types = vec![Span::from("Clients")];

    if ui_state.pane == Pane::Types {
        let n = types.len();
        if let Some(span) = types.get_mut(ui_state.position as usize % n) {
            *span = span.clone().style(selection_style.clone());
        }
    }

    frame.render_widget(
        Paragraph::new(Text::from(
            types
                .iter()
                .map(|s| Line::from(s.clone()))
                .collect::<Vec<Line>>(),
        ))
        .block(
            Block::default()
                .title("Types")
                .borders(Borders::ALL)
                .padding(block_padding.clone()),
        ),
        layout[0],
    );

    let mut object_lines = vec![];
    let mut selected = 0;

    pw_state
        .main_loop
        .lock()
        .expect("main loop lock should not fail");

    let clients = pw_state.clients.borrow();
    let n = clients.len();

    for (idx, (id, (client, props))) in clients.iter().enumerate() {
        let span = Span::from(format!(
            "#{}: {}",
            client.proxy().bound_id().unwrap_or(*id),
            props.get(keys::APP_NAME).unwrap_or("unknown"),
        ));

        let line = Line::from(
            if (ui_state.pane == Pane::Objects && ui_state.position as usize % n == idx)
                || (ui_state.pane == Pane::Details && ui_state.last_position as usize % n == idx)
            {
                selected = *id;
                span.style(selection_style.clone())
            } else {
                span
            },
        );

        object_lines.push(line);
    }

    let mut detail_lines = vec![];

    if let Some(entry) = clients.get(&selected).or_else(|| clients.values().next()) {
        for (key, value) in entry.1.iter() {
            detail_lines.push(Line::from(vec![
                Span::from(key.to_string()).blue(),
                Span::from(" ".to_string()).blue(),
                Span::from(value.to_string()).gray(),
            ]));
        }
    };

    drop(clients);

    pw_state
        .main_loop
        .unlock()
        .expect("main loop unlock should not fail");

    let objects = Paragraph::new(Text::from(object_lines)).block(
        Block::default()
            .title("Objects")
            .borders(Borders::ALL)
            .padding(block_padding.clone()),
    );

    let mut details = Paragraph::new(Text::from(detail_lines))
        .block(
            Block::default()
                .title("Details")
                .borders(Borders::ALL)
                .padding(block_padding.clone())
                .style(if ui_state.pane == Pane::Details {
                    selection_style.clone()
                } else {
                    Style::default()
                }),
        )
        .wrap(Wrap::default());

    let length = details.line_count(details.line_width() as u16) as u16;
    details = details.scroll(if ui_state.pane == Pane::Details {
        (ui_state.position as u16 % length, 0)
    } else {
        (0, 0)
    });

    frame.render_widget(objects, layout[1]);
    frame.render_widget(details, layout[2]);
}
