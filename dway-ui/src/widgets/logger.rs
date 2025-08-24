use crate::{prelude::*};
use dway_ui_framework::widgets::text::ansi_to_sections;
use dway_util::logger::LoggerCache;

#[derive(Component, SmartDefault)]
#[require(Text, Node)]
pub struct LoggerUI {
    #[default(32)]
    pub max_line: usize,
    inited: bool,
}

pub fn update_logger_ui(
    mut query: Query<(Entity, &mut LoggerUI, )>,
    logger: Option<NonSend<LoggerCache>>,
    theme: Res<Theme>,
    mut commands: Commands,
){
    let Some(logger) = logger else{
        return
    };
    for (entity, mut logger_ui) in query.iter_mut() {
        if logger.is_changed() || !logger_ui.inited {
            logger_ui.inited = true;
            let mut entity_comands = commands.entity(entity);
            entity_comands.despawn();

            let begin = if logger.lines.len() > logger_ui.max_line {logger.lines.len() - logger_ui.max_line} else {0};
            let color = theme.color("white");
            let font = theme.default_font();
            let font_size = 16.0;

            for i in begin..logger.lines.len() {
                let line = &logger.lines[i];
                let line_ansi_str = String::from_utf8_lossy(&line.data);

                for bundle in
ansi_to_sections(&line_ansi_str, &theme, color, font_size, &font) {
                    entity_comands.with_child(bundle);
                }
            }
        }
    }
}

#[derive(Default)]
pub struct LoggerUIPlugin;

impl Plugin for LoggerUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_logger_ui);
    }
}
