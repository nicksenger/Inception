use std::ops::Deref;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, punctuated::Punctuated, token::Comma, Block, FnArg, GenericParam, Ident,
    ItemTrait, Pat, PatIdent, PatType, ReturnType, TraitBound, TraitItem, TraitItemFn, Type,
    TypeParam, TypeParamBound, TypePath, Visibility,
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
        let mutref = self.refmut();
        let field = self.field();
        let varfield = self.var_field();
        let trt = self.split_trait_ident();
        let fnn = match self {
            Self::Ty => format_ident!("split_ty"),
            Self::Ref => format_ident!("split_ref"),
            Self::Mut => format_ident!("split_mut"),
            Self::Owned => format_ident!("split"),
        };
        let lifepunct2 = self.lifepunct2();
        let liferefelide = self.liferefelide();
        let vfimpl = self.split_fn(true);
        let fimpl = self.split_fn(false);

        let vbound = match self {
            Self::Ty => quote! { : Phantom },
            _ => quote! {},
        };

        quote! {
            impl<#lifepunct2 T, S, const IDX: usize, V #vbound> #trt<#property> for #wrapper<#liferefelide List<(#field<#lifepunct2 T, S, IDX>, V)>>
            {
                type Left = #field<#lifepunct2 T, S, IDX>;
                type Right = V;

                fn #fnn(#mutref self) -> (Self::Left, #mutref Self::Right) {
                    #fimpl
                }
            }
            impl<#lifepunct2 T, S, const VAR_IDX: usize, const IDX: usize, V> #trt<#property> for #wrapper<#liferefelide List<(#varfield<#lifepunct2 T, S, VAR_IDX, IDX>, V)>>
            where
                V: Phantom,
            {
                type Left = #varfield<#lifepunct2 T, S, VAR_IDX, IDX>;
                type Right = V;

                fn #fnn(#mutref self) -> (Self::Left, #mutref Self::Right) {
                    #vfimpl
                }
            }
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
            GenericParam::Type(TypeParam { bounds, .. }) => {
                let bounds = bounds.iter().cloned().collect::<Vec<_>>();
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
    fn use_inner_trait(
        mut self,
        trait_ident: &Ident,
        inductive_ident: &Ident,
        mod_ident: &Ident,
    ) -> Self {
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
}
impl Nothing {
    fn parse(f: &TraitItemFn) -> Result<Self, &str> {
        Ok(Self {
            nothing_body: f.body()?,
            nothing_ret: f.ret(),
        })
    }
}

struct MergeField {
    merge_body: Block,
    merge_ret: proc_macro2::TokenStream,
    merge_head_ident: Ident,
    merge_fields_ident: Ident,
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
        Ok(Self {
            merge_body: f.body()?,
            merge_ret: f.ret(),
            merge_head_ident: f.generic_ident(0)?,
            merge_fields_ident: f.generic_ident(1)?,
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
        Ok(Self {
            merge_var_body: f.body()?,
            merge_var_ret: f.ret(),
            merge_var_head_ident: f.generic_ident(0)?,
            merge_var_fields_ident: f.generic_ident(1)?,
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
    property_ident: Ident,
    mod_ident: Ident,
    fn_ident: Ident,
    fn_args: proc_macro2::TokenStream,
    fn_arg_idents: Punctuated<Ident, Comma>,
    fn_ret: ReturnType,
    vis: Visibility,
    kind: Kind,
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
            property_ident,
            mod_ident: format_ident!("__inception_{}", Identifier::modularize(&tr.ident)),
            fn_ident: format_ident!("unknown"),
            fn_ret: ReturnType::Default,
            fn_args: Default::default(),
            fn_arg_idents: Default::default(),
            vis: tr.vis.clone(),
            kind: Kind::Ty,
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
                        st.kind = kind;
                        st.fn_ident = f.sig.ident.clone();
                        st.fn_ret = f.sig.output.clone();
                        let args = f.sig.inputs.iter().skip(1).collect::<Vec<_>>();
                        st.fn_args = {
                            if !args.is_empty() {
                                quote! { , #(#args),* }
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
            property_ident,
            vis,
            kind,
            fn_ident,
            fn_ret,
            fn_args,
            fn_arg_idents,
            nothing,
            merge_field,
            merge_var,
            join,
            ..
        } = self;

        let Some(Nothing {
            nothing_body,
            nothing_ret,
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
            join_args,
            join_arg_idents,
            ..
        }) = join
        else {
            let msg = format!("Expected definition for \"{JOIN_FN_IDENT}\"");
            return syn::Error::new_spanned(trait_ident, msg)
                .into_compile_error()
                .into();
        };

        let inductive_ident = format_ident!("Inductive");
        let merge_field_head_bounds = merge_field_head_bounds
            .use_inner_trait(&trait_ident, &inductive_ident, &mod_ident)
            .into_tokens();
        let merge_fields_bounds = merge_fields_bounds
            .use_inner_trait(&trait_ident, &inductive_ident, &mod_ident)
            .into_tokens();
        let merge_var_field_head_bounds = merge_var_field_head_bounds
            .use_inner_trait(&trait_ident, &inductive_ident, &mod_ident)
            .into_tokens();
        let merge_var_fields_bounds = merge_var_fields_bounds
            .use_inner_trait(&trait_ident, &inductive_ident, &mod_ident)
            .into_tokens();
        let join_fields_bounds = join_fields_bounds
            .use_inner_trait(&trait_ident, &inductive_ident, &mod_ident)
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

        let fn_ret = match fn_ret {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_r, t) => quote! { #t },
        };
        let fields_fn = kind.fields_fn(&property);
        let split_impl = kind.split_impl(&property, &wrapper);
        let split_trait_ident = kind.split_trait_ident();
        let split_fn_ident = kind.split_fn_ident();
        let split_fn_receiver = kind.split_fn_receiver();

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
        let merge_args = if merge_args.is_empty() {
            quote! {}
        } else {
            quote! { , #merge_args }
        };
        let merge_var_args = if merge_args.is_empty() {
            quote! {}
        } else {
            quote! { , #merge_var_args }
        };
        let (join_args, join_trait_args) = if is_comparator {
            (
                quote! {
                    , #join_arg_idents: #mutref Self
                },
                quote! { , #join_arg_idents: F },
            )
        } else if join_args.is_empty() {
            (quote! {}, quote! {})
        } else {
            (
                quote! {
                    , #join_args
                },
                quote! { , #join_args },
            )
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

        quote! {
                pub struct #property_ident;
                #vis trait #trait_ident {
                    fn #fn_ident(#mutref #receiver #fn_args) -> #fn_ret;
                }

                mod #mod_ident {
                    use inception::{Wrapper, TruthValue, IsPrimitive, meta::Metadata, True, False};

                    impl ::inception::Property for super::#property_ident {}
                    impl<T> ::inception::Compat<T> for super::#property_ident where T: super::#trait_ident {
                        type Out = True;
                    }

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
                    pub trait #inductive_ident<P: TruthValue = <Self as IsPrimitive<super::#property_ident>>::Is> {
                        type Property: ::inception::Property;
                        type Ret;
                        fn #inner_fn(#mutref #receiver #fn_args) -> Self::Ret;
                    }
                    impl<T> #inductive_ident<True> for T
                    where
                        T: super::#trait_ident + IsPrimitive<super::#property_ident, Is = True>,
                    {
                        type Property = super::#property_ident;
                        type Ret = #fn_ret;
                        fn #inner_fn(#mutref #receiver #fn_args) -> Self::Ret {
                            #dispatcher #fn_ident( #fn_arg_idents )
                        }
                    }

                    pub trait Nothing {
                        type Ret;
                        fn nothing() -> Self::Ret;
                    }
                    pub trait MergeField<L, R> {
                        type Ret;
                        fn merge_field(l: L, r: R #merge_args) -> Self::Ret;
                    }
                    pub trait MergeVariantField<L, R> {
                        type Ret;
                        fn merge_variant_field(l: L, r: R #merge_var_args) -> Self::Ret;
                    }
                    pub trait Join<F> {
                        type Ret;
                        fn join(fields: F #join_trait_args) -> Self::Ret;
                    }
                }

                impl<T> #trait_ident for T
                where
                    T: #inner_trait <::inception::False, Ret = #fn_ret>,
                {
                    fn #fn_ident(#mutref #receiver #fn_args) -> #fn_ret {
                        #dispatcher #inner_fn(#fn_arg_idents)
                    }
                }

                impl<T> #mod_ident :: Nothing for T {
                    type Ret = #nothing_ret;
                    fn nothing() -> Self::Ret {
                        #nothing_body
                    }
                }
                impl<#lifepunct1 #merge_head_ident, S, const IDX: usize, F, L, #merge_fields_ident> #mod_ident :: MergeField<L, #merge_fields_ident>
                    for #wrapper<#liferefelide List<(#field<#lifepunct1 #merge_head_ident, S, IDX>, F)>>
                where
                    S: FieldsMeta,
                    #merge_head_ident: #inner_trait #merge_field_head_bounds + ::inception::IsPrimitive<#property>,
                    F: Fields #phantom_bound,
                    L: Field #access_bound,
                    #merge_fields_ident: Fields + #inner_trait #merge_fields_bounds + ::inception::IsPrimitive<#property>,
                {
                    type Ret = #merge_ret;
                    fn merge_field(#mutability #merge_head_arg: L, #mutability #merge_fields_arg: #merge_fields_ident #merge_args) -> Self::Ret {
                        #merge_body
                    }
                }
                impl<#lifepunct1 #merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F, L, #merge_var_fields_ident> #mod_ident :: MergeVariantField<L, #merge_var_fields_ident>
                    for #wrapper<#liferefelide List<(#var_field<#lifepunct1 #merge_var_head_ident, S, VAR_IDX, IDX>, F)>>
                where
                    S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
                    #merge_var_head_ident: #inner_trait #merge_var_field_head_bounds + ::inception::IsPrimitive<#property>,
                    F: Fields #phantom_bound,
                    L: Field<Source = S> + VarField #try_access_bound,
                    #merge_var_fields_ident: Fields + #inner_trait #merge_var_fields_bounds + ::inception::IsPrimitive<#property>,
                {
                    type Ret = #merge_var_ret;
                    fn merge_variant_field(#mutability #merge_var_head_arg: L, #mutability #merge_var_fields_arg: #merge_var_fields_ident #merge_var_args) -> Self::Ret {
                        #merge_var_body
                    }
                }
                impl<T, #join_fields_ident> #mod_ident :: Join<#join_fields_ident> for T
                where
                    T: Inception<#property>,
                    #join_fields_ident: #inner_trait #join_fields_bounds + ::inception::IsPrimitive<#property>,
                {
                    type Ret = #join_ret;
                    fn join(#mutability #join_fields_arg: #join_fields_ident #join_trait_args) -> Self::Ret {
                        #join_body
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

                impl #inner_trait for #wrapper<#liferefelide List<()>> {
                    type Property = #property;
                    type Ret = #nothing_ret;
                    #[allow(unused)]
                    fn #inner_fn(#mutref #receiver #fn_args) -> Self::Ret {
                        <Self as #mod_ident :: Nothing>::nothing()
                    }
                }

                impl<#merge_head_ident, S, const IDX: usize, F> #inner_trait for #wrapper<#liferefelide List<(#field<#lifepunctelide #merge_head_ident, S, IDX>, F)>>
                where
                    S: FieldsMeta,
                    #merge_head_ident: #inner_trait #merge_field_head_bounds + ::inception::IsPrimitive<#property>,
                    F: Fields #phantom_bound,
                    <F as Fields>::Owned: Fields,
                    for<#lifepunct1 #life2> #wrapper<#liferef1 List<(#field<#lifepunct2 #merge_head_ident, S, IDX>, F)>>:
                        #split_trait_ident<#property, Left = #field<#lifepunct2 #merge_head_ident, S, IDX>>,
                    for<#lifepunct1 #lifepunct2 #life3> #wrapper<#liferef1 <#wrapper<#liferef2 List<(#field<#lifepunct3 #merge_head_ident, S, IDX>, F)>> as #split_trait_ident<#property>>::Right>:
                        #inner_trait #merge_fields_bounds + Fields,
                {
                    type Property = #property;
                    type Ret = #merge_ret;
                    fn #inner_fn(#mutref #receiver #fn_args) -> Self::Ret {
                        use #split_trait_ident;
                        let (#mutability l, #mutability r) = #dispatcher #split_fn_ident(#split_fn_receiver);
                        let #mutability r = #wrapper(r);
                        #merge_comparator_body
                        <Self as #mod_ident :: MergeField<_, _>>::merge_field(l, r #merge_arg_idents)
                    }
                }

                impl<#merge_var_head_ident, S, const VAR_IDX: usize, const IDX: usize, F> #inner_trait
                    for #wrapper<#liferefelide List<(#var_field<#lifepunctelide #merge_var_head_ident, S, VAR_IDX, IDX>, F)>>
                where
                    S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
                    #merge_var_head_ident: #inner_trait #merge_var_field_head_bounds + ::inception::IsPrimitive<#property>,
                    F: Fields #phantom_bound,
                    <F as Fields>::Owned: Fields,
                    for<#lifepunct1 #life2> #wrapper<#liferef1 List<(#var_field<#lifepunct2 #merge_var_head_ident, S, VAR_IDX, IDX>, F)>>:
                        #split_trait_ident<#property, Left = #var_field<#lifepunct2 #merge_var_head_ident, S, VAR_IDX, IDX>>,
                    for<#lifepunct1 #lifepunct2 #life3> #wrapper<#liferef1 <#wrapper<#liferef2 List<(#var_field<#lifepunct3 #merge_var_head_ident, S, VAR_IDX, IDX>, F)>> as #split_trait_ident<#property>>::Right>:
                        #inner_trait #merge_var_fields_bounds + Fields,
                {
                    type Property = #property;
                    type Ret = #merge_var_ret;
                    fn #inner_fn(#mutref #receiver #fn_args) -> Self::Ret {
                        use #split_trait_ident;
                        let (#mutability l, #mutability r) = #dispatcher #split_fn_ident(#split_fn_receiver);
                        let #mutability r = #wrapper(r);
                        #merge_var_comparator_body
                        <Self as #mod_ident :: MergeVariantField<_, _>>::merge_variant_field(l, r #merge_var_arg_idents)
                    }
                }

                impl<T> #inner_trait<False> for T
                where
                    T: Inception<#property> + Meta,
                    for<#lifepunct1 #life2> #wrapper<#liferef1 <T as Inception<#property>>::#fields_ident #bracketlife2>: #inner_trait #join_fields_bounds,
                {
                    type Property = #property;
                    type Ret = #join_ret;
                    fn #inner_fn(#mutref #receiver #join_args) -> Self::Ret {
                        use #mod_ident :: Join;
                        #fields_fn
                        let f = #wrapper(#mutref fields);
                        #join_comparator_body
                        Self::join(f #join_arg_idents)
                    }
                }
            }
            .into()
    }
}
