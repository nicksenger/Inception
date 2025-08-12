use inception::Inception;

#[derive(Inception)]
pub struct Actor {
    pub name: String,
    pub kind: Kind,
    pub net_worth: u128,
}

#[derive(Inception)]
pub enum Kind {
    BigName { salary: u64 },
    Aspiring { salary: u8 },
}

#[derive(Inception)]
pub struct Movie {
    pub title: String,
    pub year: u64,
    pub lead: Actor,
    pub also_important: Actor,
    pub director: Director,
}

#[derive(Inception)]
pub struct Director {
    pub name: String,
    pub num_movies: u8,
    pub age: u8,
}

#[derive(Inception)]
pub enum Version {
    One(Movie),
    Two(Movie),
}
