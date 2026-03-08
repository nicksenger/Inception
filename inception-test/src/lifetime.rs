use inception::*;

#[inception(property = LifetimeCountProperty)]
trait LifetimeCountTy {
    fn count() -> usize;

    fn nothing() -> usize {
        0
    }

    fn merge<H: LifetimeCountTy<Ret = usize>, R: LifetimeCountTy<Ret = usize>>(
        _l: L,
        _r: R,
    ) -> usize {
        <H as __inception_lifetime_count_ty::Inductive>::count()
            + <R as __inception_lifetime_count_ty::Inductive>::count()
    }

    fn merge_variant_field<H: LifetimeCountTy<Ret = usize>, R: LifetimeCountTy<Ret = usize>>(
        _l: L,
        _r: R,
    ) -> usize {
        <H as __inception_lifetime_count_ty::Inductive>::count()
            + <R as __inception_lifetime_count_ty::Inductive>::count()
    }

    fn join<F: LifetimeCountTy<Ret = usize>>(_f: F) -> usize {
        <F as __inception_lifetime_count_ty::Inductive>::count()
    }
}

#[primitive(property = LifetimeCountProperty)]
impl<'a> LifetimeCountTy for &'a u8 {
    fn count() -> usize {
        1
    }
}

#[primitive(property = LifetimeCountProperty)]
impl<'a> LifetimeCountTy for &'a str {
    fn count() -> usize {
        1
    }
}

#[derive(Inception)]
#[inception(properties = [LifetimeCountProperty])]
struct BorrowedPair<'a> {
    left: &'a u8,
    right: &'a str,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn lifetime_ty_property_on_derived_struct() {
        assert_eq!(BorrowedPair::count(), 2);
    }

    #[test]
    fn lifetime_derive_handles_non_static_borrows() {
        let n = 7u8;
        let data = BorrowedPair {
            left: &n,
            right: "ok",
        };
        assert_eq!(*data.left, 7);
        assert_eq!(data.right, "ok");
    }
}
