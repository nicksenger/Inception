type E = ();

#[macro_export]
macro_rules! list {
    [$(,)?] => { $crate::ty::List(()) };
    [$a:expr$(,)?] => { $crate::ty::List(($a, $crate::ty::List(()))) };
    [$a:expr,$($bs:expr),+$(,)?] => { $crate::ty::List(($a, $crate::list![$($bs),+])) };
}
#[macro_export]
macro_rules! list_ty {
    [$(,)?] => { $crate::ty::List<()> };
    [$a:ty$(,)?] => { $crate::ty::List::<($a, $crate::ty::List<()>)> };
    [$a:ty,$($bs:ty),+$(,)?] => { $crate::ty::List::<($a, $crate::list_ty![$($bs),+])> };
}

macro_rules! unbounded {
    [$a:ident] => { $crate::ty::List($a) };
    [$a:ident,$b:ident$(,)?] => { $crate::ty::List(($a, $b)) };
    [$a:ident,$($bs:ident),+$(,)?] => { $crate::ty::List(($a, unbounded![$($bs),+])) };
}
macro_rules! unbounded_ty {
    [$a:ty] => { $crate::ty::List::<$a> };
    [$a:ty,$b:ty$(,)?] => { $crate::ty::List::<($a, $b)> };
    [$a:ty,$($bs:ty),+$(,)?] => { $crate::ty::List::<($a, unbounded_ty![$($bs),+])> };
}
macro_rules! unpack {
    [$(,)?] => {
        List(())
    };
    [$id:ident $(,)? $($ids:ident),*] => {
        List(($id, unpack![$($ids),*]))
    };
}
macro_rules! padding {
    [$($n:ident,$m:ident => [$($tts:tt),*]),*] => {
        $(pub type $m = list_ty![$($tts),*];)*
        $(pub const $n: $m = list![$($tts),*];)*
    };
}
padding![
    PAD_0, Pad0 => [],
    PAD_1, Pad1 => [()],
    PAD_2, Pad2 => [(), ()],
    PAD_3, Pad3 => [(), (), ()],
    PAD_4, Pad4 => [(), (), (), ()],
    PAD_5, Pad5 => [(), (), (), (), ()],
    PAD_6, Pad6 => [(), (), (), (), (), ()],
    PAD_7, Pad7 => [(), (), (), (), (), (), ()],
    PAD_8, Pad8 => [(), (), (), (), (), (), (), ()]
];

pub trait Interleave<Rhs> {
    type Out;
    fn interleave(self, rhs: Rhs) -> Self::Out;
}

