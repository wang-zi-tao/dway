use bitflags::bitflags;

use crate::prelude::*;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
    pub struct DwayUiDirection: u8 {
        const TOP =     0b00000001;
        const BOTTOM =  0b00000010;
        const LEFT =    0b00000100;
        const RIGHT =   0b00001000;
    }
}

pub(crate) fn set_component_or_insert<C: Component>(component: Option<&mut C>, mut commands: EntityCommands, value: C){
    if let Some(c) = component {
        *c = value;
    }else {
        commands.queue(move|mut entity_mut: EntityWorldMut<'_>|{
            entity_mut.insert(value);
        });
    }
}

pub(crate) fn modify_component_or_insert<C: Component + Default>(component: Option<&mut C>, mut commands: EntityCommands, f: impl FnOnce(&mut C) + Send + 'static){
    if let Some(c) = component {
        f(c);
    }else {
        commands.queue(move|mut entity_mut: EntityWorldMut<'_>|{
            let mut c= entity_mut.entry::<C>().or_default();
            f(&mut c);
        });
    }
}
