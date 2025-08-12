use criterion::{criterion_group, criterion_main, Criterion};

use inception_test::clone::Duplicate;
use inception_test::data::Version;
use inception_test::default::Standard;
use inception_test::eq::Same;
use inception_test::hash::Digest;

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Actor2 {
    pub name: String,
    pub kind: Kind2,
    pub net_worth: u128,
}
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Kind2 {
    BigName { salary: u64 },
    Aspiring { salary: u8 },
}
impl Default for Kind2 {
    fn default() -> Self {
        Self::BigName { salary: u64::MAX }
    }
}
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Movie2 {
    pub title: String,
    pub year: u64,
    pub lead: Actor2,
    pub also_important: Actor2,
    pub director: Director2,
}

#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Director2 {
    pub name: String,
    pub num_movies: u8,
    pub age: u8,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Version2 {
    One(Movie2),
    Two(Movie2),
}
impl Default for Version2 {
    fn default() -> Self {
        Self::One(Movie2::default())
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    use std::hash::Hash;
    let mut h = std::hash::DefaultHasher::new();
    let data1 = Version::standard();
    let data2 = Version2::default();
    c.bench_function("dupe 20", |b| b.iter(|| data1.dupe()));
    c.bench_function("clone 20", |b| b.iter(|| data2.clone()));
    c.bench_function("standard 20", |b| b.iter(Version::standard));
    c.bench_function("default 20", |b| b.iter(Version2::default));
    c.bench_function("same 20", |b| b.iter(|| data1.same(&data1)));
    c.bench_function("eq 20", |b| b.iter(|| data2.eq(&data2)));
    c.bench_function("digest 20", |b| b.iter(|| data1.digest(&mut h)));
    c.bench_function("hash 20", |b| b.iter(|| data2.hash(&mut h)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
