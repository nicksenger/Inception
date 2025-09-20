In case you are short on time: this is _not_ currently "better" than using a derive macro in any way for the vast majority of potential use-cases. It is more of an academic curiosity than anything at this point.

I suspect there are some more interesting uses for this aside from achieving what we can already do with derive macros, but that's all I've looked at since it was the obvious application. If you are aware of or interested in other potential use-cases for this work, please reach out! I'd love to hear any thoughts. 


### _Inception_ explores the following concept in Rust

> Given a type `T`, if we can prove some property exists for all of `T`'s minimal substructures, and all of `T`'s immediate substructures, then this property must also hold for `T` itself.

To give a more practical example, imagine the situation where we have some types which require many derive macros:

```rust
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OneFish {
    name: String,
    age: u32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TwoFish {
    id: u64,
    size: i32
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NumberedFishes {
    One(OneFish),
    Two(TwoFish)
}
```

Code expecting the constituent fields to implement the respective trait will be generated for each of these derives. But what if we could just have a single derive which provides the field information at compile time to any trait implementations automatically? Then we would only need to generate code once to achieve identical behavior.

```rust
#[derive(Inception)]
pub struct OneFish {
    name: String,
    age: u32
}

#[derive(Inception)]
pub struct TwoFish {
    id: u64,
    size: i32
}

#[derive(Inception)]
pub enum NumberedFishes {
    One(OneFish),
    Two(TwoFish)
}

// Or, as opt-in. The same amount of code is generated regardless of the number of properties.
mod opt_in {
    #[derive(Inception)]
    #[inception(properties = [Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize])]
    pub struct OneFish {
        name: String,
        age: u32
    }

    #[derive(Inception)]
    #[inception(properties = [Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize])]
    pub struct TwoFish {
        id: u64,
        size: i32
    }

    #[derive(Inception)]
    #[inception(properties = [Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize])]
    pub enum NumberedFishes {
        One(OneFish),
        Two(TwoFish)
    }
}
```

