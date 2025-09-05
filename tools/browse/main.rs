// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

mod components;
mod pw;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use tuirealm::{
    props::{Alignment, Color, TableBuilder, TextSpan},
    ratatui::{layout, widgets::Clear},
    terminal::{CrosstermTerminalAdapter, TerminalBridge},
    Application, AttrValue, Attribute, EventListenerCfg, NoUserEvent, PollStrategy, Update,
};

use components::{
    help::Help, object_details::ObjectDetails, object_list::ObjectList, param_pane::ParamPane,
    renderable::Renderable, type_list::TypeList, type_list::TypeSelection,
};

#[derive(Debug, Eq, PartialEq, Clone)]
enum Msg {
    FocusChanged(ComponentId),
    TypeChanged(TypeSelection),
    ObjectChanged(usize),
    ShowParams(bool),
    ShowHelp(bool),
    Quit,
    None,
}

#[derive(Debug, Eq, PartialEq, Clone, Hash)]
enum ComponentId {
    Types,
    Objects,
    Details,
    Params,
    Help,
}

struct Model {
    app: Application<ComponentId, Msg, NoUserEvent>,
    pw_state: Arc<pw::State>,
    component_selection: ComponentId,
    type_selection: TypeSelection,
    object_selection: usize,
    show_params: bool,
    show_help: bool,
    quit: bool,
    redraw: bool,
}

impl Model {
    fn new(pw_state: Arc<pw::State>) -> Self {
        let mut app = Application::init(
            EventListenerCfg::default().crossterm_input_listener(Duration::from_millis(20), 10),
        );

        app.mount(ComponentId::Types, Box::new(TypeList::default()), vec![])
            .unwrap();
        app.mount(
            ComponentId::Objects,
            Box::new(ObjectList::default()),
            vec![],
        )
        .unwrap();
        app.mount(
            ComponentId::Details,
            Box::new(ObjectDetails::default()),
            vec![],
        )
        .unwrap();
        app.mount(ComponentId::Params, Box::new(ParamPane::default()), vec![])
            .unwrap();
        app.mount(ComponentId::Help, Box::new(Help::default()), vec![])
            .unwrap();

        app.active(&ComponentId::Types).unwrap();

        Self {
            app,
            pw_state,
            component_selection: ComponentId::Types,
            type_selection: TypeSelection::Clients,
            object_selection: 0,
            show_params: false,
            show_help: false,
            quit: false,
            redraw: true,
        }
    }

    fn view(&mut self, terminal: &mut TerminalBridge<CrosstermTerminalAdapter>) {
        let _ = terminal.raw_mut().draw(|frame| {
            let layout = layout::Layout::default()
                .direction(layout::Direction::Horizontal)
                .constraints([
                    layout::Constraint::Percentage(10),
                    layout::Constraint::Percentage(30),
                    layout::Constraint::Percentage(60),
                ])
                .split(frame.area());

            self.app.view(&ComponentId::Types, frame, layout[0]);
            self.app.view(&ComponentId::Objects, frame, layout[1]);
            self.app.view(&ComponentId::Details, frame, layout[2]);

            if self.show_params {
                let area = frame.area();
                let popup_area = layout::Rect {
                    x: area.width / 8,
                    y: area.height / 8,
                    width: area.width * 6 / 8,
                    height: area.height * 6 / 8,
                };
                frame.render_widget(Clear, popup_area);
                self.app.view(&ComponentId::Params, frame, popup_area);
            }

            if self.show_help {
                let area = frame.area();
                let popup_area = layout::Rect {
                    x: area.width / 4,
                    y: area.height / 4,
                    width: area.width * 2 / 4,
                    height: area.height * 2 / 4,
                };
                frame.render_widget(Clear, popup_area);
                self.app.view(&ComponentId::Help, frame, popup_area);
            }
        });
    }

