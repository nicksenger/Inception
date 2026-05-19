#![no_std]

pub use field::{Field, Fields};
pub use inception_macros::{inception, inception_derive, primitive, Inception};
pub use meta::{
    DataType, DerivedDataType, DerivedMetaAdapter, EnumMeta, EnumTy, FieldsMeta, IsPrimitive, Meta,
    NamedFieldsMeta, StructMeta, StructTy, UnnamedFieldsMeta, VariantOffset,
};
pub use ty::{
    Compat, False, IntoTuples, List, Mask, Pad, Pad0, Pad1, Pad2, Pad3, Pad4, Pad5, Pad6, Pad7,
    Pad8, SplitOff, SplitOffInfix, True, TruthValue, PAD_0, PAD_1, PAD_2, PAD_3, PAD_4, PAD_5,
    PAD_6, PAD_7, PAD_8,
};

pub mod field;
pub mod meta;
pub mod ty;

pub use field::{
    Access, Empty, MutEnumAccessError, MutField, OwnedEnumAccessError, OwnedField, Phantom,
    RefEnumAccessError, RefField, TryAccess, TyField, VarField, VarMutField, VarOwnedField,
    VarRefField, VarTyField, VariantHeader,
};
pub use ty::Nothing;

#[macro_export]
macro_rules! inception_field_aliases {
    () => {
        type RefFields<'__inception_ref>
            = <Self::TyFields as $crate::Fields>::Referenced<'__inception_ref>
        where
            Self: '__inception_ref;
        type MutFields<'__inception_mut>
            = <Self::TyFields as $crate::Fields>::MutablyReferenced<'__inception_mut>
        where
            Self: '__inception_mut;
        type OwnedFields = <Self::TyFields as $crate::Fields>::Owned;
    };
}

pub trait Property {}
pub trait OptIn<T: DataType> {}

#[doc(hidden)]
pub trait DerivedOptInList: DataType {
    type OptIns;
}

#[doc(hidden)]
pub trait HasOptIn<P: Property, T: DataType> {}

#[cfg(feature = "opt-in")]
impl<P, T> OptIn<T> for P
where
    P: Property,
    T: DataType,
    (): HasOptIn<P, T>,
{
}

#[cfg(not(feature = "opt-in"))]
impl<P, T> OptIn<T> for P
where
    P: Property,
    T: DataType,
{
}

#[macro_export]
macro_rules! inception_opt_in_declare {
    (impl [] $ty:ty where [] : [$($prop:path),* $(,)?]) => {
        impl $crate::DerivedOptInList for $ty {
            type OptIns = $crate::list_ty![$($prop),*];
        }
    };
    (impl [] $ty:ty where [$($where_toks:tt)*] : [$($prop:path),* $(,)?]) => {
        impl $crate::DerivedOptInList for $ty
        where
            $($where_toks)*
        {
            type OptIns = $crate::list_ty![$($prop),*];
        }
    };
    (impl [$($impl_g:tt)*] $ty:ty where [] : [$($prop:path),* $(,)?]) => {
        impl<$($impl_g)*> $crate::DerivedOptInList for $ty {
            type OptIns = $crate::list_ty![$($prop),*];
        }
    };
    (impl [$($impl_g:tt)*] $ty:ty where [$($where_toks:tt)*] : [$($prop:path),* $(,)?]) => {
        impl<$($impl_g)*> $crate::DerivedOptInList for $ty
        where
            $($where_toks)*
        {
            type OptIns = $crate::list_ty![$($prop),*];
        }
    };
}

#[macro_export]
macro_rules! inception_opt_in_register {
    (impl [$($impl_g:tt)*] $ty:ty where [] : [$($prop:path),* $(,)?]) => {
        const _: () = {
            macro_rules! __inception_register_optin_for_type {
                ($prop_ty:path) => {
                    impl<$($impl_g)*> $crate::HasOptIn<$prop_ty, $ty> for () {}
                };
            }
            $(__inception_register_optin_for_type!($prop);)*
        };
    };
    (impl [$($impl_g:tt)*] $ty:ty where [$($where_toks:tt)*] : [$($prop:path),* $(,)?]) => {
        const _: () = {
            macro_rules! __inception_register_optin_for_type {
                ($prop_ty:path) => {
                    impl<$($impl_g)*> $crate::HasOptIn<$prop_ty, $ty> for ()
                    where
                        $($where_toks)*
                    {}
                };
            }
            $(__inception_register_optin_for_type!($prop);)*
        };
    };
}

pub trait Inception<X: Property, P: TruthValue = False>: DataType {
    type TyFields: field::Fields + field::Phantom;
    type RefFields<'a>: field::Fields
    where
        Self: 'a;
    type MutFields<'a>: field::Fields
    where
        Self: 'a;
    type OwnedFields: field::Fields;

    fn ty_fields() -> Self::TyFields {
        Self::TyFields::phantom()
    }
    fn fields(&self) -> Self::RefFields<'_>;
    fn fields_mut<'a>(&'a mut self, variant_header: &mut VariantHeader) -> Self::MutFields<'a>;
    fn into_fields(self) -> Self::OwnedFields;
    fn from_fields(fields: Self::OwnedFields) -> Self;
}

pub trait Split<X: Property> {
    type Left;
    type Right;

    fn split(self) -> (Self::Left, Self::Right);
}
pub trait SplitTy<X: Property> {
    type Left;
    type Right;

    fn split_ty(self) -> (Self::Left, Self::Right);
}
pub trait SplitRef<X: Property> {
    type Left;
    type Right;

    fn split_ref(&self) -> (Self::Left, &Self::Right);
}
pub trait SplitMut<X: Property> {
    type Left;
    type Right;

    fn split_mut(&mut self) -> (Self::Left, &mut Self::Right);
}

pub trait Wrapper {
    type Content;
    fn wrap(t: Self::Content) -> Self;
}
impl<T> Phantom for T
where
    T: Wrapper,
    <T as Wrapper>::Content: Phantom,
{
    fn phantom() -> Self {
        <T as Wrapper>::wrap(<<T as Wrapper>::Content as Phantom>::phantom())
    }
}
