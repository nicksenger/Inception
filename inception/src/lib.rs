#![no_std]

pub use field::{Field, Fields};
pub use inception_macros::{inception, primitive, Inception};
pub use meta::{
    DataType, EnumMeta, EnumTy, FieldsMeta, IsPrimitive, Meta, NamedFieldsMeta, StructMeta,
    StructTy, UnnamedFieldsMeta, VariantOffset,
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

pub trait Property {}
pub trait OptIn<T: DataType> {}

pub trait Inception<X: Property, P: TruthValue = <Self as IsPrimitive<X>>::Is>:
    IsPrimitive<X> + DataType
{
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
    fn fields_mut<'a: 'b, 'b>(
        &'a mut self,
        variant_header: &'b mut VariantHeader,
    ) -> Self::MutFields<'b>;
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
