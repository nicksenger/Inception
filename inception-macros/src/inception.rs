use std::ops::Deref;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    braced,
    parse::{Parse, ParseStream, Parser},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    Block, Expr, FnArg, GenericParam, Ident, ItemTrait, Meta, Pat, PatIdent, PatType, ReturnType,
    TraitBound, TraitItem, TraitItemFn, TraitItemType, Type, TypeParam, TypeParamBound, TypePath,
    Visibility, WherePredicate,
};

use crate::derive::Identifier;

const NOTHING_FN_IDENT: &str = "nothing";
const MERGE_FN_IDENT: &str = "merge";
const MERGE_VAR_FN_IDENT: &str = "merge_variant_field";
const JOIN_FN_IDENT: &str = "join";

struct Attributes {
    property: Ident,
    comparator: bool,
    types_only: bool,
    signature: Option<Signature>,
}

#[derive(Clone)]
struct Signature {
    input: Ident,
    output: Ident,
}

#[derive(Clone)]
struct InducePhaseSpec {
    ty: Type,
    where_preds: Vec<WherePredicate>,
}

#[derive(Clone)]
struct InduceSpec {
    base: InducePhaseSpec,
    merge: InducePhaseSpec,
    merge_variant: InducePhaseSpec,
    join: InducePhaseSpec,
}

impl InduceSpec {
    fn has_where_bounds(&self) -> bool {
        !self.base.where_preds.is_empty()
            || !self.merge.where_preds.is_empty()
            || !self.merge_variant.where_preds.is_empty()
            || !self.join.where_preds.is_empty()
    }
}

#[derive(Clone)]
struct AssocTypeSpec {
    item: TraitItemType,
    induce: Option<InduceSpec>,
}

impl Signature {
    fn parse_ident_expr(expr: Expr, field: &str) -> Result<Ident, syn::Error> {
        let Expr::Path(path_expr) = expr else {
            let msg = format!("Expected `{field}` to be a type identifier.");
            return Err(syn::Error::new(proc_macro2::Span::call_site(), msg));
        };
        let Some(ident) = path_expr.path.get_ident() else {
            let msg = format!("Expected `{field}` to be a single identifier.");
            return Err(syn::Error::new(path_expr.path.span(), msg));
        };
        Ok(ident.clone())
    }

    fn parse_tokens(tokens: proc_macro2::TokenStream) -> Result<Self, syn::Error> {
        let nested = Punctuated::<Meta, Comma>::parse_terminated.parse2(tokens)?;
        let mut input = None;
        let mut output = None;
        for meta in nested {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("input") => {
                    input = Some(Self::parse_ident_expr(nv.value, "input")?);
                }
                Meta::NameValue(nv) if nv.path.is_ident("output") => {
                    output = Some(Self::parse_ident_expr(nv.value, "output")?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Unknown `signature` setting; expected `input = ...` or `output = ...`.",
                    ));
                }
            }
        }
        let Some(input) = input else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Missing `input = ...` in `signature(...)`.",
            ));
        };
        let Some(output) = output else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Missing `output = ...` in `signature(...)`.",
            ));
        };
        Ok(Self { input, output })
    }
}

impl Parse for Attributes {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let metas = Punctuated::<Meta, Comma>::parse_terminated(input)?;
        let mut property = None;
        let mut comparator = false;
        let mut types_only = false;
        let mut signature = None;
        for meta in metas {
            match meta {
                Meta::NameValue(nv) if nv.path.is_ident("property") => {
                    if property.is_some() {
                        return Err(syn::Error::new_spanned(
                            nv.path,
                            "`property` can only be set once.",
                        ));
                    }
                    property = Some(Signature::parse_ident_expr(nv.value, "property")?);
                }
                Meta::Path(path) if path.is_ident("comparator") => {
                    comparator = true;
                }
                Meta::Path(path) if path.is_ident("types") => {
                    types_only = true;
                }
                Meta::NameValue(nv) if nv.path.is_ident("comparator") => {
                    let Expr::Lit(expr_lit) = nv.value else {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "Expected `comparator` to be a boolean literal.",
                        ));
                    };
                    let syn::Lit::Bool(v) = expr_lit.lit else {
                        return Err(syn::Error::new_spanned(
                            expr_lit,
                            "Expected `comparator` to be a boolean literal.",
                        ));
                    };
                    comparator = v.value;
                }
                Meta::List(list) if list.path.is_ident("signature") => {
                    if signature.is_some() {
                        return Err(syn::Error::new_spanned(
                            list.path,
                            "`signature(...)` can only be set once.",
                        ));
                    }
                    signature = Some(Signature::parse_tokens(list.tokens)?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "Invalid `#[inception(...)]` argument. Expected `property = ...`, optional `comparator`, optional `types`, and optional `signature(input = ..., output = ...)`.",
                    ));
                }
            }
        }
        let Some(property) = property else {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "Missing `property = ...` in `#[inception(...)]`.",
            ));
        };
        Ok(Self {
            property,
            comparator,
            types_only,
            signature,
        })
    }
}

fn walk_type_paths<F>(ty: &Type, cb: &mut F)
where
    F: FnMut(&TypePath),
{
    match ty {
        Type::Path(tp) => {
            cb(tp);
            if let Some(q) = &tp.qself {
                walk_type_paths(q.ty.as_ref(), cb);
            }
            for seg in tp.path.segments.iter() {
                match &seg.arguments {
                    syn::PathArguments::AngleBracketed(args) => {
                        for arg in args.args.iter() {
                            match arg {
                                syn::GenericArgument::Type(inner) => walk_type_paths(inner, cb),
                                syn::GenericArgument::AssocType(assoc) => {
                                    walk_type_paths(&assoc.ty, cb)
                                }
                                _ => {}
                            }
                        }
                    }
                    syn::PathArguments::Parenthesized(args) => {
                        for input in args.inputs.iter() {
                            walk_type_paths(input, cb);
                        }
                        if let ReturnType::Type(_, out) = &args.output {
                            walk_type_paths(out.as_ref(), cb);
                        }
                    }
                    syn::PathArguments::None => {}
                }
            }
        }
        Type::Reference(r) => walk_type_paths(r.elem.as_ref(), cb),
        Type::Ptr(p) => walk_type_paths(p.elem.as_ref(), cb),
        Type::Slice(s) => walk_type_paths(s.elem.as_ref(), cb),
        Type::Array(a) => walk_type_paths(a.elem.as_ref(), cb),
        Type::Tuple(t) => {
            for elem in t.elems.iter() {
                walk_type_paths(elem, cb);
            }
        }
        Type::Paren(p) => walk_type_paths(p.elem.as_ref(), cb),
        Type::Group(g) => walk_type_paths(g.elem.as_ref(), cb),
        _ => {}
    }
}