    fn update_object_list(&mut self) {
        let mut objects: Vec<Box<dyn Renderable>> = vec![];

        match self.type_selection {
            TypeSelection::Clients => {
                let clients = self.pw_state.clients.lock().unwrap();

                for client in clients.values() {
                    objects.push(Box::new(client.clone()));
                }
            }
            TypeSelection::Devices => {
                let devices = self.pw_state.devices.lock().unwrap();

                for device in devices.values() {
                    objects.push(Box::new(device.clone()));
                }
            }
            TypeSelection::Factories => {
                let factories = self.pw_state.factories.lock().unwrap();

                for factory in factories.values() {
                    objects.push(Box::new(factory.clone()));
                }
            }
            TypeSelection::Links => {
                let links = self.pw_state.links.lock().unwrap();

                for link in links.values() {
                    objects.push(Box::new(link.clone()));
                }
            }
            TypeSelection::Metadata => {
                let metadata = self.pw_state.metadata.lock().unwrap();

                for metadata in metadata.values() {
                    objects.push(Box::new(metadata.clone()));
                }
            }
            TypeSelection::Modules => {
                let modules = self.pw_state.modules.lock().unwrap();

                for module in modules.values() {
                    objects.push(Box::new(module.clone()));
                }
            }
            TypeSelection::Nodes => {
                let nodes = self.pw_state.nodes.lock().unwrap();

                for node in nodes.values() {
                    objects.push(Box::new(node.clone()));
                }
            }
            TypeSelection::Ports => {
                let ports = self.pw_state.ports.lock().unwrap();

                for port in ports.values() {
                    objects.push(Box::new(port.clone()));
                }
            }
        }

        let mut table = TableBuilder::default();

        for (idx, object) in objects.iter().enumerate() {
            table.add_col(object.title());

            if idx < objects.len() - 1 {
                table.add_row();
            }
        }

        self.app
            .attr(
                &ComponentId::Objects,
                Attribute::Content,
                AttrValue::Table(table.build()),
            )
            .unwrap();
        self.update_object_details();
    }

