use inception::*;

use crate::default::Default;
use crate::eq::SameSame;
use crate::hash::Digestible;

#[inception_derive(properties = [Default, Digestible])]
struct Wrapped {
    id: u64,
    name: String,
}

#[inception_derive(properties = [SameSame])]
struct GenericWrapped<T> {
    left: T,
    right: T,
}

#[cfg(test)]
mod test {
    use std::hash::Hasher;

    use crate::default::Standard;
    use crate::eq::Same;
    use crate::hash::Digest;

    use super::*;

    #[test]
    fn helper_applies_inception_and_properties() {
        use std::hash::DefaultHasher;

        let data = Wrapped::standard();
        let mut hasher = DefaultHasher::new();
        data.digest(&mut hasher);
        let _ = hasher.finish();
    }

    #[test]
    fn helper_supports_generic_types() {
        let pair = GenericWrapped {
            left: 7_u64,
            right: 11_u64,
        };
        assert!(pair.same(&pair));
    }
}
