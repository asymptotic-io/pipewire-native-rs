// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tui_realm_stdlib::Table;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Color, Style, TableBuilder},
    Component, Event, MockComponent, NoUserEvent,
};

use crate::Msg;

#[derive(MockComponent)]
pub struct Popup {
    component: Table,
}

impl Default for Popup {
    fn default() -> Self {
        Self {
            component: Table::default()
                .inactive(Style::default().fg(Color::Magenta))
                .scroll(true)
                .rewind(true)
                .highlighted_str(" ")
                .table(TableBuilder::default().build()),
        }
    }
}

impl Component<Msg, NoUserEvent> for Popup {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Down, ..
            }) => self.perform(Cmd::Move(Direction::Down)),
            Event::Keyboard(KeyEvent { code: Key::Up, .. }) => {
                self.perform(Cmd::Move(Direction::Up))
            }
            Event::Keyboard(KeyEvent {
                code: Key::PageDown,
                ..
            }) => self.perform(Cmd::Scroll(Direction::Down)),
            Event::Keyboard(KeyEvent {
                code: Key::PageUp, ..
            }) => self.perform(Cmd::Scroll(Direction::Up)),
            Event::Keyboard(KeyEvent {
                code: Key::Home, ..
            }) => self.perform(Cmd::GoTo(Position::Begin)),
            Event::Keyboard(KeyEvent { code: Key::End, .. }) => {
                self.perform(Cmd::GoTo(Position::End))
            }
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => {
                return Some(Msg::ShowParams(false))
            }
            _ => CmdResult::None,
        };
        Some(Msg::None)
    }
}
