### _Inception_ explores the following concept in Rust

> Given a type `T`, if we can prove some property exists for all of `T`'s minimal substructures, and all of `T`'s immediate substructures, then this property must also hold for `T` itself.

People didn't like my original explanation, they wanted _more code_, and I agree. We are _coders_ after all, so I will try a different way.

First, I will provide an example of using _Inception_ to define some new behavior for some types, let's say a loose replica of `Eq` and friends:

```rust
// Some types we want to implement traits for
#[derive(Inception)]
struct Cobb {
    actor: String,
    height: u8,
    quote: String,
}

#[derive(Inception)]
struct Fischer {
    actor: String,
    weight: u8,
}

#[derive(Inception)]
enum Character {
    Cobb(Cobb),
    Fischer(Fischer)
}

// Some traits we want automatically implemented for many types
#[inception(property = SameSame, comparator)]
pub trait Same {
    fn same(&self, _other: &Self) -> bool;

    fn nothing() -> bool {
        true
    }
    fn merge<H: Same<Ret = bool>, R: Same<Ret = bool>>(l: L, r: R, l2: L, r2: R) -> bool {
        l.access().same(l2.access()) && r.same(&r2)
    }
    fn merge_variant_field<H: Same<Ret = bool>, R: Same<Ret = bool>>(
        l: L,
        r: R,
        l2: L,
        r2: R,
    ) -> bool {
        match (l.try_access(), l2.try_access()) {
            (Ok(l), Ok(l2)) => l.same(l2) && r.same(&r2),
            (Err(_), Err(_)) => true,
            _ => false,
        }
    }
    fn join<F: Same<Ret = bool>>(fields: F, fields2: F) -> bool {
        fields.same(&fields2)
    }
}

// Some primitives 
#[primitive(property = SameSame)]
impl Same for u8 {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
#[primitive(property = SameSame)]
impl Same for String {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
#[primitive(property = SameSame)]
impl Same for u64 { .. }
#[primitive(property = SameSame)]
impl Same for bool { .. }
#[primitive(property = SameSame)]
impl Same for i32 { .. }

// More behaviors
#[inception( .. )]
pub trait Digestible { .. }
#[inception( .. )]
pub trait Serializable { .. }
#[inception( .. )]
pub trait Deserializable { .. }
#[inception( .. )]
pub trait Debuggable { .. }
#[inception( .. )]
pub trait Duplicatable { .. }
#[inception( .. )]
pub trait Databasable { .. }
#[inception( .. )]
pub trait Streamable { .. }
#[inception( .. )]
pub trait ArtificallyIntelligible { .. }
#[inception( .. )]
pub trait Profitable { .. }

// `Cobb`, `Fischer`, `Character` and whatever other types have been annotated with #[derive(Inception)] now implement these behaviors 
```

Now I will show the exact same code with all macros expanded. I'll try to add more comments to this whenever I have time, wherever I think they may be useful.