macro_rules! interleave {
    [($($a:ident)? , $($b:ident)?) -> ($($c:ident)? , $($d:ident)?)] => {
        impl< $($a)? $($b)? > Interleave<list_ty![$($b)?]> for list_ty![$($a)?] {
            type Out = list_ty![$($a)? $($b)?];
            fn interleave(self, unpack![$($d)?]: list_ty![$($b)?]) -> Self::Out {
                let unpack![$($c)?] = self;
                list![$($c)?$($d)?]
            }
        }
    };
    [([$a:ident$(,)?$($as:ident),*], [$b:ident$(,)?$($bs:ident),*]) -> ([$($cs:ident),*], [$($ds:ident),*])] => {
        impl< $a,$b,$($as,$bs),* > Interleave<list_ty![$b,$($bs),*]> for list_ty![$a,$($as),*]
            where list_ty![$($as),*]: Interleave<list_ty![$($bs),*]> {
            type Out = list_ty![$a,$b,$($as,$bs),*];
            fn interleave(self, unpack![$($ds),*]: list_ty![$b,$($bs),*]) -> Self::Out {
                let unpack![$($cs),*] = self;
                list![$($cs,$ds),*]
            }
        }
    };
    [[$a:ident$(,)?$($as:ident),* : $y:ident], [$b:ident$(,)?$($bs:ident),* : $z:ident] => [$($cs:ident),* : $ys:ident], [$($ds:ident),* : $zs:ident]] => {
        impl< $a,$b,$y,$z,$($as,$bs),* > Interleave<unbounded_ty![$b,$($bs),* , $z]> for unbounded_ty![$a,$($as),* , $y]
            where $y: Interleave<$z> {
            type Out = unbounded_ty![$a,$b,$($as,$bs),* , <$y as Interleave<$z>>::Out];
            fn interleave(self, unbounded![$($ds),* , $zs]: unbounded_ty![$b,$($bs),* , $z]) -> Self::Out {
                let unbounded![$($cs),* , $ys] = self;
                let tail = $ys.interleave($zs);
                unbounded![$($cs,$ds),* , tail]
            }
        }
    };
}
interleave![(,) -> (,)];
interleave![(,B) -> (,b)];
interleave![(A,) -> (a,)];
interleave![([A], [B]) -> ([a], [b])];
#[cfg(not(feature = "opt"))]
interleave![[A, C: E], [B, D: F] => [a, c: e], [b, d: f]];
#[cfg(feature = "opt")]
const _: () = {
    interleave![([A, C], [B, D]) -> ([a, c], [b, d])];
    interleave![([A, C, E], [B, D, F]) -> ([a, c, e], [b, d, f])];
    interleave![([A, C, E, G], [B, D, F, H]) -> ([a, c, e, g], [b, d, f, h])];
    interleave![([A, C, E, G, I], [B, D, F, H, J]) -> ([a, c, e, g, i], [b, d, f, h, j])];
    interleave![([A, C, E, G, I, K], [B, D, F, H, J, L]) -> ([a, c, e, g, i, k], [b, d, f, h, j, l])];
    interleave![([A, C, E, G, I, K, M], [B, D, F, H, J, L, N]) -> ([a, c, e, g, i, k, m], [b, d, f, h, j, l, n])];
    interleave![([A, C, E, G, I, K, M, O], [B, D, F, H, J, L, N, P]) -> ([a, c, e, g, i, k, m, o], [b, d, f, h, j, l, n, p])];
    interleave![[A, C, E, G, I, K, M, O, Q : S], [B, D, F, H, J, L, N, P, R : T] => [a, c, e, g, i, k, m, o, q: s], [b, d, f, h, j, l,n,p,r:t]];
};

pub trait Compat<T> {
    type Out: TruthValue;
}

pub struct True;
pub struct False;
pub trait TruthValue {}
impl TruthValue for True {}
impl TruthValue for False {}

pub type Nothing = List<()>;
pub struct List<T>(pub T);
impl<T, U> List<(T, U)> {
    pub fn split(self) -> (T, U) {
        (self.0 .0, self.0 .1)
    }
}

pub trait IntoTuples {
    type Left;
    type Right;
    fn into_tuples(self) -> (Self::Left, Self::Right);
}
impl IntoTuples for List<()> {
    type Left = ();
    type Right = ();
    fn into_tuples(self) -> (Self::Left, Self::Right) {
        ((), ())
    }
}
macro_rules! into_tuples {
    [([$a:ident$(,)?$($as:ident),*])] => {
        impl< $a,$($as),* > IntoTuples for list_ty![$a,$($as),*] {
            type Left = $a;
            type Right = (<list_ty![$($as),*] as IntoTuples>::Left, <list_ty![$($as),*] as IntoTuples>::Right);
            fn into_tuples(self) -> (Self::Left, Self::Right) {
                (self.0 .0, self.0 .1.into_tuples())
            }
        }
    };
    [[$a:ident,$($as:ident),+]] => {
        impl< $a,$($as),* > IntoTuples for unbounded_ty![$a,$($as),*]
        where unbounded_ty![$($as),*]: IntoTuples {
            type Left = $a;
            type Right = (<unbounded_ty![$($as),*] as IntoTuples>::Left, <unbounded_ty![$($as),*] as IntoTuples>::Right);
            fn into_tuples(self) -> (Self::Left, Self::Right) {
                (self.0 .0, self.0 .1.into_tuples())
            }
        }
    };
}
into_tuples![([A])];
#[cfg(not(feature = "opt"))]
into_tuples![[A, B, C]];
#[cfg(feature = "opt")]
const _: () = {
    into_tuples![([A, B])];
    into_tuples![([A, B, C])];
    into_tuples![([A, B, C, D])];
    into_tuples![([A, B, C, D, E])];
    into_tuples![([A, B, C, D, E, F])];
    into_tuples![([A, B, C, D, E, F, G])];
    into_tuples![([A, B, C, D, E, F, G, H])];
    into_tuples![[A, B, C, D, E, F, G, H, I, J]];
};

