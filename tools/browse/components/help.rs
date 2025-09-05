// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tui_realm_stdlib::Table;
use tuirealm::{
    command::CmdResult,
    event::{Key, KeyEvent},
    props::{Alignment, Color, Style, TableBuilder, TextSpan},
    Component, Event, MockComponent, NoUserEvent,
};

use crate::Msg;

#[derive(MockComponent)]
pub struct Help {
    component: Table,
}

impl Default for Help {
    fn default() -> Self {
        Self {
            component: Table::default()
                .inactive(Style::default().fg(Color::Magenta))
                .scroll(false)
                .widths(&[5, 20, 80, 5])
                .title("Help", Alignment::Left)
                .table(
                    TableBuilder::default()
                        .add_col(TextSpan::from(""))
                        .add_col(TextSpan::from("Keybindings").bold())
                        .add_row()
                        .add_row()
                        .add_col(TextSpan::from(""))
                        .add_col(TextSpan::from("q").italic())
                        .add_col(TextSpan::from("Quit application"))
                        .add_row()
                        .add_row()
                        .add_col(TextSpan::from(""))
                        .add_col(TextSpan::from("Esc").italic())
                        .add_col(TextSpan::from("Close current popup / quit application"))
                        .add_row()
                        .add_row()
                        .add_col(TextSpan::from(""))
                        .add_col(TextSpan::from("h | ?").italic())
                        .add_col(TextSpan::from("Show this help window"))
                        .add_row()
                        .add_row()
                        .add_col(TextSpan::from(""))
                        .add_col(TextSpan::from("p").italic())
                        .add_col(TextSpan::from("Show params for this object, if available"))
                        .add_row()
                        .add_row()
                        .add_col(TextSpan::from(""))
                        .add_col(TextSpan::from("d").italic())
                        .add_col(TextSpan::from("Delete an object (only works for links)"))
                        .build(),
                ),
        }
    }
}

impl Component<Msg, NoUserEvent> for Help {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => return Some(Msg::ShowHelp(false)),
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
