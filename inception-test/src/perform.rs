use inception::*;

#[inception(property = Perform, signature(input = Input, output = Output))]
pub trait Performer<Input> {
    type Output;

    fn perform(input: Input) -> Self::Output;

    fn nothing(input: Input) -> Input {
        input
    }

    fn merge<H: Performer<Input>, R: Performer<<H as Performer<Input>>::Output>>(
        _l: L,
        _r: R,
        input: Input,
    ) -> <R as Performer<<H as Performer<Input>>::Output>>::Output {
        let next = <H as Performer<_>>::perform(input);
        <R as Performer<_>>::perform(next)
    }

    fn merge_variant_field<H: Performer<Input>, R: Performer<<H as Performer<Input>>::Output>>(
        _l: L,
        _r: R,
        input: Input,
    ) -> <R as Performer<<H as Performer<Input>>::Output>>::Output {
        let next = <H as Performer<_>>::perform(input);
        <R as Performer<_>>::perform(next)
    }

    fn join<F: Performer<Input>>(_fields: F, input: Input) -> <F as Performer<Input>>::Output {
        <F as Performer<_>>::perform(input)
    }
}

pub struct ToText;
pub struct TextLen;
pub struct AddOne;

#[primitive(property = Perform)]
impl Performer<u32> for ToText {
    type Output = String;

    fn perform(input: u32) -> Self::Output {
        format!("{input}")
    }
}

#[primitive(property = Perform)]
impl Performer<String> for TextLen {
    type Output = usize;

    fn perform(input: String) -> Self::Output {
        input.len()
    }
}

#[primitive(property = Perform)]
impl Performer<usize> for AddOne {
    type Output = usize;

    fn perform(input: usize) -> Self::Output {
        input + 1
    }
}

#[derive(Inception)]
#[inception(properties = [Perform])]
pub struct Pipeline {
    _a: ToText,
    _b: TextLen,
}

#[derive(Inception)]
#[inception(properties = [Perform])]
pub struct NestedPipeline {
    _head: Pipeline,
    _tail: AddOne,
}

#[inception(property = PerformRef, signature(input = Input, output = Output))]
pub trait RefPerformer<Input> {
    type Output;

    fn perform_ref(&self, input: Input) -> Self::Output;

    fn nothing(input: Input) -> Input {
        input
    }

    fn merge<H: RefPerformer<Input>, R: RefPerformer<<H as RefPerformer<Input>>::Output>>(
        l: L,
        r: R,
        input: Input,
    ) -> <R as RefPerformer<<H as RefPerformer<Input>>::Output>>::Output {
        let next = <H as RefPerformer<_>>::perform_ref(l.access(), input);
        <R as RefPerformer<_>>::perform_ref(&r, next)
    }

    fn merge_variant_field<H, R>(_l: L, _r: R, input: Input) -> Input {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F: RefPerformer<Input>>(f: F, input: Input) -> <F as RefPerformer<Input>>::Output {
        <F as RefPerformer<_>>::perform_ref(&f, input)
    }
}

pub struct RefToText;
pub struct RefLen;

#[primitive(property = PerformRef)]
impl RefPerformer<u32> for RefToText {
    type Output = String;

    fn perform_ref(&self, input: u32) -> Self::Output {
        format!("{input}")
    }
}

#[primitive(property = PerformRef)]
impl RefPerformer<String> for RefLen {
    type Output = usize;

    fn perform_ref(&self, input: String) -> Self::Output {
        input.len()
    }
}

#[derive(Inception)]
#[inception(properties = [PerformRef])]
pub struct RefPipeline {
    _a: RefToText,
    _b: RefLen,
}

#[inception(property = PerformMut, signature(input = Input, output = Output))]
pub trait MutPerformer<Input> {
    type Output;

    fn perform_mut(&mut self, input: Input) -> Self::Output;

    fn nothing(input: Input) -> Input {
        input
    }

