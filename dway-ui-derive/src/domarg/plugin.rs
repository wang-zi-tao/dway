use crate::prelude::*;

use super::DomDecorator;

#[derive(Parse)]
pub struct Plugin {
    #[call(Block::parse_within)]
    pub stmts: Vec<Stmt>,
}

impl DomDecorator for Plugin {
    fn key(&self) -> super::DomArgKey {
        super::DomArgKey::Plugin
    }
    fn update_context(&self, context: &mut WidgetNodeContext) {
        context
            .tree_context
            .plugin_builder
            .stmts
            .extend(self.stmts.iter().map(|s| s.to_token_stream()));
    }
}