fn validate_induce_placeholders(
    ty: &Type,
    phase: &str,
    allowed_placeholders: &[&str],
) -> Result<(), syn::Error> {
    const RESERVED: [&str; 5] = ["Head", "Tail", "Fields", "In", "Out"];
    let mut err: Option<syn::Error> = None;
    walk_type_paths(ty, &mut |tp: &TypePath| {
        if err.is_some() {
            return;
        }
        let Some(id) = tp.path.get_ident() else {
            return;
        };
        let name = id.to_string();
        let is_reserved = RESERVED.iter().any(|r| *r == name);
        let is_allowed = allowed_placeholders.iter().any(|p| *p == name);
        if is_reserved && !is_allowed {
            let msg = format!(
                "Placeholder `{name}` is not available in `induce.{phase}`. Allowed placeholders: {}.",
                allowed_placeholders.join(", ")
            );
            err = Some(syn::Error::new_spanned(tp, msg));
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn walk_trait_bound_type_paths<F>(bound: &TraitBound, cb: &mut F)
where
    F: FnMut(&TypePath),
{
    for seg in bound.path.segments.iter() {
        match &seg.arguments {
            syn::PathArguments::AngleBracketed(args) => {
                for arg in args.args.iter() {
                    match arg {
                        syn::GenericArgument::Type(inner) => walk_type_paths(inner, cb),
                        syn::GenericArgument::AssocType(assoc) => walk_type_paths(&assoc.ty, cb),
                        syn::GenericArgument::Constraint(constraint) => {
                            for b in constraint.bounds.iter() {
                                if let TypeParamBound::Trait(tb) = b {
                                    walk_trait_bound_type_paths(tb, cb);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            syn::PathArguments::Parenthesized(args) => {
                for input in args.inputs.iter() {
                    walk_type_paths(input, cb);
                }
                if let ReturnType::Type(_, out) = &args.output {
                    walk_type_paths(out.as_ref(), cb);
                }
            }
            syn::PathArguments::None => {}
        }
    }
}

fn validate_induce_where_placeholders(
    preds: &[WherePredicate],
    phase: &str,
    allowed_placeholders: &[&str],
) -> Result<(), syn::Error> {
    for pred in preds {
        match pred {
            WherePredicate::Type(tp) => {
                validate_induce_placeholders(&tp.bounded_ty, phase, allowed_placeholders)?;
                for b in tp.bounds.iter() {
                    if let TypeParamBound::Trait(tb) = b {
                        let mut err: Option<syn::Error> = None;
                        walk_trait_bound_type_paths(tb, &mut |tp: &TypePath| {
                            if err.is_some() {
                                return;
                            }
                            let Some(id) = tp.path.get_ident() else {
                                return;
                            };
                            const RESERVED: [&str; 5] = ["Head", "Tail", "Fields", "In", "Out"];
                            let name = id.to_string();
                            let is_reserved = RESERVED.iter().any(|r| *r == name);
                            let is_allowed = allowed_placeholders.iter().any(|p| *p == name);
                            if is_reserved && !is_allowed {
                                let msg = format!(
                                    "Placeholder `{name}` is not available in `induce.{phase}` where-bounds. Allowed placeholders: {}.",
                                    allowed_placeholders.join(", ")
                                );
                                err = Some(syn::Error::new_spanned(tp, msg));
                            }
                        });
                        if let Some(e) = err {
                            return Err(e);
                        }
                    }
                }
            }
            WherePredicate::Lifetime(_) => {}
            _ => {}
        }
    }
    Ok(())
}

fn parse_induce_spec_from_attr(attr: &syn::Attribute) -> Result<InduceSpec, syn::Error> {
    struct InduceValue {
        ty: Type,
        where_preds: Vec<WherePredicate>,
    }
    impl Parse for InduceValue {
        fn parse(input: ParseStream) -> Result<Self, syn::Error> {
            let ty = input.parse::<Type>()?;
            let mut where_preds = Vec::new();
            if input.peek(syn::Token![where]) {
                input.parse::<syn::Token![where]>()?;
                let content;
                braced!(content in input);
                let preds = content.parse_terminated(WherePredicate::parse, Comma)?;
                where_preds = preds.into_iter().collect();
            }
            Ok(Self { ty, where_preds })
        }
    }
    struct InduceEntry {
        key: Ident,
        value: InduceValue,
    }
    impl Parse for InduceEntry {
        fn parse(input: ParseStream) -> Result<Self, syn::Error> {
            let key = input.parse::<Ident>()?;
            input.parse::<syn::Token![=]>()?;
            let value = input.parse::<InduceValue>()?;
            Ok(Self { key, value })
        }
    }

    let nested = attr.parse_args_with(Punctuated::<InduceEntry, Comma>::parse_terminated)?;
    let mut base = None;
    let mut merge = None;
    let mut merge_variant = None;
    let mut join = None;
    for entry in nested {
        match entry.key.to_string().as_str() {
            "base" => {
                if base.is_some() {
                    return Err(syn::Error::new_spanned(
                        entry.key,
                        "`base` can only be set once in `#[induce(...)]`.",
                    ));
                }
                base = Some(InducePhaseSpec {
                    ty: entry.value.ty,
                    where_preds: entry.value.where_preds,
                });
            }
            "merge" => {
                if merge.is_some() {
                    return Err(syn::Error::new_spanned(
                        entry.key,
                        "`merge` can only be set once in `#[induce(...)]`.",
                    ));
                }
                merge = Some(InducePhaseSpec {
                    ty: entry.value.ty,
                    where_preds: entry.value.where_preds,
                });
            }
            "merge_variant" => {
                if merge_variant.is_some() {
                    return Err(syn::Error::new_spanned(
                        entry.key,
                        "`merge_variant` can only be set once in `#[induce(...)]`.",
                    ));
                }
                merge_variant = Some(InducePhaseSpec {
                    ty: entry.value.ty,
                    where_preds: entry.value.where_preds,
                });
            }
            "join" => {
                if join.is_some() {
                    return Err(syn::Error::new_spanned(
                        entry.key,
                        "`join` can only be set once in `#[induce(...)]`.",
                    ));
                }
                join = Some(InducePhaseSpec {
                    ty: entry.value.ty,
                    where_preds: entry.value.where_preds,
                });
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    entry.key,
                    "Invalid `#[induce(...)]` argument. Expected `base = ...`, `merge = ...`, `merge_variant = ...`, and `join = ...`.",
                ));
            }
        }
    }
    let Some(base) = base else {
        return Err(syn::Error::new_spanned(
            attr,
            "Missing `base = ...` in `#[induce(...)]`.",
        ));
    };
    let Some(merge) = merge else {
        return Err(syn::Error::new_spanned(
            attr,
            "Missing `merge = ...` in `#[induce(...)]`.",
        ));
    };
    let Some(merge_variant) = merge_variant else {
        return Err(syn::Error::new_spanned(
            attr,
            "Missing `merge_variant = ...` in `#[induce(...)]`.",
        ));
    };
    let Some(join) = join else {
        return Err(syn::Error::new_spanned(
            attr,
            "Missing `join = ...` in `#[induce(...)]`.",
        ));
    };
    validate_induce_placeholders(&base.ty, "base", &["In", "Out"])?;
    validate_induce_where_placeholders(&base.where_preds, "base", &["In", "Out"])?;
    validate_induce_placeholders(&merge.ty, "merge", &["Head", "Tail", "In", "Out"])?;
    validate_induce_where_placeholders(
        &merge.where_preds,
        "merge",
        &["Head", "Tail", "In", "Out"],
    )?;
    validate_induce_placeholders(
        &merge_variant.ty,
        "merge_variant",
        &["Head", "Tail", "In", "Out"],
    )?;
    validate_induce_where_placeholders(
        &merge_variant.where_preds,
        "merge_variant",
        &["Head", "Tail", "In", "Out"],
    )?;
    validate_induce_placeholders(&join.ty, "join", &["Fields", "In", "Out"])?;
    validate_induce_where_placeholders(&join.where_preds, "join", &["Fields", "In", "Out"])?;
    Ok(InduceSpec {
        base,
        merge,
        merge_variant,
        join,
    })
}

fn extract_induce_attr(attrs: &mut Vec<syn::Attribute>) -> Result<Option<InduceSpec>, syn::Error> {
    let mut spec = None;
    let mut kept = Vec::with_capacity(attrs.len());
    for attr in attrs.drain(..) {
        if attr.path().is_ident("induce") {
            if spec.is_some() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "Only one `#[induce(...)]` attribute is allowed per associated type.",
                ));
            }
            spec = Some(parse_induce_spec_from_attr(&attr)?);
        } else {
            kept.push(attr);
        }
    }
    *attrs = kept;
    Ok(spec)
}

pub enum Kind {
    Ty,
    Ref,
    Mut,
    Owned,
}

impl Kind {
    fn receiver(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! {},
            Self::Ref | Self::Mut | Self::Owned => quote! { self },
        }
    }

    fn dispatcher(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! { Self:: },
            Self::Ref | Self::Mut | Self::Owned => quote! { self. },
        }
    }

    fn comp_dispatcher(&self, id: &Ident) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! { Self:: },
            Self::Ref | Self::Mut | Self::Owned => quote! { #id. },
        }
    }

    fn access_bound(&self, is_var: bool, head_ident: &Ident) -> proc_macro2::TokenStream {
        let liferef1 = self.liferef1();
        let lifepunct1 = self.lifepunct1();
        let id = if is_var {
            format_ident!("TryAccess")
        } else {
            format_ident!("Access")
        };
        let enum_err_ident = self.enum_err_ident();
        let err = if is_var {
            quote! { , Err = #enum_err_ident<#lifepunct1 #head_ident, S, VAR_IDX, IDX> }
        } else {
            quote! {}
        };
        match self {
            Self::Ty => quote! {},
            _ => quote! { + #id <Out = #liferef1 #head_ident #err> },
        }
    }

    fn phantom_bound(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! { + Phantom },
            _ => quote! {},
        }
    }

    fn split_fn_receiver(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! { Self::phantom() },
            _ => quote! {},
        }
    }

    fn mutability(&self) -> proc_macro2::TokenStream {
        if let Self::Mut = self {
            quote! { mut }
        } else {
            quote! {}
        }
    }

    fn refmut(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref => quote! { & },
            Self::Mut => quote! { &mut },
        }
    }

    fn enum_err_ident(&self) -> Ident {
        match self {
            Self::Ty => format_ident!("TyEnumAccessError"),
            Self::Ref => format_ident!("RefEnumAccessError"),
            Self::Mut => format_ident!("MutEnumAccessError"),
            Self::Owned => format_ident!("OwnedEnumAccessError"),
        }
    }

    fn fields_fn(&self, property: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! { let fields = <T as Inception<#property>>::ty_fields(); },
            Self::Ref => quote! { let fields = self.fields(); },
            Self::Mut => quote! {
                let mut header = VariantHeader;
                let mut fields = self.fields_mut(&mut header);
            },
            Self::Owned => quote! {
                let fields = self.into_fields();
            },
        }
    }

    fn comparator_fields_fn(
        &self,
        ident: &Ident,
        property: &proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! { let #ident = <T as Inception<#property>>::ty_fields(); },
            Self::Ref => quote! { let #ident = #ident.fields(); },
            Self::Mut => quote! {
                let mut header = VariantHeader;
                let mut #ident = #ident.fields_mut(&mut header);
            },
            Self::Owned => quote! {
                let #ident = #ident.into_fields();
            },
        }
    }

    fn fields(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty => quote! { TyFields },
            Self::Owned => quote! { OwnedFields },
            Self::Ref => quote! { RefFields },
            Self::Mut => quote! { MutFields },
        }
    }

    fn bracketlife2(&self) -> proc_macro2::TokenStream {
        let life = self.life2();
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { <#life> },
        }
    }

    fn liferef1(&self) -> proc_macro2::TokenStream {
        let life = self.life1();
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref => quote! { & #life },
            Self::Mut => quote! { & #life mut },
        }
    }

    fn liferef2(&self) -> proc_macro2::TokenStream {
        let life = self.life2();
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref => quote! { & #life },
            Self::Mut => quote! { & #life mut },
        }
    }

    fn liferefelide(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref => quote! { &'_ },
            Self::Mut => quote! { &'_ mut },
        }
    }

    fn lifepunctelide(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { '_, },
        }
    }

    fn lifepunct1(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { 'a, },
        }
    }

    fn lifepunct2(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { 'b, },
        }
    }

    fn lifepunct3(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { 'c, },
        }
    }

    fn life1(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { 'a },
        }
    }
    fn life2(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { 'b },
        }
    }
    fn life3(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Ty | Self::Owned => quote! {},
            Self::Ref | Self::Mut => quote! { 'c },
        }
    }

    fn field(&self) -> Ident {
        match self {
            Self::Ty => format_ident!("TyField"),
            Self::Ref => format_ident!("RefField"),
            Self::Mut => format_ident!("MutField"),
            Self::Owned => format_ident!("OwnedField"),
        }
    }

    fn var_field(&self) -> Ident {
        match self {
            Self::Ty => format_ident!("VarTyField"),
            Self::Ref => format_ident!("VarRefField"),
            Self::Mut => format_ident!("VarMutField"),
            Self::Owned => format_ident!("VarOwnedField"),
        }
    }

    fn split_trait_ident(&self) -> Ident {
        match self {
            Self::Ty => format_ident!("SplitTy"),
            Self::Ref => format_ident!("SplitRef"),
            Self::Mut => format_ident!("SplitMut"),
            Self::Owned => format_ident!("Split"),
        }
    }

    fn split_fn_ident(&self) -> Ident {
        match self {
            Self::Ty => format_ident!("split_ty"),
            Self::Ref => format_ident!("split_ref"),
            Self::Mut => format_ident!("split_mut"),
            Self::Owned => format_ident!("split"),
        }
    }

    fn split_fn(&self, is_var: bool) -> proc_macro2::TokenStream {
        let field = if is_var {
            self.var_field()
        } else {
            self.field()
        };
        let var_idx = if is_var {
            quote! { VAR_IDX, }
        } else {
            quote! {}
        };
        match self {
            Self::Ty => quote! {
                    (<#field<T, S, #var_idx IDX> as Phantom>::phantom(), <V as Phantom>::phantom())
            },
            Self::Ref => quote! {
                    (self.0.0.0.clone(), &self.0.0 .1)
            },
            Self::Mut => quote! {
                    (self.0.0.0.take(), &mut self.0.0 .1)
            },
            Self::Owned => quote! {
                    (self.0.0 .0, self.0.0 .1)
            },
        }
    }

    fn split_impl(
        &self,
        property: &proc_macro2::TokenStream,
        wrapper: &proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        let vfimpl = self.split_fn(true);
        let fimpl = self.split_fn(false);
        match self {
            Self::Ty => quote! {
                impl<T, S, const IDX: usize, V: Phantom> SplitTy<#property> for #wrapper<List<(TyField<T, S, IDX>, V)>> {
                    type Left = TyField<T, S, IDX>;
                    type Right = V;

                    fn split_ty(self) -> (Self::Left, Self::Right) {
                        #fimpl
                    }
                }
                impl<T, S, const VAR_IDX: usize, const IDX: usize, V> SplitTy<#property> for #wrapper<List<(VarTyField<T, S, VAR_IDX, IDX>, V)>>
                where
                    V: Phantom,
                {
                    type Left = VarTyField<T, S, VAR_IDX, IDX>;
                    type Right = V;

                    fn split_ty(self) -> (Self::Left, Self::Right) {
                        #vfimpl
                    }
                }
            },
            Self::Owned => quote! {
                impl<T, S, const IDX: usize, V> Split<#property> for #wrapper<List<(OwnedField<T, S, IDX>, V)>> {
                    type Left = OwnedField<T, S, IDX>;
                    type Right = V;

                    fn split(self) -> (Self::Left, Self::Right) {
                        #fimpl
                    }
                }
                impl<T, S, const VAR_IDX: usize, const IDX: usize, V> Split<#property> for #wrapper<List<(VarOwnedField<T, S, VAR_IDX, IDX>, V)>>
                where
                    V: Phantom,
                {
                    type Left = VarOwnedField<T, S, VAR_IDX, IDX>;
                    type Right = V;

                    fn split(self) -> (Self::Left, Self::Right) {
                        #vfimpl
                    }
                }
            },
            Self::Ref => quote! {
                impl<'b, 'c, T, S, const IDX: usize, V> SplitRef<#property> for #wrapper<&'b List<(RefField<'c, T, S, IDX>, V)>> {
                    type Left = RefField<'c, T, S, IDX>;
                    type Right = V;

                    fn split_ref(&self) -> (Self::Left, &Self::Right) {
                        #fimpl
                    }
                }
                impl<'b, 'c, T, S, const VAR_IDX: usize, const IDX: usize, V> SplitRef<#property> for #wrapper<&'b List<(VarRefField<'c, T, S, VAR_IDX, IDX>, V)>>
                where
                    V: Phantom,
                {
                    type Left = VarRefField<'c, T, S, VAR_IDX, IDX>;
                    type Right = V;

                    fn split_ref(&self) -> (Self::Left, &Self::Right) {
                        #vfimpl
                    }
                }
            },
            Self::Mut => quote! {
                impl<'b, 'c, T, S, const IDX: usize, V> SplitMut<#property> for #wrapper<&'b mut List<(MutField<'c, T, S, IDX>, V)>> {
                    type Left = MutField<'c, T, S, IDX>;
                    type Right = V;

                    fn split_mut(&mut self) -> (Self::Left, &mut Self::Right) {
                        #fimpl
                    }
                }
                impl<'b, 'c, T, S, const VAR_IDX: usize, const IDX: usize, V> SplitMut<#property> for #wrapper<&'b mut List<(VarMutField<'c, T, S, VAR_IDX, IDX>, V)>>
                where
                    V: Phantom,
                {
                    type Left = VarMutField<'c, T, S, VAR_IDX, IDX>;
                    type Right = V;

                    fn split_mut(&mut self) -> (Self::Left, &mut Self::Right) {
                        #vfimpl
                    }
                }
            },
        }
    }
}

trait ParseFn {
    const RESERVED: &[&str];
    fn is_reserved(&self) -> bool;
    fn kind(&self) -> Kind;
    fn body(&self) -> Result<Block, &str>;
    fn ret(&self) -> proc_macro2::TokenStream;
    fn generic_ident(&self, i: usize) -> Result<Ident, &str>;
    fn generic_bound(&self, i: usize) -> Result<GenericBounds, &str>;
    fn arg_ident(&self, i: usize) -> Result<Ident, &str>;
    fn arg_ty(&self, i: usize) -> Result<Ident, &str>;
    fn args(&self, i: usize) -> Result<Punctuated<FnArg, Comma>, &str>;
    fn arg_idents(&self, i: usize) -> Result<Punctuated<Ident, Comma>, &str>;
}
impl ParseFn for TraitItemFn {
    const RESERVED: &[&str] = &[
        NOTHING_FN_IDENT,
        MERGE_FN_IDENT,
        MERGE_VAR_FN_IDENT,
        JOIN_FN_IDENT,
    ];
    fn is_reserved(&self) -> bool {
        let name = self.sig.ident.to_string();
        if Self::RESERVED.contains(&name.as_str()) {
            return true;
        }

        false
    }

    fn kind(&self) -> Kind {
        match self.sig.receiver() {
            Some(receiver) => match (receiver.reference.as_ref(), receiver.mutability) {
                (Some(_), Some(_)) => Kind::Mut,
                (Some(_), None) => Kind::Ref,
                (None, _) => Kind::Owned,
            },
            None => Kind::Ty,
        }
    }

    fn body(&self) -> Result<Block, &str> {
        let Some(body) = self.default.as_ref() else {
            return Err("A default implementation must be provided for Inception helpers.");
        };

        Ok(body.clone())
    }

    fn ret(&self) -> proc_macro2::TokenStream {
        match &self.sig.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ty) => quote! { #ty },
        }
    }

    fn generic_ident(&self, i: usize) -> Result<Ident, &str> {
        let Some(generic) = self.sig.generics.params.get(i) else {
            return Err("Expected generic ident");
        };

        match generic {
            GenericParam::Type(TypeParam { ident, .. }) => Ok(ident.clone()),
            GenericParam::Lifetime(_) | GenericParam::Const(_) => {
                Err("Sorry, lifetime and const generics aren't supported yet.")
            }
        }
    }

    fn generic_bound(&self, i: usize) -> Result<GenericBounds, &str> {
        let Some(generic) = self.sig.generics.params.get(i) else {
            return Err("Expected generic ident");
        };

        match generic {
            GenericParam::Type(TypeParam { ident, bounds, .. }) => {
                let mut bounds = bounds.iter().cloned().collect::<Vec<_>>();
                if let Some(where_clause) = self.sig.generics.where_clause.as_ref() {
                    for pred in where_clause.predicates.iter() {
                        let syn::WherePredicate::Type(tp) = pred else {
                            continue;
                        };
                        let Type::Path(TypePath {
                            qself: None,
                            path: bounded_path,
                        }) = &tp.bounded_ty
                        else {
                            continue;
                        };
                        if bounded_path.is_ident(ident) {
                            bounds.extend(tp.bounds.iter().cloned());
                        }
                    }
                }
                Ok(GenericBounds { bounds })
            }
            GenericParam::Lifetime(_) | GenericParam::Const(_) => {
                Err("Sorry, lifetime and const generics aren't supported yet.")
            }
        }
    }

    fn arg_ident(&self, i: usize) -> Result<Ident, &str> {
        let Some(arg) = self.sig.inputs.get(i) else {
            return Err("Expected argument at position {i}");
        };

        if let FnArg::Typed(PatType { pat: p, .. }) = arg {
            if let Pat::Ident(id) = &**p {
                return Ok(id.ident.clone());
            }
        }
        Err("Unexpected argument type")
    }

    fn arg_ty(&self, i: usize) -> Result<Ident, &str> {
        let Some(arg) = self.sig.inputs.get(i) else {
            return Err("Expected argument at position {i}");
        };

        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if let Type::Path(tp) = &**ty {
                let Ok(id) = tp.path.require_ident() else {
                    return Err("Expected ident arg type");
                };

                return Ok(id.clone());
            }
        }
        Err("Unexpected argument type")
    }

    fn args(&self, i: usize) -> Result<Punctuated<FnArg, Comma>, &str> {
        Ok(self.sig.inputs.iter().skip(i).cloned().collect())
    }

    fn arg_idents(&self, i: usize) -> Result<Punctuated<Ident, Comma>, &str> {
        Ok(self
            .sig
            .inputs
            .iter()
            .skip(i)
            .filter_map(|input| match input {
                FnArg::Typed(PatType { pat, .. }) => match pat.deref() {
                    Pat::Ident(PatIdent { ident, .. }) => Some(ident.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect())
    }
}

struct GenericBounds {
    bounds: Vec<TypeParamBound>,
}
impl GenericBounds {
    fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    fn use_inner_trait(
        mut self,
        trait_ident: &Ident,
        inductive_ident: &Ident,
        mod_ident: &Ident,
        flow_input_ident: Option<&Ident>,
        flow_output_ident: Option<&Ident>,
    ) -> Self {
        fn is_self_assoc_path(qself: &Option<syn::QSelf>, path: &syn::Path, assoc: &Ident) -> bool {
            let direct_self_assoc = path.segments.len() == 2
                && path
                    .segments
                    .first()
                    .map(|s| s.ident == "Self")
                    .unwrap_or(false)
                && path
                    .segments
                    .last()
                    .map(|s| s.ident == *assoc)
                    .unwrap_or(false);
            if direct_self_assoc {
                return true;
            }
            let Some(last) = path.segments.last() else {
                return false;
            };
            if last.ident != *assoc {
                return false;
            }
            let Some(q) = qself else {
                return false;
            };
            matches!(
                q.ty.as_ref(),
                Type::Path(TypePath { qself: None, path })
                if path.is_ident("Self")
            )
        }

        fn replace_flow_type(ty: &mut Type, flow_in: Option<&Ident>, flow_out: Option<&Ident>) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(flow) = flow_in {
                        if path.is_ident(flow) || is_self_assoc_path(qself, path, flow) {
                            *ty = syn::parse_quote!(In);
                            return;
                        }
                    }
                    if let Some(flow) = flow_out {
                        if path.is_ident(flow) || is_self_assoc_path(qself, path, flow) {
                            *ty = syn::parse_quote!(Out);
                            return;
                        }
                    }
                    if let Some(q) = qself {
                        replace_flow_type(&mut q.ty, flow_in, flow_out);
                    }
                    for seg in path.segments.iter_mut() {
                        match &mut seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter_mut() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            replace_flow_type(inner, flow_in, flow_out);
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            replace_flow_type(&mut assoc.ty, flow_in, flow_out);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter_mut() {
                                    replace_flow_type(input, flow_in, flow_out);
                                }
                                if let ReturnType::Type(_, out) = &mut args.output {
                                    replace_flow_type(out.as_mut(), flow_in, flow_out);
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                }
                Type::Reference(r) => replace_flow_type(r.elem.as_mut(), flow_in, flow_out),
                Type::Ptr(p) => replace_flow_type(p.elem.as_mut(), flow_in, flow_out),
                Type::Slice(s) => replace_flow_type(s.elem.as_mut(), flow_in, flow_out),
                Type::Array(a) => replace_flow_type(a.elem.as_mut(), flow_in, flow_out),
                Type::Tuple(t) => {
                    for elem in t.elems.iter_mut() {
                        replace_flow_type(elem, flow_in, flow_out);
                    }
                }
                Type::Paren(p) => replace_flow_type(p.elem.as_mut(), flow_in, flow_out),
                Type::Group(g) => replace_flow_type(g.elem.as_mut(), flow_in, flow_out),
                _ => {}
            }
        }

        for bound in self.bounds.iter_mut() {
            if let TypeParamBound::Trait(TraitBound { path, .. }) = bound {
                if path
                    .segments
                    .first()
                    .map(|seg| &seg.ident == trait_ident)
                    .unwrap_or_default()
                {
                    let Some(p) = path.segments.first_mut() else {
                        continue;
                    };
                    if flow_output_ident.is_some() {
                        if let syn::PathArguments::AngleBracketed(args) = &mut p.arguments {
                            for arg in args.args.iter_mut() {
                                match arg {
                                    syn::GenericArgument::Type(ty) => {
                                        replace_flow_type(ty, flow_input_ident, flow_output_ident)
                                    }
                                    syn::GenericArgument::AssocType(assoc) => replace_flow_type(
                                        &mut assoc.ty,
                                        flow_input_ident,
                                        flow_output_ident,
                                    ),
                                    _ => {}
                                }
                            }
                        }
                        continue;
                    }
                    if flow_input_ident.is_some() {
                        if let syn::PathArguments::AngleBracketed(args) = &mut p.arguments {
                            for arg in args.args.iter_mut() {
                                match arg {
                                    syn::GenericArgument::Type(ty) => {
                                        replace_flow_type(ty, flow_input_ident, flow_output_ident)
                                    }
                                    syn::GenericArgument::AssocType(assoc) => replace_flow_type(
                                        &mut assoc.ty,
                                        flow_input_ident,
                                        flow_output_ident,
                                    ),
                                    _ => {}
                                }
                            }
                        }
                        continue;
                    }
                    if let syn::PathArguments::AngleBracketed(args) = &mut p.arguments {
                        let mut in_ty: Option<Type> = None;
                        let mut out_ty: Option<Type> = None;
                        let mut passthrough = Punctuated::<syn::GenericArgument, Comma>::new();

                        for arg in args.args.iter() {
                            match arg {
                                syn::GenericArgument::Type(ty) => {
                                    let replacement = if let Some(flow) = flow_input_ident {
                                        if let Type::Path(TypePath { path, .. }) = ty {
                                            if path.is_ident(flow) {
                                                Some(syn::parse_quote!(In))
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };
                                    let ty = replacement.unwrap_or_else(|| ty.clone());
                                    if in_ty.is_none() {
                                        in_ty = Some(ty);
                                    } else if out_ty.is_none() {
                                        out_ty = Some(ty);
                                    } else {
                                        passthrough.push(syn::GenericArgument::Type(ty));
                                    }
                                }
                                _ => passthrough.push(arg.clone()),
                            }
                        }

                        let mut next_args = Punctuated::<syn::GenericArgument, Comma>::new();
                        if let Some(in_ty) = in_ty {
                            let assoc: syn::GenericArgument = syn::parse_quote!(InTy = #in_ty);
                            next_args.push(assoc);
                        }
                        if let Some(out_ty) = out_ty {
                            let assoc: syn::GenericArgument = syn::parse_quote!(OutTy = #out_ty);
                            next_args.push(assoc);
                        }
                        for arg in passthrough {
                            next_args.push(arg);
                        }
                        args.args = next_args;
                    }
                    p.ident = inductive_ident.clone();
                    path.segments.insert(
                        0,
                        syn::PathSegment {
                            ident: mod_ident.clone(),
                            arguments: syn::PathArguments::None,
                        },
                    );
                }
            }
        }

        self
    }

    fn into_tokens(self) -> proc_macro2::TokenStream {
        let v = self.bounds;
        quote! { #(+ #v)* }
    }
}

enum Step {
    Base(Nothing),
    Merge(MergeField),
    Enum(MergeVar),
    Join(Join),
}
impl Step {
    fn parse(f: &TraitItemFn) -> Result<Self, &str> {
        match f.sig.ident.to_string().as_str() {
            s if s == NOTHING_FN_IDENT => Ok(Self::Base(Nothing::parse(f)?)),
            s if s == MERGE_FN_IDENT => Ok(Self::Merge(MergeField::parse(f)?)),
            s if s == MERGE_VAR_FN_IDENT => Ok(Self::Enum(MergeVar::parse(f)?)),
            s if s == JOIN_FN_IDENT => Ok(Self::Join(Join::parse(f)?)),
            _ => Err("Unexpected step"),
        }
    }
}

struct Nothing {
    nothing_body: Block,
    nothing_ret: proc_macro2::TokenStream,
    nothing_args: Punctuated<FnArg, Comma>,
    nothing_arg_idents: Punctuated<Ident, Comma>,
}
impl Nothing {
    fn parse(f: &TraitItemFn) -> Result<Self, &str> {
        let skip = usize::from(f.sig.receiver().is_some());
        Ok(Self {
            nothing_body: f.body()?,
            nothing_ret: f.ret(),
            nothing_args: f.args(skip)?,
            nothing_arg_idents: f.arg_idents(skip)?,
        })
    }
}

struct MergeField {
    merge_body: Block,
    merge_ret: proc_macro2::TokenStream,
    merge_head_ident: Ident,
    merge_fields_ident: Ident,
    merge_extra_generics: Vec<Ident>,
    merge_head_arg: Ident,
    #[allow(unused)]
    merge_head_arg_ty: Ident,
    merge_fields_arg: Ident,
    #[allow(unused)]
    merge_fields_arg_ty: Ident,
    merge_field_head_bounds: GenericBounds,
    merge_fields_bounds: GenericBounds,
    merge_args: Punctuated<FnArg, Comma>,
    merge_arg_idents: Punctuated<Ident, Comma>,
}
impl MergeField {
    fn parse(f: &TraitItemFn) -> Result<Self, &str> {
        let mut merge_extra_generics = Vec::new();
        for param in f.sig.generics.params.iter().skip(2) {
            match param {
                GenericParam::Type(t) => merge_extra_generics.push(t.ident.clone()),
                GenericParam::Lifetime(_) | GenericParam::Const(_) => {
                    return Err("Sorry, lifetime and const generics aren't supported yet.");
                }
            }
        }
        Ok(Self {
            merge_body: f.body()?,
            merge_ret: f.ret(),
            merge_head_ident: f.generic_ident(0)?,
            merge_fields_ident: f.generic_ident(1)?,
            merge_extra_generics,
            merge_head_arg: f.arg_ident(0)?,
            merge_head_arg_ty: f.arg_ty(0)?,
            merge_fields_arg: f.arg_ident(1)?,
            merge_fields_arg_ty: f.arg_ty(1)?,
            merge_field_head_bounds: f.generic_bound(0)?,
            merge_fields_bounds: f.generic_bound(1)?,
            merge_args: f.args(2)?,
            merge_arg_idents: f.arg_idents(2)?,
        })
    }

    fn validate_comparator(&mut self) -> Result<(), &str> {
        self.merge_arg_idents.clear();
        let (Some(head), Some(tail), None) = (
            self.merge_args.get(0),
            self.merge_args.get(1),
            self.merge_args.get(2),
        ) else {
            return Err("Expected exactly 2 additional arguments in comparator merge.");
        };

        let (
            FnArg::Typed(PatType {
                pat: lp, ty: lty, ..
            }),
            FnArg::Typed(PatType {
                pat: rp, ty: rty, ..
            }),
        ) = (head, tail)
        else {
            return Err("Expected additional comparator arguments to be typed patterns.");
        };
        let (Pat::Ident(li), Pat::Ident(ri)) = (lp.deref(), rp.deref()) else {
            return Err("Expected ident patterns for additional comparator arguments");
        };
        self.merge_arg_idents.push(li.ident.clone());
        self.merge_arg_idents.push(ri.ident.clone());

        let (Type::Path(TypePath { path: ltpath, .. }), Type::Path(TypePath { path: rtpath, .. })) =
            (lty.deref(), rty.deref())
        else {
            return Err(
                "Expected head field and tail fields as types for additional comparator arguments",
            );
        };
        if !ltpath.is_ident(&self.merge_head_arg_ty) || !rtpath.is_ident(&self.merge_fields_arg_ty)
        {
            return Err(
                "Expected head field and tail fields as types for additional comparator arguments",
            );
        }

        Ok(())
    }
}
struct MergeVar {
    merge_var_body: Block,
    merge_var_ret: proc_macro2::TokenStream,
    merge_var_head_ident: Ident,
    merge_var_fields_ident: Ident,
    merge_var_extra_generics: Vec<Ident>,
    merge_var_head_arg: Ident,
    #[allow(unused)]
    merge_var_head_arg_ty: Ident,
    merge_var_fields_arg: Ident,
    #[allow(unused)]
    merge_var_fields_arg_ty: Ident,
    merge_var_field_head_bounds: GenericBounds,
    merge_var_fields_bounds: GenericBounds,
    merge_var_args: Punctuated<FnArg, Comma>,
    merge_var_arg_idents: Punctuated<Ident, Comma>,
}
impl MergeVar {
    fn parse(f: &TraitItemFn) -> Result<Self, &str> {
        let mut merge_var_extra_generics = Vec::new();
        for param in f.sig.generics.params.iter().skip(2) {
            match param {
                GenericParam::Type(t) => merge_var_extra_generics.push(t.ident.clone()),
                GenericParam::Lifetime(_) | GenericParam::Const(_) => {
                    return Err("Sorry, lifetime and const generics aren't supported yet.");
                }
            }
        }
        Ok(Self {
            merge_var_body: f.body()?,
            merge_var_ret: f.ret(),
            merge_var_head_ident: f.generic_ident(0)?,
            merge_var_fields_ident: f.generic_ident(1)?,
            merge_var_extra_generics,
            merge_var_head_arg: f.arg_ident(0)?,
            merge_var_head_arg_ty: f.arg_ty(0)?,
            merge_var_fields_arg: f.arg_ident(1)?,
            merge_var_fields_arg_ty: f.arg_ty(1)?,
            merge_var_field_head_bounds: f.generic_bound(0)?,
            merge_var_fields_bounds: f.generic_bound(1)?,
            merge_var_args: f.args(2)?,
            merge_var_arg_idents: f.arg_idents(2)?,
        })
    }

    fn validate_comparator(&mut self) -> Result<(), &str> {
        self.merge_var_arg_idents.clear();
        let (Some(head), Some(tail), None) = (
            self.merge_var_args.get(0),
            self.merge_var_args.get(1),
            self.merge_var_args.get(2),
        ) else {
            return Err("Expected exactly 2 additional arguments in comparator merge.");
        };

        let (
            FnArg::Typed(PatType {
                pat: lp, ty: lty, ..
            }),
            FnArg::Typed(PatType {
                pat: rp, ty: rty, ..
            }),
        ) = (head, tail)
        else {
            return Err("Expected additional comparator arguments to be typed patterns.");
        };
        let (Pat::Ident(li), Pat::Ident(ri)) = (lp.deref(), rp.deref()) else {
            return Err("Expected ident patterns for additional comparator arguments");
        };
        self.merge_var_arg_idents.push(li.ident.clone());
        self.merge_var_arg_idents.push(ri.ident.clone());

        let (Type::Path(TypePath { path: ltpath, .. }), Type::Path(TypePath { path: rtpath, .. })) =
            (lty.deref(), rty.deref())
        else {
            return Err(
                "Expected head field and tail fields as types for additional comparator arguments",
            );
        };
        if !ltpath.is_ident(&self.merge_var_head_arg_ty)
            || !rtpath.is_ident(&self.merge_var_fields_arg_ty)
        {
            return Err(
                "Expected head field and tail fields as types for additional comparator arguments",
            );
        }

        Ok(())
    }
}
struct Join {
    join_body: Block,
    join_ret: proc_macro2::TokenStream,
    join_fields_ident: Ident,
    join_fields_bounds: GenericBounds,
    join_fields_arg: Ident,
    #[allow(unused)]
    join_fields_arg_ty: Ident,
    join_args: Punctuated<FnArg, Comma>,
    join_arg_idents: Punctuated<Ident, Comma>,
}
impl Join {
    fn parse(f: &TraitItemFn) -> Result<Self, &str> {
        Ok(Self {
            join_body: f.body()?,
            join_ret: f.ret(),
            join_fields_ident: f.generic_ident(0)?,
            join_fields_bounds: f.generic_bound(0)?,
            join_fields_arg: f.arg_ident(0)?,
            join_fields_arg_ty: f.arg_ty(0)?,
            join_args: f.args(1)?,
            join_arg_idents: f.arg_idents(1)?,
        })
    }

    fn validate_comparator(&mut self) -> Result<(), &str> {
        self.join_arg_idents.clear();
        let (Some(fields), None) = (self.join_args.get(0), self.join_args.get(1)) else {
            return Err("Expected exactly 1 additional argument in comparator join.");
        };

        let FnArg::Typed(PatType { pat, ty: lty, .. }) = fields else {
            return Err("Expected additional comparator argument to be a typed pattern.");
        };
        let Pat::Ident(ident) = pat.deref() else {
            return Err("Expected ident for additional comparator arg");
        };
        self.join_arg_idents.push(ident.ident.clone());

        let Type::Path(TypePath { path: ltpath, .. }) = lty.deref() else {
            return Err("Expected the same fields type for additional comparator argument");
        };
        if !ltpath.is_ident(&self.join_fields_arg_ty) {
            return Err("Expected the same fields type for additional comparator argument");
        }

        Ok(())
    }
}

pub struct State {
    trait_ident: Ident,
    trait_generics: syn::Generics,
    trait_supertraits: Punctuated<TypeParamBound, syn::token::Plus>,
    property_ident: Ident,
    signature: Option<Signature>,
    mod_ident: Ident,
    fn_ident: Ident,
    fn_args: proc_macro2::TokenStream,
    fn_args_list: Punctuated<FnArg, Comma>,
    fn_arg_idents: Punctuated<Ident, Comma>,
    fn_ret: ReturnType,
    vis: Visibility,
    kind: Kind,
    assoc_types: Vec<AssocTypeSpec>,
    nothing: Option<Nothing>,
    merge_field: Option<MergeField>,
    merge_var: Option<MergeVar>,
    join: Option<Join>,
}

impl State {
    pub fn gen(attr: TokenStream, item: TokenStream) -> TokenStream {
        let input = parse_macro_input!(item as syn::Item);
        match input {
            syn::Item::Trait(x) => {
                let Attributes {
                    property,
                    comparator,
                    types_only,
                    signature,
                } = match syn::parse::<Attributes>(attr) {
                    Ok(attrs) => attrs,
                    Err(e) => return e.into_compile_error().into(),
                };

                match State::process(x, property, comparator, types_only, signature) {
                    Ok(tt) => tt,
                    Err(tt) => tt,
                }
            }
            item => syn::Error::new_spanned(
                item,
                "This macro can only be applied to trait definitions.",
            )
            .to_compile_error()
            .into(),
        }
    }

    fn process(
        tr: ItemTrait,
        property_ident: Ident,
        is_comparator: bool,
        is_types_only: bool,
        signature: Option<Signature>,
    ) -> Result<TokenStream, TokenStream> {
        let mut st = State {
            trait_ident: tr.ident.clone(),
            trait_generics: tr.generics.clone(),
            trait_supertraits: tr.supertraits.clone(),
            property_ident,
            signature,
            mod_ident: format_ident!("__inception_{}", Identifier::modularize(&tr.ident)),
            fn_ident: format_ident!("unknown"),
            fn_ret: ReturnType::Default,
            fn_args: Default::default(),
            fn_args_list: Default::default(),
            fn_arg_idents: Default::default(),
            vis: tr.vis.clone(),
            kind: Kind::Ty,
            assoc_types: vec![],
            nothing: None,
            merge_field: None,
            merge_var: None,
            join: None,
        };

        let mut is_fn_defined: bool = false;
        for item in tr.items.iter() {
            match item {
                TraitItem::Fn(f) => match (f.is_reserved(), f.kind(), is_fn_defined) {
                    _ if is_types_only => {
                        return Err(syn::Error::new_spanned(
                            f,
                            "`types` mode does not support behavior methods.",
                        )
                        .into_compile_error()
                        .into())
                    }
                    (false, kind, false) => {
                        is_fn_defined = true;
                        let is_ty = matches!(&kind, Kind::Ty);
                        st.kind = kind;
                        st.fn_ident = f.sig.ident.clone();
                        st.fn_ret = f.sig.output.clone();
                        let skip = if is_ty { 0 } else { 1 };
                        let args = f.sig.inputs.iter().skip(skip).collect::<Vec<_>>();
                        st.fn_args = {
                            if !args.is_empty() {
                                if is_ty {
                                    quote! { #(#args),* }
                                } else {
                                    quote! { , #(#args),* }
                                }
                            } else {
                                quote! {}
                            }
                        };
                        st.fn_arg_idents = {
                            args.iter()
                                .filter_map(|arg| match arg {
                                    FnArg::Receiver(_) => None,
                                    FnArg::Typed(PatType { pat, .. }) => match &**pat {
                                        Pat::Ident(p) => Some(p.ident.clone()),
                                        _ => None,
                                    },
                                })
                                .collect()
                        };
                        st.fn_args_list = args.into_iter().cloned().collect();
                    }
                    (false, _, true) => {
                        return Err(
                            syn::Error::new_spanned(f, "Unexpected function definition.")
                                .into_compile_error()
                                .into(),
                        )
                    }
                    (true, _, _) => match Step::parse(f) {
                        Ok(Step::Base(nothing)) => {
                            st.nothing = Some(nothing);
                        }
                        Ok(Step::Merge(mut merge)) => {
                            if is_comparator {
                                merge.validate_comparator().map_err(|s| {
                                    syn::Error::new_spanned(f, s).into_compile_error()
                                })?;
                            }
                            st.merge_field = Some(merge);
                        }
                        Ok(Step::Enum(mut merge_var)) => {
                            if is_comparator {
                                merge_var.validate_comparator().map_err(|s| {
                                    syn::Error::new_spanned(f, s).into_compile_error()
                                })?;
                            }
                            st.merge_var = Some(merge_var);
                        }
                        Ok(Step::Join(mut join)) => {
                            if is_comparator {
                                join.validate_comparator().map_err(|s| {
                                    syn::Error::new_spanned(f, s).into_compile_error()
                                })?;
                            }
                            st.join = Some(join);
                        }
                        Err(e) => {
                            return Err(syn::Error::new_spanned(f, e).into_compile_error().into());
                        }
                    },
                },

                TraitItem::Macro(m) => {
                    return Err(syn::Error::new_spanned(m, "Unsupported")
                        .into_compile_error()
                        .into())
                }

                TraitItem::Type(t) => {
                    let mut item = t.clone();
                    let induce = match extract_induce_attr(&mut item.attrs) {
                        Ok(spec) => spec,
                        Err(e) => return Err(e.into_compile_error().into()),
                    };
                    st.assoc_types.push(AssocTypeSpec { item, induce });
                }
                _ => {}
            }
        }

        if !is_types_only && !is_fn_defined {
            let msg = &format!(
                    "Expected 1 function besides \"{NOTHING_FN_IDENT}\" \"{MERGE_FN_IDENT}\" \"{MERGE_VAR_FN_IDENT}\" or \"{JOIN_FN_IDENT}\"");
            return Err(syn::Error::new_spanned(tr, msg).into_compile_error().into());
        }

        Ok(st.finish(is_comparator, is_types_only))
    }

    fn finish(self, is_comparator: bool, is_types_only: bool) -> TokenStream {
        if is_types_only {
            return self.finish_types_only(is_comparator);
        }

        let State {
            mod_ident,
            trait_ident,
            trait_generics,
            trait_supertraits,
            property_ident,
            signature,
            vis,
            kind,
            fn_ident,
            fn_ret,
            fn_args,
            fn_args_list,
            fn_arg_idents,
            nothing,
            merge_field,
            merge_var,
            join,
            assoc_types,
            ..
        } = self;

        let has_non_type = trait_generics
            .params
            .iter()
            .any(|p| !matches!(p, GenericParam::Type(_)));
        if has_non_type {
            let msg = "Only type generics are currently supported for #[inception] traits.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        let type_params = trait_generics
            .params
            .iter()
            .filter_map(|p| match p {
                GenericParam::Type(t) => Some(t.ident.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let type_param_defs_no_default = trait_generics
            .params
            .iter()
            .filter_map(|p| match p {
                GenericParam::Type(t) => {
                    let mut t = t.clone();
                    t.default = None;
                    Some(t)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if type_params.len() > 1 {
            let msg = "Only a single type generic is currently supported for #[inception] traits.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        let assoc_type_idents = assoc_types
            .iter()
            .map(|t| t.item.ident.clone())
            .collect::<Vec<_>>();
        let mut input_assoc_ident =
            assoc_types
                .iter()
                .find_map(|t| match t.item.ident.to_string().as_str() {
                    "Input" | "In" => Some(t.item.ident.clone()),
                    _ => None,
                });
        let mut output_assoc_ident =
            assoc_types
                .iter()
                .find_map(|t| match t.item.ident.to_string().as_str() {
                    "Output" | "Out" => Some(t.item.ident.clone()),
                    _ => None,
                });
        let (flow_input_ident, flow_output_ident, flow_input_from_assoc) = if let Some(signature) =
            signature
        {
            let input_is_type_param = type_params.iter().any(|id| id == &signature.input);
            let input_is_assoc = assoc_type_idents.iter().any(|id| id == &signature.input);
            let flow_input_from_assoc = match (input_is_type_param, input_is_assoc) {
                (true, false) => {
                    input_assoc_ident = None;
                    false
                }
                (false, true) => {
                    input_assoc_ident = Some(signature.input.clone());
                    true
                }
                (true, true) => {
                    let msg = format!(
                            "Signature input `{}` is ambiguous; it matches both a type generic and an associated type.",
                            signature.input
                        );
                    return syn::Error::new_spanned(trait_ident.clone(), msg)
                        .into_compile_error()
                        .into();
                }
                (false, false) => {
                    let msg = format!(
                            "Signature input `{}` must name either a trait type generic or an associated type.",
                            signature.input
                        );
                    return syn::Error::new_spanned(trait_ident.clone(), msg)
                        .into_compile_error()
                        .into();
                }
            };

            let output_is_type_param = type_params.iter().any(|id| id == &signature.output);
            let output_is_assoc = assoc_type_idents.iter().any(|id| id == &signature.output);
            match (output_is_type_param, output_is_assoc) {
                (false, true) => {
                    output_assoc_ident = Some(signature.output.clone());
                }
                (true, false) => {
                    let msg = "Trait-generic output signatures are not yet supported; use an associated output type.";
                    return syn::Error::new_spanned(trait_ident.clone(), msg)
                        .into_compile_error()
                        .into();
                }
                (true, true) => {
                    let msg = format!(
                            "Signature output `{}` is ambiguous; it matches both a type generic and an associated type.",
                            signature.output
                        );
                    return syn::Error::new_spanned(trait_ident.clone(), msg)
                        .into_compile_error()
                        .into();
                }
                (false, false) => {
                    let msg = format!(
                        "Signature output `{}` must name an associated type.",
                        signature.output
                    );
                    return syn::Error::new_spanned(trait_ident.clone(), msg)
                        .into_compile_error()
                        .into();
                }
            }

            (Some(signature.input), None, flow_input_from_assoc)
        } else {
            if !type_params.is_empty() && input_assoc_ident.is_some() {
                let msg = "Use either a trait input generic or an associated input type, not both.";
                return syn::Error::new_spanned(trait_ident.clone(), msg)
                    .into_compile_error()
                    .into();
            }
            let flow_input_from_assoc = type_params.is_empty() && input_assoc_ident.is_some();
            let flow_input_ident = if flow_input_from_assoc {
                input_assoc_ident.clone()
            } else {
                type_params.first().cloned()
            };
            (flow_input_ident, None, flow_input_from_assoc)
        };
        if !matches!(kind, Kind::Ty)
            && assoc_types
                .iter()
                .any(|t| t.induce.is_some() && !t.item.generics.params.is_empty())
        {
            let msg = "GAT induction is currently only supported for type-style #[inception] traits (no self receiver).";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        if assoc_types
            .iter()
            .filter_map(|t| t.induce.as_ref())
            .any(InduceSpec::has_where_bounds)
        {
            let msg = "Per-phase `where { ... }` bounds in `#[induce(...)]` are currently supported only for `#[inception(..., types)]` traits.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        let Some(Nothing {
            nothing_body,
            nothing_ret,
            nothing_args,
            nothing_arg_idents,
        }) = nothing
        else {
            let msg = format!("Expected definition for \"{NOTHING_FN_IDENT}\"");
            return syn::Error::new_spanned(trait_ident, msg)
                .into_compile_error()
                .into();
        };
        let Some(MergeField {
            merge_body,
            merge_ret,
            merge_head_ident,
            merge_field_head_bounds,
            merge_fields_ident,
            merge_fields_bounds,
            merge_extra_generics,
            merge_head_arg,
            merge_fields_arg,
            merge_args,
            merge_arg_idents,
            ..
        }) = merge_field
        else {
            let msg = format!("Expected definition for \"{MERGE_FN_IDENT}\"");
            return syn::Error::new_spanned(trait_ident, msg)
                .into_compile_error()
                .into();
        };
        let Some(MergeVar {
            merge_var_body,
            merge_var_ret,
            merge_var_head_ident,
            merge_var_fields_ident,
            merge_var_field_head_bounds,
            merge_var_fields_bounds,
            merge_var_extra_generics,
            merge_var_head_arg,
            merge_var_fields_arg,
            merge_var_args,
            merge_var_arg_idents,
            ..
        }) = merge_var
        else {
            let msg = format!("Expected definition for \"{MERGE_VAR_FN_IDENT}\"");
            return syn::Error::new_spanned(trait_ident, msg)
                .into_compile_error()
                .into();
        };
        let Some(Join {
            join_body,
            join_ret,
            join_fields_ident,
            join_fields_bounds,
            join_fields_arg,
            join_args: join_extra_args,
            join_arg_idents,
            ..
        }) = join
        else {
            let msg = format!("Expected definition for \"{JOIN_FN_IDENT}\"");
            return syn::Error::new_spanned(trait_ident, msg)
                .into_compile_error()
                .into();
        };

        let mut fn_args_inner_list = fn_args_list.clone();
        let mut nothing_args = nothing_args;
        let mut merge_args = merge_args;
        let mut merge_var_args = merge_var_args;
        let mut join_extra_args = join_extra_args;
        let mut nothing_ret = nothing_ret;
        let mut merge_ret = merge_ret;
        let mut merge_var_ret = merge_var_ret;
        let mut join_ret = join_ret;
        fn is_self_assoc_path(qself: &Option<syn::QSelf>, path: &syn::Path, assoc: &Ident) -> bool {
            let direct_self_assoc = path.segments.len() == 2
                && path
                    .segments
                    .first()
                    .map(|s| s.ident == "Self")
                    .unwrap_or(false)
                && path
                    .segments
                    .last()
                    .map(|s| s.ident == *assoc)
                    .unwrap_or(false);
            if direct_self_assoc {
                return true;
            }
            let Some(last) = path.segments.last() else {
                return false;
            };
            if last.ident != *assoc {
                return false;
            }
            let Some(q) = qself else {
                return false;
            };
            matches!(
                q.ty.as_ref(),
                Type::Path(TypePath { qself: None, path })
                if path.is_ident("Self")
            )
        }
        fn replace_flow_type(ty: &mut Type, flow_in: Option<&Ident>, flow_out: Option<&Ident>) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(flow) = flow_in {
                        if path.is_ident(flow) || is_self_assoc_path(qself, path, flow) {
                            *ty = syn::parse_quote!(In);
                            return;
                        }
                    }
                    if let Some(flow) = flow_out {
                        if path.is_ident(flow) || is_self_assoc_path(qself, path, flow) {
                            *ty = syn::parse_quote!(Out);
                            return;
                        }
                    }
                    if let Some(q) = qself {
                        replace_flow_type(&mut q.ty, flow_in, flow_out);
                    }
                    for seg in path.segments.iter_mut() {
                        match &mut seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter_mut() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            replace_flow_type(inner, flow_in, flow_out)
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            replace_flow_type(&mut assoc.ty, flow_in, flow_out)
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter_mut() {
                                    replace_flow_type(input, flow_in, flow_out);
                                }
                                if let ReturnType::Type(_, out) = &mut args.output {
                                    replace_flow_type(out.as_mut(), flow_in, flow_out);
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                }
                Type::Reference(r) => replace_flow_type(r.elem.as_mut(), flow_in, flow_out),
                Type::Ptr(p) => replace_flow_type(p.elem.as_mut(), flow_in, flow_out),
                Type::Slice(s) => replace_flow_type(s.elem.as_mut(), flow_in, flow_out),
                Type::Array(a) => replace_flow_type(a.elem.as_mut(), flow_in, flow_out),
                Type::Tuple(t) => {
                    for elem in t.elems.iter_mut() {
                        replace_flow_type(elem, flow_in, flow_out);
                    }
                }
                Type::Paren(p) => replace_flow_type(p.elem.as_mut(), flow_in, flow_out),
                Type::Group(g) => replace_flow_type(g.elem.as_mut(), flow_in, flow_out),
                _ => {}
            }
        }
        let replace_flow_ty =
            |arg: &mut FnArg, flow_in: Option<&Ident>, flow_out: Option<&Ident>| {
                let FnArg::Typed(PatType { ty, .. }) = arg else {
                    return;
                };
                replace_flow_type(ty.as_mut(), flow_in, flow_out);
            };
        if flow_input_ident.is_some() || flow_output_ident.is_some() {
            for arg in fn_args_inner_list.iter_mut() {
                replace_flow_ty(arg, flow_input_ident.as_ref(), flow_output_ident.as_ref());
            }
            for arg in nothing_args.iter_mut() {
                replace_flow_ty(arg, flow_input_ident.as_ref(), flow_output_ident.as_ref());
            }
            for arg in merge_args.iter_mut() {
                replace_flow_ty(arg, flow_input_ident.as_ref(), flow_output_ident.as_ref());
            }
            for arg in merge_var_args.iter_mut() {
                replace_flow_ty(arg, flow_input_ident.as_ref(), flow_output_ident.as_ref());
            }
            for arg in join_extra_args.iter_mut() {
                replace_flow_ty(arg, flow_input_ident.as_ref(), flow_output_ident.as_ref());
            }
            if let Ok(mut ty) = syn::parse2::<Type>(nothing_ret.clone()) {
                replace_flow_type(
                    &mut ty,
                    flow_input_ident.as_ref(),
                    flow_output_ident.as_ref(),
                );
                nothing_ret = quote! { #ty };
            }
            if let Ok(mut ty) = syn::parse2::<Type>(merge_ret.clone()) {
                replace_flow_type(
                    &mut ty,
                    flow_input_ident.as_ref(),
                    flow_output_ident.as_ref(),
                );
                merge_ret = quote! { #ty };
            }
            if let Ok(mut ty) = syn::parse2::<Type>(merge_var_ret.clone()) {
                replace_flow_type(
                    &mut ty,
                    flow_input_ident.as_ref(),
                    flow_output_ident.as_ref(),
                );
                merge_var_ret = quote! { #ty };
            }
            if let Ok(mut ty) = syn::parse2::<Type>(join_ret.clone()) {
                replace_flow_type(
                    &mut ty,
                    flow_input_ident.as_ref(),
                    flow_output_ident.as_ref(),
                );
                join_ret = quote! { #ty };
            }
        }

        let inductive_ident = format_ident!("Inductive");
        let merge_fields_bounds_empty = merge_fields_bounds.is_empty();
        let merge_var_fields_bounds_empty = merge_var_fields_bounds.is_empty();

        let merge_field_head_bounds = merge_field_head_bounds
            .use_inner_trait(
                &trait_ident,
                &inductive_ident,
                &mod_ident,
                flow_input_ident.as_ref(),
                flow_output_ident.as_ref(),
            )
            .into_tokens();
        let merge_fields_bounds = merge_fields_bounds
            .use_inner_trait(
                &trait_ident,
                &inductive_ident,
                &mod_ident,
                flow_input_ident.as_ref(),
                flow_output_ident.as_ref(),
            )
            .into_tokens();
        let merge_var_field_head_bounds = merge_var_field_head_bounds
            .use_inner_trait(
                &trait_ident,
                &inductive_ident,
                &mod_ident,
                flow_input_ident.as_ref(),
                flow_output_ident.as_ref(),
            )
            .into_tokens();
        let merge_var_fields_bounds = merge_var_fields_bounds
            .use_inner_trait(
                &trait_ident,
                &inductive_ident,
                &mod_ident,
                flow_input_ident.as_ref(),
                flow_output_ident.as_ref(),
            )
            .into_tokens();
        let join_fields_bounds = join_fields_bounds
            .use_inner_trait(
                &trait_ident,
                &inductive_ident,
                &mod_ident,
                flow_input_ident.as_ref(),
                flow_output_ident.as_ref(),
            )
            .into_tokens();

        let dispatcher = kind.dispatcher();
        let comp_dispatcher =
            kind.comp_dispatcher(fn_arg_idents.get(0).unwrap_or(&format_ident!("_id")));
        let receiver = kind.receiver();
        let mutability = kind.mutability();
        let mutref = kind.refmut();
        let field = kind.field();
        let var_field = kind.var_field();
        let life2 = kind.life2();
        let life3 = kind.life3();
        let lifepunct1 = kind.lifepunct1();
        let lifepunct2 = kind.lifepunct2();
        let lifepunct3 = kind.lifepunct3();
        let liferef1 = kind.liferef1();
        let liferef2 = kind.liferef2();
        let liferefelide = kind.liferefelide();
        let lifepunctelide = kind.lifepunctelide();
        let access_bound = kind.access_bound(false, &merge_head_ident);
        let try_access_bound = kind.access_bound(true, &merge_var_head_ident);
        let phantom_bound = kind.phantom_bound();

        let fields_ident = kind.fields();
        let bracketlife2 = kind.bracketlife2();

        let wrapper = quote! { #mod_ident :: Wrap };
        let property = quote! { #property_ident };
        let inner_trait = quote! { #mod_ident :: #inductive_ident  };
        let inner_fn = format_ident!("{}", fn_ident);
        let is_type_style = matches!(kind, Kind::Ty);

        let fn_ret_public = match fn_ret {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_r, t) => quote! { #t },
        };
        let fn_ret_inner = if flow_input_ident.is_some() || flow_output_ident.is_some() {
            if let Ok(mut ty) = syn::parse2::<Type>(fn_ret_public.clone()) {
                replace_flow_type(
                    &mut ty,
                    flow_input_ident.as_ref(),
                    flow_output_ident.as_ref(),
                );
                quote! { #ty }
            } else {
                fn_ret_public.clone()
            }
        } else {
            fn_ret_public.clone()
        };
        let inner_fn_args = if fn_args_inner_list.is_empty() {
            quote! {}
        } else if is_type_style {
            quote! { #fn_args_inner_list }
        } else {
            quote! { , #fn_args_inner_list }
        };
        let fields_fn = kind.fields_fn(&property);
        let split_impl = kind.split_impl(&property, &wrapper);
        let split_trait_ident = kind.split_trait_ident();
        let split_fn_ident = kind.split_fn_ident();
        let split_fn_receiver = kind.split_fn_receiver();
        let split_trait_args_life1 = quote! { <#property> };
        let ret_lifetime = syn::Lifetime::new("'__inception_ret", proc_macro2::Span::call_site());
        let split_trait_args_ret = quote! { <#property> };
        let has_output_assoc = output_assoc_ident.is_some();
        let flow_mode = flow_input_ident.is_some();
        let flow_two_generic = flow_output_ident.is_some();
        let flow_assoc_borrow_mode =
            flow_mode && has_output_assoc && matches!(kind, Kind::Ref | Kind::Mut);
        let flow_assoc_merge_ret_ident = format_ident!("__InceptionTailRet");
        let flow_assoc_join_ret_ident = format_ident!("__InceptionJoinRet");
        let needs_borrow_output_helpers = has_output_assoc && matches!(kind, Kind::Ref | Kind::Mut);
        let fields_output_ident = format_ident!("FieldsOutput");
        let tail_output_ident = format_ident!("TailOutput");
        let fields_input_ident = format_ident!("FieldsInput");
        let borrow_output_helpers = if needs_borrow_output_helpers {
            match kind {
                Kind::Ref => quote! {
                    pub trait #fields_output_ident<T, In, Out = In> {
                        type Ret;
                    }
                    impl<T, In, Out, Ret> #fields_output_ident<T, In, Out> for ()
                    where
                        T: ::inception::Inception<super::#property_ident>,
                        for<'a> Wrap<&'a <T as ::inception::Inception<super::#property_ident>>::#fields_ident<'a>>:
                            Inductive<::inception::False, In, Out, Ret = Ret>,
                    {
                        type Ret = Ret;
                    }
                    pub trait #tail_output_ident<F, In, Out = In> {
                        type Ret;
                    }
                    impl<F, In, Out, Ret> #tail_output_ident<F, In, Out> for ()
                    where
                        for<'a> Wrap<&'a F>: Inductive<::inception::False, In, Out, Ret = Ret>,
                    {
                        type Ret = Ret;
                    }
                },
                Kind::Mut => quote! {
                    pub trait #fields_output_ident<T, In, Out = In> {
                        type Ret;
                    }
                    impl<T, In, Out, Ret> #fields_output_ident<T, In, Out> for ()
                    where
                        T: ::inception::Inception<super::#property_ident>,
                        for<'a> Wrap<&'a mut <T as ::inception::Inception<super::#property_ident>>::#fields_ident<'a>>:
                            Inductive<::inception::False, In, Out, Ret = Ret>,
                    {
                        type Ret = Ret;
                    }
                    pub trait #tail_output_ident<F, In, Out = In> {
                        type Ret;
                    }
                    impl<F, In, Out, Ret> #tail_output_ident<F, In, Out> for ()
                    where
                        for<'a> Wrap<&'a mut F>: Inductive<::inception::False, In, Out, Ret = Ret>,
                    {
                        type Ret = Ret;
                    }
                },
                _ => quote! {},
            }
        } else {
            quote! {}
        };
        let flow_input_helpers = if flow_input_from_assoc {
            if let Some(input_assoc_ident) = input_assoc_ident.as_ref() {
                let fields_input_trait_def_params = if type_param_defs_no_default.is_empty() {
                    quote! { <T> }
                } else {
                    quote! { <T, #(#type_param_defs_no_default),*> }
                };
                let fields_input_trait_use_params = if type_params.is_empty() {
                    quote! { <T> }
                } else {
                    quote! { <T, #(#type_params),*> }
                };
                let flow_trait_generic_args = if type_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#type_params),*> }
                };
                quote! {
                    pub trait #fields_input_ident #fields_input_trait_def_params {
                        type In;
                    }
                    impl #fields_input_trait_def_params #fields_input_ident #fields_input_trait_use_params for ()
                    where
                        T: ::inception::Inception<super::#property_ident>,
                        <<T as ::inception::Inception<super::#property_ident>>::TyFields as ::inception::Fields>::Head: ::inception::Field,
                        <<<T as ::inception::Inception<super::#property_ident>>::TyFields as ::inception::Fields>::Head as ::inception::Field>::Content:
                            super::#trait_ident #flow_trait_generic_args,
                    {
                        type In =
                            <<<<T as ::inception::Inception<super::#property_ident>>::TyFields as ::inception::Fields>::Head as ::inception::Field>::Content as super::#trait_ident #flow_trait_generic_args>::#input_assoc_ident;
                    }
                }
            } else {
                quote! {}
            }
        } else {
            quote! {}
        };
        let needs_named_ret_lifetime =
            has_output_assoc && matches!(kind, Kind::Ref | Kind::Mut) && !flow_assoc_borrow_mode;
        let internal_trait_generic_args = if flow_input_from_assoc && !type_params.is_empty() {
            quote! { , #(#type_params),* }
        } else {
            quote! {}
        };
        let internal_trait_single_input_args = if flow_input_from_assoc && !type_params.is_empty() {
            quote! { , In #internal_trait_generic_args }
        } else {
            quote! {}
        };
        let ret_lifepunct = if needs_named_ret_lifetime {
            quote! { #ret_lifetime, }
        } else {
            quote! {}
        };
        let ret_liferef = if needs_named_ret_lifetime {
            match kind {
                Kind::Ref => quote! { & #ret_lifetime },
                Kind::Mut => quote! { & #ret_lifetime mut },
                _ => quote! {},
            }
        } else {
            quote! {}
        };
        let merge_head_out_ty = if flow_input_ident.is_some() {
            quote! {
                <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args>>::Ret
            }
        } else {
            quote! {
                <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args>>::OutTy
            }
        };
        let merge_split_right_ty = quote! {
            #wrapper<#liferef1 <#wrapper<#liferef2 List<(#field<#lifepunct3 #merge_head_ident, S, IDX>, F)>> as #split_trait_ident #split_trait_args_life1>::Right>
        };
        let merge_split_right_ty_named_ret = quote! {
            #wrapper<#ret_liferef <#wrapper<#ret_liferef List<(#field<#ret_lifepunct #merge_head_ident, S, IDX>, F)>> as #split_trait_ident #split_trait_args_ret>::Right>
        };
        let merge_inductive_self_ty = if needs_named_ret_lifetime {
            quote! { #wrapper<#ret_liferef List<(#field<#ret_lifepunct #merge_head_ident, S, IDX>, F)>> }
        } else {
            quote! { #wrapper<#liferefelide List<(#field<#lifepunctelide #merge_head_ident, S, IDX>, F)>> }
        };
        let merge_var_head_out_ty = if flow_input_ident.is_some() {
            quote! {
                <#merge_var_head_ident as #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args>>::Ret
            }
        } else {
            quote! {
                <#merge_var_head_ident as #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args>>::OutTy
            }
        };
        let merge_var_split_right_ty = quote! {
            #wrapper<#liferef1 <#wrapper<#liferef2 List<(#var_field<#lifepunct3 #merge_var_head_ident, S, VAR_IDX, IDX>, F)>> as #split_trait_ident #split_trait_args_life1>::Right>
        };
        let merge_var_split_right_ty_named_ret = quote! {
            #wrapper<#ret_liferef <#wrapper<#ret_liferef List<(#var_field<#ret_lifepunct #merge_var_head_ident, S, VAR_IDX, IDX>, F)>> as #split_trait_ident #split_trait_args_ret>::Right>
        };
        let merge_var_inductive_self_ty = if needs_named_ret_lifetime {
            quote! { #wrapper<#ret_liferef List<(#var_field<#ret_lifepunct #merge_var_head_ident, S, VAR_IDX, IDX>, F)>> }
        } else {
            quote! { #wrapper<#liferefelide List<(#var_field<#lifepunctelide #merge_var_head_ident, S, VAR_IDX, IDX>, F)>> }
        };
        let join_wrapper_fields_ty = quote! {
            #wrapper<#liferef1 <T as Inception<#property>>::#fields_ident #bracketlife2>
        };
        let join_wrapper_fields_ty_named_ret = quote! {
            #wrapper<#ret_liferef <T as Inception<#property>>::#fields_ident <#ret_lifetime>>
        };
        fn replace_type_ident(ty: &mut Type, target: &Ident, replacement: &Type) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if path.is_ident(target) {
                        *ty = replacement.clone();
                        return;
                    }
                    if let Some(q) = qself {
                        replace_type_ident(&mut q.ty, target, replacement);
                    }
                    for seg in path.segments.iter_mut() {
                        match &mut seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter_mut() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            replace_type_ident(inner, target, replacement)
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            replace_type_ident(&mut assoc.ty, target, replacement)
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter_mut() {
                                    replace_type_ident(input, target, replacement);
                                }
                                if let ReturnType::Type(_, out) = &mut args.output {
                                    replace_type_ident(out.as_mut(), target, replacement);
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                }
                Type::Reference(r) => replace_type_ident(r.elem.as_mut(), target, replacement),
                Type::Ptr(p) => replace_type_ident(p.elem.as_mut(), target, replacement),
                Type::Slice(s) => replace_type_ident(s.elem.as_mut(), target, replacement),
                Type::Array(a) => replace_type_ident(a.elem.as_mut(), target, replacement),
                Type::Tuple(t) => {
                    for elem in t.elems.iter_mut() {
                        replace_type_ident(elem, target, replacement);
                    }
                }
                Type::Paren(p) => replace_type_ident(p.elem.as_mut(), target, replacement),
                Type::Group(g) => replace_type_ident(g.elem.as_mut(), target, replacement),
                _ => {}
            }
        }
        let substitute_ret_ident =
            |ret: &proc_macro2::TokenStream,
             target: &Ident,
             replacement: &proc_macro2::TokenStream| {
                let Ok(mut ty) = syn::parse2::<Type>(ret.clone()) else {
                    return ret.clone();
                };
                let Ok(repl_ty) = syn::parse2::<Type>(replacement.clone()) else {
                    return ret.clone();
                };
                replace_type_ident(&mut ty, target, &repl_ty);
                quote! { #ty }
            };
        fn rewrite_assoc_projection_to_helper(
            ty: &mut Type,
            trait_ident: &Ident,
            assoc_ident: &Ident,
            source_placeholder: &Ident,
            helper_ident: &Ident,
            helper_self_ty: &proc_macro2::TokenStream,
            helper_trait_args: &proc_macro2::TokenStream,
        ) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(q) = qself {
                        rewrite_assoc_projection_to_helper(
                            &mut q.ty,
                            trait_ident,
                            assoc_ident,
                            source_placeholder,
                            helper_ident,
                            helper_self_ty,
                            helper_trait_args,
                        );
                    }
                    for seg in path.segments.iter_mut() {
                        match &mut seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter_mut() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            rewrite_assoc_projection_to_helper(
                                                inner,
                                                trait_ident,
                                                assoc_ident,
                                                source_placeholder,
                                                helper_ident,
                                                helper_self_ty,
                                                helper_trait_args,
                                            )
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            rewrite_assoc_projection_to_helper(
                                                &mut assoc.ty,
                                                trait_ident,
                                                assoc_ident,
                                                source_placeholder,
                                                helper_ident,
                                                helper_self_ty,
                                                helper_trait_args,
                                            )
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter_mut() {
                                    rewrite_assoc_projection_to_helper(
                                        input,
                                        trait_ident,
                                        assoc_ident,
                                        source_placeholder,
                                        helper_ident,
                                        helper_self_ty,
                                        helper_trait_args,
                                    );
                                }
                                if let ReturnType::Type(_, out) = &mut args.output {
                                    rewrite_assoc_projection_to_helper(
                                        out.as_mut(),
                                        trait_ident,
                                        assoc_ident,
                                        source_placeholder,
                                        helper_ident,
                                        helper_self_ty,
                                        helper_trait_args,
                                    );
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                    let Some(q) = qself.as_ref() else {
                        return;
                    };
                    if path.segments.len() < 2 {
                        return;
                    }
                    let assoc_seg = path.segments.last().map(|s| s.ident.clone());
                    let tr_seg = path.segments.iter().rev().nth(1).map(|s| s.ident.clone());
                    let (Some(assoc_seg), Some(tr_seg)) = (assoc_seg, tr_seg) else {
                        return;
                    };
                    if tr_seg != *trait_ident || assoc_seg != *assoc_ident {
                        return;
                    }
                    let Type::Path(TypePath {
                        qself: None,
                        path: q_path,
                    }) = q.ty.as_ref()
                    else {
                        return;
                    };
                    let Some(q_ident) = q_path.get_ident() else {
                        return;
                    };
                    if *q_ident != *source_placeholder {
                        return;
                    }
                    let Some(last_seg) = path.segments.last() else {
                        return;
                    };
                    let ret_ty = match &last_seg.arguments {
                        syn::PathArguments::AngleBracketed(ab) if !ab.args.is_empty() => {
                            let args = &ab.args;
                            quote! {
                                <#helper_self_ty as #helper_ident #helper_trait_args>::Ret<#args>
                            }
                        }
                        syn::PathArguments::AngleBracketed(_) | syn::PathArguments::None => {
                            quote! {
                                <#helper_self_ty as #helper_ident #helper_trait_args>::Ret
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => {
                            return;
                        }
                    };
                    *ty = syn::parse_quote! { #ret_ty };
                }
                Type::Reference(r) => rewrite_assoc_projection_to_helper(
                    r.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Ptr(p) => rewrite_assoc_projection_to_helper(
                    p.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Slice(s) => rewrite_assoc_projection_to_helper(
                    s.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Array(a) => rewrite_assoc_projection_to_helper(
                    a.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Tuple(t) => {
                    for elem in t.elems.iter_mut() {
                        rewrite_assoc_projection_to_helper(
                            elem,
                            trait_ident,
                            assoc_ident,
                            source_placeholder,
                            helper_ident,
                            helper_self_ty,
                            helper_trait_args,
                        );
                    }
                }
                Type::Paren(p) => rewrite_assoc_projection_to_helper(
                    p.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Group(g) => rewrite_assoc_projection_to_helper(
                    g.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                _ => {}
            }
        }
        let merge_ret_inductive = if flow_assoc_borrow_mode {
            quote! { #flow_assoc_merge_ret_ident }
        } else {
            substitute_ret_ident(
                &merge_ret,
                &merge_fields_ident,
                if needs_named_ret_lifetime {
                    &merge_split_right_ty_named_ret
                } else {
                    &merge_split_right_ty
                },
            )
        };
        let merge_var_ret_inductive = substitute_ret_ident(
            &merge_var_ret,
            &merge_var_fields_ident,
            if needs_named_ret_lifetime {
                &merge_var_split_right_ty_named_ret
            } else {
                &merge_var_split_right_ty
            },
        );
        let join_ret_inductive = if needs_borrow_output_helpers {
            if flow_assoc_borrow_mode {
                quote! { #flow_assoc_join_ret_ident }
            } else if flow_mode {
                quote! { <() as #mod_ident::#fields_output_ident<Self, In, Out>>::Ret }
            } else {
                quote! { <() as #mod_ident::#fields_output_ident<Self, In>>::Ret }
            }
        } else if needs_named_ret_lifetime {
            substitute_ret_ident(
                &join_ret,
                &join_fields_ident,
                &join_wrapper_fields_ty_named_ret,
            )
        } else {
            substitute_ret_ident(&join_ret, &join_fields_ident, &join_wrapper_fields_ty)
        };
        let trait_where_clause = &trait_generics.where_clause;
        let trait_generic_params = {
            let params = &trait_generics.params;
            if params.is_empty() {
                quote! {}
            } else {
                quote! { <#params> }
            }
        };
        let trait_generic_args = if type_params.is_empty() {
            quote! {}
        } else {
            quote! { <#(#type_params),*> }
        };
        let trait_impl_generic_params = if type_param_defs_no_default.is_empty() {
            quote! { <T> }
        } else {
            quote! { <T, #(#type_param_defs_no_default),*> }
        };
        let internal_trait_decl_generic_defs =
            if flow_input_from_assoc && !trait_generics.params.is_empty() {
                let params = &trait_generics.params;
                quote! { , #params }
            } else {
                quote! {}
            };
        let internal_trait_impl_generic_defs =
            if flow_input_from_assoc && !type_param_defs_no_default.is_empty() {
                quote! { , #(#type_param_defs_no_default),* }
            } else {
                quote! {}
            };
        let fields_input_trait_use_params = if type_params.is_empty() {
            quote! { <T> }
        } else {
            quote! { <T, #(#type_params),*> }
        };
        let compat_impl = if flow_input_ident.is_some() {
            quote! {}
        } else {
            quote! {
                impl<T> ::inception::Compat<T> for super::#property_ident
                where
                    T: super::#trait_ident,
                {
                    type Out = True;
                }
            }
        };
        let inferred_input_ty =
            quote! { <() as #mod_ident::#fields_input_ident #fields_input_trait_use_params>::In };
        let inferred_input_inner_trait_args = if flow_input_from_assoc && !type_params.is_empty() {
            quote! { #inferred_input_ty, #inferred_input_ty #internal_trait_generic_args }
        } else {
            quote! { #inferred_input_ty #internal_trait_generic_args }
        };
        let trait_bound_with_in = if flow_input_from_assoc {
            let Some(input_assoc_ident) = input_assoc_ident.as_ref() else {
                return syn::Error::new_spanned(
                    trait_ident,
                    "Associated-input mode requires a trait associated type named `Input` or `In`.",
                )
                .into_compile_error()
                .into();
            };
            if type_params.is_empty() {
                quote! { super::#trait_ident<#input_assoc_ident = In> }
            } else {
                quote! { super::#trait_ident<#(#type_params,)* #input_assoc_ident = In> }
            }
        } else {
            match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                (Some(_), Some(_)) => quote! { super::#trait_ident<In, Out> },
                (Some(_), None) => quote! { super::#trait_ident<In> },
                (None, _) => quote! { super::#trait_ident },
            }
        };
        let trait_path_for_ufcs = if type_params.is_empty() {
            quote! { super::#trait_ident }
        } else {
            quote! { super::#trait_ident<#(#type_params),*> }
        };
        let primitive_impl_generics = match (flow_input_ident.as_ref(), flow_output_ident.as_ref())
        {
            (Some(_), Some(_)) => quote! { <T, In, Out> },
            (Some(_), None) => {
                if flow_input_from_assoc && !type_param_defs_no_default.is_empty() {
                    quote! { <T, #(#type_param_defs_no_default),*, In> }
                } else {
                    quote! { <T, In> }
                }
            }
            (None, _) => quote! { <T, In> },
        };
        let primitive_impl_trait_args =
            match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                (Some(_), Some(_)) => quote! { <True, In, Out #internal_trait_generic_args> },
                (Some(_), None) => quote! { <True, In #internal_trait_single_input_args> },
                (None, _) => quote! { <True, In #internal_trait_generic_args> },
            };
        let blanket_impl_head = if flow_input_from_assoc {
            quote! { impl #trait_impl_generic_params #trait_ident #trait_generic_args for T }
        } else {
            match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                (Some(flow_in), Some(flow_out)) => {
                    quote! { impl<T, #flow_in, #flow_out> #trait_ident<#flow_in, #flow_out> for T }
                }
                (Some(flow), None) => quote! { impl<T, #flow> #trait_ident<#flow> for T },
                (None, _) => quote! { impl<T> #trait_ident for T },
            }
        };
        let blanket_inner_bound = if flow_input_from_assoc {
            quote! { #inner_trait<::inception::False, #inferred_input_inner_trait_args> }
        } else {
            match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                (Some(flow_in), Some(flow_out)) => {
                    quote! { #inner_trait<::inception::False, #flow_in, #flow_out, Ret = #flow_out> }
                }
                (Some(flow), None) => quote! { #inner_trait<::inception::False, #flow> },
                (None, _) => quote! { #inner_trait<::inception::False, Ret = #fn_ret_public> },
            }
        };
        let blanket_dispatch_body = if flow_input_from_assoc {
            if matches!(kind, Kind::Ty) {
                quote! {
                    <Self as #inner_trait<::inception::False, #inferred_input_inner_trait_args>>::#inner_fn(#fn_arg_idents)
                }
            } else {
                quote! {
                    <Self as #inner_trait<::inception::False, #inferred_input_inner_trait_args>>::#inner_fn(self, #fn_arg_idents)
                }
            }
        } else {
            quote! { #dispatcher #inner_fn(#fn_arg_idents) }
        };
        let primitive_ret = if let Some(output_assoc_ident) = output_assoc_ident.as_ref() {
            if flow_input_from_assoc {
                quote! { <T as super::#trait_ident #trait_generic_args>::#output_assoc_ident }
            } else {
                quote! { <T as #trait_bound_with_in>::#output_assoc_ident }
            }
        } else {
            quote! { #fn_ret_inner }
        };
        let primitive_dispatch_body = if flow_input_from_assoc {
            if matches!(kind, Kind::Ty) {
                quote! { <T as #trait_path_for_ufcs>::#fn_ident(#fn_arg_idents) }
            } else {
                quote! { <T as #trait_path_for_ufcs>::#fn_ident(self, #fn_arg_idents) }
            }
        } else {
            quote! { #dispatcher #fn_ident( #fn_arg_idents ) }
        };
        let primitive_out_ty = if flow_two_generic {
            quote! { Out }
        } else {
            quote! { In }
        };
        let assoc_trait_items = assoc_types
            .iter()
            .map(|t| {
                let item = &t.item;
                quote! { #item }
            })
            .collect::<Vec<_>>();
        let assoc_impl_items = assoc_types
            .iter()
            .filter_map(|t| {
                let ident = &t.item.ident;
                let assoc_generics = &t.item.generics;
                let assoc_params = assoc_generics.params.iter().cloned().collect::<Vec<_>>();
                let assoc_use_args = assoc_generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            let id = &tp.ident;
                            quote! { #id }
                        }
                        GenericParam::Lifetime(lp) => {
                            let lt = &lp.lifetime;
                            quote! { #lt }
                        }
                        GenericParam::Const(cp) => {
                            let id = &cp.ident;
                            quote! { #id }
                        }
                    })
                    .collect::<Vec<_>>();
                let assoc_impl_generics = if assoc_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_params),*> }
                };
                let assoc_where_clause = &assoc_generics.where_clause;
                let helper_ret_use = if assoc_use_args.is_empty() {
                    quote! { Ret }
                } else {
                    quote! { Ret<#(#assoc_use_args),*> }
                };
                if t.induce.is_some() {
                    let helper_ident = format_ident!("__InceptionInduce{}", ident);
                    if flow_input_from_assoc {
                        Some(quote! {
                            type #ident #assoc_impl_generics = <T as #mod_ident::#helper_ident<::inception::False, #inferred_input_inner_trait_args>>::#helper_ret_use #assoc_where_clause;
                        })
                    } else if let Some(flow_in) = flow_input_ident.as_ref() {
                        if let Some(flow_out) = flow_output_ident.as_ref() {
                            Some(quote! {
                                type #ident #assoc_impl_generics = <T as #mod_ident::#helper_ident<::inception::False, #flow_in, #flow_out>>::#helper_ret_use #assoc_where_clause;
                            })
                        } else {
                            Some(quote! {
                                type #ident #assoc_impl_generics = <T as #mod_ident::#helper_ident<::inception::False, #flow_in>>::#helper_ret_use #assoc_where_clause;
                            })
                        }
                    } else {
                        Some(quote! {
                            type #ident #assoc_impl_generics = <T as #mod_ident::#helper_ident<::inception::False>>::#helper_ret_use #assoc_where_clause;
                        })
                    }
                } else if flow_input_from_assoc && input_assoc_ident.as_ref() == Some(ident) {
                    Some(quote! {
                        type #ident #assoc_impl_generics = #inferred_input_ty #assoc_where_clause;
                    })
                } else if output_assoc_ident.as_ref() == Some(ident) {
                    if flow_input_from_assoc {
                        Some(quote! {
                            type #ident #assoc_impl_generics = <T as #inner_trait<::inception::False, #inferred_input_inner_trait_args>>::Ret #assoc_where_clause;
                        })
                    } else if let Some(flow) = flow_input_ident.as_ref() {
                        Some(quote! {
                            type #ident #assoc_impl_generics = <T as #inner_trait<::inception::False, #flow>>::Ret #assoc_where_clause;
                        })
                    } else {
                        Some(quote! {
                            type #ident #assoc_impl_generics = <T as #inner_trait<::inception::False>>::Ret #assoc_where_clause;
                        })
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let trait_supertrait_clause = if trait_supertraits.is_empty() {
            quote! {}
        } else {
            quote! { : #trait_supertraits }
        };
        let trait_supertrait_bounds = if trait_supertraits.is_empty() {
            quote! {}
        } else {
            quote! { + #trait_supertraits }
        };
        let induced_blanket_where_preds = assoc_types
            .iter()
            .filter(|t| t.induce.is_some())
            .map(|t| {
                let helper_ident = format_ident!("__InceptionInduce{}", t.item.ident);
                if flow_input_from_assoc {
                    quote! {
                        T: #mod_ident::#helper_ident<::inception::False, #inferred_input_inner_trait_args>,
                    }
                } else if let Some(flow_in) = flow_input_ident.as_ref() {
                    if let Some(flow_out) = flow_output_ident.as_ref() {
                        quote! {
                            T: #mod_ident::#helper_ident<::inception::False, #flow_in, #flow_out>,
                        }
                    } else {
                        quote! {
                            T: #mod_ident::#helper_ident<::inception::False, #flow_in>,
                        }
                    }
                } else {
                    quote! {
                        T: #mod_ident::#helper_ident<::inception::False>,
                    }
                }
            })
            .collect::<Vec<_>>();
        let blanket_assoc_where_preds =
            if flow_input_from_assoc || !induced_blanket_where_preds.is_empty() {
                let flow_input_preds = if flow_input_from_assoc {
                    quote! {
                        T: Inception<#property>,
                        (): #mod_ident::#fields_input_ident #fields_input_trait_use_params,
                    }
                } else {
                    quote! {}
                };
                quote! {
                    #flow_input_preds
                    #(#induced_blanket_where_preds)*
                }
            } else {
                quote! {}
            };

        let placeholder = format_ident!("_");
        let (merge_arg_head, merge_arg_tail) = (
            merge_arg_idents.get(0).unwrap_or(&placeholder),
            merge_arg_idents.get(1).unwrap_or(&placeholder),
        );
        let (merge_var_arg_head, merge_var_arg_tail) = (
            merge_arg_idents.get(0).unwrap_or(&placeholder),
            merge_arg_idents.get(1).unwrap_or(&placeholder),
        );
        let join_arg_fields = join_arg_idents
            .get(0)
            .cloned()
            .unwrap_or(placeholder.clone());
        let comparator_fields_fn = if is_comparator {
            kind.comparator_fields_fn(&join_arg_fields, &property)
        } else {
            quote! {}
        };
        let merge_comparator_body = if is_comparator {
            quote! {
                let (#mutability #merge_arg_head, #mutability #merge_arg_tail) = #comp_dispatcher #split_fn_ident();
                let #mutability #merge_arg_tail = #wrapper(#merge_arg_tail);
            }
        } else {
            quote! {}
        };
        let merge_var_comparator_body = if is_comparator {
            quote! {
                    let (#mutability #merge_var_arg_head, #mutability #merge_var_arg_tail) = #comp_dispatcher #split_fn_ident();
                    let #mutability #merge_var_arg_tail = #wrapper(#merge_var_arg_tail);
            }
        } else {
            quote! {}
        };
        let join_comparator_body = if is_comparator {
            quote! {
                #comparator_fields_fn
                let #join_arg_fields = #wrapper(#mutref #join_arg_fields);
            }
        } else {
            quote! {}
        };
        let merge_extra_tuple_ty = if merge_extra_generics.is_empty() {
            quote! { () }
        } else {
            quote! { (#(#merge_extra_generics),*,) }
        };
        let merge_var_extra_tuple_ty = if merge_var_extra_generics.is_empty() {
            quote! { () }
        } else {
            quote! { (#(#merge_var_extra_generics),*,) }
        };
        let merge_field_impl_generics = if flow_mode {
            quote! { <#lifepunct1 #merge_head_ident, S, const IDX: usize, F, L, #merge_fields_ident, #(#merge_extra_generics,)* In, Out #internal_trait_impl_generic_defs> }
        } else {
            quote! { <#lifepunct1 #merge_head_ident, S, const IDX: usize, F, L, #merge_fields_ident, In #internal_trait_impl_generic_defs> }
        };
        let merge_field_impl_trait_args = if flow_mode {
            quote! { <L, #merge_fields_ident, In, Out, #merge_extra_tuple_ty #internal_trait_generic_args> }
        } else {
            quote! { <L, #merge_fields_ident, In, In, #merge_extra_tuple_ty #internal_trait_generic_args> }
        };
        let merge_field_head_bound = if flow_two_generic {
            quote! { ::core::marker::Sized #merge_field_head_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            if flow_assoc_borrow_mode {
                quote! { #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args> + ::inception::IsPrimitive<#property> }
            } else {
                quote! { #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args> #merge_field_head_bounds + ::inception::IsPrimitive<#property> }
            }
        } else {
            quote! { #inner_trait #merge_field_head_bounds + ::inception::IsPrimitive<#property> }
        };
        let merge_field_tail_bound = if flow_two_generic {
            quote! { Fields #merge_fields_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            if flow_assoc_borrow_mode {
                quote! { Fields + #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args> + ::inception::IsPrimitive<#property> }
            } else {
                quote! { Fields + #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args> #merge_fields_bounds + ::inception::IsPrimitive<#property> }
            }
        } else {
            quote! { Fields + #inner_trait #merge_fields_bounds + ::inception::IsPrimitive<#property> }
        };
        let merge_field_impl_out_ty = if flow_mode {
            quote! { Out }
        } else {
            quote! { In }
        };
        let merge_variant_impl_generics = if flow_mode {
            quote! { <#lifepunct1 #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, L, #merge_var_fields_ident, #(#merge_var_extra_generics,)* In, Out #internal_trait_impl_generic_defs> }
        } else {
            quote! { <#lifepunct1 #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, L, #merge_var_fields_ident, In #internal_trait_impl_generic_defs> }
        };
        let merge_variant_impl_trait_args = if flow_mode {
            quote! { <L, #merge_var_fields_ident, In, Out, #merge_var_extra_tuple_ty #internal_trait_generic_args> }
        } else {
            quote! { <L, #merge_var_fields_ident, In, In, #merge_var_extra_tuple_ty #internal_trait_generic_args> }
        };
        let merge_variant_head_bound = if flow_two_generic {
            quote! { ::core::marker::Sized #merge_var_field_head_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            quote! { #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args> #merge_var_field_head_bounds + ::inception::IsPrimitive<#property> }
        } else {
            quote! { #inner_trait #merge_var_field_head_bounds + ::inception::IsPrimitive<#property> }
        };
        let merge_variant_tail_bound = if flow_two_generic {
            quote! { Fields #merge_var_fields_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            quote! { Fields + #inner_trait<::inception::False, #merge_var_head_out_ty, Out #internal_trait_generic_args> #merge_var_fields_bounds + ::inception::IsPrimitive<#property> }
        } else {
            quote! { Fields + #inner_trait #merge_var_fields_bounds + ::inception::IsPrimitive<#property> }
        };
        let merge_variant_impl_out_ty = if flow_mode {
            quote! { Out }
        } else {
            quote! { In }
        };
        let join_impl_generics = if flow_mode {
            quote! { <T, #join_fields_ident, In, Out #internal_trait_impl_generic_defs> }
        } else {
            quote! { <T, #join_fields_ident, In #internal_trait_impl_generic_defs> }
        };
        let join_impl_trait_args = if flow_mode {
            quote! { <#join_fields_ident, In, Out #internal_trait_generic_args> }
        } else {
            quote! { <#join_fields_ident, In, In #internal_trait_generic_args> }
        };
        let join_fields_bound = if flow_mode {
            if flow_assoc_borrow_mode {
                quote! { #inner_trait<::inception::False, In, Out #internal_trait_generic_args> + ::inception::IsPrimitive<#property> }
            } else {
                quote! { #inner_trait<::inception::False, In, Out #internal_trait_generic_args> #join_fields_bounds + ::inception::IsPrimitive<#property> }
            }
        } else {
            quote! { #inner_trait #join_fields_bounds + ::inception::IsPrimitive<#property> }
        };
        let join_impl_out_ty = if flow_mode {
            quote! { Out }
        } else {
            quote! { In }
        };
        let join_impl_ret_ty = if flow_assoc_borrow_mode {
            quote! { <#join_fields_ident as #inner_trait<::inception::False, In, Out #internal_trait_generic_args>>::Ret }
        } else {
            quote! { #join_ret }
        };
        let merge_call_trait_args = if flow_mode {
            quote! { <_, _, In, Out, _ #internal_trait_generic_args> }
        } else {
            quote! { <_, _, In, In, _ #internal_trait_generic_args> }
        };
        let merge_variant_call_trait_args = if flow_mode {
            quote! { <_, _, In, Out, _ #internal_trait_generic_args> }
        } else {
            quote! { <_, _, In, In, _ #internal_trait_generic_args> }
        };
        let join_call_trait_args = if flow_mode {
            quote! { <_, In, Out #internal_trait_generic_args> }
        } else {
            quote! { <_, In #internal_trait_generic_args> }
        };
        let ret_lifetime_decl = if needs_named_ret_lifetime {
            quote! { #ret_lifetime, }
        } else {
            quote! {}
        };
        let split_for_2 = quote! { for<#lifepunct1 #life2> };
        let split_for_3 = quote! { for<#lifepunct1 #lifepunct2 #life3> };
        let merge_split_trait_generic = quote! {
            #split_trait_ident<#property, Left = #field<#lifepunct2 #merge_head_ident, S, IDX>, Right = F>
        };
        let merge_split_bound = if needs_named_ret_lifetime {
            quote! {
                #split_for_2 #wrapper<#liferef1 List<(#field<#lifepunct2 #merge_head_ident, S, IDX>, F)>>:
                    #merge_split_trait_generic,
            }
        } else {
            quote! {
                #split_for_2 #wrapper<#liferef1 List<(#field<#lifepunct2 #merge_head_ident, S, IDX>, F)>>:
                    #merge_split_trait_generic,
            }
        };
        let merge_tail_extra_bounds = if flow_assoc_borrow_mode {
            quote! {}
        } else if merge_extra_generics.is_empty() {
            quote! { #merge_fields_bounds }
        } else {
            quote! {}
        };
        let merge_tail_two_generic_rhs = if merge_extra_generics.is_empty() {
            if merge_fields_bounds_empty {
                quote! { Fields }
            } else {
                quote! { #merge_fields_bounds + Fields }
            }
        } else {
            quote! { Fields }
        };
        let merge_tail_bound = if flow_two_generic {
            quote! {
                #split_for_3 #merge_split_right_ty:
                    #merge_tail_two_generic_rhs,
            }
        } else if flow_assoc_borrow_mode {
            match kind {
                Kind::Ref => quote! {
                    for<'a> #wrapper<&'a F>:
                        #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args, Ret = #flow_assoc_merge_ret_ident> + Fields,
                },
                Kind::Mut => quote! {
                    for<'a> #wrapper<&'a mut F>:
                        #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args, Ret = #flow_assoc_merge_ret_ident> + Fields,
                },
                _ => quote! {
                    #split_for_3 #merge_split_right_ty:
                        #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args> #merge_tail_extra_bounds + Fields,
                },
            }
        } else {
            quote! {
                #split_for_3 #merge_split_right_ty:
                    #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args> #merge_tail_extra_bounds + Fields,
            }
        };
        let merge_named_output_eq_bound = if flow_assoc_borrow_mode {
            quote! {}
        } else if needs_named_ret_lifetime && has_output_assoc {
            quote! {
                #split_for_3 #merge_split_right_ty:
                    #inner_trait<::inception::False, #merge_head_out_ty, Ret = #merge_ret_inductive>,
            }
        } else {
            quote! {}
        };
        let merge_var_split_bound = if needs_named_ret_lifetime {
            let merge_var_split_trait_generic = quote! {
                #split_trait_ident<#property, Left = #var_field<#lifepunct2 #merge_var_head_ident, S, VAR_IDX, IDX>, Right = F>
            };
            quote! {
                #split_for_2 #wrapper<#liferef1 List<(#var_field<#lifepunct2 #merge_var_head_ident, S, VAR_IDX, IDX>, F)>>:
                    #merge_var_split_trait_generic,
            }
        } else {
            let merge_var_split_trait_generic = quote! {
                #split_trait_ident<#property, Left = #var_field<#lifepunct2 #merge_var_head_ident, S, VAR_IDX, IDX>, Right = F>
            };
            quote! {
                #split_for_2 #wrapper<#liferef1 List<(#var_field<#lifepunct2 #merge_var_head_ident, S, VAR_IDX, IDX>, F)>>:
                    #merge_var_split_trait_generic,
            }
        };
        let merge_var_tail_extra_bounds = if merge_var_extra_generics.is_empty() {
            quote! { #merge_var_fields_bounds }
        } else {
            quote! {}
        };
        let merge_var_tail_two_generic_rhs = if merge_var_extra_generics.is_empty() {
            if merge_var_fields_bounds_empty {
                quote! { Fields }
            } else {
                quote! { #merge_var_fields_bounds + Fields }
            }
        } else {
            quote! { Fields }
        };
        let merge_var_tail_bound = if flow_two_generic {
            quote! {
                #split_for_3 #merge_var_split_right_ty:
                    #merge_var_tail_two_generic_rhs,
            }
        } else {
            quote! {
                #split_for_3 #merge_var_split_right_ty:
                    #inner_trait<::inception::False, #merge_var_head_out_ty, Out #internal_trait_generic_args> #merge_var_tail_extra_bounds + Fields,
            }
        };
        let join_fields_inner_bound = if flow_assoc_borrow_mode {
            match kind {
                Kind::Ref => quote! {
                    for<'a, 'b> #wrapper<&'a <T as Inception<#property>>::#fields_ident<'b>>:
                        #inner_trait<::inception::False, In, Out #internal_trait_generic_args, Ret = #flow_assoc_join_ret_ident>,
                },
                Kind::Mut => quote! {
                    for<'a, 'b> #wrapper<&'a mut <T as Inception<#property>>::#fields_ident<'b>>:
                        #inner_trait<::inception::False, In, Out #internal_trait_generic_args, Ret = #flow_assoc_join_ret_ident>,
                },
                _ => quote! {
                    for<#lifepunct1 #life2> #join_wrapper_fields_ty:
                        #inner_trait<::inception::False, In, Out #internal_trait_generic_args>,
                },
            }
        } else {
            quote! {
                for<#lifepunct1 #life2> #join_wrapper_fields_ty: #inner_trait<::inception::False, In, Out #internal_trait_generic_args> #join_fields_bounds,
            }
        };
        let join_named_output_eq_bound = quote! {};
        let join_named_output_bound = quote! {};
        let merge_inner_head_extra_bounds = if merge_extra_generics.is_empty() {
            quote! { #merge_field_head_bounds }
        } else {
            quote! {}
        };
        let merge_inner_head_bound = if flow_two_generic {
            quote! { ::inception::IsPrimitive<#property> }
        } else {
            if flow_assoc_borrow_mode {
                quote! {
                    #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args> + ::inception::IsPrimitive<#property>
                }
            } else {
                quote! {
                    #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args> #merge_inner_head_extra_bounds + ::inception::IsPrimitive<#property>
                }
            }
        };
        let merge_var_inner_head_extra_bounds = if merge_var_extra_generics.is_empty() {
            quote! { #merge_var_field_head_bounds }
        } else {
            quote! {}
        };
        let merge_var_inner_head_bound = if flow_two_generic {
            quote! { ::inception::IsPrimitive<#property> }
        } else {
            quote! {
                #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args> #merge_var_inner_head_extra_bounds + ::inception::IsPrimitive<#property>
            }
        };
        let empty_impl_generics = if flow_mode {
            quote! { <In, Out #internal_trait_impl_generic_defs> }
        } else {
            quote! { <In #internal_trait_impl_generic_defs> }
        };
        let empty_impl_trait_args = if flow_mode {
            quote! { <::inception::False, In, Out #internal_trait_generic_args> }
        } else {
            quote! { <::inception::False, In #internal_trait_generic_args> }
        };
        let merge_var_args = if merge_var_args.is_empty() {
            quote! {}
        } else {
            quote! { , #merge_var_args }
        };
        let merge_args = if merge_args.is_empty() {
            quote! {}
        } else {
            quote! { , #merge_args }
        };
        let nothing_args = if nothing_args.is_empty() {
            quote! {}
        } else {
            quote! { #nothing_args }
        };
        let (join_args, join_trait_args) = if is_comparator {
            let join_args = if is_type_style {
                quote! { #join_arg_idents: #mutref Self }
            } else {
                quote! { , #join_arg_idents: #mutref Self }
            };
            (join_args, quote! { , #join_arg_idents: F })
        } else if join_extra_args.is_empty() {
            (quote! {}, quote! {})
        } else {
            let join_fn_args = if is_type_style {
                quote! { #join_extra_args }
            } else {
                quote! { , #join_extra_args }
            };
            (join_fn_args, quote! { , #join_extra_args })
        };
        let flow_merge_input_ident = merge_arg_idents
            .iter()
            .last()
            .cloned()
            .unwrap_or_else(|| format_ident!("_in"));
        let flow_merge_passthrough_idents = merge_arg_idents
            .iter()
            .take(merge_arg_idents.len().saturating_sub(1))
            .cloned()
            .collect::<Vec<_>>();
        let flow_merge_passthrough_args = if flow_merge_passthrough_idents.is_empty() {
            quote! {}
        } else {
            quote! { #(#flow_merge_passthrough_idents,)* }
        };
        let flow_join_arg_idents = join_arg_idents.iter().cloned().collect::<Vec<_>>();
        let flow_join_args = if flow_join_arg_idents.is_empty() {
            quote! {}
        } else {
            quote! { #(#flow_join_arg_idents),* }
        };
        let merge_impl_ret_ty = if flow_assoc_borrow_mode {
            quote! { <#merge_fields_ident as #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args>>::Ret }
        } else {
            quote! { #merge_ret }
        };
        let flow_assoc_merge_ret_generic = if flow_assoc_borrow_mode {
            quote! { , #flow_assoc_merge_ret_ident }
        } else {
            quote! {}
        };
        let flow_assoc_join_ret_generic = if flow_assoc_borrow_mode {
            quote! { , #flow_assoc_join_ret_ident }
        } else {
            quote! {}
        };
        let merge_impl_body = if flow_assoc_borrow_mode {
            match kind {
                Kind::Ref => quote! {
                    {
                        let next = <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args>>::#inner_fn(
                            #merge_head_arg.access(),
                            #flow_merge_passthrough_args #flow_merge_input_ident,
                        );
                        <#merge_fields_ident as #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args>>::#inner_fn(
                            &#merge_fields_arg,
                            #flow_merge_passthrough_args next,
                        )
                    }
                },
                Kind::Mut => quote! {
                    {
                        let next = <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_single_input_args>>::#inner_fn(
                            #merge_head_arg.access(),
                            #flow_merge_passthrough_args #flow_merge_input_ident,
                        );
                        <#merge_fields_ident as #inner_trait<::inception::False, #merge_head_out_ty, Out #internal_trait_generic_args>>::#inner_fn(
                            &mut #merge_fields_arg,
                            #flow_merge_passthrough_args next,
                        )
                    }
                },
                _ => quote! { #merge_body },
            }
        } else {
            quote! { #merge_body }
        };
        let join_impl_body = if flow_assoc_borrow_mode {
            match kind {
                Kind::Ref => quote! {
                    {
                        <#join_fields_ident as #inner_trait<::inception::False, In, Out #internal_trait_generic_args>>::#inner_fn(
                            &#join_fields_arg,
                            #flow_join_args,
                        )
                    }
                },
                Kind::Mut => quote! {
                    {
                        <#join_fields_ident as #inner_trait<::inception::False, In, Out #internal_trait_generic_args>>::#inner_fn(
                            &mut #join_fields_arg,
                            #flow_join_args,
                        )
                    }
                },
                _ => quote! { #join_body },
            }
        } else {
            quote! { #join_body }
        };
        let merge_arg_idents = if merge_arg_idents.is_empty() {
            quote! {}
        } else {
            quote! { , #merge_arg_idents }
        };
        let merge_var_arg_idents = if merge_var_arg_idents.is_empty() {
            quote! {}
        } else {
            quote! { , #merge_var_arg_idents }
        };
        let join_arg_idents = if join_arg_idents.is_empty() {
            quote! {}
        } else {
            quote! { , #join_arg_idents }
        };
        let induce_head_placeholder = format_ident!("Head");
        let induce_tail_placeholder = format_ident!("Tail");
        let induce_fields_placeholder = format_ident!("Fields");
        let induce_in_placeholder = format_ident!("In");
        let induce_out_placeholder = format_ident!("Out");
        let merge_tail_ty = if needs_named_ret_lifetime {
            quote! { #wrapper<#ret_liferef F> }
        } else {
            quote! { #wrapper<#liferef1 F> }
        };
        let merge_var_tail_ty = if needs_named_ret_lifetime {
            quote! { #wrapper<#ret_liferef F> }
        } else {
            quote! { #wrapper<#liferef1 F> }
        };
        let join_fields_ty = if needs_named_ret_lifetime {
            join_wrapper_fields_ty_named_ret.clone()
        } else {
            join_wrapper_fields_ty.clone()
        };
        let induced_assoc_helpers = assoc_types
            .iter()
            .filter_map(|t| {
                let induce = t.induce.as_ref()?;
                let assoc_ident = &t.item.ident;
                let helper_ident = format_ident!("__InceptionInduce{}", assoc_ident);
                let assoc_generics = &t.item.generics;
                let assoc_decl_params = assoc_generics.params.iter().cloned().collect::<Vec<_>>();
                let assoc_impl_params = assoc_generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            let mut tp = tp.clone();
                            tp.default = None;
                            GenericParam::Type(tp)
                        }
                        GenericParam::Lifetime(lp) => GenericParam::Lifetime(lp.clone()),
                        GenericParam::Const(cp) => GenericParam::Const(cp.clone()),
                    })
                    .collect::<Vec<_>>();
                let assoc_use_args = assoc_generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            let id = &tp.ident;
                            quote! { #id }
                        }
                        GenericParam::Lifetime(lp) => {
                            let lt = &lp.lifetime;
                            quote! { #lt }
                        }
                        GenericParam::Const(cp) => {
                            let id = &cp.ident;
                            quote! { #id }
                        }
                    })
                    .collect::<Vec<_>>();
                let assoc_where_clause = &assoc_generics.where_clause;
                let assoc_decl_generics = if assoc_decl_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_decl_params),*> }
                };
                let assoc_impl_generics = if assoc_impl_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_impl_params),*> }
                };
                let assoc_proj_generics = if assoc_use_args.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_use_args),*> }
                };
                let helper_primitive_impl_generics = match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                    (Some(_), Some(_)) => quote! { <T, In, Out> },
                    (Some(_), None) => {
                        if flow_input_from_assoc && !type_param_defs_no_default.is_empty() {
                            quote! { <T, #(#type_param_defs_no_default),*, In> }
                        } else {
                            quote! { <T, In> }
                        }
                    }
                    (None, _) => quote! { <T, In> },
                };
                let helper_primitive_trait_args = match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                    (Some(_), Some(_)) => quote! { <True, In, Out #internal_trait_generic_args> },
                    (Some(_), None) => quote! { <True, In #internal_trait_single_input_args> },
                    (None, _) => quote! { <True, In #internal_trait_generic_args> },
                };
                let helper_false_trait_args = if flow_mode {
                    quote! { <::inception::False, In, Out #internal_trait_generic_args> }
                } else {
                    quote! { <::inception::False, In #internal_trait_generic_args> }
                };
                let helper_empty_impl_generics = if flow_mode {
                    quote! { <In, Out #internal_trait_impl_generic_defs> }
                } else {
                    quote! { <In #internal_trait_impl_generic_defs> }
                };
                let helper_head_trait_args = if flow_mode {
                    quote! { <<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In, Out #internal_trait_generic_args> }
                } else {
                    quote! { <<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_generic_args> }
                };
                let helper_merge_var_head_trait_args = if flow_mode {
                    quote! { <<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In, Out #internal_trait_generic_args> }
                } else {
                    quote! { <<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In #internal_trait_generic_args> }
                };
                let helper_merge_impl_generics = if flow_mode {
                    quote! { <#ret_lifetime_decl #merge_head_ident, S, const IDX: usize, F, In, Out #flow_assoc_merge_ret_generic #internal_trait_impl_generic_defs> }
                } else {
                    quote! { <#ret_lifetime_decl #merge_head_ident, S, const IDX: usize, F, In #internal_trait_impl_generic_defs> }
                };
                let helper_merge_var_impl_generics = if flow_mode {
                    quote! { <#ret_lifetime_decl #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, In, Out #internal_trait_impl_generic_defs> }
                } else {
                    quote! { <#ret_lifetime_decl #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, In #internal_trait_impl_generic_defs> }
                };
                let helper_join_impl_generics = if flow_mode {
                    quote! { <T, In, Out #flow_assoc_join_ret_generic #internal_trait_impl_generic_defs> }
                } else {
                    quote! { <T, In #internal_trait_impl_generic_defs> }
                };
                let merge_head_self_ty = quote! { #merge_head_ident };
                let merge_tail_self_ty = quote! { #merge_tail_ty };
                let merge_var_head_self_ty = quote! { #merge_var_head_ident };
                let merge_var_tail_self_ty = quote! { #merge_var_tail_ty };
                let join_fields_self_ty = quote! { #join_fields_ty };
                let out_placeholder_ty = if flow_mode {
                    quote! { Out }
                } else {
                    quote! { In }
                };
                let tail_assoc_ty = quote! {
                    <#merge_tail_ty as #helper_ident #helper_false_trait_args>::Ret #assoc_proj_generics
                };
                let merge_var_tail_assoc_ty = quote! {
                    <#merge_var_tail_ty as #helper_ident #helper_false_trait_args>::Ret #assoc_proj_generics
                };
                let fields_assoc_ty = quote! {
                    <#join_fields_ty as #helper_ident #helper_false_trait_args>::Ret #assoc_proj_generics
                };
                let base = &induce.base.ty;
                let merge = &induce.merge.ty;
                let merge_variant = &induce.merge_variant.ty;
                let join = &induce.join.ty;
                let base_ty_src = base.clone();
                let mut base_ty = quote! { #base_ty_src };
                base_ty = substitute_ret_ident(&base_ty, &induce_in_placeholder, &quote! { In });
                base_ty = substitute_ret_ident(&base_ty, &induce_out_placeholder, &out_placeholder_ty);
                let mut merge_ty_src = merge.clone();
                rewrite_assoc_projection_to_helper(
                    &mut merge_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_head_placeholder,
                    &helper_ident,
                    &merge_head_self_ty,
                    &helper_head_trait_args,
                );
                rewrite_assoc_projection_to_helper(
                    &mut merge_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_tail_placeholder,
                    &helper_ident,
                    &merge_tail_self_ty,
                    &helper_false_trait_args,
                );
                let mut merge_ty = quote! { #merge_ty_src };
                merge_ty = substitute_ret_ident(&merge_ty, &induce_head_placeholder, &quote! { #merge_head_ident });
                merge_ty = substitute_ret_ident(&merge_ty, &induce_tail_placeholder, &tail_assoc_ty);
                merge_ty = substitute_ret_ident(&merge_ty, &induce_in_placeholder, &quote! { In });
                merge_ty = substitute_ret_ident(&merge_ty, &induce_out_placeholder, &out_placeholder_ty);
                let mut merge_var_ty_src = merge_variant.clone();
                rewrite_assoc_projection_to_helper(
                    &mut merge_var_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_head_placeholder,
                    &helper_ident,
                    &merge_var_head_self_ty,
                    &helper_merge_var_head_trait_args,
                );
                rewrite_assoc_projection_to_helper(
                    &mut merge_var_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_tail_placeholder,
                    &helper_ident,
                    &merge_var_tail_self_ty,
                    &helper_false_trait_args,
                );
                let mut merge_var_ty = quote! { #merge_var_ty_src };
                merge_var_ty = substitute_ret_ident(&merge_var_ty, &induce_head_placeholder, &quote! { #merge_var_head_ident });
                merge_var_ty = substitute_ret_ident(&merge_var_ty, &induce_tail_placeholder, &merge_var_tail_assoc_ty);
                merge_var_ty = substitute_ret_ident(&merge_var_ty, &induce_in_placeholder, &quote! { In });
                merge_var_ty = substitute_ret_ident(&merge_var_ty, &induce_out_placeholder, &out_placeholder_ty);
                let mut join_ty_src = join.clone();
                rewrite_assoc_projection_to_helper(
                    &mut join_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_fields_placeholder,
                    &helper_ident,
                    &join_fields_self_ty,
                    &helper_false_trait_args,
                );
                let mut join_ty = quote! { #join_ty_src };
                join_ty = substitute_ret_ident(&join_ty, &induce_fields_placeholder, &fields_assoc_ty);
                join_ty = substitute_ret_ident(&join_ty, &induce_in_placeholder, &quote! { In });
                join_ty = substitute_ret_ident(&join_ty, &induce_out_placeholder, &out_placeholder_ty);
                Some(quote! {
                    pub trait #helper_ident<P: TruthValue = <Self as IsPrimitive<super::#property_ident>>::Is, In = (), Out = In #internal_trait_decl_generic_defs> {
                        type Ret #assoc_decl_generics #assoc_where_clause;
                    }

                    impl #helper_primitive_impl_generics #helper_ident #helper_primitive_trait_args for T
                    where
                        T: #trait_bound_with_in + IsPrimitive<super::#property_ident, Is = True>,
                    {
                        type Ret #assoc_impl_generics = <T as super::#trait_ident #trait_generic_args>::#assoc_ident #assoc_proj_generics #assoc_where_clause;
                    }

                    impl #helper_empty_impl_generics #helper_ident #helper_false_trait_args for #wrapper<#liferefelide List<()>> {
                        type Ret #assoc_impl_generics = #base_ty #assoc_where_clause;
                    }

                    impl #helper_merge_impl_generics
                        #helper_ident #helper_false_trait_args for #merge_inductive_self_ty
                    where
                        S: FieldsMeta,
                        #merge_head_ident: ::inception::IsPrimitive<#property>,
                        #merge_head_ident: #helper_ident #helper_head_trait_args,
                        #merge_tail_ty: #helper_ident #helper_false_trait_args,
                        F: Fields #phantom_bound,
                        <F as Fields>::Owned: Fields,
                        #merge_split_bound
                    {
                        type Ret #assoc_impl_generics = #merge_ty #assoc_where_clause;
                    }

                    impl #helper_merge_var_impl_generics
                        #helper_ident #helper_false_trait_args for #merge_var_inductive_self_ty
                    where
                        S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
                        #merge_var_head_ident: ::inception::IsPrimitive<#property>,
                        #merge_var_head_ident: #helper_ident #helper_merge_var_head_trait_args,
                        #merge_var_tail_ty: #helper_ident #helper_false_trait_args,
                        F: Fields #phantom_bound,
                        <F as Fields>::Owned: Fields,
                        #merge_var_split_bound
                    {
                        type Ret #assoc_impl_generics = #merge_var_ty #assoc_where_clause;
                    }

                    impl #helper_join_impl_generics
                        #helper_ident #helper_false_trait_args for T
                    where
                        T: Inception<#property> + Meta,
                        #join_fields_ty: #helper_ident #helper_false_trait_args,
                    {
                        type Ret #assoc_impl_generics = #join_ty #assoc_where_clause;
                    }
                })
            })
            .collect::<Vec<_>>();

        let expanded = quote! {
            pub struct #property_ident;
            #vis trait #trait_ident #trait_generic_params #trait_supertrait_clause #trait_where_clause {
                #(#assoc_trait_items)*
                fn #fn_ident(#mutref #receiver #fn_args) -> #fn_ret_public;
            }

            mod #mod_ident {
                use inception::{Wrapper, TruthValue, IsPrimitive, meta::Metadata, True, False};
                use super::*;

                impl ::inception::Property for super::#property_ident {}
                #compat_impl

                pub struct Wrap<T>(pub T);
                impl<T> Wrapper for Wrap<T> {
                    type Content = T;
                    fn wrap(t: Self::Content) -> Self {
                        Self(t)
                    }
                }

                impl<T> IsPrimitive<super::#property_ident> for Wrap<T> {
                    type Is = False;
                }
                pub trait #inductive_ident<P: TruthValue = <Self as IsPrimitive<super::#property_ident>>::Is, In = (), Out = In #internal_trait_decl_generic_defs> {
                    type Property: ::inception::Property;
                    type InTy;
                    type OutTy;
                    type Ret;
                    fn #inner_fn(#mutref #receiver #inner_fn_args) -> Self::Ret;
                }
                impl #primitive_impl_generics #inductive_ident #primitive_impl_trait_args for T
                where
                    T: #trait_bound_with_in + IsPrimitive<super::#property_ident, Is = True>,
                {
                    type Property = super::#property_ident;
                    type InTy = In;
                    type OutTy = #primitive_out_ty;
                    type Ret = #primitive_ret;
                    fn #inner_fn(#mutref #receiver #inner_fn_args) -> Self::Ret {
                        #primitive_dispatch_body
                    }
                }

                pub trait Nothing<In = () #internal_trait_decl_generic_defs> {
                    type InTy;
                    type OutTy;
                    type Ret;
                    fn nothing(#nothing_args) -> Self::Ret;
                }
                pub trait MergeField<L, R, In = (), Out = In, Extra = () #internal_trait_decl_generic_defs> {
                    type InTy;
                    type OutTy;
                    type Ret;
                    fn merge_field(l: L, r: R #merge_args) -> Self::Ret;
                }
                pub trait MergeVariantField<L, R, In = (), Out = In, Extra = () #internal_trait_decl_generic_defs> {
                    type InTy;
                    type OutTy;
                    type Ret;
                    fn merge_variant_field(l: L, r: R #merge_var_args) -> Self::Ret;
                }
                pub trait Join<F, In = (), Out = In #internal_trait_decl_generic_defs> {
                    type InTy;
                    type OutTy;
                    type Ret;
                    fn join(fields: F #join_trait_args) -> Self::Ret;
                }
                #borrow_output_helpers
                #flow_input_helpers
                #(#induced_assoc_helpers)*
            }

            #blanket_impl_head
            where
                #blanket_assoc_where_preds
                T: #blanket_inner_bound + ::inception::IsPrimitive<#property, Is = ::inception::False> #trait_supertrait_bounds,
            {
                #(#assoc_impl_items)*
                fn #fn_ident(#mutref #receiver #fn_args) -> #fn_ret_public {
                    #blanket_dispatch_body
                }
            }

            impl<T, In #internal_trait_impl_generic_defs> #mod_ident :: Nothing<In #internal_trait_generic_args> for T {
                type InTy = In;
                type OutTy = In;
                type Ret = #nothing_ret;
                fn nothing(#nothing_args) -> Self::Ret {
                    #nothing_body
                }
            }
            impl #merge_field_impl_generics #mod_ident :: MergeField #merge_field_impl_trait_args
                for #wrapper<#liferefelide List<(#field<#lifepunct1 #merge_head_ident, S, IDX>, F)>>
            where
                S: FieldsMeta,
                #merge_head_ident: #merge_field_head_bound,
                F: Fields #phantom_bound,
                L: Field #access_bound,
                #merge_fields_ident: #merge_field_tail_bound,
            {
                type InTy = In;
                type OutTy = #merge_field_impl_out_ty;
                type Ret = #merge_impl_ret_ty;
                fn merge_field(#mutability #merge_head_arg: L, #mutability #merge_fields_arg: #merge_fields_ident #merge_args) -> Self::Ret {
                    #merge_impl_body
                }
            }
            impl #merge_variant_impl_generics #mod_ident :: MergeVariantField #merge_variant_impl_trait_args
                for #wrapper<#liferefelide List<(#var_field<#lifepunct1 #merge_var_head_ident, S, VAR_IDX, IDX>, F)>>
            where
                S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
                #merge_var_head_ident: #merge_variant_head_bound,
                F: Fields #phantom_bound,
                L: Field<Source = S> + VarField #try_access_bound,
                #merge_var_fields_ident: #merge_variant_tail_bound,
            {
                type InTy = In;
                type OutTy = #merge_variant_impl_out_ty;
                type Ret = #merge_var_ret;
                fn merge_variant_field(#mutability #merge_var_head_arg: L, #mutability #merge_var_fields_arg: #merge_var_fields_ident #merge_var_args) -> Self::Ret {
                    #merge_var_body
                }
            }
            impl #join_impl_generics #mod_ident :: Join #join_impl_trait_args for T
            where
                T: Inception<#property>,
                #join_fields_ident: #join_fields_bound #join_named_output_bound,
            {
                type InTy = In;
                type OutTy = #join_impl_out_ty;
                type Ret = #join_impl_ret_ty;
                fn join(#mutability #join_fields_arg: #join_fields_ident #join_trait_args) -> Self::Ret {
                    #join_impl_body
                }
            }

            impl<T> Fields for #wrapper<#mutref T>
            where
                T: Fields,
            {
                type Head = <T as Fields>::Head;
                type Tail = <T as Fields>::Tail;
                type Referenced<'a>
                    = <T as Fields>::Referenced<'a>
                where
                    Self::Head: 'a,
                    Self::Tail: 'a;
                type MutablyReferenced<'a>
                    = <T as Fields>::MutablyReferenced<'a>
                where
                    Self::Head: 'a,
                    Self::Tail: 'a;
                type Owned = <T as Fields>::Owned;
            }

            #split_impl

            impl #empty_impl_generics #inner_trait #empty_impl_trait_args for #wrapper<#liferefelide List<()>> {
                type Property = #property;
                type InTy = <Self as #mod_ident :: Nothing<In #internal_trait_generic_args>>::InTy;
                type OutTy = <Self as #mod_ident :: Nothing<In #internal_trait_generic_args>>::OutTy;
                type Ret = #nothing_ret;
                #[allow(unused)]
                fn #inner_fn(#mutref #receiver #inner_fn_args) -> Self::Ret {
                    <Self as #mod_ident :: Nothing<In #internal_trait_generic_args>>::nothing(#nothing_arg_idents)
                }
            }

            impl<#ret_lifetime_decl #merge_head_ident, S, const IDX: usize, F, In, Out #flow_assoc_merge_ret_generic #internal_trait_impl_generic_defs> #inner_trait<::inception::False, In, Out #internal_trait_generic_args> for #merge_inductive_self_ty
            where
                S: FieldsMeta,
                #merge_head_ident: #merge_inner_head_bound,
                F: Fields #phantom_bound,
                <F as Fields>::Owned: Fields,
                #merge_split_bound
                #merge_tail_bound
                #merge_named_output_eq_bound
            {
                type Property = #property;
                type InTy = In;
                type OutTy = Out;
                type Ret = #merge_ret_inductive;
                fn #inner_fn(#mutref #receiver #inner_fn_args) -> Self::Ret {
                    use #split_trait_ident;
                    let (#mutability l, #mutability r) = #dispatcher #split_fn_ident(#split_fn_receiver);
                    let #mutability r = #wrapper(r);
                    #merge_comparator_body
                    <Self as #mod_ident :: MergeField #merge_call_trait_args>::merge_field(l, r #merge_arg_idents)
                }
            }

            impl<#ret_lifetime_decl #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, In, Out #internal_trait_impl_generic_defs> #inner_trait<::inception::False, In, Out #internal_trait_generic_args>
                for #merge_var_inductive_self_ty
            where
                S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
                #merge_var_head_ident: #merge_var_inner_head_bound,
                F: Fields #phantom_bound,
                <F as Fields>::Owned: Fields,
                #merge_var_split_bound
                #merge_var_tail_bound
            {
                type Property = #property;
                type InTy = In;
                type OutTy = Out;
                type Ret = #merge_var_ret_inductive;
                fn #inner_fn(#mutref #receiver #inner_fn_args) -> Self::Ret {
                    use #split_trait_ident;
                    let (#mutability l, #mutability r) = #dispatcher #split_fn_ident(#split_fn_receiver);
                    let #mutability r = #wrapper(r);
                    #merge_var_comparator_body
                    <Self as #mod_ident :: MergeVariantField #merge_variant_call_trait_args>::merge_variant_field(l, r #merge_var_arg_idents)
                }
            }

            impl<T, In, Out #flow_assoc_join_ret_generic #internal_trait_impl_generic_defs> #inner_trait<False, In, Out #internal_trait_generic_args> for T
            where
                T: Inception<#property> + Meta,
                #join_fields_inner_bound
                #join_named_output_eq_bound
            {
                type Property = #property;
                type InTy = In;
                type OutTy = Out;
                type Ret = #join_ret_inductive;
                fn #inner_fn(#mutref #receiver #join_args) -> Self::Ret {
                    #fields_fn
                    let f = #wrapper(#mutref fields);
                    #join_comparator_body
                    <Self as #mod_ident :: Join #join_call_trait_args>::join(f #join_arg_idents)
                }
            }
        };
        expanded.into()
    }

    fn finish_types_only(self, is_comparator: bool) -> TokenStream {
        let State {
            mod_ident,
            trait_ident,
            trait_generics,
            trait_supertraits,
            property_ident,
            signature,
            vis,
            assoc_types,
            ..
        } = self;

        if is_comparator {
            let msg = "`types` mode is not compatible with `comparator`.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        if signature.is_some() {
            let msg = "`types` mode does not support `signature(...)`.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        let has_non_type = trait_generics
            .params
            .iter()
            .any(|p| !matches!(p, GenericParam::Type(_)));
        if has_non_type {
            let msg = "Only type generics are currently supported for #[inception] traits.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        if assoc_types.is_empty() {
            let msg = "`types` mode requires at least one associated type.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        if assoc_types.iter().any(|a| a.induce.is_none()) {
            let msg = "`types` mode requires every associated type to use `#[induce(...)]`.";
            return syn::Error::new_spanned(trait_ident.clone(), msg)
                .into_compile_error()
                .into();
        }
        for assoc in assoc_types.iter().filter_map(|a| a.induce.as_ref()) {
            for (phase, value) in [
                ("base", &assoc.base),
                ("merge", &assoc.merge),
                ("merge_variant", &assoc.merge_variant),
                ("join", &assoc.join),
            ] {
                let mut bad: Option<syn::Error> = None;
                walk_type_paths(&value.ty, &mut |tp: &TypePath| {
                    if bad.is_some() {
                        return;
                    }
                    let Some(id) = tp.path.get_ident() else {
                        return;
                    };
                    if id == "In" || id == "Out" {
                        let msg = format!(
                            "Placeholder `{}` is not supported in `types` mode (`induce.{phase}`).",
                            id
                        );
                        bad = Some(syn::Error::new_spanned(tp, msg));
                    }
                });
                if let Some(err) = bad {
                    return err.into_compile_error().into();
                }
                for pred in value.where_preds.iter() {
                    match pred {
                        WherePredicate::Type(tp) => {
                            walk_type_paths(&tp.bounded_ty, &mut |p: &TypePath| {
                                if bad.is_some() {
                                    return;
                                }
                                let Some(id) = p.path.get_ident() else {
                                    return;
                                };
                                if id == "In" || id == "Out" {
                                    let msg = format!(
                                        "Placeholder `{}` is not supported in `types` mode (`induce.{phase}` where-bounds).",
                                        id
                                    );
                                    bad = Some(syn::Error::new_spanned(p, msg));
                                }
                            });
                            if bad.is_some() {
                                break;
                            }
                            for b in tp.bounds.iter() {
                                if let TypeParamBound::Trait(tb) = b {
                                    walk_trait_bound_type_paths(tb, &mut |p: &TypePath| {
                                        if bad.is_some() {
                                            return;
                                        }
                                        let Some(id) = p.path.get_ident() else {
                                            return;
                                        };
                                        if id == "In" || id == "Out" {
                                            let msg = format!(
                                                "Placeholder `{}` is not supported in `types` mode (`induce.{phase}` where-bounds).",
                                                id
                                            );
                                            bad = Some(syn::Error::new_spanned(p, msg));
                                        }
                                    });
                                }
                                if bad.is_some() {
                                    break;
                                }
                            }
                        }
                        WherePredicate::Lifetime(_) => {}
                        _ => {}
                    }
                }
                if let Some(err) = bad {
                    return err.into_compile_error().into();
                }
            }
        }

        fn replace_type_ident(ty: &mut Type, target: &Ident, replacement: &Type) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if qself.is_none() && path.is_ident(target) {
                        *ty = replacement.clone();
                        return;
                    }
                    if let Some(q) = qself {
                        replace_type_ident(&mut q.ty, target, replacement);
                    }
                    for seg in path.segments.iter_mut() {
                        match &mut seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter_mut() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            replace_type_ident(inner, target, replacement)
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            replace_type_ident(&mut assoc.ty, target, replacement)
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter_mut() {
                                    replace_type_ident(input, target, replacement);
                                }
                                if let ReturnType::Type(_, out) = &mut args.output {
                                    replace_type_ident(out.as_mut(), target, replacement);
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                }
                Type::Reference(r) => replace_type_ident(r.elem.as_mut(), target, replacement),
                Type::Ptr(p) => replace_type_ident(p.elem.as_mut(), target, replacement),
                Type::Slice(s) => replace_type_ident(s.elem.as_mut(), target, replacement),
                Type::Array(a) => replace_type_ident(a.elem.as_mut(), target, replacement),
                Type::Tuple(t) => {
                    for elem in t.elems.iter_mut() {
                        replace_type_ident(elem, target, replacement);
                    }
                }
                Type::Paren(p) => replace_type_ident(p.elem.as_mut(), target, replacement),
                Type::Group(g) => replace_type_ident(g.elem.as_mut(), target, replacement),
                _ => {}
            }
        }
        fn replace_type_ident_in_path_args(
            args: &mut syn::PathArguments,
            target: &Ident,
            replacement: &Type,
        ) {
            match args {
                syn::PathArguments::AngleBracketed(ab) => {
                    for arg in ab.args.iter_mut() {
                        match arg {
                            syn::GenericArgument::Type(inner) => {
                                replace_type_ident(inner, target, replacement)
                            }
                            syn::GenericArgument::AssocType(assoc) => {
                                replace_type_ident(&mut assoc.ty, target, replacement)
                            }
                            syn::GenericArgument::Constraint(constraint) => {
                                for b in constraint.bounds.iter_mut() {
                                    if let TypeParamBound::Trait(tb) = b {
                                        for seg in tb.path.segments.iter_mut() {
                                            replace_type_ident_in_path_args(
                                                &mut seg.arguments,
                                                target,
                                                replacement,
                                            );
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                syn::PathArguments::Parenthesized(pb) => {
                    for input in pb.inputs.iter_mut() {
                        replace_type_ident(input, target, replacement);
                    }
                    if let ReturnType::Type(_, out) = &mut pb.output {
                        replace_type_ident(out.as_mut(), target, replacement);
                    }
                }
                syn::PathArguments::None => {}
            }
        }
        fn substitute_where_preds(
            preds: &[WherePredicate],
            replacements: &[(&Ident, &Type)],
        ) -> Vec<WherePredicate> {
            let mut out = preds.to_vec();
            for pred in out.iter_mut() {
                if let WherePredicate::Type(tp) = pred {
                    for (target, replacement) in replacements.iter() {
                        replace_type_ident(&mut tp.bounded_ty, target, replacement);
                        for b in tp.bounds.iter_mut() {
                            if let TypeParamBound::Trait(tb) = b {
                                for seg in tb.path.segments.iter_mut() {
                                    replace_type_ident_in_path_args(
                                        &mut seg.arguments,
                                        target,
                                        replacement,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            out
        }
        fn type_is_plain_ident(ty: &Type, ident: &Ident) -> bool {
            matches!(
                ty,
                Type::Path(TypePath { qself: None, path }) if path.is_ident(ident)
            )
        }
        fn remove_self_trait_bounds(
            bounds: &mut Punctuated<TypeParamBound, syn::token::Plus>,
            trait_ident: &Ident,
        ) {
            let mut kept = Punctuated::new();
            for bound in bounds.clone().into_iter() {
                let is_self_trait = matches!(
                    &bound,
                    TypeParamBound::Trait(TraitBound { path, .. })
                        if path
                            .segments
                            .last()
                            .map(|s| s.ident == *trait_ident)
                            .unwrap_or(false)
                );
                if !is_self_trait {
                    kept.push(bound);
                }
            }
            *bounds = kept;
        }
        fn type_contains_helper_ret_projection(ty: &Type, helper_ident: &Ident) -> bool {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(q) = qself {
                        if type_contains_helper_ret_projection(q.ty.as_ref(), helper_ident) {
                            return true;
                        }
                    }
                    if path.segments.len() >= 2 {
                        let assoc_seg = path.segments.last().map(|s| s.ident.clone());
                        let helper_seg = path.segments.iter().rev().nth(1).map(|s| s.ident.clone());
                        if let (Some(assoc_seg), Some(helper_seg)) = (assoc_seg, helper_seg) {
                            if helper_seg == *helper_ident && assoc_seg == "Ret" {
                                return true;
                            }
                        }
                    }
                    for seg in path.segments.iter() {
                        match &seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            if type_contains_helper_ret_projection(
                                                inner,
                                                helper_ident,
                                            ) {
                                                return true;
                                            }
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            if type_contains_helper_ret_projection(
                                                &assoc.ty,
                                                helper_ident,
                                            ) {
                                                return true;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter() {
                                    if type_contains_helper_ret_projection(input, helper_ident) {
                                        return true;
                                    }
                                }
                                if let ReturnType::Type(_, out) = &args.output {
                                    if type_contains_helper_ret_projection(
                                        out.as_ref(),
                                        helper_ident,
                                    ) {
                                        return true;
                                    }
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                    false
                }
                Type::Reference(r) => {
                    type_contains_helper_ret_projection(r.elem.as_ref(), helper_ident)
                }
                Type::Ptr(p) => type_contains_helper_ret_projection(p.elem.as_ref(), helper_ident),
                Type::Slice(s) => {
                    type_contains_helper_ret_projection(s.elem.as_ref(), helper_ident)
                }
                Type::Array(a) => {
                    type_contains_helper_ret_projection(a.elem.as_ref(), helper_ident)
                }
                Type::Tuple(t) => t
                    .elems
                    .iter()
                    .any(|elem| type_contains_helper_ret_projection(elem, helper_ident)),
                Type::Paren(p) => {
                    type_contains_helper_ret_projection(p.elem.as_ref(), helper_ident)
                }
                Type::Group(g) => {
                    type_contains_helper_ret_projection(g.elem.as_ref(), helper_ident)
                }
                _ => false,
            }
        }
        fn replace_helper_ret_projection(ty: &mut Type, helper_ident: &Ident, replacement: &Type) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(q) = qself {
                        replace_helper_ret_projection(q.ty.as_mut(), helper_ident, replacement);
                    }
                    for seg in path.segments.iter_mut() {
                        match &mut seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter_mut() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            replace_helper_ret_projection(
                                                inner,
                                                helper_ident,
                                                replacement,
                                            )
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            replace_helper_ret_projection(
                                                &mut assoc.ty,
                                                helper_ident,
                                                replacement,
                                            )
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter_mut() {
                                    replace_helper_ret_projection(input, helper_ident, replacement);
                                }
                                if let ReturnType::Type(_, out) = &mut args.output {
                                    replace_helper_ret_projection(
                                        out.as_mut(),
                                        helper_ident,
                                        replacement,
                                    );
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                    if qself.is_some() && path.segments.len() >= 2 {
                        let assoc_seg = path.segments.last().map(|s| s.ident.clone());
                        let helper_seg = path.segments.iter().rev().nth(1).map(|s| s.ident.clone());
                        if let (Some(assoc_seg), Some(helper_seg)) = (assoc_seg, helper_seg) {
                            if helper_seg == *helper_ident && assoc_seg == "Ret" {
                                *ty = replacement.clone();
                            }
                        }
                    }
                }
                Type::Reference(r) => {
                    replace_helper_ret_projection(r.elem.as_mut(), helper_ident, replacement)
                }
                Type::Ptr(p) => {
                    replace_helper_ret_projection(p.elem.as_mut(), helper_ident, replacement)
                }
                Type::Slice(s) => {
                    replace_helper_ret_projection(s.elem.as_mut(), helper_ident, replacement)
                }
                Type::Array(a) => {
                    replace_helper_ret_projection(a.elem.as_mut(), helper_ident, replacement)
                }
                Type::Tuple(t) => {
                    for elem in t.elems.iter_mut() {
                        replace_helper_ret_projection(elem, helper_ident, replacement);
                    }
                }
                Type::Paren(p) => {
                    replace_helper_ret_projection(p.elem.as_mut(), helper_ident, replacement)
                }
                Type::Group(g) => {
                    replace_helper_ret_projection(g.elem.as_mut(), helper_ident, replacement)
                }
                _ => {}
            }
        }
        fn lower_helper_projection_pred(
            pred: &WherePredicate,
            helper_ident: &Ident,
        ) -> WherePredicate {
            let WherePredicate::Type(tp) = pred else {
                return pred.clone();
            };
            let Type::Path(TypePath { qself, path }) = &tp.bounded_ty else {
                return pred.clone();
            };
            let Some(qself) = qself else {
                return pred.clone();
            };
            if path.segments.len() < 2 {
                return pred.clone();
            }
            let Some(assoc_seg) = path.segments.last() else {
                return pred.clone();
            };
            let Some(helper_seg) = path.segments.iter().rev().nth(1) else {
                return pred.clone();
            };
            if helper_seg.ident != *helper_ident || assoc_seg.ident != "Ret" {
                return pred.clone();
            }
            let constraint_generics = match &assoc_seg.arguments {
                syn::PathArguments::AngleBracketed(ab) => Some(ab.clone()),
                syn::PathArguments::None => None,
                syn::PathArguments::Parenthesized(_) => {
                    return pred.clone();
                }
            };
            let constraint = syn::Constraint {
                ident: assoc_seg.ident.clone(),
                generics: constraint_generics,
                colon_token: Default::default(),
                bounds: tp.bounds.clone(),
            };
            let mut helper_path = path.clone();
            helper_path.segments.pop();
            helper_path.segments.pop_punct();
            let Some(last_seg) = helper_path.segments.last_mut() else {
                return pred.clone();
            };
            match &mut last_seg.arguments {
                syn::PathArguments::AngleBracketed(ab) => {
                    ab.args.push(syn::GenericArgument::Constraint(constraint));
                }
                syn::PathArguments::None => {
                    let mut args = Punctuated::new();
                    args.push(syn::GenericArgument::Constraint(constraint));
                    last_seg.arguments =
                        syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: Default::default(),
                            args,
                            gt_token: Default::default(),
                        });
                }
                syn::PathArguments::Parenthesized(_) => {
                    return pred.clone();
                }
            }
            let self_ty = qself.ty.as_ref().clone();
            let mut bounds = Punctuated::new();
            bounds.push(TypeParamBound::Trait(TraitBound {
                paren_token: None,
                modifier: syn::TraitBoundModifier::None,
                lifetimes: None,
                path: helper_path,
            }));
            WherePredicate::Type(syn::PredicateType {
                lifetimes: tp.lifetimes.clone(),
                bounded_ty: self_ty,
                colon_token: tp.colon_token,
                bounds,
            })
        }
        fn lower_helper_projection_preds(
            preds: Vec<WherePredicate>,
            helper_ident: &Ident,
        ) -> Vec<WherePredicate> {
            preds
                .iter()
                .map(|pred| lower_helper_projection_pred(pred, helper_ident))
                .collect::<Vec<_>>()
        }
        let substitute_type_ident =
            |ty: &Type, target: &Ident, replacement: &proc_macro2::TokenStream| {
                let mut out = ty.clone();
                let Ok(repl_ty) = syn::parse2::<Type>(replacement.clone()) else {
                    return quote! { #out };
                };
                replace_type_ident(&mut out, target, &repl_ty);
                quote! { #out }
            };
        fn rewrite_assoc_projection_to_helper(
            ty: &mut Type,
            trait_ident: &Ident,
            assoc_ident: &Ident,
            source_placeholder: &Ident,
            helper_ident: &Ident,
            helper_self_ty: &proc_macro2::TokenStream,
            helper_trait_args: &proc_macro2::TokenStream,
        ) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(q) = qself {
                        rewrite_assoc_projection_to_helper(
                            &mut q.ty,
                            trait_ident,
                            assoc_ident,
                            source_placeholder,
                            helper_ident,
                            helper_self_ty,
                            helper_trait_args,
                        );
                    }
                    for seg in path.segments.iter_mut() {
                        match &mut seg.arguments {
                            syn::PathArguments::AngleBracketed(args) => {
                                for arg in args.args.iter_mut() {
                                    match arg {
                                        syn::GenericArgument::Type(inner) => {
                                            rewrite_assoc_projection_to_helper(
                                                inner,
                                                trait_ident,
                                                assoc_ident,
                                                source_placeholder,
                                                helper_ident,
                                                helper_self_ty,
                                                helper_trait_args,
                                            )
                                        }
                                        syn::GenericArgument::AssocType(assoc) => {
                                            rewrite_assoc_projection_to_helper(
                                                &mut assoc.ty,
                                                trait_ident,
                                                assoc_ident,
                                                source_placeholder,
                                                helper_ident,
                                                helper_self_ty,
                                                helper_trait_args,
                                            )
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            syn::PathArguments::Parenthesized(args) => {
                                for input in args.inputs.iter_mut() {
                                    rewrite_assoc_projection_to_helper(
                                        input,
                                        trait_ident,
                                        assoc_ident,
                                        source_placeholder,
                                        helper_ident,
                                        helper_self_ty,
                                        helper_trait_args,
                                    );
                                }
                                if let ReturnType::Type(_, out) = &mut args.output {
                                    rewrite_assoc_projection_to_helper(
                                        out.as_mut(),
                                        trait_ident,
                                        assoc_ident,
                                        source_placeholder,
                                        helper_ident,
                                        helper_self_ty,
                                        helper_trait_args,
                                    );
                                }
                            }
                            syn::PathArguments::None => {}
                        }
                    }
                    let Some(q) = qself.as_ref() else {
                        return;
                    };
                    if path.segments.len() < 2 {
                        return;
                    }
                    let assoc_seg = path.segments.last().map(|s| s.ident.clone());
                    let tr_seg = path.segments.iter().rev().nth(1).map(|s| s.ident.clone());
                    let (Some(assoc_seg), Some(tr_seg)) = (assoc_seg, tr_seg) else {
                        return;
                    };
                    if tr_seg != *trait_ident || assoc_seg != *assoc_ident {
                        return;
                    }
                    let Type::Path(TypePath {
                        qself: None,
                        path: q_path,
                    }) = q.ty.as_ref()
                    else {
                        return;
                    };
                    let Some(q_ident) = q_path.get_ident() else {
                        return;
                    };
                    if *q_ident != *source_placeholder {
                        return;
                    }
                    let Some(last_seg) = path.segments.last() else {
                        return;
                    };
                    let ret_ty = match &last_seg.arguments {
                        syn::PathArguments::AngleBracketed(ab) if !ab.args.is_empty() => {
                            let args = &ab.args;
                            quote! {
                                <#helper_self_ty as #helper_ident #helper_trait_args>::Ret<#args>
                            }
                        }
                        syn::PathArguments::AngleBracketed(_) | syn::PathArguments::None => {
                            quote! {
                                <#helper_self_ty as #helper_ident #helper_trait_args>::Ret
                            }
                        }
                        syn::PathArguments::Parenthesized(_) => {
                            return;
                        }
                    };
                    *ty = syn::parse_quote! { #ret_ty };
                }
                Type::Reference(r) => rewrite_assoc_projection_to_helper(
                    r.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Ptr(p) => rewrite_assoc_projection_to_helper(
                    p.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Slice(s) => rewrite_assoc_projection_to_helper(
                    s.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Array(a) => rewrite_assoc_projection_to_helper(
                    a.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Tuple(t) => {
                    for elem in t.elems.iter_mut() {
                        rewrite_assoc_projection_to_helper(
                            elem,
                            trait_ident,
                            assoc_ident,
                            source_placeholder,
                            helper_ident,
                            helper_self_ty,
                            helper_trait_args,
                        );
                    }
                }
                Type::Paren(p) => rewrite_assoc_projection_to_helper(
                    p.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                Type::Group(g) => rewrite_assoc_projection_to_helper(
                    g.elem.as_mut(),
                    trait_ident,
                    assoc_ident,
                    source_placeholder,
                    helper_ident,
                    helper_self_ty,
                    helper_trait_args,
                ),
                _ => {}
            }
        }

        let trait_where_clause = &trait_generics.where_clause;
        let trait_generic_params = {
            let params = &trait_generics.params;
            if params.is_empty() {
                quote! {}
            } else {
                quote! { <#params> }
            }
        };
        let type_params = trait_generics
            .params
            .iter()
            .filter_map(|p| match p {
                GenericParam::Type(t) => Some(t.ident.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let type_param_defs_no_default = trait_generics
            .params
            .iter()
            .filter_map(|p| match p {
                GenericParam::Type(t) => {
                    let mut t = t.clone();
                    t.default = None;
                    Some(t)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let trait_generic_args = if type_params.is_empty() {
            quote! {}
        } else {
            quote! { <#(#type_params),*> }
        };
        let trait_impl_generic_params = if type_param_defs_no_default.is_empty() {
            quote! { <T> }
        } else {
            quote! { <T, #(#type_param_defs_no_default),*> }
        };
        let blanket_impl_head =
            quote! { impl #trait_impl_generic_params #trait_ident #trait_generic_args for T };
        let compat_impl = if type_params.is_empty() {
            quote! {
                impl<T> ::inception::Compat<T> for super::#property_ident
                where
                    T: super::#trait_ident,
                {
                    type Out = True;
                }
            }
        } else {
            quote! {}
        };
        let trait_supertrait_clause = if trait_supertraits.is_empty() {
            quote! {}
        } else {
            quote! { : #trait_supertraits }
        };
        let trait_supertrait_bounds = if trait_supertraits.is_empty() {
            quote! {}
        } else {
            quote! { + #trait_supertraits }
        };

        let kind = Kind::Ty;
        let field = kind.field();
        let var_field = kind.var_field();
        let fields_ident = kind.fields();
        let split_trait_ident = kind.split_trait_ident();
        let phantom_bound = kind.phantom_bound();
        let wrapper = quote! { #mod_ident :: Wrap };
        let property = quote! { #property_ident };
        let split_impl = kind.split_impl(&property, &wrapper);
        let liferefelide = kind.liferefelide();

        let assoc_trait_items = assoc_types
            .iter()
            .map(|t| {
                let item = &t.item;
                quote! { #item }
            })
            .collect::<Vec<_>>();
        let assoc_impl_items = assoc_types
            .iter()
            .map(|t| {
                let ident = &t.item.ident;
                let assoc_generics = &t.item.generics;
                let assoc_params = assoc_generics.params.iter().cloned().collect::<Vec<_>>();
                let assoc_use_args = assoc_generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            let id = &tp.ident;
                            quote! { #id }
                        }
                        GenericParam::Lifetime(lp) => {
                            let lt = &lp.lifetime;
                            quote! { #lt }
                        }
                        GenericParam::Const(cp) => {
                            let id = &cp.ident;
                            quote! { #id }
                        }
                    })
                    .collect::<Vec<_>>();
                let assoc_impl_generics = if assoc_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_params),*> }
                };
                let assoc_where_clause = &assoc_generics.where_clause;
                let helper_ret_use = if assoc_use_args.is_empty() {
                    quote! { Ret }
                } else {
                    quote! { Ret<#(#assoc_use_args),*> }
                };
                let helper_ident = format_ident!("__InceptionInduce{}", ident);
                quote! {
                    type #ident #assoc_impl_generics = <T as #mod_ident::#helper_ident<::inception::False>>::#helper_ret_use #assoc_where_clause;
                }
            })
            .collect::<Vec<_>>();
        let induced_blanket_where_preds = assoc_types
            .iter()
            .map(|t| {
                let helper_ident = format_ident!("__InceptionInduce{}", t.item.ident);
                quote! {
                    T: #mod_ident::#helper_ident<::inception::False>,
                }
            })
            .collect::<Vec<_>>();

        let induce_head_placeholder = format_ident!("Head");
        let induce_tail_placeholder = format_ident!("Tail");
        let induce_fields_placeholder = format_ident!("Fields");
        let induced_assoc_helpers = assoc_types
            .iter()
            .filter_map(|t| {
                let induce = t.induce.as_ref()?;
                let assoc_ident = &t.item.ident;
                let helper_ident = format_ident!("__InceptionInduce{}", assoc_ident);
                let assoc_generics = &t.item.generics;
                let assoc_decl_params = assoc_generics.params.iter().cloned().collect::<Vec<_>>();
                let assoc_impl_params = assoc_generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            let mut tp = tp.clone();
                            tp.default = None;
                            GenericParam::Type(tp)
                        }
                        GenericParam::Lifetime(lp) => GenericParam::Lifetime(lp.clone()),
                        GenericParam::Const(cp) => GenericParam::Const(cp.clone()),
                    })
                    .collect::<Vec<_>>();
                let assoc_use_args = assoc_generics
                    .params
                    .iter()
                    .map(|p| match p {
                        GenericParam::Type(tp) => {
                            let id = &tp.ident;
                            quote! { #id }
                        }
                        GenericParam::Lifetime(lp) => {
                            let lt = &lp.lifetime;
                            quote! { #lt }
                        }
                        GenericParam::Const(cp) => {
                            let id = &cp.ident;
                            quote! { #id }
                        }
                    })
                    .collect::<Vec<_>>();
                let assoc_where_clause = &assoc_generics.where_clause;
                let assoc_decl_generics = if assoc_decl_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_decl_params),*> }
                };
                let assoc_impl_generics = if assoc_impl_params.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_impl_params),*> }
                };
                let assoc_proj_generics = if assoc_use_args.is_empty() {
                    quote! {}
                } else {
                    quote! { <#(#assoc_use_args),*> }
                };

                let helper_false_trait_args = quote! { <::inception::False> };
                let merge_head_ident = format_ident!("H");
                let merge_var_head_ident = format_ident!("H");
                let merge_tail_ty = quote! { #wrapper<F> };
                let merge_var_tail_ty = quote! { #wrapper<F> };
                let join_fields_ty = quote! { #wrapper<<T as Inception<#property>>::#fields_ident> };
                let helper_head_trait_args = quote! {
                    <<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is>
                };
                let helper_merge_var_head_trait_args = quote! {
                    <<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is>
                };
                let merge_head_self_ty = quote! { #merge_head_ident };
                let merge_tail_self_ty = quote! { #merge_tail_ty };
                let merge_var_head_self_ty = quote! { #merge_var_head_ident };
                let merge_var_tail_self_ty = quote! { #merge_var_tail_ty };
                let join_fields_self_ty = quote! { #join_fields_ty };
                let tail_assoc_ty = quote! {
                    <#merge_tail_ty as #helper_ident #helper_false_trait_args>::Ret #assoc_proj_generics
                };
                let merge_var_tail_assoc_ty = quote! {
                    <#merge_var_tail_ty as #helper_ident #helper_false_trait_args>::Ret #assoc_proj_generics
                };
                let fields_assoc_ty = quote! {
                    <#join_fields_ty as #helper_ident #helper_false_trait_args>::Ret #assoc_proj_generics
                };
                let use_join_ret_var = assoc_use_args.is_empty();
                let join_ret_ident = format_ident!("__InceptionJoinRet");
                let Ok(join_ret_ty) = syn::parse2::<Type>(quote! { #join_ret_ident }) else {
                    return Some(
                        syn::Error::new_spanned(
                            assoc_ident,
                            "Failed to parse induced join ret helper type.",
                        )
                        .into_compile_error(),
                    );
                };
                let merge_split_bound = quote! {
                    #wrapper<List<(#field<#merge_head_ident, S, IDX>, F)>>:
                        #split_trait_ident<#property, Left = #field<#merge_head_ident, S, IDX>, Right = F>,
                };
                let merge_var_split_bound = quote! {
                    #wrapper<List<(#var_field<#merge_var_head_ident, S, VAR_IDX, IDX>, F)>>:
                        #split_trait_ident<#property, Left = #var_field<#merge_var_head_ident, S, VAR_IDX, IDX>, Right = F>,
                };

                let base = &induce.base.ty;
                let merge = &induce.merge.ty;
                let merge_variant = &induce.merge_variant.ty;
                let join = &induce.join.ty;
                let base_ty = quote! { #base };
                let mut merge_ty_src = merge.clone();
                rewrite_assoc_projection_to_helper(
                    &mut merge_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_head_placeholder,
                    &helper_ident,
                    &merge_head_self_ty,
                    &helper_head_trait_args,
                );
                rewrite_assoc_projection_to_helper(
                    &mut merge_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_tail_placeholder,
                    &helper_ident,
                    &merge_tail_self_ty,
                    &helper_false_trait_args,
                );
                let merge_ty = substitute_type_ident(
                    &merge_ty_src,
                    &induce_head_placeholder,
                    &quote! { #merge_head_ident },
                );
                let merge_ty = {
                    let Ok(mut merge_ty_parsed) = syn::parse2::<Type>(merge_ty) else {
                        return Some(
                            syn::Error::new_spanned(merge_ty_src, "Failed to parse induced merge type.")
                                .into_compile_error(),
                        );
                    };
                    let Ok(tail_ty) = syn::parse2::<Type>(tail_assoc_ty.clone()) else {
                        return Some(
                            syn::Error::new_spanned(merge_ty_src, "Failed to parse induced tail helper type.")
                                .into_compile_error(),
                        );
                    };
                    replace_type_ident(&mut merge_ty_parsed, &induce_tail_placeholder, &tail_ty);
                    quote! { #merge_ty_parsed }
                };
                let mut merge_var_ty_src = merge_variant.clone();
                rewrite_assoc_projection_to_helper(
                    &mut merge_var_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_head_placeholder,
                    &helper_ident,
                    &merge_var_head_self_ty,
                    &helper_merge_var_head_trait_args,
                );
                rewrite_assoc_projection_to_helper(
                    &mut merge_var_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_tail_placeholder,
                    &helper_ident,
                    &merge_var_tail_self_ty,
                    &helper_false_trait_args,
                );
                let merge_var_ty = substitute_type_ident(
                    &merge_var_ty_src,
                    &induce_head_placeholder,
                    &quote! { #merge_var_head_ident },
                );
                let merge_var_ty = {
                    let Ok(mut merge_var_ty_parsed) = syn::parse2::<Type>(merge_var_ty) else {
                        return Some(
                            syn::Error::new_spanned(merge_var_ty_src, "Failed to parse induced variant-merge type.")
                                .into_compile_error(),
                        );
                    };
                    let Ok(tail_ty) = syn::parse2::<Type>(merge_var_tail_assoc_ty.clone()) else {
                        return Some(
                            syn::Error::new_spanned(merge_var_ty_src, "Failed to parse induced variant tail helper type.")
                                .into_compile_error(),
                        );
                    };
                    replace_type_ident(&mut merge_var_ty_parsed, &induce_tail_placeholder, &tail_ty);
                    quote! { #merge_var_ty_parsed }
                };
                let mut join_ty_src = join.clone();
                rewrite_assoc_projection_to_helper(
                    &mut join_ty_src,
                    &trait_ident,
                    assoc_ident,
                    &induce_fields_placeholder,
                    &helper_ident,
                    &join_fields_self_ty,
                    &helper_false_trait_args,
                );
                let join_ty = substitute_type_ident(
                    &join_ty_src,
                    &induce_fields_placeholder,
                    &fields_assoc_ty,
                );
                let join_ty = if use_join_ret_var {
                    let Ok(mut join_ty_parsed) = syn::parse2::<Type>(join_ty.clone()) else {
                        return Some(
                            syn::Error::new_spanned(
                                join_ty_src,
                                "Failed to parse induced join type.",
                            )
                            .into_compile_error(),
                        );
                    };
                    replace_helper_ret_projection(&mut join_ty_parsed, &helper_ident, &join_ret_ty);
                    quote! { #join_ty_parsed }
                } else {
                    join_ty
                };
                let Ok(merge_head_ty) = syn::parse2::<Type>(quote! { #merge_head_ident }) else {
                    return Some(
                        syn::Error::new_spanned(
                            assoc_ident,
                            "Failed to parse induced merge head bound type.",
                        )
                        .into_compile_error(),
                    );
                };
                let Ok(merge_var_head_ty) =
                    syn::parse2::<Type>(quote! { #merge_var_head_ident })
                else {
                    return Some(
                        syn::Error::new_spanned(
                            assoc_ident,
                            "Failed to parse induced merge-variant head bound type.",
                        )
                        .into_compile_error(),
                    );
                };
                let Ok(tail_assoc_bound_ty) = syn::parse2::<Type>(tail_assoc_ty.clone()) else {
                    return Some(
                        syn::Error::new_spanned(
                            assoc_ident,
                            "Failed to parse induced merge tail bound type.",
                        )
                        .into_compile_error(),
                    );
                };
                let Ok(merge_var_tail_assoc_bound_ty) =
                    syn::parse2::<Type>(merge_var_tail_assoc_ty.clone())
                else {
                    return Some(
                        syn::Error::new_spanned(
                            assoc_ident,
                            "Failed to parse induced merge-variant tail bound type.",
                        )
                        .into_compile_error(),
                    );
                };
                let base_extra_where_preds = substitute_where_preds(&induce.base.where_preds, &[]);
                let mut merge_where_preds_src = induce.merge.where_preds.clone();
                for pred in merge_where_preds_src.iter_mut() {
                    if let WherePredicate::Type(tp) = pred {
                        rewrite_assoc_projection_to_helper(
                            &mut tp.bounded_ty,
                            &trait_ident,
                            assoc_ident,
                            &induce_head_placeholder,
                            &helper_ident,
                            &merge_head_self_ty,
                            &helper_head_trait_args,
                        );
                        rewrite_assoc_projection_to_helper(
                            &mut tp.bounded_ty,
                            &trait_ident,
                            assoc_ident,
                            &induce_tail_placeholder,
                            &helper_ident,
                            &merge_tail_self_ty,
                            &helper_false_trait_args,
                        );
                    }
                }
                let mut merge_variant_where_preds_src = induce.merge_variant.where_preds.clone();
                for pred in merge_variant_where_preds_src.iter_mut() {
                    if let WherePredicate::Type(tp) = pred {
                        rewrite_assoc_projection_to_helper(
                            &mut tp.bounded_ty,
                            &trait_ident,
                            assoc_ident,
                            &induce_head_placeholder,
                            &helper_ident,
                            &merge_var_head_self_ty,
                            &helper_merge_var_head_trait_args,
                        );
                        rewrite_assoc_projection_to_helper(
                            &mut tp.bounded_ty,
                            &trait_ident,
                            assoc_ident,
                            &induce_tail_placeholder,
                            &helper_ident,
                            &merge_var_tail_self_ty,
                            &helper_false_trait_args,
                        );
                    }
                }
                let base_extra_where_clause = if base_extra_where_preds.is_empty() {
                    quote! {}
                } else {
                    quote! { where #(#base_extra_where_preds,)* }
                };
                let merge_extra_where_preds = substitute_where_preds(
                    &merge_where_preds_src,
                    &[
                        (&induce_head_placeholder, &merge_head_ty),
                        (&induce_tail_placeholder, &tail_assoc_bound_ty),
                    ],
                );
                let merge_variant_extra_where_preds = substitute_where_preds(
                    &merge_variant_where_preds_src,
                    &[
                        (&induce_head_placeholder, &merge_var_head_ty),
                        (&induce_tail_placeholder, &merge_var_tail_assoc_bound_ty),
                    ],
                );
                let Ok(join_fields_bound_ty) =
                    syn::parse2::<Type>(quote! { <T as Inception<#property>>::#fields_ident })
                else {
                    return Some(
                        syn::Error::new_spanned(
                            assoc_ident,
                            "Failed to parse induced join fields type.",
                        )
                        .into_compile_error(),
                    );
                };
                let mut join_where_preds_src = induce.join.where_preds.clone();
                for pred in join_where_preds_src.iter_mut() {
                    if let WherePredicate::Type(tp) = pred {
                        rewrite_assoc_projection_to_helper(
                            &mut tp.bounded_ty,
                            &trait_ident,
                            assoc_ident,
                            &induce_fields_placeholder,
                            &helper_ident,
                            &join_fields_self_ty,
                            &helper_false_trait_args,
                        );
                        // `Fields: Trait` is already represented by the helper bound added to the
                        // join impl (`Wrap<TyFields>: __InceptionInduce*<False>`). Keeping both
                        // creates a recursive obligation through the blanket trait impl.
                        if type_is_plain_ident(&tp.bounded_ty, &induce_fields_placeholder) {
                            remove_self_trait_bounds(&mut tp.bounds, &trait_ident);
                        }
                        if use_join_ret_var {
                            replace_helper_ret_projection(
                                &mut tp.bounded_ty,
                                &helper_ident,
                                &join_ret_ty,
                            );
                        }
                    }
                }
                join_where_preds_src.retain(|pred| match pred {
                    WherePredicate::Type(tp) => !tp.bounds.is_empty(),
                    _ => true,
                });
                let join_extra_where_preds = substitute_where_preds(
                    &join_where_preds_src,
                    &[(&induce_fields_placeholder, &join_fields_bound_ty)],
                );
                let merge_extra_where_preds =
                    lower_helper_projection_preds(merge_extra_where_preds, &helper_ident);
                let merge_variant_extra_where_preds =
                    lower_helper_projection_preds(merge_variant_extra_where_preds, &helper_ident);
                let join_extra_where_preds =
                    lower_helper_projection_preds(join_extra_where_preds, &helper_ident);
                let join_extra_where_preds = join_extra_where_preds
                    .into_iter()
                    .filter(|pred| match pred {
                        // Bounds like `(Prefix, <Wrap<TyFields> as __InceptionInduce*>::Ret): Bound`
                        // force recursive normalization of the helper projection during impl WF
                        // checking; they are redundant with the induced `join` return type itself.
                        WherePredicate::Type(tp) => {
                            !type_contains_helper_ret_projection(&tp.bounded_ty, &helper_ident)
                        }
                        _ => true,
                    })
                    .collect::<Vec<_>>();
                let join_impl_generics = if use_join_ret_var {
                    quote! { <T, #join_ret_ident> }
                } else {
                    quote! { <T> }
                };
                let join_fields_helper_bound = if use_join_ret_var {
                    quote! { #join_fields_ty: #helper_ident<::inception::False, Ret = #join_ret_ident>, }
                } else {
                    quote! { #join_fields_ty: #helper_ident #helper_false_trait_args, }
                };

                Some(quote! {
                    pub trait #helper_ident<P: TruthValue = <Self as IsPrimitive<super::#property_ident>>::Is> {
                        type Ret #assoc_decl_generics #assoc_where_clause;
                    }

                    impl<T> #helper_ident<True> for T
                    where
                        T: super::#trait_ident #trait_generic_args + IsPrimitive<super::#property_ident, Is = True>,
                    {
                        type Ret #assoc_impl_generics = <T as super::#trait_ident #trait_generic_args>::#assoc_ident #assoc_proj_generics #assoc_where_clause;
                    }

                    impl #helper_ident<::inception::False> for #wrapper<#liferefelide List<()>> #base_extra_where_clause
                    {
                        type Ret #assoc_impl_generics = #base_ty #assoc_where_clause;
                    }

                    impl<#merge_head_ident, S, const IDX: usize, F> #helper_ident<::inception::False> for #wrapper<List<(#field<#merge_head_ident, S, IDX>, F)>>
                    where
                        S: FieldsMeta,
                        #merge_head_ident: ::inception::IsPrimitive<#property>,
                        #merge_head_ident: #helper_ident #helper_head_trait_args,
                        #merge_tail_ty: #helper_ident #helper_false_trait_args,
                        F: Fields #phantom_bound,
                        <F as Fields>::Owned: Fields,
                        #merge_split_bound
                        #(#merge_extra_where_preds,)*
                    {
                        type Ret #assoc_impl_generics = #merge_ty #assoc_where_clause;
                    }

                    impl<#merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F> #helper_ident<::inception::False> for #wrapper<List<(#var_field<#merge_var_head_ident, S, VAR_IDX, IDX>, F)>>
                    where
                        S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
                        #merge_var_head_ident: ::inception::IsPrimitive<#property>,
                        #merge_var_head_ident: #helper_ident #helper_merge_var_head_trait_args,
                        #merge_var_tail_ty: #helper_ident #helper_false_trait_args,
                        F: Fields #phantom_bound,
                        <F as Fields>::Owned: Fields,
                        #merge_var_split_bound
                        #(#merge_variant_extra_where_preds,)*
                    {
                        type Ret #assoc_impl_generics = #merge_var_ty #assoc_where_clause;
                    }

                    impl #join_impl_generics #helper_ident<::inception::False> for T
                    where
                        T: Inception<#property> + Meta,
                        #join_fields_helper_bound
                        #(#join_extra_where_preds,)*
                    {
                        type Ret #assoc_impl_generics = #join_ty #assoc_where_clause;
                    }
                })
            })
            .collect::<Vec<_>>();

        let expanded = quote! {
            pub struct #property_ident;
            #vis trait #trait_ident #trait_generic_params #trait_supertrait_clause #trait_where_clause {
                #(#assoc_trait_items)*
            }

            mod #mod_ident {
                use inception::{Wrapper, TruthValue, IsPrimitive, meta::Metadata, True, False};
                use super::*;

                impl ::inception::Property for super::#property_ident {}
                #compat_impl

                pub struct Wrap<T>(pub T);
                impl<T> Wrapper for Wrap<T> {
                    type Content = T;
                    fn wrap(t: Self::Content) -> Self {
                        Self(t)
                    }
                }

                impl<T> IsPrimitive<super::#property_ident> for Wrap<T> {
                    type Is = False;
                }

                #split_impl
                #(#induced_assoc_helpers)*
            }

            #blanket_impl_head
            where
                #(#induced_blanket_where_preds)*
                T: ::inception::IsPrimitive<#property, Is = ::inception::False> #trait_supertrait_bounds,
            {
                #(#assoc_impl_items)*
            }
        };

        expanded.into()
    }
}