This is similar to the experience provided by [Facet](https://crates.io/crates/facet) and other runtime reflection crates, but in this case we will use type-level reflection to have the trait solver and monomorphization achieve, in effect, what would normally be achieved by a macro. No dynamic dispatch is required, no type information is lost, and, at least in theory, no additional runtime overhead should be incurred compared to a derive. We will just reach the same destination by a different compilation path.

This can be achieved through "structural" or "well-founded" induction, a concept first introduced in 1917 by a mathematician named Dmitry Mirimanoff. While we won't sacrifice memory or type safety in order to do this in Rust over a century later, we _will_ have to employ a fair bit of type-level programming in what could definitely be labeled a _mis_-use of the trait solver, so the resulting code will not necessarily be idiomatic.

### Approach

I think the most straightforward way to explain this is with an example use case, so imagine we have two big-name celebrities: Leonardo Di Caprio and Cillian Murphy.

```rust
struct LeonardoDiCaprio;
struct CillianMurphy;
```

Let's make an explicit assumption that the following statement is true:

> (A film/recording consisting solely of) either Leonardo Di Caprio or Cillian Murphy is guaranteed to be a blockbuster

(it's tempting to argue that this holds trivially, but we can save that debate for another time)

In Rust, this can be expressed as follows:

```rust
impl Blockbuster for LeonardoDiCaprio {
    fn profit() -> Profits {
        Profits::GUARANTEED_DI_CAPRIO
    }
}
impl Blockbuster for CillianMurphy {
    fn profit() -> Profits {
        Profits::GUARANTEED_MURPHY
    }
}
```

Now let's introduce some additional structures involving these actors:

```rust
enum Character {
    Cobb(LeonardoDiCaprio),
    Fischer {
        played_by: CillianMurphy
    }
}

struct PlotHole {
    involving: Character,
}

struct Scene {
    featuring: Character,
    introduces: PlotHole,
}

struct Inception1 {
    starring: LeonardoDiCaprio,
    and_also: CillianMurphy,
    exposition: Scene,
    rising_action: Scene,
    climax: Scene,
    resolution: Scene
}
```

We can propose that `Inception1` implements `Blockbuster`, but Rust won't let us call `profit` on an instance of this type until we _prove_ that this is the case. Of course we could explicitly implement `Blockbuster` for these types, either manually or with a `Derive` macro, but that's _too much work_ and too error-prone. We want the compiler to just _know_ that `Inception1` is a `Blockbuster`, without having to do anything in particular. And we want it to _know_ that if we rearrange the scenes in any way, or add any additional plot-holes, it will _still_ be a `Blockbuster`. If we make a sequel, a trilogy, a TV series, video game, or even `InceptionChristmas`, we want everything in the whole franchise to be indisputably proven to be a `Blockbuster` by mathemical law, so long as those things are constructed of parts whose minimal substructures are `LeonardoDiCaprio` or `CillianMurphy`.

Revisiting the work of Mirimanoff, we note that the first requirement for our proof, about minimal substructures, is already met for the new types we've defined. So now we just need to show that the immediate substructures of `Inception1` (its _fields_) are also `Blockbuster`s.

This is where Rust starts making things a bit tricky for us, because the compiler doesn't serve us up information about a type's fields in where-bounds. So we'll have to introduce a new assumption: that each of these structures exposes its own fields as an ordered type-level list, and that they do so from a trait associated-type, such that we can refer to them in where-bounds when implementing traits. Shown below is a comment with the assumption for each type, and a derive macro added to the type to handle the actual plumbing of exposing this list to the rest of our code:

```rust
// type Fields = list![ LeonardoDiCaprio, CillianMurphy ];
#[derive(Inception)]
enum Character {
    Cobb(LeonardoDiCaprio),
    Fischer {
        played_by: CillianMurphy
    }
}

// type Fields = list![ Character ];
#[derive(Inception)]
struct PlotHole {
    involving: Character,
}

// type Fields = list![ Character, PlotHole ];
#[derive(Inception)]
struct Scene {
    featuring: Character,
    introduces: PlotHole,
}

// type Fields = list! [ LeonardoDiCaprio, CillianMurphy, Scene, Scene, Scene, Scene ];
#[derive(Inception)]
struct Inception1 {
    starring: LeonardoDiCaprio,
    and_also: CillianMurphy,
    exposition: Scene,
    rising_action: Scene,
    climax: Scene,
    resolution: Scene
}
```


<details>
    <summary>Show generated code</summary>

```rust
impl ::inception::DataType for Character {
    const NAME: &str = stringify!(Character);
    type Ty = ::inception::EnumTy;
}
impl ::inception::EnumMeta for Character {
    const VARIANT_NAMES: &[&str] = &["Cobb", "Fischer"];
    const FIELD_NAMES: &[&[&str]] = &[&[], &["played_by"]];
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
    type TyFields =
        ::inception::enum_field_tys![[0, [0, LeonardoDiCaprio]], [1, [0, CillianMurphy]]];
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
            Self::Fischer { played_by } => fields.mask(
                ::inception::list![
                    VarRefField::header(&inception::VariantHeader),
                    VarRefField::new(played_by)
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
            Self::Fischer { played_by } => fields.mask(
                ::inception::list![VarMutField::header(header), VarMutField::new(played_by)]
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
            Self::Fischer { played_by } => fields.mask(
                ::inception::list![
                    VarOwnedField::header(::inception::VariantHeader),
                    VarOwnedField::new(played_by)
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
            let (header, (played_by, _)) = l.access().into_tuples();
            return Self::Fischer { played_by };
        }
        panic!("Failed to determine enum variant.");
    }
}

impl ::inception::DataType for PlotHole {
    const NAME: &str = stringify!(PlotHole);
    type Ty = ::inception::StructTy<::inception::True>;
}
impl ::inception::StructMeta for PlotHole {
    const NUM_FIELDS: usize = 1;
    type NamedFields = ::inception::True;
}
impl ::inception::NamedFieldsMeta for PlotHole {
    const FIELD_NAMES: &[&str] = &["involving"];
}
impl<X: ::inception::Property> ::inception::IsPrimitive<X> for PlotHole {
    type Is = ::inception::False;
}
impl<X: ::inception::Property> ::inception::Inception<X, ::inception::False> for PlotHole {
    type TyFields = ::inception::struct_field_tys![0, Character];
    type RefFields<'a> = <Self::TyFields as ::inception::Fields>::Referenced<'a>;
    type MutFields<'a> = <Self::TyFields as ::inception::Fields>::MutablyReferenced<'a>;
    type OwnedFields = <Self::TyFields as ::inception::Fields>::Owned;
    fn fields(&self) -> Self::RefFields<'_> {
        ::inception::list![::inception::RefField::new(&self.involving)]
    }
    fn fields_mut<'a: 'b, 'b>(
        &'a mut self,
        header: &'b mut ::inception::VariantHeader,
    ) -> Self::MutFields<'b> {
        ::inception::list![::inception::MutField::new(&mut self.involving)]
    }
    fn into_fields(self) -> Self::OwnedFields {
        ::inception::list![::inception::OwnedField::new(self.involving)]
    }
    fn from_fields(fields: Self::OwnedFields) -> Self {
        use ::inception::Access;
        Self {
            involving: fields.0 .0.access(),
        }
    }
}
```
</details>
<br />


At this point, it may be more helpful to think of this as writing a recursive function, only the actual body of our function will live in where-bounds and be closer to a logic programming language like Prolog than the Rust we're used to writing.

_Inception_ tries to hide most of these gory details behind another proc-macro on the trait definition itself. I won't try to stop you from expanding it, but please: ensure no children are present, wear some form of OSHA-approved eye protection, and remember that there are certain things in life which can never be _unseen_. Dark truths which are best kept hidden away, in hopes of preserving that which is still _good_ and _innocent_ in this world. I'll speak no more of its inner devil-work.

Instead, let's focus on the intended use of the resulting API, which requires us to define our trait in terms of building up a recursive datatype having some property from simpler elements already known to possess this property. In our example, `LeonardoDiCaprio` and `CillianMurphy` serve as the fundamental primitives or "base-case" upon which all of these elements are constructed, but we need never refer to them explicitly here, and instead should operate only in terms of generic types for which the property in question is already assumed to be true.

Here is a definition of the `Blockbuster` trait that will be automatically implemented for our types, and where the profits will be the sum of the profits from all of the individual substructures:

```rust
#[inception(property = BoxOfficeHit)]
pub trait Blockbuster {
    // // This will be the only method in our final `Blockbuster` trait
    fn profit(self) -> Profits;

    // // Everything below this point is to prove that our trait applies to the recursive structures
    //
    // Define what should happen for nothing at all (e.g. the end of a list)
    fn nothing() -> Profits {
        Profits::Nothing
    }

    // Define how we should merge a single item (field) having this property with  _some number_
    // of (possibly 0) items (fields) which together also exhibit this property
    fn merge<H: Blockbuster, R: Blockbuster>(l: L, r: R) -> Profits {
        l.access().profit() + r.profit()
    }

    // Same as above, but different because.. enums.
    fn merge_variant_field<H: Blockbuster, R: Blockbuster>(
        l: L,
        r: R,
    ) -> Profits {
        l.try_access().map(|l| l.profit()).unwrap_or(Profits::Nothing) + r.profit()
    }

    // Define how we should join a "container" (struct or enum) with a collection of
    // its fields (that together are known to have this property)
    fn join<F: Blockbuster>(fields: F) -> Profits {
        fields.profit()
    }
}
```

<details>
    <summary>Show generated code</summary>

```rust
pub struct BoxOfficeHit;
pub trait Blockbuster {
    fn profit(self) -> Profits;
}
mod __inception_blockbuster {
    use inception::{meta::Metadata, False, IsPrimitive, True, TruthValue, Wrapper};
    impl ::inception::Property for super::BoxOfficeHit {}
    impl<T> ::inception::Compat<T> for super::BoxOfficeHit
    where
        T: super::Blockbuster,
    {
        type Out = True;
    }
    pub struct Wrap<T>(pub T);
    impl<T> Wrapper for Wrap<T> {
        type Content = T;
        fn wrap(t: Self::Content) -> Self {
            Self(t)
        }
    }
    impl<T> IsPrimitive<super::BoxOfficeHit> for Wrap<T> {
        type Is = False;
    }
    pub trait Inductive<P: TruthValue = <Self as IsPrimitive<super::BoxOfficeHit>>::Is> {
        type Property: ::inception::Property;
        type Ret;
        fn profit(self) -> Self::Ret;
    }
    impl<T> Inductive<True> for T
    where
        T: super::Blockbuster + IsPrimitive<super::BoxOfficeHit, Is = True>,
    {
        type Property = super::BoxOfficeHit;
        type Ret = Profits;
        fn profit(self) -> Self::Ret {
            self.profit()
        }
    }
    pub trait Nothing {
        type Ret;
        fn nothing() -> Self::Ret;
    }
    pub trait MergeField<L, R> {
        type Ret;
        fn merge_field(l: L, r: R) -> Self::Ret;
    }
    pub trait MergeVariantField<L, R> {
        type Ret;
        fn merge_variant_field(l: L, r: R) -> Self::Ret;
    }
    pub trait Join<F> {
        type Ret;
        fn join(fields: F) -> Self::Ret;
    }
}
impl<T> Blockbuster for T
where
    T: __inception_blockbuster::Inductive<::inception::False, Ret = Profits>,
{
    fn profit(self) -> Profits {
        self.profit()
    }
}
impl<T> __inception_blockbuster::Nothing for T {
    type Ret = Profits;
    fn nothing() -> Self::Ret {
        {
            Profits::Nothing
        }
    }
}
impl<H, S, const IDX: usize, F, L, R> __inception_blockbuster::MergeField<L, R>
    for __inception_blockbuster::Wrap<List<(OwnedField<H, S, IDX>, F)>>
where
    S: FieldsMeta,
    H: __inception_blockbuster::Inductive
        + __inception_blockbuster::Inductive
        + ::inception::IsPrimitive<BoxOfficeHit>,
    F: Fields,
    L: Field + Access<Out = H>,
    R: Fields
        + __inception_blockbuster::Inductive
        + __inception_blockbuster::Inductive
        + ::inception::IsPrimitive<BoxOfficeHit>,
{
    type Ret = Profits;
    fn merge_field(l: L, r: R) -> Self::Ret {
        {
            l.access().profit() + r.profit()
        }
    }
}
impl<H, S, const VAR_IDX: usize, const IDX: usize, F, L, R>
    __inception_blockbuster::MergeVariantField<L, R>
    for __inception_blockbuster::Wrap<List<(VarOwnedField<H, S, VAR_IDX, IDX>, F)>>
where
    S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
    H: __inception_blockbuster::Inductive
        + __inception_blockbuster::Inductive
        + ::inception::IsPrimitive<BoxOfficeHit>,
    F: Fields,
    L: Field<Source = S>
        + VarField
        + TryAccess<Out = H, Err = OwnedEnumAccessError<H, S, VAR_IDX, IDX>>,
    R: Fields
        + __inception_blockbuster::Inductive
        + __inception_blockbuster::Inductive
        + ::inception::IsPrimitive<BoxOfficeHit>,
{
    type Ret = Profits;
    fn merge_variant_field(l: L, r: R) -> Self::Ret {
        {
            l.try_access()
                .map(|l| l.profit())
                .unwrap_or(Profits::Nothing)
                + r.profit()
        }
    }
}
impl<T, F> __inception_blockbuster::Join<F> for T
where
    T: Inception<BoxOfficeHit>,
    F: __inception_blockbuster::Inductive
        + __inception_blockbuster::Inductive
        + ::inception::IsPrimitive<BoxOfficeHit>,
{
    type Ret = Profits;
    fn join(fields: F) -> Self::Ret {
        {
            fields.profit()
        }
    }
}
impl<T> Fields for __inception_blockbuster::Wrap<T>
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
impl<T, S, const IDX: usize, V> Split<BoxOfficeHit>
    for __inception_blockbuster::Wrap<List<(OwnedField<T, S, IDX>, V)>>
{
    type Left = OwnedField<T, S, IDX>;
    type Right = V;
    fn split(self) -> (Self::Left, Self::Right) {
        (self.0 .0 .0, self.0 .0 .1)
    }
}
impl<T, S, const VAR_IDX: usize, const IDX: usize, V> Split<BoxOfficeHit>
    for __inception_blockbuster::Wrap<List<(VarOwnedField<T, S, VAR_IDX, IDX>, V)>>
where
    V: Phantom,
{
    type Left = VarOwnedField<T, S, VAR_IDX, IDX>;
    type Right = V;
    fn split(self) -> (Self::Left, Self::Right) {
        (self.0 .0 .0, self.0 .0 .1)
    }
}
impl __inception_blockbuster::Inductive for __inception_blockbuster::Wrap<List<()>> {
    type Property = BoxOfficeHit;
    type Ret = Profits;
    #[allow(unused)]
    fn profit(self) -> Self::Ret {
        <Self as __inception_blockbuster::Nothing>::nothing()
    }
}
impl <H,S,const IDX:usize,F>__inception_blockbuster::Inductive for __inception_blockbuster::Wrap<List<(OwnedField<H,S,IDX> ,F)>>where S:FieldsMeta,H:__inception_blockbuster::Inductive+__inception_blockbuster::Inductive+ ::inception::IsPrimitive<BoxOfficeHit> ,F:Fields, <F as Fields> ::Owned:Fields,for< >__inception_blockbuster::Wrap<List<(OwnedField<H,S,IDX> ,F)>> :Split<BoxOfficeHit,Left = OwnedField<H,S,IDX>> ,for< >__inception_blockbuster::Wrap< <__inception_blockbuster::Wrap<List<(OwnedField<H,S,IDX> ,F)>>as Split<BoxOfficeHit>> ::Right> :__inception_blockbuster::Inductive+__inception_blockbuster::Inductive+Fields,{
    type Property = BoxOfficeHit;
    type Ret = Profits;
    fn profit(self) -> Self::Ret {
        use Split;
        let(l,r) = self.split();
        let r = __inception_blockbuster::Wrap(r);
        <Self as __inception_blockbuster::MergeField<_,_>> ::merge_field(l,r)
    }
    }
impl<H, S, const VAR_IDX: usize, const IDX: usize, F> __inception_blockbuster::Inductive
    for __inception_blockbuster::Wrap<List<(VarOwnedField<H, S, VAR_IDX, IDX>, F)>>
where
    S: FieldsMeta + EnumMeta + VariantOffset<VAR_IDX>,
    H: __inception_blockbuster::Inductive
        + __inception_blockbuster::Inductive
        + ::inception::IsPrimitive<BoxOfficeHit>,
    F: Fields,
    <F as Fields>::Owned: Fields,
    __inception_blockbuster::Wrap<List<(VarOwnedField<H, S, VAR_IDX, IDX>, F)>>:
        Split<BoxOfficeHit, Left = VarOwnedField<H, S, VAR_IDX, IDX>>,
    __inception_blockbuster::Wrap<
        <__inception_blockbuster::Wrap<List<(VarOwnedField<H, S, VAR_IDX, IDX>, F)>> as Split<
            BoxOfficeHit,
        >>::Right,
    >: __inception_blockbuster::Inductive + __inception_blockbuster::Inductive + Fields,
{
    type Property = BoxOfficeHit;
    type Ret = Profits;
    fn profit(self) -> Self::Ret {
        use Split;
        let (l, r) = self.split();
        let r = __inception_blockbuster::Wrap(r);
        <Self as __inception_blockbuster::MergeVariantField<_, _>>::merge_variant_field(l, r)
    }
}
impl<T> __inception_blockbuster::Inductive<False> for T
where
    T: Inception<BoxOfficeHit> + Meta,
    __inception_blockbuster::Wrap<<T as Inception<BoxOfficeHit>>::OwnedFields>:
        __inception_blockbuster::Inductive + __inception_blockbuster::Inductive,
{
    type Property = BoxOfficeHit;
    type Ret = Profits;
    fn profit(self) -> Self::Ret {
        use __inception_blockbuster::Join;
        let fields = self.into_fields();
        let f = __inception_blockbuster::Wrap(fields);
        Self::join(f)
    }
}

```
</details>
<br />



Now we can call `.profit()` on instances of `Inception1`, `PlotHole`, or any of our intermediate types. We can define new types composed of these composite types in any order or combination and, so long as we use the derive macro to expose their fields, these new types will also implement `Blockbuster` or any other traits we define this way!

We can create as many behaviors as we want, for serialization/deserialization, debugging, etc, for whatever sets of primitives, and share them all through the single `#[derive(Inception)]`. "Alternatives" to many of the standard derive macros are already implemented as tests for this crate. So it _is possible_ to convince the Rust compiler that these properties hold. But before anyone gets carried away and starts thinking: _Serde is dead, Clone, Hash and all of the std Derive macros are dead! Praise Dmitry Mirimanoff! Long live Inception! The last macro we'll ever need!_

Slow down, because there's a big blaring plot-hole, and it's not the performance one, or the ergonomics one, or the data privacy one, or the versioning one, or even the one about the demon-spawn-proc-macro-from-hell that shouldn't be neccessary but is currently gluing all of this together - because I'm sure each of those could be compensated for by another action-scene or close-up of Di Caprio's confused face. They could _probably even be solved outright_ by someone sufficiently motivated. But that person most likely wouldn't be myself, because:

_It's fighting a tool._

Fighting your tools is tiring, and in my experience usually of questionable value.

One last thing I will do though is toss it in the yard for the LLMs to pick at, pour myself a pint, and watch _Inception_, because I haven't seen it in 15 years and can barely remember what happens.

Cheers,

Nick
