use std::marker::PhantomData;

use inception::*;

pub struct Node<T, U>(PhantomData<T>, PhantomData<U>);
pub struct Marker<T, const N: usize>(PhantomData<T>);

#[inception(property = TypeTree)]
pub trait TypeNode {
    #[induce(
        base = List<()>,
        merge = List<(Node<Head, <Head as TypeNode>::Children>, <Tail as TypeNode>::Children)>,
        merge_variant = List<(Node<Head, <Head as TypeNode>::Children>, <Tail as TypeNode>::Children)>,
        join = <Fields as TypeNode>::Children
    )]
    type Children;

    fn noop() -> ();
    fn nothing() -> () {}
    fn merge<H: TypeNode, R: TypeNode>(_l: L, _r: R) -> () {}
    fn merge_variant_field<H: TypeNode, R: TypeNode>(_l: L, _r: R) -> () {}
    fn join<F: TypeNode>(_fields: F) -> () {}
}

#[primitive(property = TypeTree)]
impl TypeNode for u8 {
    type Children = List<()>;
    fn noop() {}
}
#[primitive(property = TypeTree)]
impl TypeNode for String {
    type Children = List<()>;
    fn noop() {}
}
#[primitive(property = TypeTree)]
impl TypeNode for bool {
    type Children = List<()>;
    fn noop() {}
}
#[primitive(property = TypeTree)]
impl TypeNode for VariantHeader {
    type Children = List<()>;
    fn noop() {}
}

#[inception(property = TypeTreeGat)]
pub trait TypeNodeGat {
    #[induce(
        base = List<()>,
        merge = List<(Node<Head, <Head as TypeNodeGat>::Children<'a>>, <Tail as TypeNodeGat>::Children<'a>)>,
        merge_variant = List<(Node<Head, <Head as TypeNodeGat>::Children<'a>>, <Tail as TypeNodeGat>::Children<'a>)>,
        join = <Fields as TypeNodeGat>::Children<'a>
    )]
    type Children<'a>
    where
        Self: 'a;

    fn noop() -> ();
    fn nothing() -> () {}
    fn merge<H: TypeNodeGat, R: TypeNodeGat>(_l: L, _r: R) -> () {}
    fn merge_variant_field<H: TypeNodeGat, R: TypeNodeGat>(_l: L, _r: R) -> () {}
    fn join<F: TypeNodeGat>(_fields: F) -> () {}
}

#[primitive(property = TypeTreeGat)]
impl TypeNodeGat for u8 {
    type Children<'a>
        = List<()>
    where
        Self: 'a;
    fn noop() {}
}
#[primitive(property = TypeTreeGat)]
impl TypeNodeGat for String {
    type Children<'a>
        = List<()>
    where
        Self: 'a;
    fn noop() {}
}
#[primitive(property = TypeTreeGat)]
impl TypeNodeGat for bool {
    type Children<'a>
        = List<()>
    where
        Self: 'a;
    fn noop() {}
}
#[primitive(property = TypeTreeGat)]
impl TypeNodeGat for VariantHeader {
    type Children<'a>
        = List<()>
    where
        Self: 'a;
    fn noop() {}
}

pub trait Oscillator {
    type Inverse: Oscillator;
}

#[inception(property = TypeTreeGatParams)]
pub trait TypeNodeGatParams {
    #[induce(
        base = List<()>,
        merge = List<(Node<Head, <Head as TypeNodeGatParams>::Children<'a, X, N>>, <Tail as TypeNodeGatParams>::Children<'a, <X as Oscillator>::Inverse, N>)>,
        merge_variant = List<(Node<Head, <Head as TypeNodeGatParams>::Children<'a, X, N>>, <Tail as TypeNodeGatParams>::Children<'a, <X as Oscillator>::Inverse, N>)>,
        join = <Fields as TypeNodeGatParams>::Children<'a, <X as Oscillator>::Inverse, N>
    )]
    type Children<'a, X, const N: usize>
    where
        X: Oscillator,
        Self: 'a;

    fn noop() -> ();
    fn nothing() -> () {}
    fn merge<H: TypeNodeGatParams, R: TypeNodeGatParams>(_l: L, _r: R) -> () {}
    fn merge_variant_field<H: TypeNodeGatParams, R: TypeNodeGatParams>(_l: L, _r: R) -> () {}
    fn join<F: TypeNodeGatParams>(_fields: F) -> () {}
}

