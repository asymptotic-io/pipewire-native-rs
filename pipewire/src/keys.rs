// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

/// Configuration directory prefix.
pub const CONFIG_PREFIX: &str = "config.prefix";
/// Configuration file name.
pub const CONFIG_NAME: &str = "config.name";
/// Override configuration directory prefix.
pub const CONFIG_OVERRIDE_PREFIX: &str = "config.override.prefix";
/// Override configuration file name.
pub const CONFIG_OVERRIDE_NAME: &str = "config.override.name";

/// Core object name.
pub const CORE_NAME: &str = "core.name";
/// Core object version.
pub const CORE_VERSION: &str = "core.version";
/// Whether the core object is listening for connections.
pub const CORE_DAEMON: &str = "core.daemon";
/// Id of the core object.
pub const CORE_ID: &str = "core.id";
/// APIs monitored by the core.
pub const CORE_MONITORS: &str = "core.monitors";

/// Maximum alignment needed for all CPU optimisations.
pub const CPU_MAX_ALIGN: &str = "cpu.max-align";
/// Number of CPU cores on the machine.
pub const CPU_CORES: &str = "cpu.cores";

/// Name of the PipeWire remote to connect to.
pub const REMOTE_NAME: &str = "remote.name";
/// Connection intent ("generic", "screencast", or "internal").
pub const REMOTE_INTENTION: &str = "remote.intention";

/// Name of the application.
pub const APP_NAME: &str = "application.name";
/// Textual ID for the application (e.g. org.gnome.Rhythmbox).
pub const APP_ID: &str = "application.id";

/// Application version.
pub const APP_VERSION: &str = "application.version";
/// Application icon as a base64-encoded blob.
pub const APP_ICON: &str = "application.icon";
/// XDG icon name for the application.
pub const APP_ICON_NAME: &str = "application.icon-name";

/// Language of the application (e.g. en_US).
pub const APP_LANGUAGE: &str = "application.language";

/// Process ID ofr the application.
pub const APP_PROCESS_ID: &str = "application.process.id";
/// Name of the application binary.
pub const APP_PROCESS_BINARY: &str = "application.process.binary";
/// Name of the user that the application is running as.
pub const APP_PROCESS_USER: &str = "application.process.user";
/// Host name of the machine that the application is running on.
pub const APP_PROCESS_HOST: &str = "application.process.host";
/// Machine ID of the machine that the application is running on.
pub const APP_PROCESS_MACHINE_ID: &str = "application.process.machine-id";
/// XDG session ID that the applicatoin is running in.
pub const APP_PROCESS_SESSION_ID: &str = "application.process.session-id";

/// X11 display string for the application window.
pub const WINDOW_X11_DISPLAY: &str = "window.x11.display";

/// Link ID.
pub const LINK_ID: &str = "link.id";
/// Input node ID of the link.
pub const LINK_INPUT_NODE: &str = "link.input.node";
/// Input port ID of the link.
pub const LINK_INPUT_PORT: &str = "link.input.port";
/// Output node ID of the link.
pub const LINK_OUTPUT_NODE: &str = "link.output.node";
/// Output port ID of the link.
pub const LINK_OUTPUT_PORT: &str = "link.output.port";
/// Link is passive (does not cause the graph to start running).
pub const LINK_PASSIVE: &str = "link.passive";
/// Link is for feedback (target receives data for the current cycle in the next cycle).
pub const LINK_FEEDBACK: &str = "link.feedback";
/// Link is asynchronous
pub const LINK_ASYNC: &str = "link.async";
