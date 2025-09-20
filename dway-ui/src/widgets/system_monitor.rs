use std::f32::consts::PI;

use dway_client_core::controller::systeminfo::{
    human_readable_byte, human_readable_fresequency, SystemInfo,
};
use dway_ui_framework::reexport::shape::*;

use crate::prelude::*;

#[derive(Component, SmartDefault)]
pub struct PanelSystemMonitor {
    #[default(0.8)]
    pub memory_threshold: f32,
    #[default(0.8)]
    pub global_cpu_threshold: f32,
}

dway_widget! {
PanelSystemMonitor=>
@state_reflect()
@use_state(memory_usage: f32)
@use_state(global_cpu_usage: f32)
@use_state(global_cpu_frequency: u64)
@use_state(cpu_usage: Vec<f32>)
@use_state(upload: u64)
@use_state(download: u64)
@arg(mut shape_query: Query<(&ComputedNode, &mut Shape)>)
@global(system_info: SystemInfo -> {
    state.set_memory_usage(1.0 - system_info.available_memory() as f32 / system_info.total_memory() as f32);
    state.set_global_cpu_usage(system_info.cpu_usage());
    state.set_global_cpu_frequency(system_info.cpu_frequency());
    state.set_cpu_usage(system_info.cpu_list().iter().map(|c|c.used).collect());
    state.set_upload(system_info.network_upload());
    state.set_download(system_info.network_download());
})
@global(theme:Theme)
<Node @style="h-full align-items:center">
    <Node @style="flex-col">
        <Node Text=(Text::new(format!("CPU {:.0}%",state.global_cpu_usage() * 100.0)))
            TextFont=(theme.text_font(12.0)) TextColor=(theme.color("foreground").into())
            TextLayout=(TextLayout::new_with_justify(JustifyText::Left) ) />
        <Node Text=(Text::new(human_readable_fresequency(*state.global_cpu_frequency())))
            TextFont=(theme.text_font(12.0)) TextColor=(theme.color("foreground").into())
            TextLayout=(TextLayout::new_with_justify(JustifyText::Left) ) />
    </Node>
    <Node @style="flex-col">
        <Node Text=(Text::new(format!("up {}",  human_readable_byte(*state.upload()))))
            TextFont=(theme.text_font(12.0)) TextColor=(theme.color("foreground").into())
            TextLayout=(TextLayout::new_with_justify(JustifyText::Left) ) />
        <Node Text=(Text::new( format!("down {}", human_readable_byte(*state.download()))))
            TextFont=(theme.text_font(12.0)) TextColor=(theme.color("foreground").into())
            TextLayout=(TextLayout::new_with_justify(JustifyText::Left) ) />
    </Node>
    <Node @style="h-full w-auto"
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 8.0))
    >
        <UiShape @id="cpu_shape"
        Shape
        Node=(Node{
            width: Val::Px(4.0 * state.cpu_usage().len() as f32),
            ..style!("h-full align-items:center")
        })
        @after_update{if state.cpu_usage_is_changed(){
            if let Ok((computed_node,mut shape)) = shape_query.get_mut(node!(cpu_shape)){
                let mut path = ShapePath::new();
                let size = computed_node.size();
                for (i,cpu) in state.cpu_usage().iter().enumerate() {
                    let w = 4.0;
                    let x = -0.5 * size.x + 0.5 * w + i as f32 * w;
                    path = path
                        .move_to(Vec2::new(x, -0.5 * size.y))
                        .line_to(Vec2::new(x, -0.5 * size.y + cpu * size.y));
                }
                *shape = ShapeBuilder::with(&path)
                    .stroke(Stroke {
                        options: StrokeOptions::default()
                            .with_end_cap(LineCap::Round)
                            .with_start_cap(LineCap::Square)
                            .with_line_width(4.0),
                        color: if *state.global_cpu_usage() > prop.global_cpu_threshold
                                {color!("#DF5B61")}
                            else {color!("#6791C9")},
                    }).build()
            }
        }}/>
        <UiShape @id="mem_shape" @style="h-full p-4 ratio-1.0 align-items:center"
        Shape=(ShapeBuilder::with(&ShapePath::new())
            .stroke(Stroke {
                options: StrokeOptions::default()
                    .with_end_cap(LineCap::Round)
                    .with_start_cap(LineCap::Round)
                    .with_line_width(4.0),
                color: if *state.memory_usage() > prop.memory_threshold
                        {color!("#DF5B61")}
                    else {color!("#6791C9")},
            }).build())
        @after_update{if state.memory_usage_is_changed(){
            if let Ok((node,mut shape)) = shape_query.get_mut(node!(mem_shape)){
                let size = node.size();
                let mut path = ShapePath::new()
                    .move_to(Vec2::new(0.0, -0.5 * size.y))
                    .arc(Vec2::ZERO, Vec2::splat(0.5*size.y), 2.0*PI * *state.memory_usage(), 1.0);
                *shape = ShapeBuilder::with(&ShapePath::new())
                    .stroke(Stroke {
                        options: StrokeOptions::default()
                            .with_end_cap(LineCap::Round)
                            .with_start_cap(LineCap::Round)
                            .with_line_width(4.0),
                        color: if *state.memory_usage() > prop.memory_threshold
                                {color!("#DF5B61")}
                            else {color!("#6791C9")},
                    }).build();
            }
        }}>
            <Node Text=(Text::new("mem")) TextFont=(theme.text_font(12.0)) TextColor=(theme.color("foreground").into()) />
        </>
    </Node>
</Node>
}
