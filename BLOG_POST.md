### _Inception_ explores the following concept in Rust

> Given a type `T`, if we can prove some property exists for all of `T`'s minimal substructures, and all of `T`'s immediate substructures, then this property must also hold for `T` itself.

This is called "structural" or "well-founded" induction, a concept first introduced in 1917 by Dmitry Mirimanoff.

Though people have known about this for over 100 years, as far as I know the Rust compiler doesn't provide a means for us to apply it to our own types and behaviors in any useful way. _Inception_ attempts to change that - not by direct modification of the compiler (that would be cheating!), but instead by persuasion. The goal is to "teach" the existing compiler that Mirimanoff's methods are, as he originally demonstrated, mathematical fact. And this lesson will be administered to the compiler from user code, by force or by fire.

To that last point though: there is no use of `unsafe` or glazing things over with dynamism, downcasting, etc. That would be cheating, and this is _persuasion_ after all, not coercion. The only things that will be broken are language conventions and, if I'm lucky, a few peoples' preexisting assumptions about what is or isn't possible in stable Rust today.

### Approach

I think the most straightforward way to explain this is with an example use case, so imagine we have two big-name celebrities: Leonardo Di Caprio and Cillian Murphy.

```rust
struct LeonardoDiCaprio;
struct CillianMurphy;
```

Let's make an explicit assumption that the following statement is true:

> (A film/recording consisting solely of) either Leonardo Di Caprio or Cillian Murphy is guaranteed to be a blockbuster

