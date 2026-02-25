use std::ops::Deref;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, Block, FnArg, GenericParam, Ident,
    ItemTrait, Pat, PatIdent, PatType, ReturnType, TraitBound, TraitItem, TraitItemFn,
    TraitItemType, Type, TypeParam, TypeParamBound, TypePath, Visibility,
};

use crate::derive::Identifier;

const NOTHING_FN_IDENT: &str = "nothing";
const MERGE_FN_IDENT: &str = "merge";
const MERGE_VAR_FN_IDENT: &str = "merge_variant_field";
const JOIN_FN_IDENT: &str = "join";

#[derive(deluxe::ParseMetaItem, deluxe::ExtractAttributes)]
#[deluxe(attributes(inception))]
struct Attributes {
    property: Ident,
    #[deluxe(default)]
    comparator: bool,
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
        fn replace_flow_type(ty: &mut Type, flow_in: Option<&Ident>, flow_out: Option<&Ident>) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(flow) = flow_in {
                        if path.is_ident(flow) {
                            *ty = syn::parse_quote!(In);
                            return;
                        }
                    }
                    if let Some(flow) = flow_out {
                        if path.is_ident(flow) {
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
                                    syn::GenericArgument::AssocType(assoc) => {
                                        replace_flow_type(
                                            &mut assoc.ty,
                                            flow_input_ident,
                                            flow_output_ident,
                                        )
                                    }
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
                                    syn::GenericArgument::AssocType(assoc) => {
                                        replace_flow_type(
                                            &mut assoc.ty,
                                            flow_input_ident,
                                            flow_output_ident,
                                        )
                                    }
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
    mod_ident: Ident,
    fn_ident: Ident,
    fn_args: proc_macro2::TokenStream,
    fn_args_list: Punctuated<FnArg, Comma>,
    fn_arg_idents: Punctuated<Ident, Comma>,
    fn_ret: ReturnType,
    vis: Visibility,
    kind: Kind,
    assoc_types: Vec<TraitItemType>,
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
                let Ok(Attributes {
                    property,
                    comparator,
                }) = deluxe::parse(attr)
                else {
                    return syn::Error::new_spanned(
                        x,
                        "Invalid attributes, expected \"property = ..\"",
                    )
                    .into_compile_error()
                    .into();
                };

                match State::process(x, property, comparator) {
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

    pub fn process(
        tr: ItemTrait,
        property_ident: Ident,
        is_comparator: bool,
    ) -> Result<TokenStream, TokenStream> {
        let mut st = State {
            trait_ident: tr.ident.clone(),
            trait_generics: tr.generics.clone(),
            trait_supertraits: tr.supertraits.clone(),
            property_ident,
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
                    st.assoc_types.push(t.clone());
                }
                _ => {}
            }
        }

        if !is_fn_defined {
            let msg = &format!(
                    "Expected 1 function besides \"{NOTHING_FN_IDENT}\" \"{MERGE_FN_IDENT}\" \"{MERGE_VAR_FN_IDENT}\" or \"{JOIN_FN_IDENT}\"");
            return Err(syn::Error::new_spanned(tr, msg).into_compile_error().into());
        }

        Ok(st.finish(is_comparator))
    }

    fn finish(self, is_comparator: bool) -> TokenStream {
        let State {
            mod_ident,
            trait_ident,
            trait_generics,
            trait_supertraits,
            property_ident,
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

        let (flow_input_ident, flow_output_ident) = {
            let has_non_type = trait_generics
                .params
                .iter()
                .any(|p| !matches!(p, GenericParam::Type(_)));
            if has_non_type {
                let msg = "Only type generics are currently supported for #[inception] traits.";
                return syn::Error::new_spanned(trait_ident, msg)
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
            if type_params.len() > 1 {
                let msg = "Only a single type generic is currently supported for #[inception] traits.";
                return syn::Error::new_spanned(trait_ident, msg)
                    .into_compile_error()
                    .into();
            }
            let flow_input_ident = type_params.first().cloned();
            let flow_output_ident = None;
            (flow_input_ident, flow_output_ident)
        };

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
        fn replace_flow_type(ty: &mut Type, flow_in: Option<&Ident>, flow_out: Option<&Ident>) {
            match ty {
                Type::Path(TypePath { qself, path }) => {
                    if let Some(flow) = flow_in {
                        if path.is_ident(flow) {
                            *ty = syn::parse_quote!(In);
                            return;
                        }
                    }
                    if let Some(flow) = flow_out {
                        if path.is_ident(flow) {
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
                replace_flow_type(&mut ty, flow_input_ident.as_ref(), flow_output_ident.as_ref());
                nothing_ret = quote! { #ty };
            }
            if let Ok(mut ty) = syn::parse2::<Type>(merge_ret.clone()) {
                replace_flow_type(&mut ty, flow_input_ident.as_ref(), flow_output_ident.as_ref());
                merge_ret = quote! { #ty };
            }
            if let Ok(mut ty) = syn::parse2::<Type>(merge_var_ret.clone()) {
                replace_flow_type(&mut ty, flow_input_ident.as_ref(), flow_output_ident.as_ref());
                merge_var_ret = quote! { #ty };
            }
            if let Ok(mut ty) = syn::parse2::<Type>(join_ret.clone()) {
                replace_flow_type(&mut ty, flow_input_ident.as_ref(), flow_output_ident.as_ref());
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
                replace_flow_type(&mut ty, flow_input_ident.as_ref(), flow_output_ident.as_ref());
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
        let has_output_assoc = assoc_types.iter().any(|t| t.ident == "Output");
        let flow_mode = flow_input_ident.is_some();
        let flow_two_generic = flow_output_ident.is_some();
        let flow_assoc_borrow_mode =
            flow_mode && has_output_assoc && matches!(kind, Kind::Ref | Kind::Mut);
        let flow_assoc_merge_ret_ident = format_ident!("__InceptionTailRet");
        let flow_assoc_join_ret_ident = format_ident!("__InceptionJoinRet");
        let needs_borrow_output_helpers = has_output_assoc && matches!(kind, Kind::Ref | Kind::Mut);
        let fields_output_ident = format_ident!("FieldsOutput");
        let tail_output_ident = format_ident!("TailOutput");
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
        let needs_named_ret_lifetime =
            has_output_assoc && matches!(kind, Kind::Ref | Kind::Mut) && !flow_assoc_borrow_mode;
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
                <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In>>::Ret
            }
        } else {
            quote! {
                <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In>>::OutTy
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
                <#merge_var_head_ident as #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In>>::Ret
            }
        } else {
            quote! {
                <#merge_var_head_ident as #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In>>::OutTy
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
        let substitute_ret_ident = |ret: &proc_macro2::TokenStream,
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
            substitute_ret_ident(&join_ret, &join_fields_ident, &join_wrapper_fields_ty_named_ret)
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
        let trait_bound_with_in =
            match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                (Some(_), Some(_)) => quote! { super::#trait_ident<In, Out> },
                (Some(_), None) => quote! { super::#trait_ident<In> },
                (None, _) => quote! { super::#trait_ident },
            };
        let primitive_impl_generics = match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
            (Some(_), Some(_)) => quote! { <T, In, Out> },
            (Some(_), None) => quote! { <T, In> },
            (None, _) => quote! { <T, In> },
        };
        let primitive_impl_trait_args =
            match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
                (Some(_), Some(_)) => quote! { <True, In, Out> },
                (Some(_), None) => quote! { <True, In> },
                (None, _) => quote! { <True, In> },
            };
        let blanket_impl_head = match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
            (Some(flow_in), Some(flow_out)) => {
                quote! { impl<T, #flow_in, #flow_out> #trait_ident<#flow_in, #flow_out> for T }
            }
            (Some(flow), None) => quote! { impl<T, #flow> #trait_ident<#flow> for T },
            (None, _) => quote! { impl<T> #trait_ident for T },
        };
        let blanket_inner_bound = match (flow_input_ident.as_ref(), flow_output_ident.as_ref()) {
            (Some(flow_in), Some(flow_out)) => {
                quote! { #inner_trait<::inception::False, #flow_in, #flow_out, Ret = #flow_out> }
            }
            (Some(flow), None) => quote! { #inner_trait<::inception::False, #flow> },
            (None, _) => quote! { #inner_trait<::inception::False, Ret = #fn_ret_public> },
        };
        let primitive_ret = if has_output_assoc {
            quote! { <T as #trait_bound_with_in>::Output }
        } else {
            quote! { #fn_ret_inner }
        };
        let primitive_out_ty = if flow_two_generic {
            quote! { Out }
        } else {
            quote! { In }
        };
        let assoc_trait_items = assoc_types.iter().map(|t| quote! { #t }).collect::<Vec<_>>();
        let assoc_impl_items = assoc_types
            .iter()
            .filter_map(|t| {
                let ident = &t.ident;
                if ident == "Output" {
                    if let Some(flow) = flow_input_ident.as_ref() {
                        Some(quote! {
                            type #ident = <T as #inner_trait<::inception::False, #flow>>::Ret;
                        })
                    } else {
                        Some(quote! {
                            type #ident = <T as #inner_trait<::inception::False>>::Ret;
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
            quote! { <#lifepunct1 #merge_head_ident, S, const IDX: usize, F, L, #merge_fields_ident, #(#merge_extra_generics,)* In, Out> }
        } else {
            quote! { <#lifepunct1 #merge_head_ident, S, const IDX: usize, F, L, #merge_fields_ident, In> }
        };
        let merge_field_impl_trait_args = if flow_mode {
            quote! { <L, #merge_fields_ident, In, Out, #merge_extra_tuple_ty> }
        } else {
            quote! { <L, #merge_fields_ident, In, In, #merge_extra_tuple_ty> }
        };
        let merge_field_head_bound = if flow_two_generic {
            quote! { ::core::marker::Sized #merge_field_head_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            if flow_assoc_borrow_mode {
                quote! { #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In> + ::inception::IsPrimitive<#property> }
            } else {
                quote! { #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In> #merge_field_head_bounds + ::inception::IsPrimitive<#property> }
            }
        } else {
            quote! { #inner_trait #merge_field_head_bounds + ::inception::IsPrimitive<#property> }
        };
        let merge_field_tail_bound = if flow_two_generic {
            quote! { Fields #merge_fields_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            if flow_assoc_borrow_mode {
                quote! { Fields + #inner_trait<::inception::False, #merge_head_out_ty, Out> + ::inception::IsPrimitive<#property> }
            } else {
                quote! { Fields + #inner_trait<::inception::False, #merge_head_out_ty, Out> #merge_fields_bounds + ::inception::IsPrimitive<#property> }
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
            quote! { <#lifepunct1 #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, L, #merge_var_fields_ident, #(#merge_var_extra_generics,)* In, Out> }
        } else {
            quote! { <#lifepunct1 #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, L, #merge_var_fields_ident, In> }
        };
        let merge_variant_impl_trait_args = if flow_mode {
            quote! { <L, #merge_var_fields_ident, In, Out, #merge_var_extra_tuple_ty> }
        } else {
            quote! { <L, #merge_var_fields_ident, In, In, #merge_var_extra_tuple_ty> }
        };
        let merge_variant_head_bound = if flow_two_generic {
            quote! { ::core::marker::Sized #merge_var_field_head_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            quote! { #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In> #merge_var_field_head_bounds + ::inception::IsPrimitive<#property> }
        } else {
            quote! { #inner_trait #merge_var_field_head_bounds + ::inception::IsPrimitive<#property> }
        };
        let merge_variant_tail_bound = if flow_two_generic {
            quote! { Fields #merge_var_fields_bounds + ::inception::IsPrimitive<#property> }
        } else if flow_mode {
            quote! { Fields + #inner_trait<::inception::False, #merge_var_head_out_ty, Out> #merge_var_fields_bounds + ::inception::IsPrimitive<#property> }
        } else {
            quote! { Fields + #inner_trait #merge_var_fields_bounds + ::inception::IsPrimitive<#property> }
        };
        let merge_variant_impl_out_ty = if flow_mode {
            quote! { Out }
        } else {
            quote! { In }
        };
        let join_impl_generics = if flow_mode {
            quote! { <T, #join_fields_ident, In, Out> }
        } else {
            quote! { <T, #join_fields_ident, In> }
        };
        let join_impl_trait_args = if flow_mode {
            quote! { <#join_fields_ident, In, Out> }
        } else {
            quote! { <#join_fields_ident, In, In> }
        };
        let join_fields_bound = if flow_mode {
            if flow_assoc_borrow_mode {
                quote! { #inner_trait<::inception::False, In, Out> + ::inception::IsPrimitive<#property> }
            } else {
                quote! { #inner_trait<::inception::False, In, Out> #join_fields_bounds + ::inception::IsPrimitive<#property> }
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
            quote! { <#join_fields_ident as #inner_trait<::inception::False, In, Out>>::Ret }
        } else {
            quote! { #join_ret }
        };
        let merge_call_trait_args = if flow_mode {
            quote! { <_, _, In, Out, _> }
        } else {
            quote! { <_, _, In, In, _> }
        };
        let merge_variant_call_trait_args = if flow_mode {
            quote! { <_, _, In, Out, _> }
        } else {
            quote! { <_, _, In, In, _> }
        };
        let join_call_trait_args = if flow_mode {
            quote! { <_, In, Out> }
        } else {
            quote! { <_, In> }
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
                        #inner_trait<::inception::False, #merge_head_out_ty, Out, Ret = #flow_assoc_merge_ret_ident> + Fields,
                },
                Kind::Mut => quote! {
                    for<'a> #wrapper<&'a mut F>:
                        #inner_trait<::inception::False, #merge_head_out_ty, Out, Ret = #flow_assoc_merge_ret_ident> + Fields,
                },
                _ => quote! {
                    #split_for_3 #merge_split_right_ty:
                        #inner_trait<::inception::False, #merge_head_out_ty, Out> #merge_tail_extra_bounds + Fields,
                },
            }
        } else {
            quote! {
                #split_for_3 #merge_split_right_ty:
                    #inner_trait<::inception::False, #merge_head_out_ty, Out> #merge_tail_extra_bounds + Fields,
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
                    #inner_trait<::inception::False, #merge_var_head_out_ty, Out> #merge_var_tail_extra_bounds + Fields,
            }
        };
        let join_fields_inner_bound = if flow_assoc_borrow_mode {
            match kind {
                Kind::Ref => quote! {
                    for<'a, 'b> #wrapper<&'a <T as Inception<#property>>::#fields_ident<'b>>:
                        #inner_trait<::inception::False, In, Out, Ret = #flow_assoc_join_ret_ident>,
                },
                Kind::Mut => quote! {
                    for<'a, 'b> #wrapper<&'a mut <T as Inception<#property>>::#fields_ident<'b>>:
                        #inner_trait<::inception::False, In, Out, Ret = #flow_assoc_join_ret_ident>,
                },
                _ => quote! {
                    for<#lifepunct1 #life2> #join_wrapper_fields_ty:
                        #inner_trait<::inception::False, In, Out>,
                },
            }
        } else {
            quote! {
                for<#lifepunct1 #life2> #join_wrapper_fields_ty: #inner_trait<::inception::False, In, Out> #join_fields_bounds,
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
                    #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In> + ::inception::IsPrimitive<#property>
                }
            } else {
                quote! {
                    #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In> #merge_inner_head_extra_bounds + ::inception::IsPrimitive<#property>
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
                #inner_trait<<#merge_var_head_ident as ::inception::IsPrimitive<#property>>::Is, In> #merge_var_inner_head_extra_bounds + ::inception::IsPrimitive<#property>
            }
        };
        let empty_impl_generics = if flow_mode {
            quote! { <In, Out> }
        } else {
            quote! { <In> }
        };
        let empty_impl_trait_args = if flow_mode {
            quote! { <::inception::False, In, Out> }
        } else {
            quote! { <::inception::False, In> }
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
            (
                join_fn_args,
                quote! { , #join_extra_args },
            )
        };
        let flow_merge_input_ident = merge_arg_idents
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| format_ident!("_in"));
        let flow_join_input_ident = join_arg_idents
            .iter()
            .next()
            .cloned()
            .unwrap_or_else(|| format_ident!("_in"));
        let merge_impl_ret_ty = if flow_assoc_borrow_mode {
            quote! { <#merge_fields_ident as #inner_trait<::inception::False, #merge_head_out_ty, Out>>::Ret }
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
                        let next = <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In>>::#inner_fn(
                            #merge_head_arg.access(),
                            #flow_merge_input_ident,
                        );
                        <#merge_fields_ident as #inner_trait<::inception::False, #merge_head_out_ty, Out>>::#inner_fn(
                            &#merge_fields_arg,
                            next,
                        )
                    }
                },
                Kind::Mut => quote! {
                    {
                        let next = <#merge_head_ident as #inner_trait<<#merge_head_ident as ::inception::IsPrimitive<#property>>::Is, In>>::#inner_fn(
                            #merge_head_arg.access(),
                            #flow_merge_input_ident,
                        );
                        <#merge_fields_ident as #inner_trait<::inception::False, #merge_head_out_ty, Out>>::#inner_fn(
                            &mut #merge_fields_arg,
                            next,
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
                        <#join_fields_ident as #inner_trait<::inception::False, In, Out>>::#inner_fn(
                            &#join_fields_arg,
                            #flow_join_input_ident,
                        )
                    }
                },
                Kind::Mut => quote! {
                    {
                        <#join_fields_ident as #inner_trait<::inception::False, In, Out>>::#inner_fn(
                            &mut #join_fields_arg,
                            #flow_join_input_ident,
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

        let expanded = quote! {
                pub struct #property_ident;
                #vis trait #trait_ident #trait_generic_params #trait_supertrait_clause #trait_where_clause {
                    #(#assoc_trait_items)*
                    fn #fn_ident(#mutref #receiver #fn_args) -> #fn_ret_public;
                }

                mod #mod_ident {
                    use inception::{Wrapper, TruthValue, IsPrimitive, meta::Metadata, True, False};

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
                    pub trait #inductive_ident<P: TruthValue = <Self as IsPrimitive<super::#property_ident>>::Is, In = (), Out = In> {
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
                            #dispatcher #fn_ident( #fn_arg_idents )
                        }
                    }

                    pub trait Nothing<In = ()> {
                        type InTy;
                        type OutTy;
                        type Ret;
                        fn nothing(#nothing_args) -> Self::Ret;
                    }
                    pub trait MergeField<L, R, In = (), Out = In, Extra = ()> {
                        type InTy;
                        type OutTy;
                        type Ret;
                        fn merge_field(l: L, r: R #merge_args) -> Self::Ret;
                    }
                    pub trait MergeVariantField<L, R, In = (), Out = In, Extra = ()> {
                        type InTy;
                        type OutTy;
                        type Ret;
                        fn merge_variant_field(l: L, r: R #merge_var_args) -> Self::Ret;
                    }
                    pub trait Join<F, In = (), Out = In> {
                        type InTy;
                        type OutTy;
                        type Ret;
                        fn join(fields: F #join_trait_args) -> Self::Ret;
                    }
                    #borrow_output_helpers
                }

                #blanket_impl_head
                where
                    T: #blanket_inner_bound + ::inception::IsPrimitive<#property, Is = ::inception::False> #trait_supertrait_bounds,
                {
                    #(#assoc_impl_items)*
                    fn #fn_ident(#mutref #receiver #fn_args) -> #fn_ret_public {
                        #dispatcher #inner_fn(#fn_arg_idents)
                    }
                }

                impl<T, In> #mod_ident :: Nothing<In> for T {
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
                    type InTy = <Self as #mod_ident :: Nothing<In>>::InTy;
                    type OutTy = <Self as #mod_ident :: Nothing<In>>::OutTy;
                    type Ret = #nothing_ret;
                    #[allow(unused)]
                    fn #inner_fn(#mutref #receiver #inner_fn_args) -> Self::Ret {
                        <Self as #mod_ident :: Nothing<In>>::nothing(#nothing_arg_idents)
                    }
                }

                impl<#ret_lifetime_decl #merge_head_ident, S, const IDX: usize, F, In, Out #flow_assoc_merge_ret_generic> #inner_trait<::inception::False, In, Out> for #merge_inductive_self_ty
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

                impl<#ret_lifetime_decl #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, In, Out> #inner_trait<::inception::False, In, Out>
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

                impl<T, In, Out #flow_assoc_join_ret_generic> #inner_trait<False, In, Out> for T
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
}
