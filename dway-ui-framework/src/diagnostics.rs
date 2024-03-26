use bevy::diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic};

use crate::prelude::*;

const UI_NODE_COUNT: DiagnosticPath = DiagnosticPath::const_new("dway_ui_framework/ui_node_count");

pub fn diagnostic_system(mut diagnostics: Diagnostics, query: Query<(), With<Node>>) {
    diagnostics.add_measurement(&UI_NODE_COUNT, || query.iter().count() as f64);
}

pub struct UiDiagnosticsPlugin;
impl Plugin for UiDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(Diagnostic::new(UI_NODE_COUNT))
            .add_systems(Update, diagnostic_system);
    }
}
