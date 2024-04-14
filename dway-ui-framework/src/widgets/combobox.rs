use bevy::ui::widget::TextFlags;
use std::sync::Arc;

use super::{button::UiRawButtonBundle, text::UiTextExt};
use crate::{prelude::*, theme::{ComboBoxNodeKind, StyleFlags, ThemeComponent, WidgetKind}};

pub trait UiComboboxItem: 'static + Send + Sync {
    fn spawn(&self, entity_mut: EntityWorldMut);
}

#[derive(Component, SmartDefault)]
pub struct UiComboBox {
    pub items: Vec<Arc<dyn UiComboboxItem>>,
    pub default_index: Option<usize>,
}

dway_widget! {
UiComboBox=>
@plugin{
    app.register_type::<UiComboBoxState>();
    app.register_type::<UiComboBoxSubStateList>();
}
@callback{ [UiButtonEvent]
    fn open_popup(
        In(event): In<UiButtonEvent>,
        mut query: Query<&mut UiComboBoxState>,
    ){
        let Ok(mut state) = query.get_mut(event.receiver) else {return};
        if event.kind==UiButtonEventKind::Pressed{
            state.set_open(true);
        }
    }
}
@callback{ [PopupEvent]
    fn close_popup(
        In(event): In<PopupEvent>,
        mut query: Query<&mut UiComboBoxState>,
    ){
        let Ok(mut state) = query.get_mut(event.receiver) else {return};
        if event.kind==PopupEventKind::Closed{
            state.set_open(false);
        }
    }
}
@callback{ [UiButtonEvent]
    fn select(
        In(event): In<UiButtonEvent>,
        mut item_query: Query<&mut UiComboBoxSubStateList>,
        mut combobox_query: Query<&mut UiComboBoxState>,
    ){
        let Ok(item_state) = item_query.get_mut(event.receiver) else { return};
        let Ok(mut combobox_state) = combobox_query.get_mut(item_state.combobox) else {return};
        if event.kind==UiButtonEventKind::Pressed{
            combobox_state.set_selected(Some(*item_state.index()));
        }
        if event.kind==UiButtonEventKind::Released{
            combobox_state.set_open(false);
        }
    }
}
@bundle{{ theme: ThemeComponent = ThemeComponent::widget(WidgetKind::ComboBox(ComboBoxNodeKind::Root)) }}
@state_component(#[derive(Reflect)])
@use_state(pub selected: Option<usize> @ prop.default_index)
@use_state(pub open: bool)
<UiRawButtonBundle @id="selected" @style="full align-items:center justify-content:center"
    UiButton=(UiButton::new(this_entity, open_popup))
    @if(state.selected().and_then(|i|prop.items.get(i)).is_some())>
    <MiniNodeBundle @id="selected" @command({let item = prop.items[state.selected().unwrap()].clone();move|e:EntityWorldMut|item.spawn(e) })/>
</UiRawButtonBundle>
<MiniNodeBundle @style="full absolute" @if(*state.open())>
    <UiPopupBundle @id="List" @style="absolute top-110% align-self:center flex-col w-full p-2"
        UiPopup=(UiPopup::default().with_callback(this_entity, close_popup))
        @for((index,item):(usize, &Arc<dyn UiComboboxItem> ) in prop.items.iter().enumerate() => {
            state.set_item(Some(item.clone()));
            state.set_index(index);
        }) 
        ZIndex=(ZIndex::Global(1024)) 
        ThemeComponent=(ThemeComponent::widget(WidgetKind::ComboBox(ComboBoxNodeKind::Popup)))
    >
        <MiniNodeBundle @id="item" @style="full"
                @use_state(pub combobox: Entity = Entity::PLACEHOLDER @ this_entity)
                @use_state(pub index: usize)
                @use_state(#[reflect(ignore)] pub item: Option<Arc<dyn UiComboboxItem>>)
                @state_component(#[derive(Reflect)]) >
            <UiRawButtonBundle UiButton=(UiButton::new(node!(item), select)) 
                @style="full align-items:center justify-content:center " 
                ThemeComponent=(ThemeComponent::widget(WidgetKind::ComboBox(ComboBoxNodeKind::Item)).with_flag_value(StyleFlags::HIGHLIGHT, Some(*state.index()) == *root_state.selected()))
            >
                <MiniNodeBundle @command({let item = state.item().clone().unwrap();move|e:EntityWorldMut|item.spawn(e) })/>
            </UiRawButtonBundle>
        </MiniNodeBundle>
    </UiPopupBundle>
</MiniNodeBundle>
}

pub struct StringItem {
    data: String,
}

impl StringItem {
    pub fn new(data: String) -> Self {
        Self { data }
    }
}

impl UiComboboxItem for StringItem {
    fn spawn(&self, mut entity_mut: EntityWorldMut) {
        entity_mut.insert(UiTextExt {
            text: Text::from_section(
                &self.data,
                TextStyle {
                    font_size: 32.0,
                    color: Color::BLACK,
                    ..Default::default()
                },
            ),
            ..Default::default()
        });
    }
}
