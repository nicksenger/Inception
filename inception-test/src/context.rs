use inception::*;

#[inception(property = CtxArrow, signature(input = In, output = Out))]
pub trait CtxCombinator<Ctx = ()> {
    type In;
    type Out;

    fn forward(&mut self, ctx: &mut Ctx, input: Self::In) -> Self::Out;

    fn nothing(input: Self::In) -> Self::In {
        input
    }

    fn merge<H, R>(l: H, r: R, ctx: &mut Ctx, input: Self::In) -> <R as CtxCombinator<Ctx>>::Out
    where
        H: CtxCombinator<Ctx, In = Self::In>,
        R: CtxCombinator<Ctx, In = <H as CtxCombinator<Ctx>>::Out>,
    {
        let next = <H as CtxCombinator<Ctx>>::forward(l.access(), ctx, input);
        <R as CtxCombinator<Ctx>>::forward(r, ctx, next)
    }

    fn merge_variant_field<H, R>(_l: H, _r: R, ctx: &mut Ctx, input: Self::In) -> Self::In {
        let _ = (_l, _r, ctx);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F>(fields: F, ctx: &mut Ctx, input: Self::In) -> <F as CtxCombinator<Ctx>>::Out
    where
        F: CtxCombinator<Ctx, In = Self::In>,
    {
        <F as CtxCombinator<Ctx>>::forward(fields, ctx, input)
    }
}

#[derive(Default)]
struct CountCtx {
    n: i32,
}

struct BumpAdd;
#[primitive(property = CtxArrow)]
impl CtxCombinator<CountCtx> for BumpAdd {
    type In = i32;
    type Out = i32;

    fn forward(&mut self, ctx: &mut CountCtx, input: Self::In) -> Self::Out {
        ctx.n += 1;
        input + ctx.n
    }
}

struct TimesTwo;
#[primitive(property = CtxArrow)]
impl<Ctx> CtxCombinator<Ctx> for TimesTwo {
    type In = i32;
    type Out = i32;

    fn forward(&mut self, _ctx: &mut Ctx, input: Self::In) -> Self::Out {
        input * 2
    }
}

#[derive(Inception)]
#[inception(properties = [CtxArrow])]
struct CtxPipeline(BumpAdd, TimesTwo);

#[test]
fn test_ctx_assoc_input_with_trait_generic() {
    let mut flow = CtxPipeline(BumpAdd, TimesTwo);
    let mut ctx = CountCtx::default();

    let a = flow.forward(&mut ctx, 3);
    let b = flow.forward(&mut ctx, 3);

    assert_eq!(a, 8);
    assert_eq!(b, 10);
}
