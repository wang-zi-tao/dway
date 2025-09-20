use std::sync::Arc;

use crate::{
    event::UiEvent,
    prelude::*,
    theme::{ComboBoxNodeKind, NoTheme, StyleFlags, ThemeComponent, WidgetKind},
};

pub trait UiComboboxItem: 'static + Send + Sync {
    fn spawn(&self, entity_mut: EntityWorldMut);
}

#[derive(Component, SmartDefault)]
#[require(
    Node,
    UiComboBoxState,
    UiComboBoxWidget,
    UiComboBoxSubStateList,
    ThemeComponent
)]
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
@callback{ [UiEvent<UiButtonEvent>]
    fn open_popup(
        event: UiEvent<UiButtonEvent>,
        mut query: Query<&mut UiComboBoxState>,
    ){
        let Ok(mut state) = query.get_mut(event.receiver()) else {return};
        if event.kind==UiButtonEventKind::Pressed{
            state.set_open(true);
        }
    }
}
@callback{ [UiEvent<UiPopupEvent>]
    fn close_popup(
        event: UiEvent<UiPopupEvent>,
        mut query: Query<&mut UiComboBoxState>,
    ){
        let Ok(mut state) = query.get_mut(event.receiver()) else {return};
        if *event==UiPopupEvent::Closed{
            state.set_open(false);
        }
    }
}
@callback{ [UiEvent<UiButtonEvent>]
    fn select(
        event: UiEvent<UiButtonEvent>,
        mut item_query: Query<&mut UiComboBoxSubStateList>,
        mut combobox_query: Query<&mut UiComboBoxState>,
    ){
        let Ok(item_state) = item_query.get_mut(event.receiver()) else { return};
        let Ok(mut combobox_state) = combobox_query.get_mut(item_state.combobox) else {return};
        if event.kind==UiButtonEventKind::Pressed{
            combobox_state.set_selected(Some(*item_state.index()));
        }
        if event.kind==UiButtonEventKind::Released{
            combobox_state.set_open(false);
        }
    }
}
@state_component(#[derive(Reflect)])
@use_state(pub selected: Option<usize> @ prop.default_index)
@use_state(pub open: bool)
<UiButton @style="full align-items:center justify-content:center" NoTheme @on_event(open_popup)
    @if(state.selected().and_then(|i|prop.items.get(i)).is_some())>
    <Node @id="selected" @command({let item = prop.items[state.selected().unwrap()].clone();move|e:EntityWorldMut|item.spawn(e) })/>
</UiButton>
<Node @style="full absolute" @if(*state.open())>
    <UiPopup @id="List" @style="absolute top-110% align-self:center flex-col w-full p-2" @on_event(close_popup)
        @for((index,item):(usize, &Arc<dyn UiComboboxItem> ) in prop.items.iter().enumerate() => {
            state.set_item(Some(item.clone()));
            state.set_index(index);
        })
        GlobalZIndex=(GlobalZIndex(1024))
        ThemeComponent=(ThemeComponent::widget(WidgetKind::ComboBox(ComboBoxNodeKind::Popup)))
    >
        <Node @id="item" @style="full"
                @use_state(pub combobox: Entity = Entity::PLACEHOLDER @ this_entity)
                @use_state(pub index: usize)
                @use_state(#[reflect(ignore)] pub item: Option<Arc<dyn UiComboboxItem>>)
                @state_component(#[derive(Reflect)]) >
            <Node NoTheme @on_event(select)
                @style="full align-items:center justify-content:center "
                ThemeComponent=(ThemeComponent::widget(WidgetKind::ComboBox(ComboBoxNodeKind::Item)).with_flag_value(StyleFlags::HIGHLIGHT, Some(*state.index()) == *root_state.selected()))
            >
                <Node @command({let item = state.item().clone().unwrap();move|e:EntityWorldMut|item.spawn(e) })/>
            </Node>
        </Node>
    </UiPopup>
</Node>
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
        entity_mut.insert((
            Text::new(&self.data),
            TextFont {
                font_size: 32.0,
                ..Default::default()
            },
        ));
    }
}