#[primitive(property = TypeTreeGatParams)]
impl TypeNodeGatParams for u8 {
    type Children<'a, X, const N: usize>
        = List<(Marker<X, N>, &'a ())>
    where
        X: Oscillator,
        Self: 'a;
    fn noop() {}
}
#[primitive(property = TypeTreeGatParams)]
impl TypeNodeGatParams for String {
    type Children<'a, X, const N: usize>
        = List<(Marker<X, N>, &'a ())>
    where
        X: Oscillator,
        Self: 'a;
    fn noop() {}
}
#[primitive(property = TypeTreeGatParams)]
impl TypeNodeGatParams for bool {
    type Children<'a, X, const N: usize>
        = List<(Marker<X, N>, &'a ())>
    where
        X: Oscillator,
        Self: 'a;
    fn noop() {}
}
#[primitive(property = TypeTreeGatParams)]
impl TypeNodeGatParams for VariantHeader {
    type Children<'a, X, const N: usize>
        = List<(Marker<X, N>, &'a ())>
    where
        X: Oscillator,
        Self: 'a;
    fn noop() {}
}

#[inception(property = BehaviorlessTypeTree, types)]
pub trait BehaviorlessTypeNode {
    #[induce(
        base = List<()>,
        merge = List<(Node<Head, <Head as BehaviorlessTypeNode>::Children>, <Tail as BehaviorlessTypeNode>::Children)>,
        merge_variant = List<(Node<Head, <Head as BehaviorlessTypeNode>::Children>, <Tail as BehaviorlessTypeNode>::Children)>,
        join = <Fields as BehaviorlessTypeNode>::Children
    )]
    type Children;
}

#[primitive(property = BehaviorlessTypeTree)]
impl BehaviorlessTypeNode for u8 {
    type Children = List<()>;
}
#[primitive(property = BehaviorlessTypeTree)]
impl BehaviorlessTypeNode for String {
    type Children = List<()>;
}
#[primitive(property = BehaviorlessTypeTree)]
impl BehaviorlessTypeNode for bool {
    type Children = List<()>;
}
#[primitive(property = BehaviorlessTypeTree)]
impl BehaviorlessTypeNode for VariantHeader {
    type Children = List<()>;
}

pub trait MyBound {}
#[inception(property = BoundTypeTree, types)]
pub trait BoundTypeNode {
    #[induce(
        base = List<()>,
        merge = List<(Node<Head, <Head as BoundTypeNode>::Children>, <Tail as BoundTypeNode>::Children)> where { Head: MyBound, Tail: MyBound },
        merge_variant = List<(Node<Head, <Head as BoundTypeNode>::Children>, <Tail as BoundTypeNode>::Children)> where { Head: MyBound, Tail: MyBound },
        join = <Fields as BoundTypeNode>::Children where { Fields: MyBound }
    )]
    type Children;
}

#[primitive(property = BoundTypeTree)]
impl BoundTypeNode for u8 {
    type Children = List<()>;
}
impl MyBound for u8 {}
#[primitive(property = BoundTypeTree)]
impl BoundTypeNode for String {
    type Children = List<()>;
}
impl MyBound for String {}
#[primitive(property = BoundTypeTree)]
impl BoundTypeNode for bool {
    type Children = List<()>;
}
impl MyBound for bool {}
#[primitive(property = BoundTypeTree)]
impl BoundTypeNode for VariantHeader {
    type Children = List<()>;
}
impl MyBound for VariantHeader {}
impl MyBound for List<()> {}
impl<H, S, const IDX: usize, F> MyBound for List<(TyField<H, S, IDX>, F)>
where
    H: MyBound,
    F: MyBound,
{
}
impl<H, S, const VAR_IDX: usize, const IDX: usize, F> MyBound
    for List<(VarTyField<H, S, VAR_IDX, IDX>, F)>
where
    H: MyBound,
    F: MyBound,
{
}

pub trait NoBlanketPrimitive {}
#[inception(property = NoBlanketTypeTree, types, no_blanket)]
pub trait NoBlanketTypeNode {
    #[induce(
        base = List<()>,
        merge = List<(Node<Head, <Head as NoBlanketTypeNode>::Children>, <Tail as NoBlanketTypeNode>::Children)>,
        merge_variant = List<(Node<Head, <Head as NoBlanketTypeNode>::Children>, <Tail as NoBlanketTypeNode>::Children)>,
        join = <Fields as NoBlanketTypeNode>::Children
    )]
    type Children;
}

impl<T> NoBlanketTypeNode for T
where
    T: NoBlanketPrimitive,
{
    type Children = List<()>;
}

pub struct NoBlanketLeaf;
impl NoBlanketPrimitive for NoBlanketLeaf {}

pub trait Animal {}

pub struct Cat;
pub struct Dog;

impl Animal for Cat {}
impl Animal for Dog {}

pub trait EffectSchema {}

pub struct Hybrid;
impl Animal for Hybrid {}
impl EffectSchema for Hybrid {}

pub struct AnimalOnly;
impl Animal for AnimalOnly {}

pub struct EffectOnly;
impl EffectSchema for EffectOnly {}

pub struct DomainPartitionedPrimitive;
impl inception::Property for DomainPartitionedPrimitive {}

pub struct AnimalPrimitiveDomain;
pub struct EffectPrimitiveDomain;

impl<T> inception::Compat<T, AnimalPrimitiveDomain> for DomainPartitionedPrimitive
where
    T: Animal,
{
    type Out = inception::True;
}

impl<T> inception::Compat<T, EffectPrimitiveDomain> for DomainPartitionedPrimitive
where
    T: EffectSchema,
{
    type Out = inception::True;
}

