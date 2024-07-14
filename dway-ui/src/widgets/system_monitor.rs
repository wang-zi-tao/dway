use std::f32::consts::PI;

use dway_client_core::controller::systeminfo::{human_readable_byte, human_readable_fresequency, SystemInfo};
use dway_ui_framework::widgets::shape::UiShapeBundle;
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
@arg(mut shape_query: Query<(&Node, &mut Path)>)
@global(system_info: SystemInfo -> {
    state.set_memory_usage(1.0 - system_info.available_memory() as f32 / system_info.total_memory() as f32);
    state.set_global_cpu_usage(system_info.cpu_usage());
    state.set_global_cpu_frequency(system_info.cpu_frequency());
    state.set_cpu_usage(system_info.cpu_list().iter().map(|c|c.used).collect());
    state.set_upload(system_info.network_upload());
    state.set_download(system_info.network_download());
})
@global(theme:Theme)
<MiniNodeBundle @style="h-full align-items:center">
    <MiniNodeBundle @style="flex-col">
        <TextBundle Text=(Text::from_section(format!("CPU {:.0}%",state.global_cpu_usage() * 100.0), theme.text_style(12.0, "foreground"),).with_no_wrap())/>
        <TextBundle Text=(Text::from_section(human_readable_fresequency(*state.global_cpu_frequency()), theme.text_style(12.0, "foreground"),).with_no_wrap())/>
    </MiniNodeBundle>
    <MiniNodeBundle @style="flex-col">
        <TextBundle Text=(Text::from_section( format!("up {}",  human_readable_byte(*state.upload())), theme.text_style(12.0, "foreground"),).with_no_wrap())/>
        <TextBundle Text=(Text::from_section( format!("down {}", human_readable_byte(*state.download())), theme.text_style(12.0, "foreground"),).with_no_wrap())/>
    </MiniNodeBundle>
    <MiniNodeBundle @style="h-full w-auto"
        @material(RoundedUiRectMaterial=>rounded_rect(theme.color("panel-popup1"), 8.0))
    >
        <UiShapeBundle @id="cpu_shape"
        Style=(Style{
            width: Val::Px(4.0 * state.cpu_usage().len() as f32),
            ..style!("h-full align-items:center")
        })
        Stroke=(Stroke {
            options: StrokeOptions::default()
                .with_end_cap(LineCap::Round)
                .with_start_cap(LineCap::Square)
                .with_line_width(4.0),
            color: if *state.global_cpu_usage() > prop.global_cpu_threshold
                    {color!("#DF5B61")}
                else {color!("#6791C9")},
        })
        @after_update{if state.cpu_usage_is_changed(){
            if let Ok((node,mut path)) = shape_query.get_mut(node!(cpu_shape)){
                let mut builder = PathBuilder::new();
                let size = node.size();
                for (i,cpu) in state.cpu_usage().iter().enumerate() {
                    let w = 4.0;
                    let x = -0.5 * size.x + 0.5 * w + i as f32 * w;
                    builder.move_to(Vec2::new(x, -0.5 * size.y));
                    builder.line_to(Vec2::new(x, -0.5 * size.y + cpu * size.y));
                }
                *path = builder.build();
            }
        }}/>
        <UiShapeBundle @id="mem_shape" @style="h-full p-4 ratio-1.0 align-items:center"
        Stroke=(Stroke {
            options: StrokeOptions::default()
                .with_end_cap(LineCap::Round)
                .with_start_cap(LineCap::Round)
                .with_line_width(4.0),
            color: if *state.memory_usage() > prop.memory_threshold
                    {color!("#DF5B61")}
                else {color!("#6791C9")},
        })
        @after_update{if state.memory_usage_is_changed(){
            if let Ok((node,mut path)) = shape_query.get_mut(node!(mem_shape)){
                let mut builder = PathBuilder::new();
                let size = node.size();
                builder.move_to(Vec2::new(0.0, -0.5 * size.y));
                builder.arc(Vec2::ZERO, Vec2::splat(0.5*size.y), 2.0*PI * *state.memory_usage(), 1.0);
                *path = builder.build();
            }
        }}>
            <TextBundle Text=(Text::from_section( "mem", theme.text_style(12.0, "foreground"),))/>
        </>
    </MiniNodeBundle>
</MiniNodeBundle>
}
