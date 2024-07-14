use bevy::render::render_resource::encase::internal::{BufferMut, Writer};

use super::{BuildBindGroup, Material, ShaderBuilder, ShaderVariables, Transformed};
use crate::prelude::*;
pub trait Transform: BuildBindGroup {
    fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables;

    fn then<R: Material>(self, material: R) -> Transformed<R, Self> {
        Transformed::new(material, self)
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct Translation {
    pub offset: Vec2,
}

impl Translation {
    pub fn new(offset: Vec2) -> Self {
        Self { offset }
    }
}
impl Transform for Translation {
    fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
        let ShaderVariables { pos, .. } = var;
        let uniform_offset = builder.get_uniform("offset", "", "vec2<f32>");
        ShaderVariables {
            pos: format!("({pos}-{uniform_offset})"),
            ..var.clone()
        }
    }
}
impl BuildBindGroup for Translation {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.offset);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.offset, writer);
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct Rotation {
    pub rotation: f32,
}

impl Rotation {
    pub fn new(rotation: f32) -> Self {
        Self { rotation }
    }
}
impl Transform for Rotation {
    fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
        let ShaderVariables { pos, .. } = var;
        builder.import_from_builtin("sdf_rotation");
        let uniform_rotation = builder.get_uniform("rotation", "", "f32");
        ShaderVariables {
            pos: format!("sdf_rotation({pos}, {uniform_rotation})"),
            ..var.clone()
        }
    }
}
impl BuildBindGroup for Rotation {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.rotation);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.rotation, writer);
    }
}

#[derive(Clone, Default, Debug, Interpolation)]
pub struct Margins {
    pub margins: Vec4,
}
impl From<Vec4> for Margins {
    fn from(value: Vec4) -> Self {
        Self { margins: value }
    }
}
impl Margins {
    pub fn all(value: f32) -> Self {
        Self {
            margins: Vec4::splat(value),
        }
    }

    pub fn axes(horizontal: f32, vertical: f32) -> Self {
        Self {
            margins: Vec4::new(horizontal, horizontal, vertical, vertical),
        }
    }

    pub fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            margins: Vec4::new(left, right, top, bottom),
        }
    }
}
impl Transform for Margins {
    fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
        let ShaderVariables { size, pos } = var;
        let uniform_margins = builder.get_uniform("margins", "", "vec4<f32>");
        ShaderVariables {
                size: format!("( {size} - vec2({uniform_margins}.x+{uniform_margins}.y,{uniform_margins}.z+{uniform_margins}.w) )"),
                pos: format!("( {pos} - 0.5 * vec2({uniform_margins}.x-{uniform_margins}.y,{uniform_margins}.z-{uniform_margins}.w) )"),
            }
    }
}
impl BuildBindGroup for Margins {
    fn update_layout(&self, layout: &mut super::UniformLayout) {
        layout.update_layout(&self.margins);
    }

    fn write_uniform<B: BufferMut>(
        &self,
        layout: &mut super::UniformLayout,
        writer: &mut Writer<B>,
    ) {
        layout.write_uniform(&self.margins, writer);
    }
}

macro_rules! impl_transform_for_tuple {
        () => { };
        ($first_elem:ident,$($elem:ident,)*) => {
            impl<$first_elem: Transform,$($elem: Transform),* > Transform for ($first_elem,$($elem),*){
                #[allow(non_snake_case)]
                fn transform(builder: &mut ShaderBuilder, var: &ShaderVariables) -> ShaderVariables {
                    let mut var = var.clone();
                    var = builder.in_new_namespace(stringify!($first_elem), |builder|$first_elem::transform(builder, &var).clone());
                    $( var = builder.in_new_namespace(stringify!($elem), |builder|$elem::transform(builder, &var).clone()); )*
                    var
                }
            }
            impl_transform_for_tuple!($($elem,)*);
        };
    }
impl_transform_for_tuple!(E8, E7, E6, E5, E4, E3, E2, E1, E0,);
