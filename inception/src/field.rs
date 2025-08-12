use core::{fmt::Display, marker::PhantomData};

use crate::{meta::FieldsMeta, List, VariantOffset};

#[macro_export]
macro_rules! struct_field_tys {
    [$($ixs:literal,$tys:ty),*$(,)?] => {
        $crate::list_ty![
            $($crate::TyField::<$tys, Self, $ixs>),*
        ]
    };
}
#[macro_export]
macro_rules! enum_field_tys {
    [
        $(
            [$vxs:literal, [
                $($ixs:literal, $tys:ty),*
            ]]
        ),*$(,)?
    ] => {
        $crate::list_ty![
            $(
                $crate::VarTyField::<$crate::VariantHeader, Self, $vxs, { 0 }>,
                $($crate::VarTyField::<$tys, Self, $vxs, $ixs>),*
            ),*
        ]
    }
}

pub const VARIANT_HEADER: VariantHeader = VariantHeader;
pub struct VariantHeader;

pub trait Access {
    type Out;
    fn access(self) -> Self::Out;
}
impl Access for List<()> {
    type Out = List<()>;
    fn access(self) -> Self::Out {
        self
    }
}
impl<T, U> Access for List<(T, U)>
where
    T: Access,
    U: Access,
{
    type Out = List<(<T as Access>::Out, <U as Access>::Out)>;
    fn access(self) -> Self::Out {
        List((self.0 .0.access(), self.0 .1.access()))
    }
}
impl<'a, T, S, const IDX: usize> Access for RefField<'a, T, S, IDX> {
    type Out = &'a T;
    fn access(self) -> Self::Out {
        self.0.unwrap()
    }
}
impl<'a, T, S, const IDX: usize> Access for MutField<'a, T, S, IDX> {
    type Out = &'a mut T;
    fn access(self) -> Self::Out {
        self.0.unwrap()
    }
}
impl<T, S, const IDX: usize> Access for OwnedField<T, S, IDX> {
    type Out = T;
    fn access(self) -> Self::Out {
        self.0.unwrap()
    }
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> Access
    for VarRefField<'a, T, S, VAR_IDX, IDX>
{
    type Out = &'a T;
    fn access(self) -> Self::Out {
        match self {
            Self::Ref(t) => t.access(),
            Self::Header(t) => t.access(),
            _ => unreachable!(
                "Attempted to access a variant which has no data. This is likely a bug."
            ),
        }
    }
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> Access
    for VarMutField<'a, T, S, VAR_IDX, IDX>
{
    type Out = &'a mut T;
    fn access(self) -> Self::Out {
        match self {
            Self::Mut(t) => t.access(),
            Self::Header(t) => t.access(),
            _ => unreachable!(
                "Attempted to access a variant which has no data. This is likely a bug."
            ),
        }
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Access for VarOwnedField<T, S, VAR_IDX, IDX> {
    type Out = T;
    fn access(self) -> Self::Out {
        match self {
            Self::Owned(t) => t.access(),
            Self::Header(t) => t.access(),
            _ => unreachable!(
                "Attempted to access a variant which has no data. This is likely a bug."
            ),
        }
    }
}

pub trait TryAccess {
    type Err;
    type Out;
    fn try_access(self) -> Result<Self::Out, Self::Err>;
}
pub enum RefEnumAccessError<'a, T, S, const VAR_IDX: usize, const IDX: usize> {
    EmptyField(VarTyField<T, S, VAR_IDX, IDX>),
    Header(RefField<'a, T, S, IDX>),
}
pub enum MutEnumAccessError<'a, T, S, const VAR_IDX: usize, const IDX: usize> {
    EmptyField(VarTyField<T, S, VAR_IDX, IDX>),
    Header(MutField<'a, T, S, IDX>),
}
pub enum OwnedEnumAccessError<T, S, const VAR_IDX: usize, const IDX: usize> {
    EmptyField(VarTyField<T, S, VAR_IDX, IDX>),
    Header(OwnedField<T, S, IDX>),
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> TryAccess
    for VarRefField<'a, T, S, VAR_IDX, IDX>
{
    type Err = RefEnumAccessError<'a, T, S, VAR_IDX, IDX>;
    type Out = &'a T;
    fn try_access(self) -> Result<Self::Out, Self::Err> {
        match self {
            Self::Ref(r) => Ok(r.access()),
            Self::Header(RefField(Some(d), _)) => Err(RefEnumAccessError::Header(RefField::new(d))),
            Self::Empty(_) | Self::Header(RefField(None, _)) => {
                Err(RefEnumAccessError::EmptyField(VarTyField::new()))
            }
        }
    }
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> TryAccess
    for VarMutField<'a, T, S, VAR_IDX, IDX>
{
    type Err = MutEnumAccessError<'a, T, S, VAR_IDX, IDX>;
    type Out = &'a mut T;
    fn try_access(self) -> Result<Self::Out, Self::Err> {
        match self {
            Self::Mut(r) => Ok(r.access()),
            Self::Header(MutField(Some(d), _)) => Err(MutEnumAccessError::Header(MutField::new(d))),
            Self::Empty(_) | Self::Header(MutField(None, _)) => {
                Err(MutEnumAccessError::EmptyField(VarTyField::new()))
            }
        }
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> TryAccess for VarOwnedField<T, S, VAR_IDX, IDX> {
    type Err = OwnedEnumAccessError<T, S, VAR_IDX, IDX>;
    type Out = T;
    fn try_access(self) -> Result<Self::Out, Self::Err> {
        match self {
            Self::Owned(r) => Ok(r.access()),
            Self::Header(OwnedField(Some(d), _)) => {
                Err(OwnedEnumAccessError::Header(OwnedField::new(d)))
            }
            Self::Empty(_) | Self::Header(OwnedField(None, _)) => {
                Err(OwnedEnumAccessError::EmptyField(VarTyField::new()))
            }
        }
    }
}

pub trait Phantom: Sized {
    fn phantom() -> Self;
    fn copy(&self) -> Self {
        Self::phantom()
    }
}
impl Phantom for () {
    fn phantom() -> Self {}
}
impl Phantom for VariantHeader {
    fn phantom() -> Self {
        Self
    }
}
impl<T, S, const IDX: usize> Phantom for TyField<T, S, IDX> {
    fn phantom() -> Self {
        Self::new()
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Phantom for VarTyField<T, S, VAR_IDX, IDX> {
    fn phantom() -> Self {
        Self::new()
    }
}
impl<T, S, const IDX: usize> Phantom for RefField<'_, T, S, IDX> {
    fn phantom() -> Self {
        Self::empty()
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Phantom for VarRefField<'_, T, S, VAR_IDX, IDX> {
    fn phantom() -> Self {
        Self::empty()
    }
}
impl<T, S, const IDX: usize> Phantom for MutField<'_, T, S, IDX> {
    fn phantom() -> Self {
        Self::empty()
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Phantom for VarMutField<'_, T, S, VAR_IDX, IDX> {
    fn phantom() -> Self {
        Self::empty()
    }
}
impl<T, S, const IDX: usize> Phantom for OwnedField<T, S, IDX> {
    fn phantom() -> Self {
        Self::empty()
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Phantom for VarOwnedField<T, S, VAR_IDX, IDX> {
    fn phantom() -> Self {
        Self::empty()
    }
}
impl<T, U> Phantom for List<(T, U)>
where
    T: Phantom,
    U: Phantom,
{
    fn phantom() -> Self {
        List((<T as Phantom>::phantom(), <U as Phantom>::phantom()))
    }
}
impl Phantom for List<()> {
    fn phantom() -> Self {
        List(())
    }
}

pub struct Empty<T, S, const VAR_IDX: usize, const IDX: usize>(PhantomData<T>, PhantomData<S>);
impl<T, S, const VAR_IDX: usize, const IDX: usize> Default for Empty<T, S, VAR_IDX, IDX> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Empty<T, S, VAR_IDX, IDX> {
    pub fn new() -> Self {
        Self(PhantomData, PhantomData)
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Field for Empty<T, S, VAR_IDX, IDX> {
    const IDX: usize = IDX;
    type Content = T;
    type Source = S;
    type Referenced<'a>
        = Empty<T, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = Empty<T, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type Owned = Empty<T, S, VAR_IDX, IDX>;
}

pub trait Fields {
    type Head: Field;
    type Tail: Fields;
    type Referenced<'a>
    where
        Self::Head: 'a,
        Self::Tail: 'a;
    type MutablyReferenced<'a>
    where
        Self::Head: 'a,
        Self::Tail: 'a;
    type Owned;
}
impl<T, U> Fields for List<(T, U)>
where
    T: Field,
    U: Fields,
{
    type Head = T;
    type Tail = U;
    type Referenced<'a>
        = List<(<T as Field>::Referenced<'a>, <U as Fields>::Referenced<'a>)>
    where
        Self::Head: 'a,
        Self::Tail: 'a;
    type MutablyReferenced<'a>
        = List<(
        <T as Field>::MutablyReferenced<'a>,
        <U as Fields>::MutablyReferenced<'a>,
    )>
    where
        Self::Head: 'a,
        Self::Tail: 'a;
    type Owned = List<(<T as Field>::Owned, <U as Fields>::Owned)>;
}
impl Fields for List<()> {
    type Head = Empty<(), (), { usize::MAX }, { usize::MAX }>;
    type Tail = List<()>;
    type Referenced<'a>
        = Self
    where
        Self::Head: 'a,
        Self::Tail: 'a;
    type MutablyReferenced<'a>
        = Self
    where
        Self::Head: 'a,
        Self::Tail: 'a;
    type Owned = Self;
}

pub trait Kind {
    fn identifier() -> impl Display;
}

pub trait Field {
    const IDX: usize;
    type Source;
    type Content;
    type Referenced<'a>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
    where
        Self::Content: 'a;
    type Owned;
}

pub trait VarField: Field {
    const VAR_IDX: usize;
    const FIELD_IDX: usize;
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> VarField for VarOwnedField<T, S, VAR_IDX, IDX>
where
    S: VariantOffset<VAR_IDX> + FieldsMeta,
{
    const VAR_IDX: usize = VAR_IDX;
    const FIELD_IDX: usize = <Self as Field>::IDX;
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> VarField
    for VarMutField<'a, T, S, VAR_IDX, IDX>
where
    S: VariantOffset<VAR_IDX> + FieldsMeta,
{
    const VAR_IDX: usize = VAR_IDX;
    const FIELD_IDX: usize = <Self as Field>::IDX;
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> VarField
    for VarRefField<'a, T, S, VAR_IDX, IDX>
where
    S: VariantOffset<VAR_IDX> + FieldsMeta,
{
    const VAR_IDX: usize = VAR_IDX;
    const FIELD_IDX: usize = <Self as Field>::IDX;
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> VarField for VarTyField<T, S, VAR_IDX, IDX>
where
    S: VariantOffset<VAR_IDX> + FieldsMeta,
{
    const VAR_IDX: usize = VAR_IDX;
    const FIELD_IDX: usize = <Self as Field>::IDX;
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> VarField for Empty<T, S, VAR_IDX, IDX>
where
    S: VariantOffset<VAR_IDX>,
{
    const VAR_IDX: usize = VAR_IDX;
    const FIELD_IDX: usize = <Self as Field>::IDX;
}

#[derive(Default)]
pub struct TyField<T, S, const IDX: usize>(PhantomData<T>, PhantomData<S>);
impl<T, S, const IDX: usize> TyField<T, S, IDX> {
    pub fn new() -> Self {
        Self(PhantomData, PhantomData)
    }

    pub fn empty() -> Self {
        Self(PhantomData, PhantomData)
    }

    pub fn phantom(self) -> PhantomData<T> {
        self.0
    }
}

pub enum VarTyField<T, S, const VAR_IDX: usize, const IDX: usize> {
    Header(TyField<T, S, IDX>),
    Ty(TyField<T, S, IDX>),
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> Default for VarTyField<T, S, VAR_IDX, IDX> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> VarTyField<T, S, VAR_IDX, IDX> {
    pub fn new() -> Self {
        Self::Ty(TyField::new())
    }

    pub fn header() -> Self {
        Self::Header(TyField::new())
    }

    pub fn empty() -> Self {
        Self::Ty(TyField::new())
    }

    pub fn has_value(&self) -> bool {
        false
    }
}

pub enum VarRefField<'a, T, S, const VAR_IDX: usize, const IDX: usize> {
    Ref(RefField<'a, T, S, IDX>),
    Header(RefField<'a, T, S, IDX>),
    Empty(Empty<T, S, VAR_IDX, IDX>),
}
impl<'a, S, const VAR_IDX: usize, const IDX: usize>
    VarRefField<'a, VariantHeader, S, VAR_IDX, IDX>
{
    pub fn header(header: &'a VariantHeader) -> Self {
        Self::Header(RefField::new(header))
    }
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> VarRefField<'a, T, S, VAR_IDX, IDX> {
    pub fn new(t: &'a T) -> Self {
        Self::Ref(RefField::new(t))
    }

    pub fn empty() -> Self {
        Self::Empty(Empty::new())
    }

    pub fn has_value(&self) -> bool {
        matches!(self, Self::Ref(_) | Self::Header(RefField(Some(_), _)))
    }
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> Clone for VarRefField<'a, T, S, VAR_IDX, IDX>
where
    RefField<'a, T, S, IDX>: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Ref(t) => Self::Ref(t.clone()),
            Self::Header(d) => Self::Header(d.clone()),
            _ => Self::Empty(Empty::new()),
        }
    }
}

pub enum VarMutField<'a, T, S, const VAR_IDX: usize, const IDX: usize> {
    Mut(MutField<'a, T, S, IDX>),
    Header(MutField<'a, T, S, IDX>),
    Empty(Empty<PhantomData<T>, S, VAR_IDX, IDX>),
}
impl<'a, S, const VAR_IDX: usize, const IDX: usize>
    VarMutField<'a, VariantHeader, S, VAR_IDX, IDX>
{
    pub fn header(header: &'a mut VariantHeader) -> Self {
        Self::Header(MutField::new(header))
    }
}
impl<'a, T, S, const VAR_IDX: usize, const IDX: usize> VarMutField<'a, T, S, VAR_IDX, IDX> {
    pub fn new(t: &'a mut T) -> Self {
        Self::Mut(MutField::new(t))
    }

    pub fn empty() -> Self {
        Self::Empty(Empty::new())
    }

    pub fn has_value(&self) -> bool {
        matches!(self, Self::Mut(_) | Self::Header(MutField(Some(_), _)))
    }

    pub fn take(&mut self) -> Self {
        match self {
            Self::Mut(m) => Self::Mut(m.take()),
            Self::Header(m) => Self::Header(m.take()),
            _ => Self::Empty(Empty::new()),
        }
    }
}

pub enum VarOwnedField<T, S, const VAR_IDX: usize, const IDX: usize> {
    Owned(OwnedField<T, S, IDX>),
    Header(OwnedField<T, S, IDX>),
    Empty(Empty<PhantomData<T>, S, VAR_IDX, IDX>),
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> VarOwnedField<T, S, VAR_IDX, IDX> {
    pub fn new(t: T) -> Self {
        Self::Owned(OwnedField::new(t))
    }

    pub fn empty() -> Self {
        Self::Empty(Empty::new())
    }

    pub fn header(d: T) -> Self {
        Self::Header(OwnedField::new(d))
    }

    pub fn has_value(&self) -> bool {
        matches!(self, Self::Owned(_) | Self::Header(OwnedField(Some(_), _)))
    }
}
impl<S, const VAR_IDX: usize, const IDX: usize> From<VariantHeader>
    for VarOwnedField<VariantHeader, S, VAR_IDX, IDX>
{
    fn from(d: VariantHeader) -> Self {
        VarOwnedField::new(d)
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize> From<List<PhantomData<T>>>
    for VarOwnedField<T, S, VAR_IDX, IDX>
{
    fn from(_: List<PhantomData<T>>) -> Self {
        Self::Empty(Empty::new())
    }
}

pub struct RefField<'a, T, S, const IDX: usize>(pub Option<&'a T>, PhantomData<S>);
impl<'a, T, S, const IDX: usize> RefField<'a, T, S, IDX> {
    pub fn new(t: &'a T) -> Self {
        Self(Some(t), PhantomData)
    }

    pub fn empty() -> Self {
        Self(None, PhantomData)
    }
}
impl<'a, T: Clone, S, const IDX: usize> RefField<'a, T, S, IDX> {
    pub fn to_owned(&self) -> OwnedField<T, S, IDX> {
        OwnedField(self.0.cloned(), PhantomData)
    }
}
impl<'a, T, S, const IDX: usize> Clone for RefField<'a, T, S, IDX> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }
}

pub struct MutField<'a, T, S, const IDX: usize>(pub Option<&'a mut T>, PhantomData<S>);
impl<'a, T, S, const IDX: usize> MutField<'a, T, S, IDX> {
    pub fn new(t: &'a mut T) -> Self {
        Self(Some(t), PhantomData)
    }

    pub fn empty() -> Self {
        Self(None, PhantomData)
    }

    pub fn take(&mut self) -> Self {
        Self(self.0.take(), PhantomData)
    }
}

pub struct OwnedField<T, S, const IDX: usize>(pub Option<T>, PhantomData<S>);
impl<T, S, const IDX: usize> OwnedField<T, S, IDX> {
    pub fn new(t: T) -> Self {
        Self(Some(t), PhantomData)
    }

    pub fn empty() -> Self {
        Self(None, PhantomData)
    }
}
impl<T, S, const IDX: usize> From<T> for OwnedField<T, S, IDX> {
    fn from(value: T) -> Self {
        Self(Some(value), PhantomData)
    }
}

impl<T, S: FieldsMeta, const IDX: usize> Field for TyField<T, S, IDX> {
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = RefField<'a, Self::Content, S, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = MutField<'a, Self::Content, S, IDX>
    where
        Self::Content: 'a;
    type Owned = OwnedField<T, S, IDX>;
}
impl<T, S: FieldsMeta, const VAR_IDX: usize, const IDX: usize> Field
    for VarTyField<T, S, VAR_IDX, IDX>
{
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = VarRefField<'a, Self::Content, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = VarMutField<'a, Self::Content, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type Owned = VarOwnedField<T, S, VAR_IDX, IDX>;
}
impl<T, S: FieldsMeta, const VAR_IDX: usize, const IDX: usize> Field
    for VarRefField<'_, T, S, VAR_IDX, IDX>
{
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = VarRefField<'a, Self::Content, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = VarMutField<'a, Self::Content, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type Owned = VarOwnedField<T, S, VAR_IDX, IDX>;
}
impl<T, S: FieldsMeta, const VAR_IDX: usize, const IDX: usize> Field
    for VarMutField<'_, T, S, VAR_IDX, IDX>
{
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = VarRefField<'a, T, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = VarMutField<'a, T, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type Owned = VarOwnedField<T, S, VAR_IDX, IDX>;
}
impl<T, S: FieldsMeta, const VAR_IDX: usize, const IDX: usize> Field
    for VarOwnedField<T, S, VAR_IDX, IDX>
{
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = VarRefField<'a, T, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = VarMutField<'a, T, S, VAR_IDX, IDX>
    where
        Self::Content: 'a;
    type Owned = VarOwnedField<T, S, VAR_IDX, IDX>;
}

impl<T, S: FieldsMeta, const IDX: usize> Field for RefField<'_, T, S, IDX> {
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = RefField<'a, Self::Content, S, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = MutField<'a, Self::Content, S, IDX>
    where
        Self::Content: 'a;
    type Owned = OwnedField<T, S, IDX>;
}
impl<T, S: FieldsMeta, const IDX: usize> Field for MutField<'_, T, S, IDX> {
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = RefField<'a, T, S, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = MutField<'a, T, S, IDX>
    where
        Self::Content: 'a;
    type Owned = OwnedField<T, S, IDX>;
}
impl<T, S: FieldsMeta, const IDX: usize> Field for OwnedField<T, S, IDX> {
    type Source = S;
    type Content = T;
    const IDX: usize = IDX;
    type Referenced<'a>
        = RefField<'a, T, S, IDX>
    where
        Self::Content: 'a;
    type MutablyReferenced<'a>
        = MutField<'a, T, S, IDX>
    where
        Self::Content: 'a;
    type Owned = OwnedField<T, S, IDX>;
}