pub trait Pad<P> {
    type Out;
    fn pad(self, padding: P) -> Self::Out;
}
impl<T> Pad<List<()>> for List<T> {
    type Out = List<T>;
    fn pad(self, _padding: List<()>) -> Self::Out {
        self
    }
}
macro_rules! pad {
    [([$a1:ident,$a2:ident], [$b:ident,$($bs:ident),*]) -> ([$c1:ident,$c2:ident], [$d:ident,$($ds:ident),*])] => {
        impl< $a1,$a2 > Pad<list_ty![$($bs),*,$b]> for unbounded_ty![$a1,$a2] {
            type Out = unbounded_ty![$b,$($bs),*,$a1,$a2];
            fn pad(self, padding: list_ty![$($bs),*,$b]) -> Self::Out {
                let unbounded![$c1,$c2] = self;
                let unpack![$($ds),*,$d] = padding;
                unbounded![$d,$($ds),*,$c1,$c2]
            }
        }
    };
    [[$a1:ident,$a2:ident], [$b:ident,$($bs:ident),*] => [$c1:ident,$c2:ident], [$d:ident,$($ds:ident),*]] => {
        impl< $a1,$a2,$b > Pad<unbounded_ty![$($bs),*,$b]> for unbounded_ty![$a1,$a2]
        where unbounded_ty![$a1,$a2]: Pad<$b>
        {
            type Out = unbounded_ty![$($bs),*,<unbounded_ty![$a1,$a2] as Pad<$b>>::Out];
            fn pad(self, padding: unbounded_ty![$($bs),*,$b]) -> Self::Out {
                let unbounded![$($ds),*,$d] = padding;
                let p = self.pad($d);
                unbounded![$($ds),*,p]
            }
        }
    };
}
pad![([T,U], [E,E]) -> ([t,u], [a, b])];
#[cfg(not(feature = "opt"))]
pad![[T,U], [P, E, E, E] => [t,u], [a, b, c, d]];
#[cfg(feature = "opt")]
const _: () = {
    pad![([T,U], [E,E,E]) -> ([t,u], [a, b, c])];
    pad![([T,U], [E,E,E,E]) -> ([t,u], [a, b, c, d])];
    pad![([T,U], [E,E,E,E,E]) -> ([t,u], [a, b, c, d, e])];
    pad![([T,U], [E,E,E,E,E,E]) -> ([t,u], [a, b, c, d, e, f])];
    pad![([T,U], [E,E,E,E,E,E,E]) -> ([t,u], [a, b, c, d, e, f, g])];
    pad![([T,U], [E,E,E,E,E,E,E,E]) -> ([t,u], [a, b, c, d, e, f, g, h])];
    pad![[T,U], [P,E,E,E,E,E,E,E,E,E] => [t,u], [a, b, c, d, e, f, g, h, i, j]];
};

