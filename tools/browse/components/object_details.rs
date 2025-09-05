// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tui_realm_stdlib::List;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, Color, Style, TableBuilder},
    Component, Event, MockComponent, NoUserEvent,
};

use crate::{ComponentId, Msg};

#[derive(MockComponent)]
pub struct ObjectDetails {
    component: List,
}

impl Default for ObjectDetails {
    fn default() -> Self {
        Self {
            component: List::default()
                .inactive(Style::default().fg(Color::Magenta))
                .scroll(true)
                .rewind(true)
                .title("Details", Alignment::Left)
                .highlighted_str(" ⋄ ")
                .rows(TableBuilder::default().build()),
        }
    }
}

impl Component<Msg, NoUserEvent> for ObjectDetails {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let mut focus_changed = false;

        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Left, ..
            }) => {
                focus_changed = true;
                CmdResult::None
            }
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
            _ => match crate::global_keybindings(ev) {
                Some(msg) => return Some(msg),
                None => CmdResult::None,
            },
        };

        if focus_changed {
            Some(Msg::FocusChanged(ComponentId::Objects))
        } else {
            Some(Msg::None)
        }
    }
}
