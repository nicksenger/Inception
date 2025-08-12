use inception::*;

#[inception(property = Def)]
pub trait Standard {
    fn standard() -> Self;

    fn nothing() -> List<()> {
        List(())
    }
    fn merge<H: Standard<Ret = H>, R: Standard<Ret = <F as Fields>::Owned>>(
        _l: L,
        _r: R,
    ) -> <Self as Fields>::Owned {
        List((H::standard().into(), R::standard()))
    }
    fn merge_variant_field<H: Standard<Ret = H>, R: Standard<Ret = <F as Fields>::Owned>>(
        _l: L,
        _r: R,
    ) -> <Self as Fields>::Owned {
        List((VarOwnedField::new(H::standard()), R::standard()))
    }
    fn join<F: Standard<Ret = <T as Inception<Def>>::OwnedFields>>(_fields: F) -> Self {
        <Self as Inception<Def>>::from_fields(F::standard())
    }
}

#[primitive(property = Def)]
impl Standard for u8 {
    fn standard() -> Self {
        0
    }
}
#[primitive(property = Def)]
impl Standard for u64 {
    fn standard() -> Self {
        0
    }
}
#[primitive(property = Def)]
impl Standard for u128 {
    fn standard() -> Self {
        0
    }
}
#[primitive(property = Def)]
impl Standard for String {
    fn standard() -> Self {
        Default::default()
    }
}
#[primitive(property = Def)]
impl Standard for VariantHeader {
    fn standard() -> Self {
        VariantHeader
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::data::Movie;

    #[test]
    fn default() {
        let _s = Movie::standard();
    }
}
