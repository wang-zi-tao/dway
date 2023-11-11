#![feature(iter_map_windows)]

mod generate;
mod style;

use derive_syn_parse::Parse;
use generate::*;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use std::collections::HashMap;
use syn::{
    parse::ParseStream,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{At, Brace, Paren, RArrow},
    *,
};

#[derive(Parse)]
struct Input {
    name: Ident,
    _colon: Option<Token![:]>,
    #[parse_if(_colon.is_some())]
    ty: Option<Type>,
}

#[derive(Parse)]
struct Output {
    name: Ident,
}

struct DomState<'l> {
    widget_name: &'l Ident,
    system_params: &'l mut HashMap<String, TokenStream2>,
    items: &'l mut Vec<TokenStream2>,
    pre_build: &'l mut Vec<TokenStream2>,
    widget_fields: &'l mut HashMap<String, (TokenStream2, TokenStream2)>,
}
impl<'l> DomState<'l> {
    pub fn get_dom_id(&mut self, dom: &Dom, upper_case: bool) -> Ident {
        if let Some(DomArg::Instruction(_, DomInstruction::Id(_, _, lit))) =
            dom.args.args.get("@id")
        {
            format_ident!("{}", lit.value(), span = lit.span())
        } else {
            if upper_case {
                format_ident!("N{}", self.widget_fields.len(), span = dom._lt0.span)
            } else {
                format_ident!("n{}", self.widget_fields.len(), span = dom._lt0.span)
            }
        }
    }
    pub fn add_ui_state_field(&mut self, dom: &Dom) -> Ident {
        let id = self.get_dom_id(dom, false);
        let ident = format_ident!("node_{}_entity", id, span = id.span());
        self.widget_fields.insert(
            ident.to_string(),
            (quote!(#ident: Entity), quote!(#ident: Entity::PLACEHOLDER)),
        );
        ident
    }
    pub fn add_ui_state_map_field(&mut self, dom: &Dom, key_type: &TokenStream2) -> Ident {
        let id = self.get_dom_id(dom, false);
        let ident = format_ident!("node_{}_entity_map", id, span = id.span());
        self.widget_fields.insert(
            ident.to_string(),
            (
                quote!(#ident: bevy::utils::HashMap<#key_type, Entity>),
                quote!(#ident: bevy::utils::HashMap::new()),
            ),
        );
        ident
    }
    pub fn add_loop_state_query(&mut self, dom: &Dom) -> (Ident, Ident) {
        let id = self.get_dom_id(dom, true);
        let widget_state_type_name = format_ident!("{}SubWidget{id}", &self.widget_name);
        let query_name = format_ident!("sub_widget_{id}_query");
        self.system_params.insert(
            query_name.to_string(),
            quote!(mut #query_name: Query<&mut #widget_state_type_name>),
        );
        (query_name, widget_state_type_name)
    }
}

#[derive(Parse)]
enum Bundle {
    #[peek(Paren, name = "Paren")]
    Expr {
        #[paren]
        _wrap: Paren,
        #[inside(_wrap)]
        expr: Expr,
    },
    #[peek(Ident, name = "Ident")]
    Ident(Type),
}
impl ToTokens for Bundle {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Bundle::Ident(ty) => tokens.extend(quote!(#ty::default())),
            Bundle::Expr { expr, .. } => tokens.extend(quote!(#expr)),
        }
    }
}

#[derive(Parse)]
struct Dom {
    _lt0: Token![<],
    bundle: Bundle,
    args: DomArguments,
    _end0: Option<Token![/]>,
    _gt0: Token![>],
    #[parse_if(_end0.is_none())]
    children: Option<DomChildren>,
    #[parse_if(_end0.is_none())]
    end_tag: Option<TagEnd>,
}
impl Dom {
    pub fn generate(&self, output: &mut DomState) -> TokenStream2 {
        let mut bundle_state = BlockState::default();
        if let Bundle::Expr { expr, .. } = &self.bundle {
            parse_expr(expr, &mut bundle_state);
        }
        let if_instruction = if let Some(DomArg::Instruction(_, DomInstruction::If(_, condition))) =
            self.args.args.get("@if")
        {
            Some(condition)
        } else {
            None
        };
        let for_instruction =
            if let Some(DomArg::Instruction(_, DomInstruction::For { pat, expr, .. })) =
                self.args.args.get("@for")
            {
                Some((pat, expr))
            } else {
                None
            };
        let key_instruction =
            if let Some(DomArg::Instruction(_, DomInstruction::Key { expr, ty, .. })) =
                self.args.args.get("@key")
            {
                Some((expr, ty))
            } else {
                None
            };

        let init_entity_expr = {
            let expr = &self.bundle;
            let init_expr = self
                .args
                .args
                .iter()
                .filter_map(|(_, arg)| {
                    if let DomArg::Component {
                        expr, component, ..
                    } = arg
                    {
                        Some(quote!({ let component: #component = #expr; component }))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            let mut spawn = match expr {
                Bundle::Expr {
                    expr: Expr::Tuple(inner),
                    ..
                } if inner.elems.is_empty() => {
                    quote!(commands.spawn_empty())
                }
                expr => {
                    quote!(commands.spawn(#expr))
                }
            };
            if !init_expr.is_empty() {
                spawn = quote!(#spawn.insert((#(#init_expr),*)));
            }
            quote! {#spawn.id()}
        };

        let mut need_update = false;
        let update_stats = {
            if let Some(DomArg::Instruction(_, DomInstruction::Id(_, _, _))) =
                self.args.args.get("@id")
            {
                need_update = true;
            }
            if for_instruction.is_some() {
                need_update = true;
            }
            let bundle_update_condition = bundle_state.generate_condition();
            let bundle_update_stats = if bundle_state.use_state.is_empty()
                && bundle_state.set_state.is_empty()
                && if_instruction.is_none()
            {
                quote!()
            } else {
                let bundle_expr = &self.bundle;
                let bundle_type = self
                    .end_tag
                    .as_ref()
                    .and_then(|end| end.end_bundle.as_ref())
                    .map(|ty| quote!(: #ty));
                need_update = true;
                quote! {
                    if #bundle_update_condition{
                        let bundle #bundle_type = #bundle_expr;
                        commands.entity(node_entity).insert(bundle);
                    }
                }
            };
            let mut update_stats = vec![bundle_update_stats];
            for (_component, arg) in self.args.args.iter() {
                if let DomArg::Component {
                    expr, component, ..
                } = arg
                {
                    let mut block_state = BlockState::default();
                    parse_expr(expr, &mut block_state);
                    if !block_state.use_state.is_empty()
                        || !block_state.set_state.is_empty()
                        || if_instruction.is_some()
                    {
                        need_update = true;
                        let component_expr = expr;
                        let condition = block_state.generate_condition();
                        let span = match arg {
                            DomArg::Component { _eq, .. } => _eq.span(),
                            DomArg::Instruction(at, _) => at.span(),
                        };
                        update_stats.push(quote_spanned! {span=>
                            if #bundle_update_condition || #condition{
                                let component: #component = #component_expr;
                                commands.entity(node_entity).insert(component);
                            }
                        });
                    };
                }
            }
            quote! {#(#update_stats)*}
        };

        let mut dom_entity_field = None;

        let update_or_init_stat = if need_update {
            let dom_entity_field =
                dom_entity_field.get_or_insert_with(|| output.add_ui_state_field(self));

            let spawn_condition =
                quote!(not_inited && widget.#dom_entity_field == Entity::PLACEHOLDER);
            let despawn_state = generate_despawn(quote!(widget.#dom_entity_field));

            let update_or_init_stat = quote! {
                match (enable_widget,not_inited) {
                    (true,true) => {
                        let node_entity: Entity = {
                            #init_entity_expr
                        };
                        widget.#dom_entity_field = node_entity;
                        node_entity
                    },
                    (true,false) => {
                        let node_entity: Entity = widget.#dom_entity_field;
                        #update_stats;
                        Entity::PLACEHOLDER
                    }
                    (false,false) => {
                        #despawn_state
                        widget.#dom_entity_field = Entity::PLACEHOLDER;
                        Entity::PLACEHOLDER
                    }
                    _=>{
                        Entity::PLACEHOLDER
                    }
                }
            };
            let update_condition = if_instruction.as_ref().map(|if_condition| {
                let condition_expr = if_condition;
                let mut condition_block_state = BlockState::default();
                let condition_update_expr = condition_block_state.generate_condition();
                parse_expr(condition_expr, &mut condition_block_state);
                Some(quote! {
                    let enable_widget = if #condition_update_expr {
                        #condition_expr
                    } else {
                        enable_widget
                    };
                })
            });
            quote! {
                {
                    let not_inited = #spawn_condition;
                    #update_condition
                    #update_or_init_stat
                }
            }
        } else {
            quote! {
                if not_inited && enable_widget {
                    #init_entity_expr
                } else {
                    Entity::PLACEHOLDER
                }
            }
        };

        let stats = if let Some((patten, iterator)) = for_instruction {
            let dom_entity_field =
                dom_entity_field.get_or_insert_with(|| output.add_ui_state_field(self));
            let (sub_widget_query, sub_widget_type) = output.add_loop_state_query(self);
            let (key_expr, key_type) = if let Some((key_expr, key_type)) = key_instruction {
                (quote!(#key_expr), quote!(#key_type))
            } else {
                (quote!(index), quote!(usize))
            };
            let dom_entity_map_field = output.add_ui_state_map_field(self, &key_type);
            let despawn_disabled = generate_despawn(quote!(node_entity));
            let despawn_removed = generate_despawn(quote!(node_entity));

            let mut update_or_init_children_stat = None;
            {
                let mut sub_widget_fields = HashMap::new();
                let mut child_output = DomState {
                    widget_name: output.widget_name,
                    system_params: output.system_params,
                    items: output.items,
                    pre_build: output.pre_build,
                    widget_fields: &mut sub_widget_fields,
                };
                for child in self.children.iter().flat_map(|c| c.list.iter()) {
                    if update_or_init_children_stat.is_some() {
                        panic!("node with `for` instruction can only has one child");
                    }
                    update_or_init_children_stat = Some(child.generate(&mut child_output));
                }
                let sub_widget_field_decl = sub_widget_fields.values().map(|w| &w.0);
                let sub_widget_init = sub_widget_fields.values().map(|w| &w.1);
                output.items.push(quote! {
                    #[allow(non_snake_case)]
                    #[allow(unused_variables)]
                    #[derive(Component)]
                    pub struct #sub_widget_type {
                        #(#sub_widget_field_decl),*
                    }
                    impl Default for #sub_widget_type {
                        fn default() -> Self{
                            Self{
                                #(#sub_widget_init),*
                            }
                        }
                    }
                });
            }
            quote_spanned! {self._lt0.span()=>
                {
                    let node_entity = #update_or_init_stat;
                    let children_map = &mut widget.#dom_entity_map_field;
                    let mut new_children_map = bevy::utils::HashMap::<#key_type, Entity>::new();
                    let mut children = Vec::new();
                    for old_child in children_map.values() {
                        commands.entity(*old_child).remove_parent();
                    }
                    for (index,#patten) in Iterator::enumerate(#iterator) {
                        let key = #key_expr;
                        match (enable_widget,children_map.remove(&key)) {
                            (true,Some(node_entity)) => {
                                let not_inited = false;
                                if let Ok(mut widget) = #sub_widget_query.get_mut(node_entity) {
                                    #update_or_init_children_stat;
                                }
                                new_children_map.insert(key, node_entity);
                                children.push(node_entity);
                            },
                            (true,None) => {
                                let mut widget = #sub_widget_type::default();
                                let not_inited = true;
                                let node_entity: Entity = {
                                    #update_or_init_children_stat
                                };
                                new_children_map.insert(key, node_entity);
                                children.push(node_entity);
                                commands.entity(node_entity).insert(widget);
                            },
                            (false,Some(node_entity)) => {
                                #despawn_disabled
                            },
                            _=>{}
                        }
                    }
                    for old_child in children_map.values() {
                        #despawn_removed
                    }
                    for child in children.iter() {
                        commands.entity(*child).set_parent(widget.#dom_entity_field);
                    }
                    widget.#dom_entity_map_field = new_children_map;
                    node_entity
                }
            }
        } else {
            let mut update_or_init_children_stat = vec![];
            for child in self.children.iter().flat_map(|c| c.list.iter()) {
                let update_or_init_child = child.generate(output);
                update_or_init_children_stat.push(quote! {
                    let child_entity = #update_or_init_child;
                    if child_entity != Entity::PLACEHOLDER {
                        commands.entity(child_entity).set_parent(node_entity);
                    }
                });
            }
            quote! {
                {
                    let node_entity = #update_or_init_stat;
                    #(#update_or_init_children_stat)*
                    node_entity
                }
            }
        };

        stats
    }
}

#[derive(Parse)]
struct TagEnd {
    _lt1: Token![<],
    _end1: Token![/],
    end_bundle: Option<Ident>,
    _gt1: Token![>],
}

struct DomArguments {
    args: HashMap<String, DomArg>,
}

impl syn::parse::Parse for DomArguments {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut args = HashMap::new();
        while !input.peek(Token![>]) && !input.peek(Token![/]) {
            let mut arg: DomArg = input.parse()?;
            let name = match &arg {
                DomArg::Component { component, .. } => quote!(#component).to_string(),
                DomArg::Instruction(_, DomInstruction::If(..)) => "@if".to_string(),
                DomArg::Instruction(_, DomInstruction::Style(..)) => "Style".to_string(),
                DomArg::Instruction(_, DomInstruction::Id(..)) => "@id".to_string(),
                DomArg::Instruction(_, DomInstruction::For { .. }) => "@for".to_string(),
                DomArg::Instruction(_, DomInstruction::Key { .. }) => "@key".to_string(),
            };
            if let DomArg::Instruction(_, DomInstruction::Style(_, _, lit)) = &arg {
                let value_tokens = style::generate(lit);
                let expr_tokens = quote!(Style=(#value_tokens));
                arg = syn::parse2(expr_tokens).unwrap();
            }
            args.insert(name, arg);
        }
        Ok(Self { args })
    }
}

#[derive(Parse)]
enum DomArg {
    #[peek(Ident, name = "Component")]
    Component {
        component: Type,
        _eq: Token![=],
        #[paren]
        _wrap: Paren,
        #[inside(_wrap)]
        expr: Expr,
    },
    #[peek(At, name = "Instruction")]
    Instruction(Token![@], DomInstruction),
}
enum DomInstruction {
    If(Token![if], Expr),
    Id(Ident, Token![=], LitStr),
    Style(Ident, Token![=], LitStr),
    For {
        _opcode: Token![for],
        _paren: Paren,
        pat: syn::Pat,
        _in: Token![in],
        expr: Expr,
    },
    Key {
        _opcode: Ident,
        _paren: Paren,
        expr: Expr,
        _as: Token![:],
        ty: Type,
    },
}
impl syn::parse::Parse for DomInstruction {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![if]) {
            Ok(Self::If(input.parse()?, input.parse()?))
        } else if input.peek(Token![for]) {
            let content;
            Ok(Self::For {
                _opcode: input.parse()?,
                _paren: parenthesized!(content in input),
                pat: Pat::parse_multi(&content)?,
                _in: content.parse()?,
                expr: content.parse()?,
            })
        } else {
            let instruction: Ident = input.parse()?;
            match &*instruction.to_string() {
                "key" => {
                    let content;
                    Ok(Self::Key {
                        _opcode: instruction,
                        _paren: parenthesized!(content in input),
                        expr: content.parse()?,
                        _as: content.parse()?,
                        ty: content.parse()?,
                    })
                }
                "id" => Ok(Self::Id(instruction, input.parse()?, input.parse()?)),
                "style" => Ok(Self::Style(instruction, input.parse()?, input.parse()?)),
                other => {
                    panic!(
                        "unsupported instruction: {other}, known instructions: {:?}",
                        ["if", "for", "key", "id"]
                    );
                }
            }
        }
    }
}

struct DomChildren {
    list: Vec<Dom>,
}
impl syn::parse::Parse for DomChildren {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut list = vec![];
        while !input.peek(Token![<]) || !input.peek2(Token![/]) {
            list.push(input.parse()?);
        }
        Ok(Self { list })
    }
}

#[derive(Parse)]
struct Stage {
    #[peek(Paren)]
    inputs: Option<StageInputs>,
    _output_split: Option<Token![->]>,
    #[parse_if(_output_split.is_some())]
    #[paren]
    _output_bracket: Option<Paren>,
    #[peek(RArrow)]
    outputs: Option<StageOutputs>,
    functoin: Block,
}

#[derive(Parse)]
struct StageInputs {
    #[paren]
    _wrap: Paren,
    #[inside(_wrap)]
    #[call(Punctuated::parse_terminated)]
    inputs: Punctuated<Input, Token![,]>,
}

#[derive(Parse)]
struct StageOutputs {
    _output_split: Token![->],
    #[paren]
    _output_bracket: Paren,
    #[inside(_output_bracket)]
    #[call(Punctuated::parse_terminated)]
    output: Punctuated<Output, Token![,]>,
}

#[derive(Parse)]
struct Params {
    #[paren]
    _paren_token: Paren,
    #[inside(_paren_token)]
    #[call(Punctuated::parse_terminated)]
    inputs: Punctuated<FnArg, Token![,]>,
}

#[derive(Parse)]
struct DWayWidget {
    name: Ident,
    #[peek(Paren)]
    params: Option<Params>,
    #[call(Attribute::parse_outer)]
    attributes: Vec<Attribute>,
    #[peek(Brace)]
    states: Option<States>,
    _split: Token![=>],
    #[call(parse_stages)]
    stages: Vec<Stage>,
    ui: Dom,
}

#[derive(Parse)]
struct States {
    #[brace]
    _wrap: Brace,
    #[inside(_wrap)]
    #[call(Punctuated::parse_terminated)]
    fields: Punctuated<DWayStateField, Token![,]>,
}

fn parse_stages(input: ParseStream) -> Result<Vec<Stage>> {
    let mut stages = vec![];
    while !input.peek(Token![<]) {
        stages.push(input.parse()?);
    }
    Ok(stages)
}

#[derive(Parse)]
struct DWayStateField {
    #[call(Field::parse_named)]
    field: Field,
}

#[derive(Default, Debug)]
struct BlockState {
    use_state: HashMap<String, Span>,
    set_state: HashMap<String, Span>,
}
impl BlockState {
    pub fn add(&mut self, ident: &Ident) {
        self.use_state.insert(ident.to_string(), ident.span());
        self.set_state.insert(ident.to_string(), ident.span());
    }
    pub fn use_state(&mut self, ident: &Ident) {
        self.use_state.insert(ident.to_string(), ident.span());
    }
    pub fn generate_condition(&self) -> TokenStream2 {
        if self.use_state.is_empty() {
            quote!(true)
        } else {
            let exprs = self
                .use_state
                .iter()
                .map(|(name, span)| generate_state_change_variable_from_raw(name, *span));
            quote!((#(#exprs)&*))
        }
    }
}

fn on_parse_field(f: &ExprField, output: &mut BlockState, is_mut: bool) -> bool {
    if let Expr::Path(p) = &*f.base {
        if p.path.is_ident("state") {
            match &f.member {
                Member::Named(n) => {
                    if is_mut {
                        output.add(n);
                    } else {
                        output.use_state(n);
                    }
                    return true;
                }
                Member::Unnamed(_) => {}
            };
        }
    }
    false
}

fn on_parse_assign(i: &ExprAssign, output: &mut BlockState, is_mut: bool) -> bool {
    if let Expr::Field(f) = &*i.left {
        return on_parse_field(f, output, is_mut);
    }
    false
}

fn on_parse_reference(i: &ExprReference, output: &mut BlockState) -> bool {
    if let Expr::Field(f) = &*i.expr {
        return on_parse_field(f, output, i.mutability.is_some());
    }
    false
}

fn parse_expr_tokens(tokens: &TokenStream2, output: &mut BlockState) {
    tokens.clone().into_iter().for_each(|token| {
        match &token {
            TokenTree::Group(g) => {
                parse_expr_tokens(&g.stream(), output);
            }
            _ => {}
        };
    });
    tokens
        .clone()
        .into_iter()
        .map_windows(|[base, dot, member]| {
            match (base, dot, member) {
                (
                    TokenTree::Ident(base_ident),
                    TokenTree::Punct(dot_punct),
                    TokenTree::Ident(member_ident),
                ) if base_ident.to_string() == "state" && dot_punct.as_char() == '.' => {
                    output.add(member_ident);
                }
                _ => {}
            };
        })
        .for_each(|_| {});
}

fn parse_expr(expr: &Expr, output: &mut BlockState) {
    match expr {
        Expr::Array(e) => e.elems.iter().for_each(|e| parse_expr(e, output)),
        Expr::Assign(i) => {
            parse_expr(&i.right, output);
            if !on_parse_assign(i, output, true) {
                parse_expr(&i.left, output)
            };
        }
        Expr::Async(i) => parse_block(&i.block, output),
        Expr::Await(i) => parse_expr(&i.base, output),
        Expr::Binary(i) => {
            parse_expr(&i.left, output);
            parse_expr(&i.right, output);
        }
        Expr::Block(b) => parse_block(&b.block, output),
        Expr::Break(b) => {
            b.expr.as_ref().map(|e| parse_expr(e, output));
        }
        Expr::Call(i) => {
            parse_expr(&i.func, output);
            i.args.iter().for_each(|e| parse_expr(e, output));
        }
        Expr::Cast(i) => parse_expr(&i.expr, output),
        Expr::Closure(c) => parse_expr(&c.body, output),
        Expr::Const(c) => parse_block(&c.block, output),
        Expr::Continue(_) => {}
        Expr::Field(f) => {
            parse_expr(&f.base, output);
            on_parse_field(f, output, true);
        }
        Expr::ForLoop(i) => {
            parse_expr(&i.expr, output);
            parse_block(&i.body, output);
        }
        Expr::Group(i) => parse_expr(&i.expr, output),
        Expr::If(i) => {
            parse_expr(&i.cond, output);
            parse_block(&i.then_branch, output);
            i.else_branch.as_ref().map(|b| parse_expr(&b.1, output));
        }
        Expr::Index(i) => {
            parse_expr(&i.expr, output);
            parse_expr(&i.index, output);
        }
        Expr::Infer(_) => {}
        Expr::Let(i) => {
            parse_expr(&i.expr, output);
        }
        Expr::Lit(_) => {}
        Expr::Loop(i) => {
            parse_block(&i.body, output);
        }
        Expr::Macro(i) => {
            if i.mac.path.is_ident("state") {
                output.use_state(&syn::parse2::<Ident>(i.mac.tokens.clone()).unwrap());
            } else {
                parse_expr_tokens(&i.mac.tokens, output);
            }
        }
        Expr::Match(i) => {
            parse_expr(&i.expr, output);
            i.arms.iter().for_each(|arm| {
                arm.guard.as_ref().map(|e| parse_expr(&e.1, output));
                parse_expr(&arm.body, output);
            });
        }
        Expr::MethodCall(c) => {
            parse_expr(&c.receiver, output);
            c.args.iter().for_each(|e| parse_expr(e, output));
        }
        Expr::Paren(i) => parse_expr(&i.expr, output),
        Expr::Path(_) => {}
        Expr::Range(i) => {
            i.start.as_ref().map(|e| parse_expr(&e, output));
            i.end.as_ref().map(|e| parse_expr(&e, output));
        }
        Expr::Reference(i) => {
            if !on_parse_reference(i, output) {
                parse_expr(&i.expr, output);
            };
        }
        Expr::Repeat(i) => {
            parse_expr(&i.expr, output);
            parse_expr(&i.len, output);
        }
        Expr::Return(i) => {
            i.expr.as_ref().map(|e| parse_expr(&e, output));
        }
        Expr::Struct(e) => {
            e.rest.as_ref().map(|e| parse_expr(e, output));
            e.fields.iter().for_each(|f| parse_expr(&f.expr, output));
        }
        Expr::Try(i) => parse_expr(&i.expr, output),
        Expr::TryBlock(b) => parse_block(&b.block, output),
        Expr::Tuple(i) => {
            i.elems.iter().for_each(|e| parse_expr(e, output));
        }
        Expr::Unary(i) => parse_expr(&i.expr, output),
        Expr::Unsafe(i) => parse_block(&i.block, output),
        Expr::Verbatim(i) => parse_expr_tokens(i, output),
        Expr::While(i) => {
            parse_expr(&i.cond, output);
            parse_block(&i.body, output);
        }
        Expr::Yield(i) => {
            i.expr.as_ref().map(|e| parse_expr(&e, output));
        }
        _ => {}
    }
}

fn parse_stmt(stmt: &Stmt, output: &mut BlockState) {
    match stmt {
        Stmt::Local(l) => {
            if let Some(init) = &l.init {
                parse_expr(&init.expr, output);
            }
        }
        Stmt::Expr(expr, _) => {
            parse_expr(expr, output);
        }
        Stmt::Item(_) => {}
        Stmt::Macro(_) => {}
    }
}

fn parse_block(block: &Block, output: &mut BlockState) {
    for stmt in &block.stmts {
        parse_stmt(stmt, output);
    }
}

#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    let lit = parse_macro_input!(input as LitStr);
    let style = style::generate(&lit);
    TokenStream::from(quote_spanned!(lit.span()=> #style))
}

#[proc_macro]
pub fn state_mut(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Ident);
    let state_name = generate_state_change_variable(&input);
    TokenStream::from(quote!(
        {
            #state_name = true;
            &mut state.#input
        }
    ))
}

#[derive(Parse)]
struct SetStateInput {
    ident: Ident,
    _eq: Token![=],
    expr: Expr,
}

#[proc_macro]
pub fn set_state(input: TokenStream) -> TokenStream {
    let SetStateInput { ident, _eq, expr } = parse_macro_input!(input as SetStateInput);
    let state_name = generate_state_change_variable(&ident);
    TokenStream::from(quote!(
        {
            #state_name = true;
            state.#ident = #expr;
        }
    ))
}

#[proc_macro]
pub fn update_state(input: TokenStream) -> TokenStream {
    let SetStateInput { ident, _eq, expr } = parse_macro_input!(input as SetStateInput);
    let state_name = generate_state_change_variable(&ident);
    TokenStream::from(quote!(
        {
            let value = #expr;
            if state.#ident != value {
                #state_name = true;
                state.#ident = value;
            }
        }
    ))
}

#[proc_macro]
pub fn state_changed(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Ident);
    let state_name = generate_state_change_variable(&input);
    TokenStream::from(quote!( { #state_name }))
}

#[proc_macro]
pub fn node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Ident);
    let ident = format_ident!("node_{}_entity", input, span = input.span());
    TokenStream::from(quote!( { widget.#ident }))
}

#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as Ident);
    let state_name = generate_state_change_variable(&input);
    TokenStream::from(quote!( { let _ = &#state_name; &state.#input }))
}

#[proc_macro]
pub fn dway_widget(input: TokenStream) -> TokenStream {
    let dsl = parse_macro_input!(input as DWayWidget);
    let function_name = format_ident!(
        "{}_render",
        &dsl.name.to_string().to_lowercase(),
        span = dsl.name.span()
    );
    let mut input_map = HashMap::<String, Vec<usize>>::new();
    for (index, stage) in dsl.stages.iter().enumerate() {
        for input in stage.inputs.iter().flat_map(|e| e.inputs.iter()) {
            input_map
                .entry(input.name.to_string())
                .or_default()
                .push(index);
        }
    }

    let mut function_args: HashMap<String, TokenStream2> = HashMap::new();
    if let Some(args) = &dsl.params {
        for (index, arg) in args.inputs.iter().enumerate() {
            function_args.insert(format!("__param_{index}"), quote!(#arg));
        }
    }
    let mut items = Vec::new();
    let this_query: Vec<TokenStream2> = Vec::new();
    let mut widget_fields: HashMap<String, (TokenStream2, TokenStream2)> = HashMap::new();
    let mut declares: Vec<TokenStream2> = Vec::new();
    let mut run_stage: Vec<TokenStream2> = Vec::new();
    let mut bundle_fields: HashMap<String, TokenStream2> = HashMap::new();
    widget_fields.insert(
        "inited".to_string(),
        (quote!(inited: bool), quote!(inited: false)),
    );
    let mut conditions = quote!(not_inited);

    for field in dsl.states.iter().flat_map(|f| f.fields.iter()) {
        let ident = generate_state_change_variable(&field.field.ident.as_ref().unwrap());
        declares.push(quote! {let mut #ident = false;});
        conditions = quote!(#conditions || #ident);
    }
    for stage in dsl.stages.iter() {
        let mut check_input_exprs = vec![];
        let mut block_stat: BlockState = Default::default();
        parse_block(&stage.functoin, &mut block_stat);
        for (state_name, span) in block_stat.use_state.iter() {
            let state_name = generate_state_change_variable_from_raw(state_name, *span);
            check_input_exprs.push(quote!(#state_name));
        }
        for (state_name, span) in block_stat.use_state.iter() {
            let state_name = generate_state_change_variable_from_raw(state_name, *span);
            check_input_exprs.push(quote!(#state_name));
        }
        for input in stage.inputs.iter().flat_map(|l| l.inputs.iter()) {
            let state_name = generate_state_change_variable(&input.name);
            check_input_exprs.push(quote!(#state_name));
            let mut state_changed_expr = quote!(false);
            if let Some(ty) = &input.ty {
                let name = &input.name;
                match ty {
                    Type::Reference(reference) => {
                        if reference.mutability.is_some() {
                            state_changed_expr = quote!(#name.is_changed());
                        }
                    }
                    Type::Path(path) => {
                        let segments = &path.path.segments;
                        if segments.len() != 1 {
                            continue;
                        }
                        let template_name = segments[0].ident.to_string();
                        match &*template_name {
                            "Res" | "ResMut" | "Mut" | "NonSendMut" | "Ref" => {
                                state_changed_expr = quote!(#name.is_changed());
                            }
                            "Option" => {
                                state_changed_expr =
                                    quote!(#name.map(|i|i.is_changed()).unwrap_or(false));
                            }
                            _ => {}
                        }
                        let arg_ty = match &segments[0].arguments {
                            PathArguments::AngleBracketed(a) => a.args.iter().next().unwrap(),
                            _ => panic!("unsupported argument: {:?}", &segments[0].arguments),
                        };
                        let arg_ty = match arg_ty {
                            GenericArgument::Type(t) => t,
                            _ => panic!("unsupported argument: {:?}", arg_ty),
                        };
                        match &*template_name {
                            "Res" | "ResMut" | "NonSendMut" => {
                                function_args.insert(name.to_string(), quote! {#name: #ty});
                                declares.push(quote! {let mut #state_name = false;});
                            }
                            "Ref" | "Mut" | "Option" => {
                                let name = format!("arg_{:?}", arg_ty);
                                let name = name.replace('_', "__");
                                let name = name.replace(
                                    |char| {
                                        !(char == '_'
                                            || char >= '0' && char <= '9'
                                            || char >= 'A' && char <= 'Z'
                                            || char >= 'a' && char <= 'z')
                                    },
                                    "__",
                                );
                                let ident = Ident::new(&name, ty.span());
                                bundle_fields.insert(name.to_string(), quote! {#ident: #arg_ty});
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }

            declares.push(quote!(#state_name = #state_changed_expr;));
        }
        let check_input_expr = if check_input_exprs.is_empty() {
            quote!(true)
        } else {
            check_input_exprs.iter().fold(
                quote!(update_all),
                |init, expr: &TokenStream2| quote!(#init || #expr),
            )
        };
        let enable_stage_stats = stage
            .outputs
            .iter()
            .flat_map(|e| e.output.iter())
            .map(|output| {
                let state_name = generate_state_change_variable(&output.name);
                quote! {#state_name = true;}
            })
            .chain(block_stat.set_state.iter().map(|(state_name, span)| {
                let state_name = generate_state_change_variable_from_raw(state_name, *span);
                quote! {#state_name = true;}
            }))
            .collect::<Vec<_>>();
        let inner = &stage.functoin;
        run_stage.push(quote! {
            if #check_input_expr {
                #inner
                #(#enable_stage_stats)*
            }
        });
    }
    {
        let dom = &dsl.ui;
        let mut dom_state = DomState {
            system_params: &mut function_args,
            pre_build: &mut declares,
            widget_fields: &mut widget_fields,
            widget_name: &dsl.name,
            items: &mut items,
        };
        let update_or_init_stat = dom.generate(&mut dom_state);

        run_stage.push(quote! {
            let not_inited = !widget.inited;
            let enable_widget = true;
            if #conditions {
                let ui_entity = #update_or_init_stat;
                if ui_entity != Entity::PLACEHOLDER {
                    commands.entity(ui_entity).set_parent(this_entity);
                }
            }
            widget.inited = true;
        });
    }
    let widget_field_decl = widget_fields.values().map(|f| &f.0);
    let widget_field_init = widget_fields.values().map(|w| &w.1);
    let bundle_fields_init = bundle_fields.iter().map(|(name, _)| quote!(#name,));
    let bundle_fields: Vec<_> = bundle_fields.values().collect();
    let function_args = function_args.values();
    let state_component = format_ident!("{}State", &dsl.name, span = dsl.name.span());
    let widget_component = format_ident!("{}Widget", &dsl.name, span = dsl.name.span());
    let bundle = format_ident!("{}Bundle", &dsl.name, span = dsl.name.span());
    let system_set = format_ident!("{}Systems", &dsl.name, span = dsl.name.span());
    let prop_type = dsl.name;
    let state_fields = dsl
        .states
        .as_ref()
        .iter()
        .flat_map(|f| f.fields.iter())
        .map(|field| &field.field)
        .collect::<Vec<_>>();
    let state_attributes = &dsl.attributes;

    let render = quote! {
        #(#state_attributes)*
        #[derive(Component)]
        pub struct #state_component {
            #(#state_fields),*
        }

        #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
        pub enum #system_set {
            Render
        }

        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        #[derive(Component, Reflect)]
        pub struct #widget_component {
            #(#widget_field_decl),*
        }
        impl Default for #widget_component {
            fn default() -> Self{
                Self{
                    #(#widget_field_init),*
                }
            }
        }

        #[derive(Bundle)]
        pub struct #bundle {
            pub node: NodeBundle,
            pub prop: #prop_type,
            pub state: #state_component,
            pub widget: #widget_component,
            #(pub #bundle_fields,)*
        }
        impl #bundle {
            pub fn new(prop: #prop_type, state: #state_component, #(#bundle_fields),*) -> Self {
                 Self {
                     node: NodeBundle::default(),
                     widget: Default::default(),
                     prop,
                     state,
                     #(#bundle_fields_init,)*
                 }
            }
        }

        #(#items)*

        #[allow(dead_code)]
        #[allow(non_snake_case)]
        #[allow(unused_variables)]
        pub fn #function_name(mut this_query: Query<(Entity, Ref<#prop_type>, &mut #state_component, &mut #widget_component, #(#this_query),*)>, mut commands: Commands, #(#function_args),*) {
            for (this_entity, prop, mut state, mut widget) in this_query.iter_mut() {
                let update_all = prop.is_changed();
                #(#declares)*
                #(#run_stage)*
            }
        }
    };
    render.into()
}
