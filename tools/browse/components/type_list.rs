// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tui_realm_stdlib::List;
use tuirealm::{
    command::{Cmd, CmdResult, Direction, Position},
    event::{Key, KeyEvent},
    props::{Alignment, Color, Style, TableBuilder, TextSpan},
    Component, Event, MockComponent, NoUserEvent, State, StateValue,
};

use crate::{ComponentId, Msg};

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub enum TypeSelection {
    Clients,
    Devices,
    Factories,
    Links,
    Metadata,
    Modules,
    Nodes,
    Ports,
}

impl TryFrom<usize> for TypeSelection {
    type Error = ();
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(TypeSelection::Clients),
            1 => Ok(TypeSelection::Devices),
            2 => Ok(TypeSelection::Factories),
            3 => Ok(TypeSelection::Links),
            4 => Ok(TypeSelection::Metadata),
            5 => Ok(TypeSelection::Modules),
            6 => Ok(TypeSelection::Nodes),
            7 => Ok(TypeSelection::Ports),
            _ => Err(()),
        }
    }
}

#[derive(MockComponent)]
pub struct TypeList {
    component: List,
}

impl Default for TypeList {
    fn default() -> Self {
        Self {
            component: List::default()
                .inactive(Style::default().fg(Color::Magenta))
                .scroll(true)
                .rewind(true)
                .title("Object types", Alignment::Left)
                .highlighted_str(" ⋄ ")
                .highlighted_color(Color::DarkGray)
                .rows(
                    TableBuilder::default()
                        .add_col(TextSpan::from("Clients"))
                        .add_row()
                        .add_col(TextSpan::from("Devices"))
                        .add_row()
                        .add_col(TextSpan::from("Factories"))
                        .add_row()
                        .add_col(TextSpan::from("Links"))
                        .add_row()
                        .add_col(TextSpan::from("Metadata"))
                        .add_row()
                        .add_col(TextSpan::from("Modules"))
                        .add_row()
                        .add_col(TextSpan::from("Nodes"))
                        .add_row()
                        .add_col(TextSpan::from("Ports"))
                        .build(),
                ),
        }
    }
}

impl Component<Msg, NoUserEvent> for TypeList {
    fn on(&mut self, ev: Event<NoUserEvent>) -> Option<Msg> {
        let old_selection = self.component.state();
        let mut focus_changed = false;

        let _ = match ev {
            Event::Keyboard(KeyEvent {
                code: Key::Right, ..
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
            Event::Keyboard(KeyEvent { code: Key::Esc, .. }) => return Some(Msg::Quit),
            _ => CmdResult::None,
        };

        let new_selection = self.component.state();
        if old_selection != new_selection {
            if let State::One(StateValue::Usize(idx)) = new_selection {
                Some(Msg::TypeChanged(TypeSelection::try_from(idx).unwrap()))
            } else {
                Some(Msg::None)
            }
        } else if focus_changed {
            Some(Msg::FocusChanged(ComponentId::Objects))
        } else {
            Some(Msg::None)
        }
    }
}