pub trait Mask<M> {
    type Out;
    fn mask(self, mask: M) -> Self::Out;
}
impl<T> Mask<List<()>> for T {
    type Out = T;
    fn mask(self, _mask: List<()>) -> Self::Out {
        self
    }
}
impl<T, U, M> Mask<List<((), M)>> for List<(T, U)>
where
    U: Mask<M>,
{
    type Out = List<(T, <U as Mask<M>>::Out)>;
    fn mask(self, mask: List<((), M)>) -> Self::Out {
        List((self.0 .0, self.0 .1.mask(mask.0 .1)))
    }
}
macro_rules! mask {
    [([$a:ident$(,)?$($as:ident),*], [$($bs:ident),*]) -> ([$c:ident,$($cs:ident),*], [$($ds:ident),*])] => {
        impl< $a,$($as,$bs),* > Mask<list_ty![$($bs),*]> for unbounded_ty![$($as),*,$a]
            where $($bs: crate::Field),* {
            type Out = unbounded_ty![$($bs),*,$a];
            fn mask(self, mask: list_ty![$($bs),*]) -> Self::Out {
                let unbounded![$($cs),*,$c] = self;
                let unpack![$($ds),*] = mask;
                unbounded![$($ds),*, $c]
            }
        }
    };
    [[$a:ident$(,)?$($as:ident),*], [$b:ident,$($bs:ident),*] => [$c:ident,$($cs:ident),*], [$d:ident,$($ds:ident),*]] => {
        impl< $a,$b,$($as,$bs),* > Mask<unbounded_ty![$($bs),*,$b]> for unbounded_ty![$($as),*,$a] where $a: Mask<$b>, $($bs: crate::Field),* {
            type Out = unbounded_ty![$($bs),*, <$a as Mask<$b>>::Out];
            fn mask(self, mask: unbounded_ty![$($bs),*,$b]) -> Self::Out {
                let unbounded![$($cs),*,$c] = self;
                let unbounded![$($ds),*,$d] = mask;
                let m = $c.mask($d);
                unbounded![$($ds),*, m]
            }
        }
    };
}
mask![([T,U], [M1]) -> ([t,_u], [a])];
#[cfg(not(feature = "opt"))]
mask![[T,U,V], [M1,M2,M3] => [t,_u,_v], [a,b,c]];
#[cfg(feature = "opt")]
const _: () = {
    mask![([T,U,V], [M1,M2]) -> ([t,_u,_v], [a,b])];
    mask![([T,U,V,W], [M1,M2,M3]) -> ([t,_u,_v,_w], [a,b,c])];
    mask![([T,U,V,W,X], [M1,M2,M3,M4]) -> ([t,_u,_v,_w,_x], [a,b,c,d])];
    mask![([T,U,V,W,X,Y], [M1,M2,M3,M4,M5]) -> ([t,_u,_v,_w,_x,_y], [a,b,c,d,e])];
    mask![([T,U,V,W,X,Y,Z], [M1,M2,M3,M4,M5,M6]) -> ([t,_u,_v,_w,_x,_y,_z], [a,b,c,d,e,f])];
    mask![([T,U,V,W,X,Y,Z,A], [M1,M2,M3,M4,M5,M6,M7]) -> ([t,_u,_v,_w,_x,_y,_z,_a], [a,b,c,d,e,f,g])];
    mask![([T,U,V,W,X,Y,Z,A,B], [M1,M2,M3,M4,M5,M6,M7,M8]) -> ([t,_u,_v,_w,_x,_y,_z,_a,_b], [a,b,c,d,e,f,g,h])];
    mask![[T,U,V,W,X,Y,Z,A,B,C], [M1,M2,M3,M4,M5,M6,M7,M8,M9,M10] => [t,_u,_v,_w,_x,_y,_z,_a,_b,_c], [a,b,c,d,e,f,g,h,i,j]];
};

