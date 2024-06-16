use bevy::ecs::system::EntityCommands;

use crate::{
    animation::{
        ui::{move_rect_by_percent, TargetStyle},
        AnimationDirection, AnimationEvent,
    },
    event::{EventDispatch, UiNodeAppearEvent},
    make_bundle,
    prelude::*,
    util::Direction,
};

structstruck::strike! {
    #[derive(Component, SmartDefault)]
    pub struct UiTranslationAnimation {
        #[default(Direction::TOP)]
        pub direction: Direction,
        pub appear: bool,
    }
}

impl UiTranslationAnimation {
    pub fn open(&mut self, animation: &mut Animation) {
        self.appear = true;
        animation.play();
    }

    pub fn close(&mut self, animation: &mut Animation) {
        self.appear = false;
        animation.play();
    }
}

impl EventDispatch<AnimationEvent> for UiTranslationAnimation {
    fn on_event(&self, mut commands: EntityCommands, event: AnimationEvent) {
        let animation_progress = event.value;
        let v = match &self.appear {
            true => animation_progress,
            false => 1.0 - animation_progress,
        };

        let just_finish = event.just_finish;
        commands.add(move |mut entity_mut: EntityWorldMut| {
            let Some(child_entity) = entity_mut
                .get::<Children>()
                .and_then(|c| c.first().cloned())
            else {
                return;
            };
            let Some(child_size) =
                entity_mut.world_scope(|world| world.get::<Node>(child_entity).map(Node::size))
            else {
                return;
            };
            let direction = entity_mut
                .get::<UiTranslationAnimation>()
                .unwrap()
                .direction;
            let target_layout = entity_mut.get::<TargetStyle>().unwrap().0.clone();
            {
                let Some(mut layout) = entity_mut.get_mut::<Style>() else {
                    return;
                };
                *layout = target_layout.clone();
                if !just_finish {
                    let offset = Vec2::new(
                        if direction.contains(Direction::LEFT) {
                            v - 1.0
                        } else if direction.contains(Direction::RIGHT) {
                            1.0 - v
                        } else {
                            0.0
                        },
                        if direction.contains(Direction::TOP) {
                            v - 1.0
                        } else if direction.contains(Direction::BOTTOM) {
                            1.0 - v
                        } else {
                            0.0
                        },
                    );
                    move_rect_by_percent(&mut layout, &target_layout, offset, child_size);
                }
            }
        });
    }
}

impl EventDispatch<UiNodeAppearEvent> for UiTranslationAnimation {
    fn on_event(&self, mut commands: EntityCommands, event: UiNodeAppearEvent) {
        commands.add(move |mut entity_mut: EntityWorldMut| {
            {
                let mut this = entity_mut.get_mut::<Self>().unwrap();
                this.appear = &event == &UiNodeAppearEvent::Appear;
            }
            {
                let mut animation = entity_mut.get_mut::<Animation>().unwrap();
                animation.play_with_direction(AnimationDirection::new(event.appear()));
            }
        });
    }
}

make_bundle! {
    @addon UiTranslationAnimationExt,
    UiTranslationAnimationBundle{
        pub collapse: UiTranslationAnimation,
        pub animation: Animation,
        pub target_style: TargetStyle,
    }
}
