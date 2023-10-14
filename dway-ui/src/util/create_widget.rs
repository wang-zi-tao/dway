#[macro_export]
macro_rules! create_widget {
    ($name:ident,$plugin:ident,$bundle:ident,$props:tt,@widget_update $widget_update:expr) => {
#[derive(bevy::prelude::Component, Debug, Clone, PartialEq, Eq)]
pub struct $name $props
impl kayak_ui::prelude::Widget for $name {}

#[derive(Default)]
pub struct $plugin;
impl kayak_ui::KayakUIPlugin for $plugin {
    fn build(&self, context: &mut kayak_ui::prelude::KayakRootContext) {
        context.add_widget_data::<$name, kayak_ui::prelude::EmptyState>();
        context.add_widget_system(
            kayak_ui::prelude::WidgetName(std::any::type_name::<$name>().into()),
            $widget_update,
            render,
        );
    }
}
#[derive(bevy::prelude::Bundle)]
pub struct $bundle {
    pub props: $name,
    pub styles: kayak_ui::prelude::KStyle,
    pub computed_styles: kayak_ui::prelude::ComputedStyles,
    pub widget_name: kayak_ui::prelude::WidgetName,
    // pub on_event: OnEvent,
    pub children: KChildren,
}

impl Default for $bundle {
    fn default() -> Self {
        Self {
            props: Default::default(),
            styles: Default::default(),
            computed_styles: Default::default(),
            widget_name: kayak_ui::prelude::WidgetName(std::any::type_name::<$name>().into()),
            // on_event: Default::default(),
            children: Default::default(),
        }
    }
}
    };
    ($name:ident,$plugin:ident,$bundle:ident,$props:tt) => {
        create_widget!($name,$plugin,$bundle,$props,@widget_update kayak_ui::prelude::widget_update::<$name, kayak_ui::prelude::EmptyState>);
    };
}

#[macro_export]
macro_rules! create_render {
    ($prop:ident) => {
pub fn render(
    In(entity): In<Entity>,
    widget_context: Res<KayakWidgetContext>,
    props_query: Query<&$prop>,
    mut commands: Commands,
) -> bool {
    let Ok(props) = props_query.get(entity) else {
        return true;
    };
    let parent_id = Some(entity);
    rsx! {
        <ElementBundle>
        </ElementBundle>
    };
    true
}
    };
}

#[macro_export]
macro_rules! create_update_function {
    ($prop:ident) => {
pub fn widget_update(
    In((entity, previous_entity)): In<(Entity, Entity)>,
    widget_context: Res<KayakWidgetContext>,
    widget_param: WidgetParam<$prop, EmptyState>,
) -> bool {
    let should_update = widget_param.has_changed(&widget_context, entity, previous_entity);
    should_update
}
    };
}