#[inception(property = BlanketTypeTree, types)]
pub trait BlanketTypeNode {
    #[induce(
        base = List<()>,
        merge = List<(Node<Head, <Head as BlanketTypeNode>::Children>, <Tail as BlanketTypeNode>::Children)>,
        merge_variant = List<(Node<Head, <Head as BlanketTypeNode>::Children>, <Tail as BlanketTypeNode>::Children)>,
        join = <Fields as BlanketTypeNode>::Children
    )]
    type Children;
}

impl<T> inception::Compat<T> for BlanketTypeTree
where
    T: Animal,
{
    type Out = inception::True;
}

impl BlanketTypeNode for Cat {
    type Children = List<()>;
}

impl BlanketTypeNode for Dog {
    type Children = List<()>;
}

#[cfg(test)]
mod test {
    use super::*;

    pub trait Same<Rhs = Self> {
        /// Should always be `Self`
        type Output;
    }

    impl<T> Same<T> for T {
        type Output = T;
    }
    macro_rules! assert_type_eq {
        ($a:ty, $b:ty) => {
            const _: core::marker::PhantomData<<$a as Same<$b>>::Output> =
                core::marker::PhantomData;
        };
    }

    #[derive(Inception)]
    #[inception(properties = [TypeTree])]
    struct MyTree {
        foo: String,
        bar: bool,
        baz: u8,
    }

    #[test]
    fn induced_types() {
        assert_type_eq!(
            <MyTree as TypeNode>::Children,
            List<(
                Node<String, List<()>>,
                List<(Node<bool, List<()>>, List<(Node<u8, List<()>>, List<()>)>)>
            )>
        );
    }

    #[derive(Inception)]
    #[inception(properties = [TypeTreeGat])]
    struct MyTreeGat {
        foo: String,
        bar: bool,
        baz: u8,
    }

    #[test]
    fn induced_gat_types() {
        assert_type_eq!(
            <MyTreeGat as TypeNodeGat>::Children<'static>,
            List<(
                Node<String, List<()>>,
                List<(Node<bool, List<()>>, List<(Node<u8, List<()>>, List<()>)>)>
            )>
        );
    }

    #[derive(Inception)]
    #[inception(properties = [TypeTreeGatParams])]
    struct MyTreeGatParams {
        foo: String,
        bar: bool,
        baz: u8,
    }

    impl Oscillator for u16 {
        type Inverse = u32;
    }
    impl Oscillator for u32 {
        type Inverse = u16;
    }

    #[test]
    fn induced_gat_type_and_const_types() {
        assert_type_eq!(
            <MyTreeGatParams as TypeNodeGatParams>::Children<'static, u16, 7>,
            List<(
                Node<String, List<(Marker<u32, 7>, &'static ())>>,
                List<(
                    Node<bool, List<(Marker<u16, 7>, &'static ())>>,
                    List<(Node<u8, List<(Marker<u32, 7>, &'static ())>>, List<()>)>
                )>
            )>
        );
    }

    #[derive(Inception)]
    #[inception(properties = [BehaviorlessTypeTree])]
    struct MyBehaviorlessTypeTree {
        foo: String,
        bar: bool,
        baz: u8,
    }

    #[test]
    fn induced_behaviorless_types() {
        assert_type_eq!(
            <MyBehaviorlessTypeTree as BehaviorlessTypeNode>::Children,
            List<(
                Node<String, List<()>>,
                List<(Node<bool, List<()>>, List<(Node<u8, List<()>>, List<()>)>)>
            )>
        );
    }

    #[test]
    fn no_blanket_allows_trait_bounded_primitive_impl() {
        assert_type_eq!(<NoBlanketLeaf as NoBlanketTypeNode>::Children, List<()>);
    }

    #[derive(Inception)]
    #[inception(properties = [BlanketTypeTree])]
    struct AnimalPack {
        cat: Cat,
        dog: Dog,
    }

    #[test]
    fn blanket_compat_selects_primitives() {
        assert_type_eq!(
            <AnimalPack as BlanketTypeNode>::Children,
            List<(Node<Cat, List<()>>, List<(Node<Dog, List<()>>, List<()>)>)>
        );
    }

    #[test]
    fn domain_partitioned_blanket_primitives_supported() {
        assert_type_eq!(
            <DomainPartitionedPrimitive as inception::Compat<AnimalOnly, AnimalPrimitiveDomain>>::Out,
            inception::True
        );
        assert_type_eq!(
            <DomainPartitionedPrimitive as inception::Compat<EffectOnly, EffectPrimitiveDomain>>::Out,
            inception::True
        );
        assert_type_eq!(
            <DomainPartitionedPrimitive as inception::Compat<Hybrid, AnimalPrimitiveDomain>>::Out,
            inception::True
        );
        assert_type_eq!(
            <DomainPartitionedPrimitive as inception::Compat<Hybrid, EffectPrimitiveDomain>>::Out,
            inception::True
        );
    }
}
