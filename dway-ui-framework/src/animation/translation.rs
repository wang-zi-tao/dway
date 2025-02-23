use bevy::ecs::system::EntityCommands;

use crate::{
    animation::{
        ui::{move_rect_by_percent, AnimationTargetNodeState},
        AnimationDirection, AnimationEvent,
    },
    command::DestroyInterceptor,
    event::{EventReceiver, UiNodeAppearEvent},
    make_bundle,
    prelude::*,
    util::DwayUiDirection,
};

structstruck::strike! {
    #[derive(Component, SmartDefault)]
    pub struct UiTranslationAnimation {
        #[default(DwayUiDirection::TOP)]
        pub direction: DwayUiDirection,
        #[default(true)]
        pub appear: bool,
    }
}

impl UiTranslationAnimation {
    pub fn new(direction: DwayUiDirection) -> Self {
        Self {
            direction,
            appear: true,
        }
    }

    pub fn open(&mut self, animation: &mut Animation) {
        self.appear = true;
        animation.play();
    }

    pub fn close(&mut self, animation: &mut Animation) {
        self.appear = false;
        animation.play();
    }
}

impl EventReceiver<AnimationEvent> for UiTranslationAnimation {
    fn on_event(&self, mut commands: EntityCommands, event: AnimationEvent) {
        let animation_progress = event.value;
        let v = match &self.appear {
            true => animation_progress,
            false => 1.0 - animation_progress,
        };

        let just_finish = event.just_finish;
        commands.queue(move |mut entity_mut: EntityWorldMut| {
            let Some(child_entity) = entity_mut
                .get::<Children>()
                .and_then(|c| c.first().cloned())
            else {
                return;
            };
            let Some(child_size) =
                entity_mut.world_scope(|world| world.get::<ComputedNode>(child_entity).map(ComputedNode::size))
            else {
                return;
            };
            let direction = entity_mut
                .get::<UiTranslationAnimation>()
                .unwrap()
                .direction;
            let target_layout = entity_mut.get::<AnimationTargetNodeState>().unwrap().0.clone();
            {
                let Some(mut layout) = entity_mut.get_mut::<Node>() else {
                    return;
                };
                *layout = target_layout.clone();
                if !just_finish {
                    let offset = Vec2::new(
                        if direction.contains(DwayUiDirection::LEFT) {
                            v - 1.0
                        } else if direction.contains(DwayUiDirection::RIGHT) {
                            1.0 - v
                        } else {
                            0.0
                        },
                        if direction.contains(DwayUiDirection::TOP) {
                            v - 1.0
                        } else if direction.contains(DwayUiDirection::BOTTOM) {
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

impl EventReceiver<UiNodeAppearEvent> for UiTranslationAnimation {
    fn on_event(&self, mut commands: EntityCommands, event: UiNodeAppearEvent) {
        commands.queue(move |mut entity_mut: EntityWorldMut| {
            {
                let mut this = entity_mut.get_mut::<Self>().unwrap();
                this.appear = event == UiNodeAppearEvent::Appear;
            }
            {
                let mut animation = entity_mut.get_mut::<Animation>().unwrap();
                animation.play_with_direction(AnimationDirection::new(event.appear()));
            }
        });
    }
}

impl DestroyInterceptor for UiTranslationAnimation {
    fn apply(&self, entity: &EntityRef, mut commands: Commands) -> bool {
        self.on_event(commands.entity(entity.id()), UiNodeAppearEvent::Disappear);
        true
    }
}

make_bundle! {
    @addon UiTranslationAnimationExt,
    UiTranslationAnimationBundle{
        pub translation: UiTranslationAnimation,
        pub animation: Animation,
        pub event_dispatcher: EventDispatcher<AnimationEvent>,
        pub target_style: AnimationTargetNodeState,
    }
}
