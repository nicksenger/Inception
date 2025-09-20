use inception::*;

#[inception(property = DupeRef)]
pub trait Duplicate {
    fn dupe(&self) -> Self;

    fn nothing() -> List<()> {
        List(())
    }
    fn merge<H: Duplicate<Ret = H>, R: Duplicate<Ret = <F as Fields>::Owned>>(
        l: L,
        r: R,
    ) -> <Self as Fields>::Owned {
        List((l.access().dupe().into(), r.dupe()))
    }
    fn merge_variant_field<H: Duplicate<Ret = H>, R: Duplicate<Ret = <F as Fields>::Owned>>(
        l: L,
        r: R,
    ) -> <Self as Fields>::Owned {
        match l.try_access() {
            Ok(f) => List((VarOwnedField::new(f.dupe()), r.dupe())),
            Err(RefEnumAccessError::Header(d)) => {
                List((VarOwnedField::header(d.access().dupe()), r.dupe()))
            }
            Err(_) => List((VarOwnedField::empty(), r.dupe())),
        }
    }
    fn join<F: Duplicate<Ret = <T as Inception<DupeRef>>::OwnedFields>>(fields: F) -> Self {
        <Self as Inception<DupeRef>>::from_fields(fields.dupe())
    }
}

#[primitive(property = DupeRef)]
impl Duplicate for u8 {
    fn dupe(&self) -> Self {
        *self
    }
}
#[primitive(property = DupeRef)]
impl Duplicate for u64 {
    fn dupe(&self) -> Self {
        *self
    }
}
#[primitive(property = DupeRef)]
impl Duplicate for u128 {
    fn dupe(&self) -> Self {
        *self
    }
}
#[primitive(property = DupeRef)]
impl Duplicate for String {
    fn dupe(&self) -> Self {
        self.clone()
    }
}
#[primitive(property = DupeRef)]
impl Duplicate for VariantHeader {
    fn dupe(&self) -> Self {
        VariantHeader
    }
}

#[inception(property = DupeMut)]
trait MutDupe {
    fn dupe_mut(&mut self) -> Self;

    fn nothing() -> List<()> {
        List(())
    }
    fn merge<H: MutDupe<Ret = H>, R: MutDupe<Ret = <F as Fields>::Owned>>(
        l: L,
        r: R,
    ) -> <Self as Fields>::Owned {
        List((l.access().dupe_mut().into(), r.dupe_mut()))
    }
    fn merge_variant_field<H: MutDupe<Ret = H>, R: MutDupe<Ret = <F as Fields>::Owned>>(
        l: L,
        r: R,
    ) -> <Self as Fields>::Owned {
        match l.try_access() {
            Ok(f) => List((VarOwnedField::new(f.dupe_mut()), r.dupe_mut())),
            Err(MutEnumAccessError::Header(d)) => {
                List((VarOwnedField::header(d.access().dupe_mut()), r.dupe_mut()))
            }
            Err(_) => List((VarOwnedField::empty(), r.dupe_mut())),
        }
    }
    fn join<F: MutDupe<Ret = <T as Inception<DupeMut>>::OwnedFields>>(fields: F) -> Self {
        <T as Inception<DupeMut>>::from_fields(fields.dupe_mut())
    }
}

#[primitive(property = DupeMut)]
impl MutDupe for u8 {
    fn dupe_mut(&mut self) -> Self {
        *self
    }
}
#[primitive(property = DupeMut)]
impl MutDupe for u64 {
    fn dupe_mut(&mut self) -> Self {
        *self
    }
}
#[primitive(property = DupeMut)]
impl MutDupe for u128 {
    fn dupe_mut(&mut self) -> Self {
        *self
    }
}
#[primitive(property = DupeMut)]
impl MutDupe for String {
    fn dupe_mut(&mut self) -> Self {
        self.clone()
    }
}
#[primitive(property = DupeMut)]
impl MutDupe for VariantHeader {
    fn dupe_mut(&mut self) -> Self {
        VariantHeader
    }
}

#[inception(property = DupeOwned)]
trait OwnDupe {
    fn dupe_owned(self) -> Self;

    fn nothing() -> List<()> {
        List(())
    }
    fn merge<H: OwnDupe<Ret = H>, R: OwnDupe<Ret = <F as Fields>::Owned>>(
        l: L,
        r: R,
    ) -> <Self as Fields>::Owned {
        List((l.access().dupe_owned().into(), r.dupe_owned()))
    }
    fn merge_variant_field<H: OwnDupe<Ret = H>, R: OwnDupe<Ret = <F as Fields>::Owned>>(
        l: L,
        r: R,
    ) -> <Self as Fields>::Owned {
        match l.try_access() {
            Ok(f) => List((VarOwnedField::new(f.dupe_owned()), r.dupe_owned())),
            Err(OwnedEnumAccessError::Header(d)) => List((
                VarOwnedField::header(d.access().dupe_owned()),
                r.dupe_owned(),
            )),
            Err(_) => List((VarOwnedField::empty(), r.dupe_owned())),
        }
    }
    fn join<F: OwnDupe<Ret = <T as Inception<DupeOwned>>::OwnedFields>>(fields: F) -> Self {
        <T as Inception<DupeOwned>>::from_fields(fields.dupe_owned())
    }
}

#[primitive(property = DupeOwned)]
impl OwnDupe for u8 {
    fn dupe_owned(self) -> Self {
        self
    }
}
#[primitive(property = DupeOwned)]
impl OwnDupe for u64 {
    fn dupe_owned(self) -> Self {
        self
    }
}
#[primitive(property = DupeOwned)]
impl OwnDupe for u128 {
    fn dupe_owned(self) -> Self {
        self
    }
}
#[primitive(property = DupeOwned)]
impl OwnDupe for String {
    fn dupe_owned(self) -> Self {
        self.clone()
    }
}
#[primitive(property = DupeOwned)]
impl OwnDupe for VariantHeader {
    fn dupe_owned(self) -> Self {
        VariantHeader
    }
}

#[test]
fn clone() {
    use crate::data::Movie;
    use crate::default::Standard;
    let mut data = Movie::standard();

    data.dupe_mut();
    data.dupe();
    data.dupe_owned();
}