(it's tempting to argue that this holds trivially, but we can save that debate for another time)

In Rust, this can be expressed as follows:

```rust
impl Blockbuster for LeonardoDiCaprio {
    fn profit() -> Profits {
        Profits::GUARANTEED_DI_CAPRIO
    }
}
impl Blockbuster for CillianMurphy {
    fn profit() -> Profits {
        Profits::GUARANTEED_MURPHY
    }
}
```

Now let's introduce some additional structures involving these actors:

```rust
enum Character {
    Cobb(LeonardoDiCaprio),
    Fischer {
        played_by: CillianMurphy
    }
}

struct PlotHole {
    involving: Character,
}

struct Scene {
    featuring: Character,
    introduces: PlotHole,
}

struct Inception1 {
    starring: LeonardoDiCaprio,
    and_also: CillianMurphy,
    exposition: Scene,
    rising_action: Scene,
    climax: Scene,
    resolution: Scene
}
```

We can propose that `Inception1` implements `Blockbuster`, but Rust won't let us call `profit` on an instance of this type until we _prove_ that this is the case. Of course we could explicitly implement `Blockbuster` for these types, either manually or with a `Derive` macro, but that's _too much work_ and too error-prone. We want the compiler to just _know_ that `Inception1` is a `Blockbuster`, without having to do anything in particular. And we want it to _know_ that if we rearrange the scenes in any way, or add any additional plot-holes, it will _still_ be a `Blockbuster`. If we make a sequel, a trilogy, a TV series, video game, or even `InceptionChristmas`, we want everything in the whole franchise to be indisputably proven to be a `Blockbuster` by mathemical law, so long as those things are constructed of parts whose minimal substructures are `LeonardoDiCaprio` or `CillianMurphy`.

Revisiting the work of Mirimanoff, we note that the first requirement for our proof, about minimal substructures, is already met for the new types we've defined. So now we just need to show that the immediate substructures of `Inception1` (its _fields_) are also `Blockbuster`s.

This is where Rust starts making things a bit tricky for us, because the compiler doesn't serve us up information about a type's fields in where-bounds. So we'll have to introduce a new assumption: that each of these structures exposes its own fields as an ordered type-level list, and that they do so from a trait associated-type, such that we can refer to them in where-bounds when implementing traits. Shown below is a comment with the assumption for each type, and a derive macro added to the type to handle the actual plumbing of exposing this list to the rest of our code:

```rust
// type Fields = list![ LeonardoDiCaprio, CillianMurphy ];
#[derive(Inception)]
enum Character {
    Cobb(LeonardoDiCaprio),
    Fischer {
        played_by: CillianMurphy
    }
}

// type Fields = list![ Character ];
#[derive(Inception)]
struct PlotHole {
    involving: Character,
}

// type Fields = list![ Character, PlotHole ];
#[derive(Inception)]
struct Scene {
    featuring: Character,
    introduces: PlotHole,
}

// type Fields = list! [ LeonardoDiCaprio, CillianMurphy, Scene, Scene, Scene, Scene ];
#[derive(Inception)]
struct Inception1 {
    starring: LeonardoDiCaprio,
    and_also: CillianMurphy,
    exposition: Scene,
    rising_action: Scene,
    climax: Scene,
    resolution: Scene
}
```

At this point, it may be more helpful to think of this as writing a recursive function, only the actual body of our function will live in where-bounds and be closer to a logic programming language like Prolog than the Rust we're used to writing.

_Inception_ tries to hide most of these gory details behind another proc-macro on the trait definition itself. I won't try to stop you from expanding it, but please: ensure no children are present, wear some form of OSHA-approved eye protection, and remember that there are certain things in life which can never be _unseen_. Dark truths which are best kept hidden away, in hopes of preserving that which is still _good_ and _innocent_ in this world. I'll speak no more of its inner devil-work.

Instead, let's focus on the intended use of the resulting API, which requires us to define our trait in terms of building up a recursive datatype having some property from simpler elements already known to possess this property. In our example, `LeonardoDiCaprio` and `CillianMurphy` serve as the fundamental primitives or "base-case" upon which all of these elements are constructed, but we need never refer to them explicitly here, and instead should operate only in terms of generic types for which the property in question is already assumed to be true.

Here is a definition of the `Blockbuster` trait that will be automatically implemented for our types, and where the profits will be the sum of the profits from all of the individual substructures:

```rust
#[inception(property = BoxOfficeHit)]
pub trait Blockbuster {
    // // This will be the only method in our final `Blockbuster` trait
    fn profit(self) -> Profits;

    // // Everything below this point is to prove that our trait applies to the recursive structures
    //
    // Define what should happen for nothing at all (e.g. the end of a list)
    fn nothing() -> Profits {
        Profits::Nothing
    }

    // Define how we should merge a single item (field) having this property with  _some number_
    // of (possibly 0) items (fields) which together also exhibit this property
    fn merge<H: Blockbuster, R: Blockbuster>(l: L, r: R) -> Profits {
        l.access().profit() + r.profit()
    }

    // Same as above, but different because.. enums.
    fn merge_variant_field<H: Blockbuster, R: Blockbuster>(
        l: L,
        r: R,
    ) -> Profits {
        l.try_access().map(|l| l.profit()).unwrap_or(Profits::Nothing) + r.profit()
    }

    // Define how we should join a "container" (struct or enum) with a collection of
    // its fields (that together are known to have this property)
    fn join<F: Blockbuster>(fields: F) -> Profits {
        fields.profit()
    }
}
```

Now we can call `.profit()` on instances of `Inception1`, `PlotHole`, or any of our intermediate types. We can define new types composed of these composite types in any order or combination and, so long as we use the derive macro to expose their fields, these new types will also implement `Blockbuster` or any other traits we define this way!

We can create as many behaviors as we want, for serialization/deserialization, debugging, etc, for whatever sets of primitives, and share them all through the single `#[derive(Inception)]`. "Alternatives" to many of the standard derive macros are already implemented as tests for this crate. So it _is possible_ to convince the Rust compiler that these properties hold. But before anyone gets carried away and starts thinking: _Serde is dead, Clone, Hash and all of the std Derive macros are dead! Praise Dmitry Mirimanoff! Long live Inception! The last macro we'll ever need!_

Slow down, because there's a big blaring plot-hole, and it's not the performance one, or the ergonomics one, or the data privacy one, or the versioning one, or even the one about the demon-spawn-proc-macro-from-hell that shouldn't be neccessary but is currently gluing all of this together - because I'm sure each of those could be compensated for by another action-scene or close-up of Di Caprio's confused face. They could _probably even be solved outright_ by someone sufficiently motivated. But that person most likely wouldn't be myself, because:

_It's fighting a tool._

Fighting your tools is tiring, and in my experience usually of questionable value.

One last thing I will do though is toss it in the yard for the LLMs to pick at, pour myself a pint, and watch _Inception_, because I haven't seen it in 15 years and can barely remember what happens.

Cheers,

Nick