```rust
struct Cobb {
    actor: String,
    height: u8,
    quote: String,
}
// These just add some metadata
impl ::inception::DataType for Cobb {
    const NAME: &str = stringify!(Cobb);
    type Ty = ::inception::StructTy<::inception::True>;
}
impl ::inception::StructMeta for Cobb {
    const NUM_FIELDS: usize = 3;
    type NamedFields = ::inception::True;
}
impl ::inception::NamedFieldsMeta for Cobb {
    const FIELD_NAMES: &[&str] = &["actor", "height", "quote"];
}
impl<X: ::inception::Property> ::inception::IsPrimitive<X> for Cobb {
    type Is = ::inception::False;
}
// This `Inception` trait is the most important impl from the derive, because it exposes as
// associated types a type-level field list for ref/ref-mut/owned/phantom data of this type, 
// allowing us to refer to it from where-bounds on the blanket implementations below.
impl<X: ::inception::Property> ::inception::Inception<X, ::inception::False> for Cobb {
    type TyFields = inception::ty::List<(
        inception::TyField<String, Self, 0>,
        inception::ty::List<(
            inception::TyField<u8, Self, 1>,
            inception::ty::List<(inception::TyField<String, Self, 2>, inception::ty::List<()>)>,
        )>,
    )>;
    type RefFields<'a> = <Self::TyFields as ::inception::Fields>::Referenced<'a>;
    type MutFields<'a> = <Self::TyFields as ::inception::Fields>::MutablyReferenced<'a>;
    type OwnedFields = <Self::TyFields as ::inception::Fields>::Owned;
    fn fields(&self) -> Self::RefFields<'_> {
        ::inception::list![
            ::inception::RefField::new(&self.actor),
            ::inception::RefField::new(&self.height),
            ::inception::RefField::new(&self.quote)
        ]
    }
    fn fields_mut<'a: 'b, 'b>(
        &'a mut self,
        header: &'b mut ::inception::VariantHeader,
    ) -> Self::MutFields<'b> {
        ::inception::list![
            ::inception::MutField::new(&mut self.actor),
            ::inception::MutField::new(&mut self.height),
            ::inception::MutField::new(&mut self.quote)
        ]
    }
    fn into_fields(self) -> Self::OwnedFields {
        ::inception::list![
            ::inception::OwnedField::new(self.actor),
            ::inception::OwnedField::new(self.height),
            ::inception::OwnedField::new(self.quote)
        ]
    }
    fn from_fields(fields: Self::OwnedFields) -> Self {
        use ::inception::Access;
        Self {
            actor: fields.0 .0.access(),
            height: fields.0 .1 .0 .0.access(),
            quote: fields.0 .1 .0 .1 .0 .0.access(),
        }
    }
}

struct Fischer {
    actor: String,
    weight: u8,
}
impl ::inception::DataType for Fischer {
    const NAME: &str = stringify!(Fischer);
    type Ty = ::inception::StructTy<::inception::True>;
}
impl ::inception::StructMeta for Fischer {
    const NUM_FIELDS: usize = 2;
    type NamedFields = ::inception::True;
}
impl ::inception::NamedFieldsMeta for Fischer {
    const FIELD_NAMES: &[&str] = &["actor", "weight"];
}
impl<X: ::inception::Property> ::inception::IsPrimitive<X> for Fischer {
    type Is = ::inception::False;
}
impl<X: ::inception::Property> ::inception::Inception<X, ::inception::False> for Fischer {
    type TyFields = inception::ty::List<(
        inception::TyField<String, Self, 0>,
        inception::ty::List<(inception::TyField<u8, Self, 1>, inception::ty::List<()>)>,
    )>;
    type RefFields<'a> = <Self::TyFields as ::inception::Fields>::Referenced<'a>;
    type MutFields<'a> = <Self::TyFields as ::inception::Fields>::MutablyReferenced<'a>;
    type OwnedFields = <Self::TyFields as ::inception::Fields>::Owned;
    fn fields(&self) -> Self::RefFields<'_> {
        ::inception::list![
            ::inception::RefField::new(&self.actor),
            ::inception::RefField::new(&self.weight)
        ]
    }
    fn fields_mut<'a: 'b, 'b>(
        &'a mut self,
        header: &'b mut ::inception::VariantHeader,
    ) -> Self::MutFields<'b> {
        ::inception::list![
            ::inception::MutField::new(&mut self.actor),
            ::inception::MutField::new(&mut self.weight)
        ]
    }
    fn into_fields(self) -> Self::OwnedFields {
        ::inception::list![
            ::inception::OwnedField::new(self.actor),
            ::inception::OwnedField::new(self.weight)
        ]
    }
    fn from_fields(fields: Self::OwnedFields) -> Self {
        use ::inception::Access;
        Self {
            actor: fields.0 .0.access(),
            weight: fields.0 .1 .0 .0.access(),
        }
    }
}

enum Character {
    Cobb(Cobb),
    Fischer(Fischer),
}
impl ::inception::DataType for Character {
    const NAME: &str = stringify!(Character);
    type Ty = ::inception::EnumTy;
}
impl ::inception::EnumMeta for Character {
    const VARIANT_NAMES: &[&str] = &["Cobb", "Fischer"];
    const FIELD_NAMES: &[&[&str]] = &[&[], &[]];
}
impl ::inception::VariantOffset<0> for Character {
    const PADDING: Self::Padding = ::inception::PAD_0;
    type Padding = ::inception::Pad0;
}
impl ::inception::VariantOffset<1> for Character {
    const PADDING: Self::Padding = ::inception::PAD_2;
    type Padding = ::inception::Pad2;
}
impl<X: ::inception::Property> ::inception::IsPrimitive<X> for Character {
    type Is = ::inception::False;
}
impl<X: ::inception::Property> ::inception::Inception<X, ::inception::False> for Character {
    type TyFields = inception::ty::List<(
        inception::VarTyField<inception::VariantHeader, Self, 0, { 0 }>,
        inception::ty::List<(
            inception::VarTyField<Cobb, Self, 0, 0>,
            inception::ty::List<(
                inception::VarTyField<inception::VariantHeader, Self, 1, { 0 }>,
                inception::ty::List<(
                    inception::VarTyField<Fischer, Self, 1, 0>,
                    inception::ty::List<()>,
                )>,
            )>,
        )>,
    )>;
    type RefFields<'a> = <Self::TyFields as ::inception::Fields>::Referenced<'a>;
    type MutFields<'a> = <Self::TyFields as ::inception::Fields>::MutablyReferenced<'a>;
    type OwnedFields = <Self::TyFields as ::inception::Fields>::Owned;
    fn fields(&self) -> Self::RefFields<'_> {
        use ::inception::{list, Mask, Pad, Phantom, VarRefField};
        let mut fields = Self::RefFields::phantom();
        match self {
            Self::Cobb(_0) => fields.mask(
                ::inception::list![
                    VarRefField::header(&inception::VariantHeader),
                    VarRefField::new(_0)
                ]
                .pad(<Self as ::inception::VariantOffset<0>>::PADDING),
            ),
            Self::Fischer(_0) => fields.mask(
                ::inception::list![
                    VarRefField::header(&inception::VariantHeader),
                    VarRefField::new(_0)
                ]
                .pad(<Self as ::inception::VariantOffset<1>>::PADDING),
            ),
        }
    }
    fn fields_mut<'a: 'b, 'b>(
        &'a mut self,
        header: &'b mut ::inception::VariantHeader,
    ) -> Self::MutFields<'b> {
        use ::inception::{list, Mask, Pad, Phantom, VarMutField};
        let mut fields = Self::MutFields::phantom();
        match self {
            Self::Cobb(_0) => fields.mask(
                ::inception::list![VarMutField::header(header), VarMutField::new(_0)]
                    .pad(<Self as ::inception::VariantOffset<0>>::PADDING),
            ),
            Self::Fischer(_0) => fields.mask(
                ::inception::list![VarMutField::header(header), VarMutField::new(_0)]
                    .pad(<Self as ::inception::VariantOffset<1>>::PADDING),
            ),
        }
    }
    fn into_fields(self) -> Self::OwnedFields {
        use ::inception::{list, Mask, Pad, Phantom, VarOwnedField};
        let mut fields = Self::OwnedFields::phantom();
        match self {
            Self::Cobb(_0) => fields.mask(
                ::inception::list![
                    VarOwnedField::header(::inception::VariantHeader),
                    VarOwnedField::new(_0)
                ]
                .pad(<Self as ::inception::VariantOffset<0>>::PADDING),
            ),
            Self::Fischer(_0) => fields.mask(
                ::inception::list![
                    VarOwnedField::header(::inception::VariantHeader),
                    VarOwnedField::new(_0)
                ]
                .pad(<Self as ::inception::VariantOffset<1>>::PADDING),
            ),
        }
    }
    fn from_fields(fields: Self::OwnedFields) -> Self {
        use ::inception::{Access, IntoTuples, SplitOff};
        let (l, fields) = fields.split_off(::inception::PAD_2);
        if l.0 .0.has_value() {
            let (header, (_0, _)) = l.access().into_tuples();
            return Self::Cobb(_0);
        }
        let (l, fields) = fields.split_off(::inception::PAD_2);
        if l.0 .0.has_value() {
            let (header, (_0, _)) = l.access().into_tuples();
            return Self::Fischer(_0);
        }
        panic!("Failed to determine enum variant.");
    }
}

// Ideally everything from here down wouldn't live in a macro at all. It makes things more 
// difficult though, and I couldn't figure it out.
pub struct SameSame;
pub trait Same {
    fn same(&self, _other: &Self) -> bool;
}
mod __inception_same {
    use inception::{meta::Metadata, False, IsPrimitive, True, TruthValue, Wrapper};
    impl ::inception::Property for super::SameSame {}
    impl<T> ::inception::Compat<T> for super::SameSame
    where
        T: super::Same,
    {
        type Out = True;
    }
    // We'll have to wrap some things to get around orphan rules
    pub struct Wrap<T>(pub T);
    impl<T> Wrapper for Wrap<T> {
        type Content = T;
        fn wrap(t: Self::Content) -> Self {
            Self(t)
        }
    }
    impl<T> IsPrimitive<super::SameSame> for Wrap<T> {
        type Is = False;
    }
    pub trait Inductive<P: TruthValue = <Self as IsPrimitive<super::SameSame>>::Is> {
        type Property: ::inception::Property;
        type Ret;
        fn same(&self, _other: &Self) -> Self::Ret;
    }
    impl<T> Inductive<True> for T
    where
        T: super::Same + IsPrimitive<super::SameSame, Is = True>,
    {
        type Property = super::SameSame;
        type Ret = bool;
        fn same(&self, _other: &Self) -> Self::Ret {
            self.same(_other)
        }
    }
    pub trait Nothing {
        type Ret;
        fn nothing() -> Self::Ret;
    }
    pub trait MergeField<L, R> {
        type Ret;
        fn merge_field(l: L, r: R, l2: L, r2: R) -> Self::Ret;
    }
    pub trait MergeVariantField<L, R> {
        type Ret;
        fn merge_variant_field(l: L, r: R, l2: L, r2: R) -> Self::Ret;
    }
    pub trait Join<F> {
        type Ret;
        fn join(fields: F, fields2: F) -> Self::Ret;
    }
}
// Blanket implementation of the original trait
impl<T> Same for T
where
    T: __inception_same::Inductive<::inception::False, Ret = bool>,
{
    fn same(&self, _other: &Self) -> bool {
        self.same(_other)
    }
}
impl<T> __inception_same::Nothing for T {
    type Ret = bool;
    fn nothing() -> Self::Ret {
        {
            true
        }
    }
}
impl<'a, H, S, const IDX: usize, F, L, R> __inception_same::MergeField<L, R>
    for __inception_same::Wrap<&'_ List<(RefField<'a, H, S, IDX>, F)>>
where
    S: FieldsMeta,
    H: __inception_same::Inductive
        + __inception_same::Inductive<Ret = bool>
        + ::inception::IsPrimitive<SameSame>,
    F: Fields,
    L: Field + Access<Out = &'a H>,
    R: Fields
        + __inception_same::Inductive
        + __inception_same::Inductive<Ret = bool>
        + ::inception::IsPrimitive<SameSame>,
{
    type Ret = bool;
    fn merge_field(l: L, r: R, l2: L, r2: R) -> Self::Ret {
        {
            l.access().same(l2.access()) && r.same(&r2)
        }
    }
}
impl<'a, H, S, const VAR_IDX: usize, const IDX: usize, F, L, R>
    __inception_same::MergeVariantField<L, R>
    for __inception_same::Wrap<&'_ List<(VarRefField<'a, H, S, VAR_IDX, IDX>, F)>>
where
    S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
    H: __inception_same::Inductive
        + __inception_same::Inductive<Ret = bool>
        + ::inception::IsPrimitive<SameSame>,
    F: Fields,
    L: Field<Source = S>
        + VarField
        + TryAccess<Out = &'a H, Err = RefEnumAccessError<'a, H, S, VAR_IDX, IDX>>,
    R: Fields
        + __inception_same::Inductive
        + __inception_same::Inductive<Ret = bool>
        + ::inception::IsPrimitive<SameSame>,
{
    type Ret = bool;
    fn merge_variant_field(l: L, r: R, l2: L, r2: R) -> Self::Ret {
        {
            match (l.try_access(), l2.try_access()) {
                (Ok(l), Ok(l2)) => l.same(l2) && r.same(&r2),
                (Err(_), Err(_)) => true,
                _ => false,
            }
        }
    }
}
impl<T, F> __inception_same::Join<F> for T
where
    T: Inception<SameSame>,
    F: __inception_same::Inductive
        + __inception_same::Inductive<Ret = bool>
        + ::inception::IsPrimitive<SameSame>,
{
    type Ret = bool;
    fn join(fields: F, fields2: F) -> Self::Ret {
        {
            fields.same(&fields2)
        }
    }
}
impl<T> Fields for __inception_same::Wrap<&T>
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
impl<'b, T, S, const IDX: usize, V> SplitRef<SameSame>
    for __inception_same::Wrap<&'_ List<(RefField<'b, T, S, IDX>, V)>>
{
    type Left = RefField<'b, T, S, IDX>;
    type Right = V;
    fn split_ref(&self) -> (Self::Left, &Self::Right) {
        (self.0 .0 .0.clone(), &self.0 .0 .1)
    }
}
impl<'b, T, S, const VAR_IDX: usize, const IDX: usize, V> SplitRef<SameSame>
    for __inception_same::Wrap<&'_ List<(VarRefField<'b, T, S, VAR_IDX, IDX>, V)>>
where
    V: Phantom,
{
    type Left = VarRefField<'b, T, S, VAR_IDX, IDX>;
    type Right = V;
    fn split_ref(&self) -> (Self::Left, &Self::Right) {
        (self.0 .0 .0.clone(), &self.0 .0 .1)
    }
}
// This is the base case
impl __inception_same::Inductive for __inception_same::Wrap<&'_ List<()>> {
    type Property = SameSame;
    type Ret = bool;
    #[allow(unused)]
    fn same(&self, _other: &Self) -> Self::Ret {
        <Self as __inception_same::Nothing>::nothing()
    }
}
// This is the induction step
impl<H, S, const IDX: usize, F> __inception_same::Inductive
    for __inception_same::Wrap<&'_ List<(RefField<'_, H, S, IDX>, F)>>
where
    S: FieldsMeta,
    H: __inception_same::Inductive
        + __inception_same::Inductive<Ret = bool>
        + ::inception::IsPrimitive<SameSame>,
    F: Fields,
    <F as Fields>::Owned: Fields,
    for<'a, 'b> __inception_same::Wrap<&'a List<(RefField<'b, H, S, IDX>, F)>>:
        SplitRef<SameSame, Left = RefField<'b, H, S, IDX>>,
    for<'a, 'b, 'c> __inception_same::Wrap<
        &'a <__inception_same::Wrap<&'b List<(RefField<'c, H, S, IDX>, F)>> as SplitRef<
            SameSame,
        >>::Right,
    >: __inception_same::Inductive + __inception_same::Inductive<Ret = bool> + Fields,
{
    type Property = SameSame;
    type Ret = bool;
    fn same(&self, _other: &Self) -> Self::Ret {
        use SplitRef;
        let (l, r) = self.split_ref();
        let r = __inception_same::Wrap(r);
        let (l2, r2) = _other.split_ref();
        let r2 = __inception_same::Wrap(r2);
        <Self as __inception_same::MergeField<_, _>>::merge_field(l, r, l2, r2)
    }
}
// Enums need to be addressed separately because of how they are implemented
impl <H,S,const VAR_IDX:usize,const IDX:usize,F>__inception_same::Inductive for __inception_same::Wrap< &'_ List<(VarRefField<'_,H,S,VAR_IDX,IDX> ,F)>>where S:FieldsMeta+EnumMeta+VariantOffset<VAR_IDX> ,H:__inception_same::Inductive+__inception_same::Inductive<Ret = bool> + ::inception::IsPrimitive<SameSame> ,F:Fields, <F as Fields> ::Owned:Fields,for<'a,'b>__inception_same::Wrap< &'a List<(VarRefField<'b,H,S,VAR_IDX,IDX> ,F)>> :SplitRef<SameSame,Left = VarRefField<'b,H,S,VAR_IDX,IDX>> ,for<'a,'b,'c>__inception_same::Wrap< &'a<__inception_same::Wrap< &'b List<(VarRefField<'c,H,S,VAR_IDX,IDX> ,F)>>as SplitRef<SameSame>> ::Right> :__inception_same::Inductive+__inception_same::Inductive<Ret = bool> +Fields,{
    type Property = SameSame;
    type Ret = bool;
    fn same(&self,_other: &Self) -> Self::Ret {
        use SplitRef;
        let(l,r) = self.split_ref();
        let r = __inception_same::Wrap(r);
        let(l2,r2) = _other.split_ref();
        let r2 = __inception_same::Wrap(r2);
        <Self as __inception_same::MergeVariantField<_,_>> ::merge_variant_field(l,r,l2,r2)
    }
    }
// Blanket impl for types with fields
impl<T> __inception_same::Inductive<False> for T
where
    T: Inception<SameSame> + Meta,
    for<'a, 'b> __inception_same::Wrap<&'a <T as Inception<SameSame>>::RefFields<'b>>:
        __inception_same::Inductive + __inception_same::Inductive<Ret = bool>,
{
    type Property = SameSame;
    type Ret = bool;
    fn same(&self, fields2: &Self) -> Self::Ret {
        use __inception_same::Join;
        let fields = self.fields();
        let f = __inception_same::Wrap(&fields);
        let fields2 = fields2.fields();
        let fields2 = __inception_same::Wrap(&fields2);
        Self::join(f, fields2)
    }
}

impl Same for u8 {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
const _: () = {
    impl ::inception::IsPrimitive<SameSame> for u8 {
        type Is = <SameSame as ::inception::Compat<Self>>::Out;
    }
};

impl Same for String {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
const _: () = {
    impl ::inception::IsPrimitive<SameSame> for String {
        type Is = <SameSame as ::inception::Compat<Self>>::Out;
    }
};

// etc etc for all behaviors and their primitives..
// there are subtle differences when dealing with owned data, mutable references,
// additional args etc, but in general it is the same idea.
```

Hopefully that helps you understand how this works!
