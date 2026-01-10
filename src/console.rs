use bevy::log::tracing;
use bevy::log::tracing_subscriber;
use bevy::log::BoxedLayer;
use bevy::prelude::*;
use bevy_egui::egui;
use std::collections::VecDeque;
use std::sync::mpsc;

use crate::editor_panels::YoleckPanelUi;

/// Log level for console messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn color(&self) -> egui::Color32 {
        match self {
            LogLevel::Debug => egui::Color32::LIGHT_GRAY,
            LogLevel::Info => egui::Color32::WHITE,
            LogLevel::Warn => egui::Color32::from_rgb(255, 200, 0),
            LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// A single log entry captured from the tracing system.
#[derive(Clone, Debug, Message)]
pub struct LogEntry {
    pub level: LogLevel,
    pub message: String,
    pub target: String,
}

/// Non-send resource containing the receiver for captured log messages.
pub struct CapturedLogMessages(mpsc::Receiver<LogEntry>);

/// Resource storing the history of log messages displayed in the console.
#[derive(Resource)]
pub struct YoleckConsoleLogHistory {
    pub logs: VecDeque<LogEntry>,
    pub max_logs: usize,
}

impl YoleckConsoleLogHistory {
    pub fn new(max_logs: usize) -> Self {
        Self {
            logs: VecDeque::with_capacity(max_logs),
            max_logs,
        }
    }

    pub fn add_log(&mut self, entry: LogEntry) {
        if self.logs.len() >= self.max_logs {
            self.logs.pop_front();
        }
        self.logs.push_back(entry);
    }

    pub fn clear(&mut self) {
        self.logs.clear();
    }
}

impl Default for YoleckConsoleLogHistory {
    fn default() -> Self {
        Self::new(1000)
    }
}

/// Resource containing the current state of the console UI.
#[derive(Resource, Default)]
pub struct YoleckConsoleState {
    pub log_filters: LogFilters,
}

/// Filters for controlling which log levels are displayed in the console.
#[derive(Resource)]
pub struct LogFilters {
    pub show_debug: bool,
    pub show_info: bool,
    pub show_warn: bool,
    pub show_error: bool,
}

impl Default for LogFilters {
    fn default() -> Self {
        Self {
            show_debug: false,
            show_info: true,
            show_warn: true,
            show_error: true,
        }
    }
}

impl LogFilters {
    pub fn should_show(&self, level: LogLevel) -> bool {
        match level {
            LogLevel::Debug => self.show_debug,
            LogLevel::Info => self.show_info,
            LogLevel::Warn => self.show_warn,
            LogLevel::Error => self.show_error,
        }
    }
}

/// Creates a console panel section for displaying log messages in the editor UI.
pub fn console_panel_section(
    mut ui: ResMut<YoleckPanelUi>,
    mut console_state: ResMut<YoleckConsoleState>,
    mut log_history: ResMut<YoleckConsoleLogHistory>,
) -> Result {
    ui.horizontal(|ui| {
        ui.label("Filters:");

        ui.checkbox(&mut console_state.log_filters.show_debug, "DEBUG");
        ui.checkbox(&mut console_state.log_filters.show_info, "INFO");
        ui.checkbox(&mut console_state.log_filters.show_warn, "WARN");
        ui.checkbox(&mut console_state.log_filters.show_error, "ERROR");

        ui.separator();

        if ui.button("Clear").clicked() {
            log_history.clear();
        }
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(&mut ui, |ui| {
            for log in log_history
                .logs
                .iter()
                .filter(|log| console_state.log_filters.should_show(log.level))
            {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(log.level.color(), format!("[{}]", log.level.label()));
                    ui.label(&log.message);
                });
            }
        });

    Ok(())
}

/// Tracing layer that captures log messages and sends them to the console.
pub struct YoleckConsoleLayer {
    sender: mpsc::Sender<LogEntry>,
}

impl YoleckConsoleLayer {
    pub fn new(sender: mpsc::Sender<LogEntry>) -> Self {
        Self { sender }
    }
}

impl<S> tracing_subscriber::Layer<S> for YoleckConsoleLayer
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            tracing::Level::TRACE => return,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        };

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        if let Some(message) = visitor.message {
            let _ = self.sender.send(LogEntry {
                level,
                message,
                target: metadata.target().to_string(),
            });
        }
    }
}

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value).trim_matches('"').to_string());
        }
    }
}

fn transfer_log_messages(
    receiver: NonSend<CapturedLogMessages>,
    mut message_writer: MessageWriter<LogEntry>,
) {
    message_writer.write_batch(receiver.0.try_iter());
}

fn store_log_messages(
    mut log_reader: MessageReader<LogEntry>,
    log_history: Option<ResMut<YoleckConsoleLogHistory>>,
) {
    let Some(mut log_history) = log_history else {
        return;
    };
    for log in log_reader.read() {
        log_history.add_log(log.clone());
    }
}

/// Factory function that creates and configures the console logging layer.
///
/// This function should be used with Bevy's `LogPlugin` to capture log messages
/// and display them in the Yoleck editor console.
///
/// # Example
///
/// ```no_run
/// # use bevy::{prelude::*, log::LogPlugin};
/// # use bevy_yoleck::console_layer_factory;
///
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins.set(LogPlugin {
///             custom_layer: console_layer_factory,
///             ..default()
///         }))
///         .run();
/// }
/// ```
pub fn console_layer_factory(app: &mut App) -> Option<BoxedLayer> {
    let (sender, receiver) = mpsc::channel();

    let layer = YoleckConsoleLayer::new(sender);
    let resource = CapturedLogMessages(receiver);

    app.insert_non_send_resource(resource);
    app.add_message::<LogEntry>();
    app.add_systems(Update, (transfer_log_messages, store_log_messages).chain());

    Some(Box::new(layer))
}
