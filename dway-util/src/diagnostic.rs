use std::{any::type_name, marker::PhantomData};

use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    prelude::*,
};

pub fn log_changed_component<T: Component>(
    query: Query<Entity, Changed<T>>,
    mut commands: Commands,
) {
    for entity in &query {
        commands.entity(entity).log_components();
    }
}

fn changed_component_count<T: Component>() -> DiagnosticPath {
    DiagnosticPath::new(format!("changed/{}", type_name::<T>()))
}

fn stats_changed_component<T: Component>(
    query: Query<Entity, Changed<T>>,
    mut diagnostics: Diagnostics,
) {
    diagnostics.add_measurement(&changed_component_count::<T>(), || {
        query.iter().count() as f64
    });
}

#[derive(Default)]
pub struct ChangedDiagnosticPlugin<T: Component>(PhantomData<T>);

impl<T: Component> Plugin for ChangedDiagnosticPlugin<T> {
    fn build(&self, app: &mut App) {
        app.register_diagnostic(
            Diagnostic::new(changed_component_count::<T>()).with_suffix(" entity"),
        )
        .add_systems(Last, stats_changed_component::<T>);
    }
}
