use crate::{framework::text::ansi_to_sections, prelude::*};
use bevy::utils::Instant;
use dway_util::logger::LoggerCache;

#[derive(Component, SmartDefault)]
pub struct LoggerUI {
    #[default(32)]
    max_line: usize,
}

dway_widget! {
LoggerUI=>
@global(theme: Theme)
@use_state(text: Text)
@arg(logger: Option<NonSend<LoggerCache>>)
@before{
    if let Some(logger) = logger.as_ref() {
        if logger.is_changed() || !widget.inited {
            let begin = if logger.lines.len() > prop.max_line {logger.lines.len() - prop.max_line} else {0};
            let mut text = Text::default();
            let color = theme.color("white");
            let font = theme.default_font();
            let font_size = 16.0;

            for i in begin..logger.lines.len() {
                let line = &logger.lines[i];
                let line_ansi_str = String::from_utf8_lossy(&line.data);
                text.sections.extend(ansi_to_sections(&line_ansi_str, &theme, color, font_size, &font));
            }
            state.set_text(text);
        }
    }
}
<TextBundle Text=(state.text().clone())/>
}