pub trait SplitOffInfix {
    type Left;
    type Right;
    fn split_off_infix(self) -> (Self::Left, Self::Right);
}
impl<T> SplitOffInfix for (List<T>, List<()>) {
    type Left = List<()>;
    type Right = List<T>;
    fn split_off_infix(self) -> (Self::Left, Self::Right) {
        (List(()), self.0)
    }
}
macro_rules! split_off_infix {
    [([$a:ident$(,)?$($as:ident),*], [$b:ident$(,)?$($bs:ident),*]) -> ([$c:ident$(,)?$($cs:ident),*], [$d:ident,$($ds:ident),*])] => {
        impl< $a,$($as),* > SplitOffInfix for (unbounded_ty![$($as),*,$a], list_ty![$($bs),*]) {
            type Left = list_ty![$($as),*];
            type Right = $a;
            fn split_off_infix(self) -> (Self::Left, Self::Right) {
                let unbounded![$($cs),*,$c] = self.0;
                (list![$($cs),*], $c)
            }
        }
    };
    [[$a:ident$(,)?$($as:ident),*], [$b:ident,$($bs:ident),*] => [$c:ident$(,)?$($cs:ident),*], [$d:ident,$($ds:ident),*]] => {
        impl< $b,$a,$($as),* > SplitOffInfix for (unbounded_ty![$($as),*,$a], unbounded_ty![$($bs),*,$b])
        where ($a,$b): SplitOffInfix
        {
            type Left = unbounded_ty![$($as),*,<($a, $b) as SplitOffInfix>::Left];
            type Right = <($a, $b) as SplitOffInfix>::Right;
            fn split_off_infix(self) -> (Self::Left, Self::Right) {
                let unbounded![$($cs),*,$c] = self.0;
                let unbounded![$($ds),*,$d] = self.1;
                let (l, r) = ($c, $d).split_off_infix();
                (unbounded![$($cs),*,l], r)
            }
        }
    };
}
split_off_infix![([T1, T2],[E, E]) -> ([t1, t2],[_a, _b])];
#[cfg(not(feature = "opt"))]
split_off_infix![[T1, T2, T3],[A, E, E] => [t1, t2, t3],[_a, _b, _c]];
#[cfg(feature = "opt")]
const _: () = {
    split_off_infix![([T1, T2, T3],[E, E, E]) -> ([t1, t2, t3],[_a, _b, _c])];
    split_off_infix![([T1, T2, T3, T4],[E, E, E, E]) -> ([t1, t2, t3, t4],[_a, _b, _c, _d])];
    split_off_infix![([T1, T2, T3, T4, T5],[E, E, E, E, E]) -> ([t1, t2, t3, t4, t5],[_a, _b, _c, _d, _e])];
    split_off_infix![([T1, T2, T3, T4, T5, T6],[E, E, E, E, E, E]) -> ([t1, t2, t3, t4, t5, t6],[_a, _b, _c, _d, _e, _f])];
    split_off_infix![([T1, T2, T3, T4, T5, T6, T7],[E, E, E, E, E, E, E]) -> ([t1, t2, t3, t4, t5, t6, t7],[_a, _b, _c, _d, _e, _f, _g])];
    split_off_infix![([T1, T2, T3, T4, T5, T6, T7, T8],[E, E, E, E, E, E, E, E]) -> ([t1, t2, t3, t4, t5, t6, t7, t8],[_a, _b, _c, _d, _e, _f, _g, _h])];
    split_off_infix![[T1, T2, T3, T4, T5, T6, T7, T8, T9],[A, E, E, E, E, E, E, E, E] => [t1, t2, t3, t4, t5, t6, t7, t8, t9],[_a, _b, _c, _d, _e, _f, _g, _h, _i]];
};

pub trait SplitOff<U> {
    type Left;
    type Right;
    fn split_off(self, l: U) -> (Self::Left, Self::Right);
}
impl<T, U> SplitOff<U> for T
where
    (T, U): SplitOffInfix,
{
    type Left = <(T, U) as SplitOffInfix>::Left;
    type Right = <(T, U) as SplitOffInfix>::Right;
    fn split_off(self, l: U) -> (Self::Left, Self::Right) {
        (self, l).split_off_infix()
    }
}