    fn merge<H: MutPerformer<Input>, R: MutPerformer<<H as MutPerformer<Input>>::Output>>(
        l: L,
        mut r: R,
        input: Input,
    ) -> <R as MutPerformer<<H as MutPerformer<Input>>::Output>>::Output {
        let next = <H as MutPerformer<_>>::perform_mut(l.access(), input);
        <R as MutPerformer<_>>::perform_mut(&mut r, next)
    }

    fn merge_variant_field<H, R>(_l: L, _r: R, input: Input) -> Input {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F: MutPerformer<Input>>(mut f: F, input: Input) -> <F as MutPerformer<Input>>::Output {
        <F as MutPerformer<_>>::perform_mut(&mut f, input)
    }
}

pub struct MutToText;
pub struct MutLen;

#[primitive(property = PerformMut)]
impl MutPerformer<u32> for MutToText {
    type Output = String;

    fn perform_mut(&mut self, input: u32) -> Self::Output {
        format!("{input}")
    }
}

#[primitive(property = PerformMut)]
impl MutPerformer<String> for MutLen {
    type Output = usize;

    fn perform_mut(&mut self, input: String) -> Self::Output {
        input.len()
    }
}

#[derive(Inception)]
#[inception(properties = [PerformMut])]
pub struct MutPipeline {
    _a: MutToText,
    _b: MutLen,
}

#[inception(property = PerformOwn, signature(input = Input, output = Output))]
pub trait OwnPerformer<Input> {
    type Output;

    fn perform_own(self, input: Input) -> Self::Output;

    fn nothing(input: Input) -> Input {
        input
    }

    fn merge<H: OwnPerformer<Input>, R: OwnPerformer<<H as OwnPerformer<Input>>::Output>>(
        l: L,
        r: R,
        input: Input,
    ) -> <R as OwnPerformer<<H as OwnPerformer<Input>>::Output>>::Output {
        let next = <H as OwnPerformer<_>>::perform_own(l.access(), input);
        <R as OwnPerformer<_>>::perform_own(r, next)
    }

    fn merge_variant_field<H, R>(_l: L, _r: R, input: Input) -> Input {
        let _ = (_l, _r);
        let _ = core::marker::PhantomData::<(H, R)>;
        input
    }

    fn join<F: OwnPerformer<Input>>(f: F, input: Input) -> <F as OwnPerformer<Input>>::Output {
        <F as OwnPerformer<_>>::perform_own(f, input)
    }
}

pub struct OwnToText;
pub struct OwnLen;

#[primitive(property = PerformOwn)]
impl OwnPerformer<u32> for OwnToText {
    type Output = String;

    fn perform_own(self, input: u32) -> Self::Output {
        format!("{input}")
    }
}

#[primitive(property = PerformOwn)]
impl OwnPerformer<String> for OwnLen {
    type Output = usize;

    fn perform_own(self, input: String) -> Self::Output {
        input.len()
    }
}

#[derive(Inception)]
#[inception(properties = [PerformOwn])]
pub struct OwnPipeline {
    _a: OwnToText,
    _b: OwnLen,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn typed_flow() {
        let out = Pipeline::perform(42_u32);
        let _: usize = out;
        assert_eq!(out, 2);
    }

    #[test]
    fn nested_typed_flow() {
        let out = NestedPipeline::perform(42_u32);
        let _: usize = out;
        assert_eq!(out, 3);
    }

    #[test]
    fn ref_typed_flow() {
        let p = RefPipeline {
            _a: RefToText,
            _b: RefLen,
        };
        let out = p.perform_ref(42_u32);
        let _: usize = out;
        assert_eq!(out, 2);
    }

    #[test]
    fn mut_typed_flow() {
        let mut p = MutPipeline {
            _a: MutToText,
            _b: MutLen,
        };
        let out = p.perform_mut(42_u32);
        let _: usize = out;
        assert_eq!(out, 2);
    }

    #[test]
    fn own_typed_flow() {
        let p = OwnPipeline {
            _a: OwnToText,
            _b: OwnLen,
        };
        let out = p.perform_own(42_u32);
        let _: usize = out;
        assert_eq!(out, 2);
    }
}
