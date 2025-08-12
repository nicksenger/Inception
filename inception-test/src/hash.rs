use inception::*;

#[inception(property = Digestible)]
pub trait Digest {
    fn digest(&self, state: &mut std::hash::DefaultHasher) -> ();

    fn nothing() -> bool {
        true
    }
    fn merge<H: Digest, R: Digest>(l: L, r: R, state: &mut std::hash::DefaultHasher) -> () {
        l.access().digest(state);
        r.digest(state);
    }
    fn merge_variant_field<H: Digest, R: Digest>(
        l: L,
        r: R,
        state: &mut std::hash::DefaultHasher,
    ) -> () {
        if let Ok(value) = l.try_access() {
            value.digest(state);
        }
        r.digest(state);
    }
    fn join<F: Digest>(fields: F, state: &mut std::hash::DefaultHasher) -> () {
        fields.digest(state);
    }
}

use std::hash::Hash;
#[primitive(property = Digestible)]
impl Digest for u8 {
    fn digest(&self, state: &mut std::hash::DefaultHasher) {
        use std::hash::Hash;
        self.hash(state);
    }
}
#[primitive(property = Digestible)]
impl Digest for u64 {
    fn digest(&self, state: &mut std::hash::DefaultHasher) {
        use std::hash::Hash;
        self.hash(state);
    }
}
#[primitive(property = Digestible)]
impl Digest for u128 {
    fn digest(&self, state: &mut std::hash::DefaultHasher) {
        use std::hash::Hash;
        self.hash(state);
    }
}
#[primitive(property = Digestible)]
impl Digest for String {
    fn digest(&self, state: &mut std::hash::DefaultHasher) {
        self.hash(state);
    }
}
#[primitive(property = Digestible)]
impl Digest for VariantHeader {
    fn digest(&self, _state: &mut std::hash::DefaultHasher) {}
}

#[cfg(test)]
mod test {
    use std::hash::Hasher;

    use crate::data::{Actor, Movie};
    use crate::default::Standard;

    use super::*;

    #[test]
    fn digest() {
        use std::hash::DefaultHasher;
        let mut h = DefaultHasher::new();
        let inception = Movie::standard();
        inception.digest(&mut h);
        let a = h.finish();

        let mut h = DefaultHasher::new();
        let leo = Actor::standard();
        leo.digest(&mut h);
        let b = h.finish();

        assert_ne!(a, b);
    }
}
