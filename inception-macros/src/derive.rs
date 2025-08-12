use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_quote, Data, DataEnum, DeriveInput, GenericParam, Ident, Type};

enum Outcome<T> {
    #[allow(unused)]
    Skip,
    Process(T),
}

#[derive(deluxe::ParseMetaItem, deluxe::ExtractAttributes)]
#[deluxe(attributes(inception))]
struct Attributes {}

pub enum State {
    Enum(EnumState),
    Struct(StructState),
}

impl State {
    pub fn gen(input: TokenStream) -> TokenStream {
        let mut input: DeriveInput = parse_macro_input!(input);
        let Attributes { .. } = match deluxe::extract_attributes(&mut input) {
            Ok(desc) => desc,
            Err(e) => return e.into_compile_error().into(),
        };

        let mut transform_generics = input.generics.clone();
        transform_generics.params.push(GenericParam::Type(
            parse_quote! { X: ::inception::Property },
        ));
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
        let (transform_generics, _, _) = transform_generics.split_for_impl();

        let state = match State::try_from_data(&mut input.data, &input.ident) {
            Ok(Outcome::Process(st)) => st,
            Ok(Outcome::Skip) => {
                return quote! {}.into();
            }
            Err(tt) => {
                return tt;
            }
        };

        match state {
            State::Struct(state) => {
                let field_names = state
                    .field_identifiers
                    .names()
                    .into_iter()
                    .map(|n| proc_macro2::Literal::string(n.to_string().as_str()));
                let is_named = state.field_identifiers.is_named();
                let ty_fields = state.field_tokens(Kind::Ty);
                let ref_fields = state.field_tokens(Kind::Ref);
                let mut_fields = state.field_tokens(Kind::Mut);
                let owned_fields = state.field_tokens(Kind::Owned);
                let fields_impl = state.field_impl(Kind::Ref);
                let fields_mut_impl = state.field_impl(Kind::Mut);
                let into_fields_impl = state.field_impl(Kind::Owned);
                let from_fields_impl = state.impl_from_fields();
                let StructState { name, .. } = state;

                let num_fields =
                    proc_macro2::Literal::usize_unsuffixed(state.field_identifiers.size());
                let fields_meta = if is_named {
                    quote! {
                        impl #impl_generics ::inception::NamedFieldsMeta for #name #ty_generics #where_clause {
                            const FIELD_NAMES: &[&str] = &[#(#field_names),*];
                        }
                    }
                } else {
                    quote! {
                        impl #impl_generics ::inception::UnnamedFieldsMeta for #name #ty_generics #where_clause {
                            const NUM_FIELDS: usize = #num_fields;
                        }
                    }
                };

                let is_named = if is_named {
                    quote! { ::inception::True }
                } else {
                    quote! { ::inception::False }
                };

                quote! {
                    impl #impl_generics ::inception::DataType for #name #ty_generics #where_clause {
                        const NAME: &str = stringify!(#name);
                        type Ty = ::inception::StructTy<#is_named>;
                    }
                    impl #impl_generics ::inception::StructMeta for #name #ty_generics #where_clause {
                            const NUM_FIELDS: usize = #num_fields;
                            type NamedFields = #is_named;
                    }
                    #fields_meta
                    impl #transform_generics ::inception::IsPrimitive<X> for #name #ty_generics #where_clause {
                        type Is = ::inception::False;
                    }
                    impl #transform_generics ::inception::Inception<X, ::inception::False> for #name #ty_generics #where_clause {
                        #ty_fields
                        #ref_fields
                        #mut_fields
                        #owned_fields
                        #fields_impl
                        #fields_mut_impl
                        #into_fields_impl
                        #from_fields_impl
                    }
                }
                .into()
            }

            State::Enum(state) => {
                let ty_fields = state.field_tokens(Kind::Ty);
                let ref_fields = state.field_tokens(Kind::Ref);
                let mut_fields = state.field_tokens(Kind::Mut);
                let owned_fields = state.field_tokens(Kind::Owned);
                let fields_impl = state.field_impl(Kind::Ref);
                let fields_mut_impl = state.field_impl(Kind::Mut);
                let into_fields_impl = state.field_impl(Kind::Owned);
                let from_fields_impl = state.impl_from_fields();
                let EnumState {
                    name,
                    variant_identifiers,
                    ..
                } = state;
                let variant_names = variant_identifiers
                    .iter()
                    .map(|id| proc_macro2::Literal::string(id.to_string().as_str()))
                    .collect::<Vec<_>>();

                let var_field_names = state
                    .field_identifiers
                    .iter()
                    .map(|ids| {
                        ids.names()
                            .into_iter()
                            .map(|n| proc_macro2::Literal::string(n.to_string().as_str()))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();

                let variant_parens = state
                    .field_identifiers
                    .iter()
                    .scan(0, |st, ids| {
                        let res = (0..*st).map(|_| quote! { () });
                        *st += ids.0.len() + 1;
                        Some(res)
                    })
                    .collect::<Vec<_>>();
                let padding = variant_parens.into_iter().enumerate().map(|(i, parens)| {
                    let parens = parens.collect::<Vec<_>>();
                    let (pad, ty) = if parens.len() > 8 {
                        (quote! { ::inception::list![#(#parens),*] }, quote! { ::inception::list_ty![#(#parens),*] })
                    } else {
                        let n = format_ident!("PAD_{}", parens.len());
                        let m = format_ident!("Pad{}", parens.len());
                        (quote! { ::inception::#n }, quote! { ::inception::#m })
                    };
                    let n = proc_macro2::Literal::usize_unsuffixed(i);
                    quote! {
                        impl #impl_generics ::inception::VariantOffset<#n> for #name #ty_generics #where_clause {
                            const PADDING: Self::Padding = #pad;
                            type Padding = #ty;
                        }
                    }
                });

                quote! {
                    impl #impl_generics ::inception::DataType for #name #ty_generics #where_clause {
                        const NAME: &str = stringify!(#name);
                        type Ty = ::inception::EnumTy;
                    }
                    impl #impl_generics ::inception::EnumMeta for #name #ty_generics #where_clause {
                        const VARIANT_NAMES: &[&str] = &[#(#variant_names),*];
                        const FIELD_NAMES: &[&[&str]] = &[#(&[#(#var_field_names),*]),*];
                    }
                    #(#padding)*
                    impl #transform_generics ::inception::IsPrimitive<X> for #name #ty_generics #where_clause {
                        type Is = ::inception::False;
                    }
                    impl #transform_generics ::inception::Inception<X, ::inception::False> for #name #ty_generics #where_clause {
                        #ty_fields
                        #ref_fields
                        #mut_fields
                        #owned_fields
                        #fields_impl
                        #fields_mut_impl
                        #into_fields_impl
                        #from_fields_impl
                    }
                }
                .into()
            }
        }
    }
}

pub struct EnumState {
    name: Ident,
    mod_label: Ident,
    variant_identifiers: Vec<Ident>,
    field_identifiers: Vec<Identifiers>,
    field_tys: Vec<Vec<Type>>,
}

enum Kind {
    Ty,
    Ref,
    Mut,
    Owned,
}

impl EnumState {
    fn field_tokens(&self, kind: Kind) -> proc_macro2::TokenStream {
        let fields = self.field_tys.iter().enumerate().map(|(i, tys)| {
            let var_idx = proc_macro2::Literal::usize_unsuffixed(i);
            let ixs = (0..tys.len()).map(proc_macro2::Literal::usize_unsuffixed);
            quote! {
                [#var_idx, [#(#ixs, #tys),*]]
            }
        });

        match kind {
            Kind::Ty => quote! {
                type TyFields = ::inception::enum_field_tys![#(#fields),*];
            },
            Kind::Ref => quote! {
                type RefFields<'a> = <Self::TyFields as ::inception::Fields>::Referenced<'a>;
            },
            Kind::Mut => quote! {
                type MutFields<'a> = <Self::TyFields as ::inception::Fields>::MutablyReferenced<'a>;
            },
            Kind::Owned => quote! {
                type OwnedFields = <Self::TyFields as ::inception::Fields>::Owned;
            },
        }
    }

    fn field_impl(&self, kind: Kind) -> proc_macro2::TokenStream {
        let variants = self
            .field_tys
            .iter()
            .zip(&self.field_identifiers)
            .zip(&self.variant_identifiers)
            .map(|((tys, ids), var)| {
                let mut named = false;
                let fields = tys
                    .iter()
                    .zip(&ids.0)
                    .map(|(_ty, id)| match id {
                        Identifier::Unnamed(n) => {
                            let n = format_ident!("_{n}");
                            match kind {
                                Kind::Ty => quote! {
                                    VarTyField::new()
                                },
                                Kind::Ref => quote! {
                                    VarRefField::new(#n)
                                },
                                Kind::Mut => quote! {
                                    VarMutField::new(#n)
                                },
                                Kind::Owned => quote! {
                                    VarOwnedField::new(#n)
                                },
                            }
                        }

                        Identifier::Named(n) => {
                            named = true;
                            match kind {
                                Kind::Ty => quote! {
                                    VarTyField::new()
                                },
                                Kind::Ref => quote! {
                                    VarRefField::new(#n)
                                },
                                Kind::Mut => quote! {
                                    VarMutField::new(#n)
                                },
                                Kind::Owned => quote! {
                                    VarOwnedField::new(#n)
                                },
                            }
                        }
                    })
                    .collect::<Vec<_>>();

                let field_ids = ids
                    .0
                    .iter()
                    .map(|id| match id {
                        Identifier::Named(n) => n.clone(),
                        Identifier::Unnamed(n) => format_ident!("_{n}"),
                    })
                    .collect::<Vec<_>>();

                (var.clone(), field_ids, fields, named)
            })
            .collect::<Vec<_>>();

        let expanded_variants = (0..variants.len())
            .map(|i| {
                let (var, field_ids, fields, named) = &variants[i];
                let field_ids = field_ids.clone();

                let header = match kind {
                    Kind::Ty => {
                        quote! { VarTyField::header() }
                    }
                    Kind::Ref => {
                        quote! { VarRefField::header(&inception::VariantHeader) }
                    }
                    Kind::Mut => {
                        quote! { VarMutField::header(header) }
                    }
                    Kind::Owned => {
                        quote! { VarOwnedField::header(::inception::VariantHeader) }
                    }
                };
                let variant_fields = std::iter::once(header)
                    .chain(fields.clone())
                    .collect::<Vec<_>>();

                let i = proc_macro2::Literal::usize_unsuffixed(i);
                let toks = if *named {
                    quote! {
                        Self::#var {
                            #(#field_ids),*
                        } => fields.mask(::inception::list![
                            #(#variant_fields),*
                        ].pad(<Self as ::inception::VariantOffset<#i>>::PADDING)),
                    }
                } else {
                    quote! {
                        Self::#var(#(#field_ids),*) => fields.mask(::inception::list![
                            #(#variant_fields),*
                        ].pad(<Self as ::inception::VariantOffset<#i>>::PADDING)),
                    }
                };

                toks
            })
            .collect::<Vec<_>>();

        match kind {
            Kind::Ty => quote! {},

            Kind::Ref => quote! {
                fn fields(&self) -> Self::RefFields<'_> {
                    use ::inception::{Pad, Mask, Phantom, VarRefField, list};
                    let mut fields = Self::RefFields::phantom();
                    match self {
                        #(#expanded_variants)*
                    }
                }
            },

            Kind::Mut => quote! {
                fn fields_mut<'a: 'b, 'b>(&'a mut self, header: &'b mut ::inception::VariantHeader) -> Self::MutFields<'b> {
                    use ::inception::{Pad, Mask, Phantom, VarMutField, list};
                    let mut fields = Self::MutFields::phantom();
                    match self {
                        #(#expanded_variants)*
                    }
                }
            },

            Kind::Owned => quote! {
                fn into_fields(self) -> Self::OwnedFields {
                    use ::inception::{Pad, Mask, Phantom, VarOwnedField, list};
                    let mut fields = Self::OwnedFields::phantom();
                    match self {
                        #(#expanded_variants)*
                    }
                }
            },
        }
    }

    fn impl_from_fields(&self) -> proc_macro2::TokenStream {
        let (split, check): (Vec<_>, Vec<_>) = self
            .field_tys
            .iter()
            .zip(&self.field_identifiers)
            .zip(&self.variant_identifiers)
            .enumerate()
            .map(|(i, ((tys, ids), var))| {
                let idx = proc_macro2::Literal::usize_unsuffixed(i);
                let mut named = false;
                let fields = tys
                    .iter()
                    .zip(&ids.0)
                    .map(|(_ty, id)| match id {
                        Identifier::Unnamed(n) => {
                            let n = format_ident!("_{}", n);
                            quote! { #n }
                        }

                        Identifier::Named(n) => {
                            named = true;
                            quote! { #n }
                        }
                    })
                    .collect::<Vec<_>>();

                let split_list = if (fields.len() + 1) < 8 {
                    let n = format_ident!("PAD_{}", fields.len() + 1);
                    quote! { ::inception::#n }
                } else {
                    let split_parens = (0..fields.len() + 1).map(|_| quote! { () });
                    quote! { ::inception::list![#(#split_parens),*] }
                };
                quote! { <Self as ::inception::VariantOffset<#idx>>::PADDING };
                let destruct_parens = fields
                    .iter()
                    .rev()
                    .fold(quote! { _ }, |st, f| quote! { (#f, #st) });
                let split = quote! {
                    let (l, fields) = fields.split_off(#split_list);
                };
                let destructure = quote! {
                    let (header, #destruct_parens) = l.access().into_tuples();
                };
                (
                    quote! {
                        #split
                    },
                    if named {
                        quote! {
                            if l.0.0.has_value() {
                                #destructure
                                return Self :: #var {
                                    #(#fields),*
                                };
                            }
                        }
                    } else {
                        quote! {
                            if l.0.0.has_value() {
                                #destructure
                                return Self :: #var(#(#fields),*);
                            }
                        }
                    },
                )
            })
            .unzip();

        quote! {
            fn from_fields(fields: Self::OwnedFields) -> Self {
                use ::inception::{SplitOff, Access, IntoTuples};
                #(
                    #split
                    #check
                )*
                panic!("Failed to determine enum variant.");
            }
        }
    }
}

pub struct StructState {
    name: Ident,
    mod_label: Ident,
    field_identifiers: Identifiers,
    field_tys: Vec<Type>,
}

impl StructState {
    fn field_tokens(&self, kind: Kind) -> proc_macro2::TokenStream {
        let (ixs, tys): (Vec<_>, Vec<_>) = self
            .field_tys
            .iter()
            .enumerate()
            .map(|(i, x)| (proc_macro2::Literal::usize_unsuffixed(i), x))
            .unzip();

        match kind {
            Kind::Ty => quote! {
                type TyFields = ::inception::struct_field_tys![#(#ixs,#tys),*];
            },
            Kind::Ref => quote! {
                type RefFields<'a> = <Self::TyFields as ::inception::Fields>::Referenced<'a>;
            },
            Kind::Mut => quote! {
                type MutFields<'a> = <Self::TyFields as ::inception::Fields>::MutablyReferenced<'a>;
            },
            Kind::Owned => quote! {
                type OwnedFields = <Self::TyFields as ::inception::Fields>::Owned;
            },
        }
    }

    fn field_impl(&self, kind: Kind) -> proc_macro2::TokenStream {
        let fields =
            self.field_tys
                .iter()
                .zip(&self.field_identifiers.0)
                .map(|(_ty, id)| match id {
                    Identifier::Unnamed(n) => {
                        let idx = proc_macro2::Literal::usize_unsuffixed(*n);
                        match kind {
                            Kind::Ty => quote! {
                                ::inception::TyField::new()
                            },
                            Kind::Ref => quote! {
                                ::inception::RefField::new(&self.#idx)
                            },
                            Kind::Mut => quote! {
                                ::inception::MutField::new(&mut self.#idx)
                            },
                            Kind::Owned => quote! {
                                ::inception::OwnedField::new(self.#idx)
                            },
                        }
                    }

                    Identifier::Named(n) => match kind {
                        Kind::Ty => quote! {
                            ::inception::TyField::new()
                        },
                        Kind::Ref => quote! {
                            ::inception::RefField::new(&self.#n)
                        },
                        Kind::Mut => quote! {
                            ::inception::MutField::new(&mut self.#n)
                        },
                        Kind::Owned => quote! {
                            ::inception::OwnedField::new(self.#n)
                        },
                    },
                });

        match kind {
            Kind::Ty => quote! {
                fn ty_fields() -> Self::TyFields {
                    ::inception::list![#(#fields),*]
                }
            },

            Kind::Ref => quote! {
                fn fields(&self) -> Self::RefFields<'_> {
                    ::inception::list![#(#fields),*]
                }
            },

            Kind::Mut => quote! {
                fn fields_mut<'a: 'b, 'b>(&'a mut self, header: &'b mut ::inception::VariantHeader) -> Self::MutFields<'b> {
                    ::inception::list![#(#fields),*]
                }
            },

            Kind::Owned => quote! {
                fn into_fields(self) -> Self::OwnedFields {
                    ::inception::list![#(#fields),*]
                }
            },
        }
    }

    fn impl_from_fields(&self) -> proc_macro2::TokenStream {
        let mut named = false;
        let fields = self
            .field_tys
            .iter()
            .zip(&self.field_identifiers.0)
            .enumerate()
            .map(|(depth, (_ty, id))| match id {
                Identifier::Named(n) => {
                    named = true;
                    let path = (0..depth).map(|_| quote! { .0.1 });
                    quote! { #n: fields #(#path)* .0.0.access() }
                }
                Identifier::Unnamed(_) => {
                    let path = (0..depth).map(|_| quote! { .0.1 });
                    quote! { fields #(#path)* .0.0.access() }
                }
            })
            .collect::<Vec<_>>();

        if named {
            quote! {
                fn from_fields(fields: Self::OwnedFields) -> Self {
                    use ::inception::Access;
                    Self {
                        #(#fields),*
                    }
                }
            }
        } else {
            quote! {
                fn from_fields(fields: Self::OwnedFields) -> Self {
                    use ::inception::Access;
                    Self(#(#fields),*)
                }
            }
        }
    }
}

#[derive(Default)]
struct Identifiers(Vec<Identifier>);
impl Identifiers {
    fn names(&self) -> Vec<Ident> {
        self.0
            .iter()
            .filter_map(|id| match id {
                Identifier::Named(id) => Some(id.clone()),
                Identifier::Unnamed(_) => None,
            })
            .collect()
    }

    fn is_named(&self) -> bool {
        self.0
            .first()
            .map(|t| matches!(t, Identifier::Named(_)))
            .unwrap_or_default()
    }

    fn size(&self) -> usize {
        self.0.len()
    }
}

pub enum Identifier {
    Named(Ident),
    Unnamed(usize),
}

impl Identifier {
    pub fn modularize(ident: &Ident) -> Ident {
        format_ident!(
            "{}",
            ident
                .to_string()
                .chars()
                .enumerate()
                .flat_map(|(i, c)| if i > 0 && c == c.to_ascii_uppercase() {
                    ['_', c.to_ascii_lowercase()]
                } else {
                    [c, ' ']
                })
                .filter(|c| *c != ' ')
                .map(|c| c.to_ascii_lowercase())
                .collect::<String>()
        )
    }
}

impl State {
    fn new_struct(ident: &Ident) -> Self {
        Self::Struct(StructState {
            name: ident.clone(),
            mod_label: format_ident!("inception_struct_{}", Identifier::modularize(ident)),
            field_identifiers: Default::default(),
            field_tys: Default::default(),
        })
    }

    fn new_enum(ident: &Ident) -> Self {
        Self::Enum(EnumState {
            name: ident.clone(),
            mod_label: format_ident!("inception_enum_{}", Identifier::modularize(ident)),
            variant_identifiers: Default::default(),
            field_identifiers: Default::default(),
            field_tys: Default::default(),
        })
    }

    fn try_from_data(data: &mut syn::Data, ident: &Ident) -> Result<Outcome<Self>, TokenStream> {
        match data {
            Data::Enum(DataEnum { variants, .. }) => {
                let State::Enum(EnumState {
                    name,
                    mut variant_identifiers,
                    mut field_identifiers,
                    mut field_tys,
                    mod_label,
                }) = State::new_enum(ident)
                else {
                    return Err(syn::Error::new_spanned(variants, "Expected enum.")
                        .into_compile_error()
                        .into());
                };

                for v in variants {
                    variant_identifiers.push(v.ident.clone());
                    let (ids, tys): (Vec<_>, Vec<_>) = v
                        .fields
                        .iter()
                        .enumerate()
                        .map(|(i, f)| {
                            (
                                f.ident
                                    .clone()
                                    .map(Identifier::Named)
                                    .unwrap_or(Identifier::Unnamed(i)),
                                f.ty.clone(),
                            )
                        })
                        .unzip();

                    field_identifiers.push(Identifiers(ids));
                    field_tys.push(tys);
                }

                Ok(Outcome::Process(State::Enum(EnumState {
                    name,
                    mod_label,
                    variant_identifiers,
                    field_identifiers,
                    field_tys,
                })))
            }

            Data::Struct(x) => {
                let State::Struct(StructState {
                    mut field_tys,
                    mod_label,
                    name,
                    ..
                }) = State::new_struct(ident)
                else {
                    return Err(syn::Error::new_spanned(&x.fields, "Expected struct.")
                        .into_compile_error()
                        .into());
                };

                let (ids, tys): (Vec<_>, Vec<_>) = x
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        (
                            f.ident
                                .clone()
                                .map(Identifier::Named)
                                .unwrap_or(Identifier::Unnamed(i)),
                            f.ty.clone(),
                        )
                    })
                    .unzip();

                let field_identifiers = Identifiers(ids);
                field_tys.extend(tys);

                Ok(Outcome::Process(State::Struct(StructState {
                    field_identifiers,
                    field_tys,
                    mod_label,
                    name,
                })))
            }

            Data::Union(x) => Err(
                syn::Error::new_spanned(&x.fields, "Unions are not supported.")
                    .to_compile_error()
                    .into(),
            ),
        }
    }
}
