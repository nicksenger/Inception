use inception::Inception;

use crate::clone::{DupeMut, DupeOwned, DupeRef};
use crate::debug::{DebugRef, DebugTy};
use crate::default::Default;
use crate::eq::SameSame;
use crate::hash::Digestible;

#[derive(Inception)]
#[inception(properties = [Default, Digestible, SameSame, DebugTy, DebugRef, DupeRef, DupeMut, DupeOwned])]
pub struct Actor {
    pub name: String,
    pub kind: Kind,
    pub net_worth: u128,
}

#[derive(Inception)]
#[inception(properties = [Default, Digestible, SameSame, DebugTy, DebugRef, DupeRef, DupeMut, DupeOwned])]
pub enum Kind {
    BigName { salary: u64 },
    Aspiring { salary: u8 },
}

#[derive(Inception)]
#[inception(properties = [Default, Digestible, SameSame, DebugTy, DebugRef, DupeRef, DupeMut, DupeOwned])]
pub struct Movie {
    pub title: String,
    pub year: u64,
    pub lead: Actor,
    pub also_important: Actor,
    pub director: Director,
}

#[derive(Inception)]
#[inception(properties = [Default, Digestible, SameSame, DebugTy, DebugRef, DupeRef, DupeMut, DupeOwned])]
pub struct Director {
    pub name: String,
    pub num_movies: u8,
    pub age: u8,
}

#[derive(Inception)]
#[inception(properties = [Default, Digestible, SameSame, DebugTy, DebugRef, DupeRef, DupeMut, DupeOwned])]
pub enum Version {
    One(Movie),
    Two(Movie),
}