    fn get_current_object(&self) -> Option<Box<dyn Renderable>> {
        match self.type_selection {
            TypeSelection::Clients => {
                let clients = self.pw_state.clients.lock().unwrap();
                let entries = clients.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
            TypeSelection::Devices => {
                let devices = self.pw_state.devices.lock().unwrap();
                let entries = devices.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
            TypeSelection::Links => {
                let links = self.pw_state.links.lock().unwrap();
                let entries = links.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
            TypeSelection::Factories => {
                let factories = self.pw_state.factories.lock().unwrap();
                let entries = factories.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
            TypeSelection::Metadata => {
                let metadata = self.pw_state.metadata.lock().unwrap();
                let entries = metadata.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
            TypeSelection::Modules => {
                let modules = self.pw_state.modules.lock().unwrap();
                let entries = modules.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
            TypeSelection::Nodes => {
                let nodes = self.pw_state.nodes.lock().unwrap();
                let entries = nodes.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
            TypeSelection::Ports => {
                let ports = self.pw_state.ports.lock().unwrap();
                let entries = ports.iter().collect::<Vec<_>>();
                entries
                    .get(self.object_selection)
                    .map(|e| Box::new(e.1.clone()) as Box<dyn Renderable>)
            }
        }
    }

    fn update_object_details(&mut self) {
        let object = self.get_current_object();
        let props = object.as_ref().map(|o| o.props());

        if let Some(props) = props {
            let mut props_str = props.iter().collect::<Vec<_>>();
            props_str.sort_by_key(|e| e.0);

            let n = props_str.len();

            let mut table = TableBuilder::default();

            for (idx, (key, value)) in props_str.iter().enumerate() {
                table.add_col(TextSpan::from(*key).fg(Color::Cyan));
                table.add_col(TextSpan::from(" "));
                table.add_col(TextSpan::from(*value));

                // Only add rows between columns
                if idx < n - 1 {
                    table.add_row();
                }
            }

            self.app
                .attr(
                    &ComponentId::Details,
                    Attribute::Content,
                    AttrValue::Table(table.build()),
                )
                .unwrap();
        }
    }

    fn update_object_params(&mut self) -> bool {
        let object = match self.get_current_object() {
            Some(o) => o,
            None => return false,
        };

        let params = object.params();
        if params.is_empty() {
            return false;
        }

        let mut table = TableBuilder::default();

        for (param_id, param) in params {
            let mut first = true;
            for pod in param.pods.iter() {
                let mut parser = spa::pod::parser::Parser::new(pod.data());
                parser
                    .pop_object_raw::<spa::param::ParamType, _>(
                        |object_parser, object_type, _id| {
                            assert!(param_id.object_type() == Some(object_type));

                            for (id, _, pod) in object_parser {
                                if first {
                                    table
                                        .add_col(
                                            TextSpan::from(format!("{:?}", param_id))
                                                .bold()
                                                .underlined(),
                                        )
                                        .add_row()
                                        .add_row();
                                    first = false;
                                }

                                table
                                    .add_col(TextSpan::from(
                                        param_id
                                            .key_to_string(id)
                                            .unwrap_or(format!("unknown key {id}")),
                                    ))
                                    .add_col(TextSpan::from(format!("{pod}")))
                                    .add_row();
                            }

                            // Add a separating space between objects
                            table.add_row();

                            Ok(())
                        },
                    )
                    .unwrap();
            }
        }

        self.app
            .attr(
                &ComponentId::Params,
                Attribute::Title,
                AttrValue::Title(("Params".into(), Alignment::Left)),
            )
            .unwrap();
        self.app
            .attr(
                &ComponentId::Params,
                Attribute::Content,
                AttrValue::Table(table.build()),
            )
            .unwrap();

        true
    }
}

impl Update<Msg> for Model {
    fn update(&mut self, msg: Option<Msg>) -> Option<Msg> {
        self.redraw = true;

        match msg.unwrap_or(Msg::None) {
            Msg::FocusChanged(component_id) => {
                self.component_selection = component_id;
                self.app.active(&self.component_selection).unwrap();
                None
            }
            Msg::TypeChanged(type_selection) => {
                self.type_selection = type_selection;
                self.update_object_list();
                None
            }
            Msg::ObjectChanged(idx) => {
                self.object_selection = idx;
                self.update_object_details();
                None
            }
            Msg::ShowParams(show_params) => {
                let have_params = self.update_object_params();
                let show_params = show_params && have_params;

                if show_params != self.show_params {
                    self.show_params = show_params;
                    if self.show_params {
                        self.app.active(&ComponentId::Params).unwrap();
                    } else {
                        self.app.active(&self.component_selection).unwrap();
                    }
                }

                None
            }
            Msg::ShowHelp(show_help) => {
                if show_help != self.show_help {
                    self.show_help = show_help;
                    if self.show_help {
                        self.app.active(&ComponentId::Help).unwrap();
                    } else {
                        self.app.active(&self.component_selection).unwrap();
                    }
                }

                None
            }
            Msg::Quit => {
                self.quit = true;
                None
            }
            Msg::None => None,
        }
    }
}

fn main() {
    let mut terminal =
        TerminalBridge::init_crossterm().expect("Could not initialise terminal bridge");
    let _ = terminal.enable_raw_mode();
    let _ = terminal.enter_alternate_screen();

    let pw_update = Arc::new(AtomicBool::new(false));
    let pw_state = pw::State::new("pw-browse", pw_update.clone())
        .expect("PipeWire initialisation should succeed");
    let mut model = Model::new(pw_state);

    model.pw_state.run();

    // And use this main thread as the UI event loop
    while !model.quit {
        if let Ok(messages) = model.app.tick(PollStrategy::Once) {
            for msg in messages {
                let mut msg = Some(msg);
                while msg.is_some() {
                    msg = model.update(msg);
                }
            }

            // We have an update from PipeWire
            if pw_update.swap(false, Ordering::Relaxed) {
                model.update_object_list();
                model.redraw = true;
            }

            if model.redraw {
                model.view(&mut terminal);
                model.redraw = false;
            }
        }
    }

    model.pw_state.stop();

    let _ = terminal.leave_alternate_screen();
    let _ = terminal.disable_raw_mode();
}
