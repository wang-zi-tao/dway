# dway: a wayland compositor base on bevy

# dway: 一个基于bevy引擎的linux wayland混成器

## crates

### dway-ui-framework
a responsive gui framework

```rust
/// the prop of the widget
#[derive(Component, SmartDefault)]
pub struct Clock {
    #[default("%B-%e  %H:%M:%S %A".to_string())]
    pub format: String,
}
dway_ui_derive::dway_widget!{ 
Clock=>
@use_state{ pub text:String="".to_string() }
@state_component{#[derive(Debug)]}
@before{
    let date = chrono::Local::now().naive_local();
    let date_string = date.format(&prop.format).to_string();
    if state.text() != &date_string{ state.set_text(date_string); }
}
@global(theme:Theme)
<TextBundle Text=(Text::from_section(
    state.text(),
    TextStyle {
        font_size: 24.0,
        color: theme.color("panel-foreground"),
        ..default()
    },
)) /> 
}

app.add_plugins(ClockPlugin);

commands.spwan(ClockBundle::default());
```

### dway-ui-derive
provide macros for gui framework

### bevy-relationship
query ECS items link query a graph database

```rust
graph_query!(
XWindowGraph=>[
    surface=<(&'static Geometry, &'static mut WlSurfacePointerState, Option<&'static PinedWindow> ),With<DWayToplevel>>,
    xwindow=&'static mut XWindow,
    client=Entity,
]=>{
    path=surface-[XWindowAttachSurface]->xwindow,
    seat_path=surface-[ClientHasSurface]->client,
});
```

### dway-tty
connect gpu and screens on linux tty, and display onto screens.

### dway-server
connect apps on the desktop

### dway-client-core
desktop logical module

### dway-ui
the widgets for the wayland compositor

### dway
the top-level crate
