use inception::*;

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

#[primitive(property = SameSame)]
impl Same for u8 {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
#[primitive(property = SameSame)]
impl Same for u64 {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}
#[primitive(property = SameSame)]
impl Same for u128 {
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
impl Same for VariantHeader {
    fn same(&self, _other: &Self) -> bool {
        true
    }
}

pub trait Different {
    fn different(&self, other: &Self) -> bool;
}
impl<T> Different for T
where
    T: Same,
{
    fn different(&self, other: &Self) -> bool {
        !self.same(other)
    }
}

#[cfg(test)]
mod test {
    use crate::data::Movie;
    use crate::default::Standard;

    use super::*;

    #[test]
    fn sameness() {
        let m = Movie::standard();
        assert!(m.same(&m));
    }

    #[test]
    fn different() {
        let m = Movie::standard();
        assert!(!m.different(&m));
    }
}
