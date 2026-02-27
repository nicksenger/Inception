use std::marker::PhantomData;

use inception::*;

#[inception(property = Arrow, signature(input = In, output = Out))]
pub trait Combinator {
    type In;
    type Out;

    fn forward(&self, input: Self::In) -> Self::Out;

    fn nothing(input: Self::In) -> Self::In {
        input
    }
    fn merge<H, R>(l: L, r: R, input: Self::In) -> <R as Combinator>::Out
    where
        H: Combinator<In = Self::In>,
        R: Combinator<In = <H as Combinator>::Out>,
    {
        let next = <H as Combinator>::forward(l.access(), input);
        <R as Combinator>::forward(r, next)
    }

    fn merge_variant_field<H, R>(_l: L, _r: R, input: Self::In) -> Self::In {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(fields: F, input: Self::In) -> <F as Combinator>::Out
    where
        F: Combinator<In = Self::In>,
    {
        <F as Combinator>::forward(fields, input)
    }
}

pub struct Identity<T>(PhantomData<T>);
#[primitive(property = Arrow)]
impl<T> Combinator for Identity<T> {
    type In = T;
    type Out = T;
    fn forward(&self, input: Self::In) -> Self::Out {
        input
    }
}
impl<T> Default for Identity<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub enum Either<Left, Right> {
    Left(Left),
    Right(Right),
}

pub struct Choose<T, U>(bool, T, U);
#[primitive(property = Arrow)]
impl<T, U> Combinator for Choose<T, U>
where
    T: Combinator,
    U: Combinator<In = <T as Combinator>::In>,
{
    type In = <T as Combinator>::In;
    type Out = Either<<T as Combinator>::Out, <U as Combinator>::Out>;
    fn forward(&self, input: Self::In) -> Self::Out {
        if self.0 {
            Either::Left(self.1.forward(input))
        } else {
            Either::Right(self.2.forward(input))
        }
    }
}
impl<T, U> Choose<T, U> {
    pub fn new(enabled: bool, on_true: T, on_false: U) -> Self {
        Self(enabled, on_true, on_false)
    }
}

pub struct JoinEither<T>(PhantomData<T>);
#[primitive(property = Arrow)]
impl<T> Combinator for JoinEither<T> {
    type In = Either<T, T>;
    type Out = T;
    fn forward(&self, input: Self::In) -> Self::Out {
        match input {
            Either::Left(a) => a,
            Either::Right(b) => b,
        }
    }
}

#[derive(Inception)]
#[inception(properties = [Arrow])]
pub struct If<Left: 'static, Right: 'static, Out: 'static>(Choose<Left, Right>, JoinEither<Out>);
impl<Left, Right, Out> If<Left, Right, Out> {
    pub fn new(enabled: bool, on_true: Left, on_false: Right) -> Self {
        Self(
            Choose::new(enabled, on_true, on_false),
            JoinEither(PhantomData),
        )
    }
}

pub struct Zip<Op1, Op2>(Op1, Op2);
#[primitive(property = Arrow)]
impl<Op1, Op2> Combinator for Zip<Op1, Op2>
where
    Op1: Combinator,
    Op2: Combinator,
{
    type In = (<Op1 as Combinator>::In, <Op2 as Combinator>::In);
    type Out = (<Op1 as Combinator>::Out, <Op2 as Combinator>::Out);
    fn forward(&self, input: Self::In) -> Self::Out {
        (self.0.forward(input.0), self.1.forward(input.1))
    }
}

impl<Op1, Op2> Zip<Op1, Op2> {
    pub fn new(left: Op1, right: Op2) -> Self {
        Self(left, right)
    }
}

pub struct Fanout<T>(PhantomData<T>);
#[primitive(property = Arrow)]
impl<T> Combinator for Fanout<T>
where
    T: Clone,
{
    type In = T;
    type Out = (T, T);
    fn forward(&self, input: Self::In) -> Self::Out {
        (input.clone(), input)
    }
}
impl<T> Default for Fanout<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

pub trait ResultLike {
    type Ok;
    type Err;
    fn resolve(self) -> Result<Self::Ok, Self::Err>;
}
impl<A, B> ResultLike for Result<A, B> {
    type Ok = A;
    type Err = B;
    fn resolve(self) -> Result<A, B> {
        self
    }
}

pub struct LiftResult<Task, In, Out>(Task, PhantomData<In>, PhantomData<Out>);
#[primitive(property = Arrow)]
impl<Task, In, Out> Combinator for LiftResult<Task, In, Out>
where
    In: ResultLike,
    Task: Combinator<In = <In as ResultLike>::Ok, Out = Result<Out, <In as ResultLike>::Err>>,
{
    type In = In;
    type Out = Result<Out, <In as ResultLike>::Err>;
    fn forward(&self, input: Self::In) -> Self::Out {
        match input.resolve() {
            Ok(x) => self.0.forward(x),
            Err(e) => Err(e),
        }
    }
}
impl<Task, In, Out> LiftResult<Task, In, Out> {
    pub fn new(t: Task) -> Self {
        Self(t, PhantomData, PhantomData)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    pub struct RefToText<T>(PhantomData<T>);
    pub struct RefCharsArr<const N: usize>;
    pub struct Exclaim;

    #[primitive(property = Arrow)]
    impl<T> Combinator for RefToText<T>
    where
        T: std::fmt::Display,
    {
        type In = T;
        type Out = String;

        fn forward(&self, input: Self::In) -> Self::Out {
            format!("{input}")
        }
    }
    impl<T> Default for RefToText<T> {
        fn default() -> Self {
            Self(PhantomData)
        }
    }

    #[primitive(property = Arrow)]
    impl<const N: usize> Combinator for RefCharsArr<N> {
        type In = String;
        type Out = Result<[char; N], Vec<char>>;

        fn forward(&self, input: Self::In) -> Self::Out {
            input.chars().collect::<Vec<_>>().try_into()
        }
    }

    #[primitive(property = Arrow)]
    impl Combinator for Exclaim {
        type In = [char; 2];
        type Out = Result<[char; 3], Vec<char>>;

        fn forward(&self, [a, b]: Self::In) -> Self::Out {
            Ok([a, b, '!'])
        }
    }

    #[derive(Inception)]
    #[inception(properties = [Arrow])]
    struct TestArrow(RefToText<u32>, Fanout<String>, Zip<SubArrow, SubArrow>);

    type TryExclaim = LiftResult<Exclaim, Result<[char; 2], Vec<char>>, [char; 3]>;

    #[derive(Inception)]
    #[inception(properties = [Arrow])]
    struct SubArrow(RefCharsArr<2>, TryExclaim);

    fn sub_arrow() -> SubArrow {
        SubArrow(RefCharsArr, LiftResult::new(Exclaim))
    }

    #[test]
    fn test_arrow() {
        let arrow = TestArrow(
            RefToText::default(),
            Fanout::default(),
            Zip::new(sub_arrow(), sub_arrow()),
        );

        let (a, b) = arrow.forward(42);
        assert_eq!(a, Ok(['4', '2', '!']));
        assert_eq!(b, Ok(['4', '2', '!']));
    }
}
