use inception::{field::VarField, meta::VariantMeta, *};

#[inception(property = Ty)]
trait DiagTy {
    fn diag() -> String;

    fn nothing() -> String {
        Default::default()
    }

    fn merge<H: DiagTy<Ret = String>, R: DiagTy<Ret = String>>(_t: L, _f: R) -> String {
        let l = <H as __inception_diag_ty::Inductive>::diag().to_string();
        let r = <R as __inception_diag_ty::Inductive>::diag();
        format!("{l}\n{r}")
    }

    fn merge_variant_field<H: DiagTy<Ret = String>, R: DiagTy<Ret = String>>(
        _t: L,
        _f: R,
    ) -> String {
        let l = format!(
            "{}: {}",
            <L as Field>::IDX,
            <H as __inception_diag_ty::Inductive>::diag()
        );
        let r = <R as __inception_diag_ty::Inductive>::diag();
        format!("{l}\n{r}")
    }

    fn join<F: DiagTy<Ret = String>>(_f: F) -> String {
        let content = <F as __inception_diag_ty::Inductive>::diag()
            .lines()
            .map(|l| format!("   {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{} {{\n{content}\n}}", Self::NAME)
    }
}

#[primitive(property = Ty)]
impl DiagTy for u8 {
    fn diag() -> String {
        "u8".to_string()
    }
}
#[primitive(property = Ty)]
impl DiagTy for u64 {
    fn diag() -> String {
        "u64".to_string()
    }
}
#[primitive(property = Ty)]
impl DiagTy for u128 {
    fn diag() -> String {
        "u128".to_string()
    }
}
#[primitive(property = Ty)]
impl DiagTy for String {
    fn diag() -> String {
        "String".to_string()
    }
}
#[primitive(property = Ty)]
impl DiagTy for VariantHeader {
    fn diag() -> String {
        "__variant__".to_string()
    }
}

#[inception(property = Ref)]
pub trait DiagRef {
    fn print(&self) -> String;

    fn nothing() -> String {
        Default::default()
    }

    fn merge<H: DiagRef<Ret = String>, R: DiagRef<Ret = String>>(l: L, r: R) -> String {
        let l = format!("{}: {}", <L as Field>::IDX, l.access().print());
        let r = r.print();
        format!("{l}\n{r}")
    }

    fn merge_variant_field<H: DiagRef<Ret = String>, R: DiagRef<Ret = String>>(
        l: L,
        r: R,
    ) -> String {
        let l = format!(
            "{}: {}",
            <L as Field>::IDX,
            match l.try_access() {
                Ok(f) => format!(
                    "{}: {}",
                    <L as VariantMeta>::VARIANT_FIELD_NAMES
                        .get(<L as Field>::IDX)
                        .unwrap_or(&"unnamed"),
                    f.print()
                ),
                Err(RefEnumAccessError::Header(_)) =>
                    format!("[[{}]]", <L as VariantMeta>::VARIANT_NAME),
                Err(RefEnumAccessError::EmptyField(_)) => <L as VariantMeta>::VARIANT_FIELD_NAMES
                    .get(<L as Field>::IDX)
                    .unwrap_or(&"unnamed")
                    .to_string(),
            }
        );
        let r = r.print();
        format!("{l}\n{r}")
    }

    fn join<F: DiagRef<Ret = String>>(f: F) -> String {
        let content = f
            .print()
            .lines()
            .map(|l| format!("   {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{} {{\n{content}\n}}", Self::NAME)
    }
}

#[primitive(property = Ref)]
impl DiagRef for u8 {
    fn print(&self) -> String {
        self.to_string()
    }
}
#[primitive(property = Ref)]
impl DiagRef for u64 {
    fn print(&self) -> String {
        self.to_string()
    }
}
#[primitive(property = Ref)]
impl DiagRef for u128 {
    fn print(&self) -> String {
        self.to_string()
    }
}
#[primitive(property = Ref)]
impl DiagRef for String {
    fn print(&self) -> String {
        self.clone()
    }
}
#[primitive(property = Ref)]
impl DiagRef for VariantHeader {
    fn print(&self) -> String {
        "__variant__".to_string()
    }
}

#[cfg(test)]
mod test {
    use super::{DiagTy, *};
    use crate::data::Movie;

    #[test]
    fn tydiag() {
        let s = Movie::diag();
        println!("{s}");
    }

    #[test]
    fn refdiag() {
        use crate::default::Standard;

        let data = Movie::standard();
        let s = data.print();
        println!("{s}");
    }
}
