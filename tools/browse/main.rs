// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use pipewire::{keys, proxy::HasProxy};
use ratatui::{
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal,
};

mod pw;

fn init_ui() -> Result<DefaultTerminal> {
    color_eyre::install()?;
    Ok(ratatui::init())
}

fn main() -> Result<()> {
    let mut terminal = init_ui()?;
    let pw_state = pw::State::new("pw-browse")?;

    let state = pw_state.clone();

    let handle = pw_state.run();

    // And use this main thread as the UI event loop
    loop {
        terminal.draw(|frame| {
            let mut lines = vec![];

            for (id, (client, props)) in state.clients.borrow().iter() {
                lines.push(Line::from(format!(
                    "Client #{}: {}",
                    client.proxy().bound_id().unwrap_or(*id),
                    props.get(keys::APP_NAME).unwrap_or("unknown"),
                )));
            }

            let para = Paragraph::new(Text::from(lines))
                .block(Block::default().title("pw-browse").borders(Borders::ALL));

            frame.render_widget(para, frame.area());
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            match event::read()? {
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
