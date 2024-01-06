use dway_util::logger::LoggerCache;
use crate::{prelude::*, framework::text::ansi_to_sections};

#[derive(Component, SmartDefault)]
pub struct LoggerUI {
    #[default(32)]
    max_line: usize,
}

dway_widget! {
LoggerUI=>
@use_state(text: Text)
@arg(logger: NonSend<LoggerCache> => {
    let begin = if logger.lines.len() > prop.max_line {logger.lines.len() - prop.max_line} else {0};
    let mut text = Text::default();
    let color = theme.color("white");
    let font = theme.default_font();
    let font_size = 24.0;

    for i in begin..logger.lines.len() {
        let line = &logger.lines[i];
        let line_ansi_str = String::from_utf8_lossy(&line.data);
        text.sections.extend(ansi_to_sections(&line_ansi_str, &theme, color, font_size, &font));
    }
    state.set_text(text);
})
@global(theme: Theme)
<TextBundle Text=(state.text().clone())/>
}
