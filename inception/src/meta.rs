use core::marker::PhantomData;

use crate::{field::VarField, False, Field, Fields, Property, True, TruthValue};

pub trait DataType {
    const NAME: &str;
    type Ty;
}
pub struct InternalTy;
pub struct EnumTy;
pub struct StructTy<N>(PhantomData<N>);

impl<T> DataType for T
where
    T: Fields,
{
    const NAME: &str = "Fields";
    type Ty = InternalTy;
}

pub trait EnumMeta: DataType {
    const VARIANT_NAMES: &[&str];
    const FIELD_NAMES: &[&[&str]];
}
pub trait Meta<K = <Self as DataType>::Ty>: DataType {
    fn metadata() -> Metadata;
}
impl<T> Meta<InternalTy> for T
where
    T: Fields + DataType,
{
    fn metadata() -> Metadata {
        Metadata::Internal
    }
}
impl<T> Meta<EnumTy> for T
where
    T: EnumMeta + DataType,
{
    fn metadata() -> Metadata {
        Metadata::Enum {
            name: <T as DataType>::NAME,
            variant_names: <T as EnumMeta>::VARIANT_NAMES,
            field_names: <T as EnumMeta>::FIELD_NAMES,
        }
    }
}
pub trait StructMeta: DataType {
    const NUM_FIELDS: usize;
    type NamedFields: TruthValue;
}
impl<T> Meta<StructTy<True>> for T
where
    T: StructMeta<NamedFields = True> + NamedFieldsMeta + DataType,
{
    fn metadata() -> Metadata {
        Metadata::Struct {
            name: <T as DataType>::NAME,
            field_names: <T as NamedFieldsMeta>::FIELD_NAMES,
        }
    }
}
impl<T> Meta<StructTy<False>> for T
where
    T: StructMeta<NamedFields = False> + UnnamedFieldsMeta + DataType,
{
    fn metadata() -> Metadata {
        Metadata::Struct {
            name: <T as DataType>::NAME,
            field_names: &[],
        }
    }
}

pub trait NamedFieldsMeta<K = <Self as DataType>::Ty>: DataType {
    const FIELD_NAMES: &[&str];
}
pub trait UnnamedFieldsMeta<K = <Self as DataType>::Ty>: DataType {
    const NUM_FIELDS: usize;
}

pub trait FieldsMeta<K = <Self as DataType>::Ty>: DataType {
    type Named: TruthValue;
}
impl<T> FieldsMeta<StructTy<True>> for T
where
    T: StructMeta + DataType<Ty = StructTy<True>>,
{
    type Named = True;
}
impl<T> FieldsMeta<StructTy<False>> for T
where
    T: StructMeta + DataType<Ty = StructTy<False>>,
{
    type Named = False;
}
impl<T> FieldsMeta<EnumTy> for T
where
    T: EnumMeta,
{
    type Named = False;
}
impl<T> FieldsMeta<InternalTy> for T
where
    T: Fields,
{
    type Named = True;
}

#[derive(Debug)]
pub enum Metadata {
    Enum {
        name: &'static str,
        variant_names: &'static [&'static str],
        field_names: &'static [&'static [&'static str]],
    },
    Struct {
        name: &'static str,
        field_names: &'static [&'static str],
    },
    Internal,
}

pub enum FieldsMetadata {
    Named { names: &'static [&'static str] },
    Unnamed { len: usize },
}

pub trait IsPrimitive<X: Property> {
    type Is: TruthValue;
}

pub trait VariantOffset<const N: usize> {
    type Padding;
    const PADDING: Self::Padding;
}

pub trait VariantMeta {
    const VARIANT_NAME: &str;
    const VARIANT_FIELD_NAMES: &[&str];
}
impl<T> VariantMeta for T
where
    T: VarField,
    <T as Field>::Source: EnumMeta,
{
    const VARIANT_NAME: &str =
        <<T as Field>::Source as EnumMeta>::VARIANT_NAMES[<T as VarField>::VAR_IDX];
    const VARIANT_FIELD_NAMES: &[&str] =
        <<T as Field>::Source as EnumMeta>::FIELD_NAMES[<T as VarField>::VAR_IDX];
}
