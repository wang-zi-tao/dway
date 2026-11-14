use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct ProbeSet;

fn missing_system() {}

fn other_system() {}

#[test]
fn ordering_against_missing_system_does_not_panic() {
    let mut app = App::new();
    app.add_systems(Update, other_system.in_set(ProbeSet));
    app.configure_sets(Update, ProbeSet.after(missing_system));
    app.update();
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct Focus;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct Layout;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct PostLayout;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct FrameworkInputs;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct UpdateLayers;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct Widgets;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct InitContexts;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct InitReading;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct EndPass;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
struct ProcessOutput;

fn before_focus() {}
fn focus_system() {}
fn after_focus() {}
fn framework_input() {}
fn widgets() {}
fn update_layers() {}
fn egui_contexts() {}
fn egui_input() {}
fn egui_end_pass() {}
fn egui_process_output() {}

/// Mirrors the constraint pattern added in InspectorPlugin::build:
/// bevy_ui + LayerManager + framework shape on the left, bevy_egui sets on the right.
#[test]
fn inspector_ordering_pattern_is_cycle_free() {
    let mut app = App::new();

    app.add_systems(
        PreUpdate,
        (
            before_focus.in_set(Focus).before(focus_system),
            focus_system.in_set(Focus).after(InputSystems),
            after_focus.in_set(Focus).after(focus_system),
            framework_input.in_set(FrameworkInputs).after(Focus),
            egui_contexts.in_set(InitContexts),
            egui_input.in_set(InitReading),
        ),
    );
    app.configure_sets(PreUpdate, FrameworkInputs.after(Focus));
    app.configure_sets(PreUpdate, (InitContexts, InitReading).chain());
    app.configure_sets(
        PreUpdate,
        InitContexts.after(before_focus).before(after_focus),
    );
    app.configure_sets(PreUpdate, InitReading.after(Focus).after(FrameworkInputs));

    app.add_systems(
        PostUpdate,
        (
            widgets.in_set(Widgets),
            update_layers.in_set(UpdateLayers),
            egui_end_pass.in_set(EndPass),
            egui_process_output.in_set(ProcessOutput),
        ),
    );
    app.configure_sets(PostUpdate, (Layout, PostLayout).chain());
    app.configure_sets(PostUpdate, Widgets.before(Layout));
    app.configure_sets(
        PostUpdate,
        (EndPass, ProcessOutput)
            .chain()
            .before(Layout)
            .before(UpdateLayers),
    );

    app.update();
}
