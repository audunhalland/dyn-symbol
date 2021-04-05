//!
//! Dynamic, plugin-based [Symbol](https://en.wikipedia.org/wiki/Symbol_(programming)) abstraction.
//!
//! A [Symbol] can be used as an _identifier_ in place of the more primitive workhorse [String].
//! There could be multiple reasons to do so:
//!
//! 1. Mixing of different domains in the same runtime code
//! 2. Handling of naming collisions in multiple namespaces
//! 3. Avoiding memory allocations for statically known namespaces
//! 4. Mix of static and dynamic allocation
//! 5. Associating metadata to the symbols themselves
//!
//! The main use case for symbols is as map keys for in-memory key/value stores.
//!
//! Note that there are probably more reasons _not_ to use symbols than to use them! In most cases, something like
//! `enum` or [String] will do just fine. But sometimes applications process a lot of semi-schematic external input,
//! and you just want Rust to work like any old dynamic programming language again.
//!
//! # Example use cases
//! * Namespaced XML/HTML attributes (in HTML, some are static and some are dynamic. i.e. `data-` attributes)
//! * Key/value stores for "anything"
//! * Some way to abstract away string interners? (this is untested)
//!
//! A [Symbol] is just one plain, non-generic type, that can represent all possible symbol values. It implements all traits to make it
//! usable as a key in maps.
//!
//! # What this crate does not do
//! * Serialization and deserialization of symbols. [Symbol] should not implement `serde` traits, ser/de should instead be handled by each namespace.
//! * Provide any namespaces.
//!
//! # Static symbols
//! Static symbols originate from a namespace where all possible values are statically known at compile time.
//! One instance of a static symbol requires no memory allocation.
//!
//! Creating a static namespace:
//!
//! ```
//! use dyn_symbol::*;
//!
//! struct MyStaticNS {
//!     symbols: &'static [(&'static str, &'static str)],
//! }
//!
//! const MY_STATIC_NS: MyStaticNS = MyStaticNS {
//!     symbols: &[
//!         ("foo", "the first symbol!"),
//!         ("bar", "the second symbol!")
//!     ]
//! };
//!
//! impl dyn_symbol::namespace::Static for MyStaticNS {
//!     fn namespace_name(&self) -> &str {
//!         "my"
//!     }
//!
//!     fn symbol_name(&self, id: u32) -> &str {
//!         self.symbols[id as usize].0
//!     }
//! }
//!
//! // Define (and export) some symbol constants
//! pub const FOO: Symbol = Symbol::Static(&MY_STATIC_NS, 0);
//! pub const BAR: Symbol = Symbol::Static(&MY_STATIC_NS, 1);
//!
//! assert_eq!(FOO, FOO);
//! assert_eq!(FOO.clone(), FOO.clone());
//! assert_ne!(FOO, BAR);
//! assert_eq!(format!("{:?}", FOO), "my::foo");
//!
//! // We can find the originating namespace later:
//! assert!(FOO.downcast_static::<MyStaticNS>().is_some());
//!
//! // To implement special metadata-extraction (or similar functionality) for a namespace:
//! fn get_symbol_description(symbol: &Symbol) -> Result<&'static str, &'static str> {
//!     if let Some((namespace, id)) = symbol.downcast_static::<MyStaticNS>() {
//!         Ok(namespace.symbols[id as usize].1)
//!     } else {
//!         Err("not from this namespace :(")
//!     }
//! }
//!
//! assert_eq!(get_symbol_description(&BAR).unwrap(), "the second symbol!");
//! ```
//!
//! For static symbols, the implementations of [Eq]/[Ord]/[Hash](std::hash::Hash) et. al use only the namespace's [type_id](std::any::Any::type_id)
//! plus the symbol's numerical `id`.
//!
//! Typically, the boilerplate code for a static namespace will be generated by macros or `build.rs`.
//!
//! # Dynamic symbols
//! Sometimes the values that a symbol can take are not known upfront. In this case we have to resort to memory allocation.
//! Dynamic symbols implement a different namespace trait: [namespace::Dynamic]. The type that implements this trait also
//! functions as the symbol _instance_ itself:
//!
//! ```
//! use dyn_symbol::*;
//!
//! // This symbol is string-based:
//! struct DynamicNS(String);
//!
//! impl namespace::Dynamic for DynamicNS {
//!     fn namespace_name(&self) -> &str {
//!         "dynamic"
//!     }
//!
//!     fn symbol_name(&self) -> &str {
//!         &self.0
//!     }
//!
//!     fn dyn_clone(&self) -> Box<dyn namespace::Dynamic> {
//!         Box::new(DynamicNS(self.0.clone()))
//!     }
//!
//!     /// Note: calling code should already have verified that these are indeed the same types, using `type_id`.
//!     fn dyn_eq(&self, rhs: &dyn namespace::Dynamic) -> bool {
//!         self.0 == rhs.downcast_ref::<DynamicNS>().unwrap().0
//!     }
//!
//!     fn dyn_cmp(&self, rhs: &dyn namespace::Dynamic) -> std::cmp::Ordering {
//!         self.0.cmp(&rhs.downcast_ref::<DynamicNS>().unwrap().0)
//!     }
//!
//!     fn dyn_hash(&self, state: &mut dyn std::hash::Hasher) {
//!         // we are now in `dyn` land, so the [std::hash::Hash] trait cannot be used:
//!         state.write(self.0.as_bytes());
//!         state.write_u8(0xff)
//!     }
//! }
//!
//! let foo0 = Symbol::Dynamic(Box::new(DynamicNS("foo".into())));
//! let foo1 = Symbol::Dynamic(Box::new(DynamicNS("foo".into())));
//! let bar = Symbol::Dynamic(Box::new(DynamicNS("bar".into())));
//!
//! assert_eq!(foo0, foo1);
//! assert_eq!(foo0.clone(), foo1.clone());
//! assert_ne!(foo0, bar);
//! ```
//!
//! It is entirely up to the Dynamic implementation to consider what kind of symbols are considered equal.
//! The `Eq`/`Hash` symmetry need to hold, though.
//!
//! Dynamic symbols are supported as a companion to static symbols. If your application works mainly with dynamic symbols,
//! you should consider using a different keying mechanism, because of the inherent overhead/indirection/boxing of dynamic symbols.
//!
//! # Type system
//! This crate makes use of [Any](std::any::Any), and consideres namespaces sharing the same [TypeId](std::any::TypeId) to be the _same namespace_.
//! This could make code reuse a bit cumbersome. If one crate exports multiple namespaces, this can be solved by using const generics:
//!
//! ```
//! struct ReusableNamespace<const N: u8>;
//!
//! // impl<const N: u8> namespace::Static for MyNamespace<N> { ... }
//!
//! const NS_1: ReusableNamespace<1> = ReusableNamespace;
//! const NS_2: ReusableNamespace<2> = ReusableNamespace;
//!
//! // assert_ne!(NS_1.type_id(), NS_2.type_id());
//! ```
//!
//! This will cause the two namespaces to have differing `type_id`s.
//!
//!

use std::cmp::Ordering;

///
/// A symbol, with support for mixed static/dynamic allocation.
///
pub enum Symbol {
    /// Construct a Symbol originating from a static namespace.
    /// The first parameter is a trait object pointing back to the namespace,
    /// the second parameter is the symbol `id` within that namespace.
    Static(&'static dyn namespace::Static, u32),

    /// Construct a Symbol with dynamic origins. Dynamic namespaces are unbounded in size,
    /// so a memory allocation is needed. This encoding allows dynamic namespaces to support
    /// the same semantics that static namespaces do. Instead of just using a [String], we
    /// can also encode what kind of string it is.
    Dynamic(Box<dyn namespace::Dynamic>),
}

impl Symbol {
    ///
    /// Get access to the associated namespace's `Any` representation.
    /// its `type_id` may be used as a reflection tool to get to know about the Symbol's origin.
    ///
    pub fn as_any(&self) -> &dyn std::any::Any {
        match self {
            Self::Static(ns, _) => ns.as_any(),
            Self::Dynamic(instance) => instance.as_any(),
        }
    }

    ///
    /// Try to downcast this Symbol's originating _static namespace_ to a concrete `&T`,
    /// and if successful, return that concrete namespace along with the symbol's static id.
    ///
    pub fn downcast_static<T: 'static>(&self) -> Option<(&T, u32)> {
        match self {
            Self::Static(ns, id) => ns.as_any().downcast_ref::<T>().map(|t| (t, *id)),
            Self::Dynamic(_) => None,
        }
    }

    ///
    /// Try to downcast this Symbol's _dynamic namespace_ as a `&T`.
    ///
    /// Always fails for static namespaces.
    ///
    pub fn downcast_dyn<T: 'static>(&self) -> Option<&T> {
        match self {
            Self::Static(_, _) => None,
            Self::Dynamic(instance) => instance.as_any().downcast_ref::<T>(),
        }
    }
}

impl Clone for Symbol {
    fn clone(&self) -> Self {
        match self {
            Self::Static(static_symbol, id) => Self::Static(*static_symbol, *id),
            Self::Dynamic(instance) => Self::Dynamic(instance.dyn_clone()),
        }
    }
}

impl std::fmt::Debug for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Static(ns, id) => {
                write!(f, "{}::{}", ns.namespace_name(), ns.symbol_name(*id))
            }
            Self::Dynamic(instance) => {
                write!(
                    f,
                    "{}::{}",
                    instance.namespace_name(),
                    instance.symbol_name()
                )
            }
        }
    }
}

impl PartialEq for Symbol {
    fn eq(&self, rhs: &Symbol) -> bool {
        match (self, rhs) {
            (Self::Static(this_ns, this_id), Self::Static(rhs_ns, rhs_id)) => {
                *this_id == *rhs_id && this_ns.type_id() == rhs_ns.type_id()
            }
            (Self::Dynamic(this), Self::Dynamic(rhs)) => {
                this.type_id() == rhs.type_id() && this.dyn_eq(rhs.as_ref())
            }
            _ => false,
        }
    }
}

impl Eq for Symbol {}

impl Ord for Symbol {
    fn cmp(&self, rhs: &Symbol) -> Ordering {
        match (self, rhs) {
            (Self::Static(this_ns, this_id), Self::Static(rhs_ns, rhs_id)) => {
                let this_type_id = this_ns.type_id();
                let rhs_type_id = rhs_ns.type_id();

                if this_type_id == rhs_type_id {
                    this_id.cmp(&rhs_id)
                } else {
                    this_type_id.cmp(&rhs_type_id)
                }
            }
            (Self::Dynamic(this), Self::Dynamic(rhs)) => {
                let this_type_id = this.type_id();
                let rhs_type_id = rhs.type_id();

                if this_type_id == rhs_type_id {
                    this.dyn_cmp(rhs.as_ref())
                } else {
                    this_type_id.cmp(&rhs_type_id)
                }
            }
            (Self::Static(_, _), Self::Dynamic(_)) => Ordering::Less,
            (Self::Dynamic(_), Self::Static(_, _)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Symbol) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::hash::Hash for Symbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Static(ns, id) => {
                ns.type_id().hash(state);
                state.write_u32(*id)
            }
            Self::Dynamic(dynamic_sym) => {
                dynamic_sym.type_id().hash(state);
                dynamic_sym.dyn_hash(state)
            }
        }
    }
}

pub mod namespace {
    //!
    //! Namespace traits that must be implemented by symbol providers.
    //!

    use downcast_rs::*;

    ///
    /// A static namespace. Symbols in a static namespace are identified with an `id` encoded as a `u32`.
    ///
    pub trait Static: Send + Sync + Downcast {
        ///
        /// The namespace's name, used for [Debug][std::fmt::Debug].
        ///
        fn namespace_name(&self) -> &str;

        ///
        /// A symbol's name, used for [Debug][std::fmt::Debug].
        ///
        fn symbol_name(&self, id: u32) -> &str;
    }

    ///
    /// A dynamic namespace. A dynamic symbol instance is tied to `Self`.
    ///
    pub trait Dynamic: Send + Sync + Downcast {
        ///
        /// The namespace's name, used for [Debug][std::fmt::Debug].
        ///
        fn namespace_name(&self) -> &str;

        ///
        /// The symbol name, used for [Debug][std::fmt::Debug].
        ///
        fn symbol_name(&self) -> &str;

        ///
        /// Clone this dynamic symbol. Must return a new symbol instance that is `eq` to `&self`.
        ///
        fn dyn_clone(&self) -> Box<dyn Dynamic>;

        ///
        /// Dynamic [eq](std::cmp::PartialEq::eq). `rhs` can be unconditionally downcasted to `Self`.
        ///
        fn dyn_eq(&self, rhs: &dyn Dynamic) -> bool;

        ///
        /// Dynamic [cmp](std::cmp::Ord::cmp). `rhs` can be unconditionally downcasted to `Self`.
        ///
        fn dyn_cmp(&self, rhs: &dyn Dynamic) -> std::cmp::Ordering;

        ///
        /// Dynamic [hash](std::hash::Hash::hash). `rhs` can be unconditionally downcasted to `Self`.
        ///
        fn dyn_hash(&self, state: &mut dyn std::hash::Hasher);
    }

    impl_downcast!(Dynamic);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, Hash, Hasher};

    mod _static {
        use super::*;

        pub struct ClassN<const N: u8> {
            class_name: &'static str,
            names: &'static [&'static str],
        }

        impl<const N: u8> namespace::Static for ClassN<N> {
            fn namespace_name(&self) -> &str {
                self.class_name
            }

            fn symbol_name(&self, id: u32) -> &str {
                self.names[id as usize]
            }
        }

        pub const STATIC_NS_CLASS_A: ClassN<1> = ClassN {
            class_name: "A",
            names: &["0", "1"],
        };
        pub const STATIC_NS_CLASS_B: ClassN<2> = ClassN {
            class_name: "B",
            names: &["0"],
        };
    }

    mod dynamic {
        use super::*;

        pub struct TestDynamic<const N: u8>(pub String, &'static str);

        impl<const N: u8> namespace::Dynamic for TestDynamic<N> {
            fn namespace_name(&self) -> &str {
                self.1
            }

            fn symbol_name(&self) -> &str {
                &self.0
            }

            fn dyn_clone(&self) -> Box<dyn namespace::Dynamic> {
                Box::new(TestDynamic::<N>(self.0.clone(), self.1))
            }

            fn dyn_eq(&self, rhs: &dyn namespace::Dynamic) -> bool {
                self.0 == rhs.downcast_ref::<TestDynamic<N>>().unwrap().0
            }

            fn dyn_cmp(&self, rhs: &dyn namespace::Dynamic) -> std::cmp::Ordering {
                self.0.cmp(&rhs.downcast_ref::<TestDynamic<N>>().unwrap().0)
            }

            fn dyn_hash(&self, state: &mut dyn std::hash::Hasher) {
                state.write(self.0.as_bytes());
                state.write_u8(0xff)
            }
        }

        pub fn sym0(str: &str) -> Symbol {
            Symbol::Dynamic(Box::new(TestDynamic::<0>(str.into(), "dyn0")))
        }

        pub fn sym1(str: &str) -> Symbol {
            Symbol::Dynamic(Box::new(TestDynamic::<1>(str.into(), "dyn1")))
        }
    }

    const STATIC_A_0: Symbol = Symbol::Static(&_static::STATIC_NS_CLASS_A, 0);
    const STATIC_A_1: Symbol = Symbol::Static(&_static::STATIC_NS_CLASS_A, 1);
    const STATIC_B_0: Symbol = Symbol::Static(&_static::STATIC_NS_CLASS_B, 0);

    struct TestState {
        random_state: std::collections::hash_map::RandomState,
    }

    impl TestState {
        pub fn new() -> Self {
            Self {
                random_state: std::collections::hash_map::RandomState::new(),
            }
        }

        fn assert_hash_match(&self, a: &Symbol, b: &Symbol, should_equal: bool) {
            let mut hasher_a = self.random_state.build_hasher();
            let mut hasher_b = self.random_state.build_hasher();

            a.hash(&mut hasher_a);
            b.hash(&mut hasher_b);

            if should_equal {
                assert_eq!(hasher_a.finish(), hasher_b.finish())
            } else {
                assert_ne!(hasher_a.finish(), hasher_b.finish())
            }
        }

        fn assert_full_eq(&self, a: &Symbol, b: &Symbol) {
            assert_eq!(a, b);
            assert_eq!(a.cmp(b), Ordering::Equal);
            self.assert_hash_match(a, b, true)
        }

        fn assert_full_ne(&self, a: &Symbol, b: &Symbol) {
            assert_ne!(a, b);
            assert_ne!(a.cmp(b), Ordering::Equal);
            self.assert_hash_match(a, b, false)
        }
    }

    #[test]
    fn test_symbol_size_of() {
        let u_size = std::mem::size_of::<usize>();

        // This size_of Symbol is computed like this:
        // It's at least two words, because of `dyn`.
        // it's more than two words because it needs to encode the A/B enum value.
        // on 64-bit arch it should be 3 words, because it contains an `u32` too,
        // and that should be encoded within the same machine word as the enum discriminant..
        // I think...
        let expected_word_size = match u_size {
            8 => 3 * u_size,
            // 4 => 4, Perhaps?
            _ => panic!("untested word size"),
        };

        assert_eq!(std::mem::size_of::<Symbol>(), expected_word_size);
    }

    #[test]
    fn test_debug() {
        assert_eq!(format!("{:?}", STATIC_A_0), "A::0");
        assert_eq!(format!("{:?}", STATIC_A_1), "A::1");
        assert_eq!(format!("{:?}", STATIC_B_0), "B::0");

        assert_eq!(format!("{:?}", dynamic::sym0("foo")), "dyn0::foo");
        assert_eq!(format!("{:?}", dynamic::sym1("bar")), "dyn1::bar");
    }

    #[test]
    fn test_equality() {
        let test_state = TestState::new();

        test_state.assert_full_eq(&STATIC_A_0, &STATIC_A_0);
        test_state.assert_full_eq(&STATIC_A_1, &STATIC_A_1);
        test_state.assert_full_eq(&STATIC_B_0, &STATIC_B_0);

        test_state.assert_full_ne(&STATIC_A_0, &STATIC_A_1);
        test_state.assert_full_ne(&STATIC_A_1, &STATIC_B_0);

        test_state.assert_full_eq(&dynamic::sym0("foo"), &dynamic::sym0("foo"));
    }

    #[test]
    fn test_inequality() {
        let test_state = TestState::new();

        test_state.assert_full_ne(&STATIC_A_0, &STATIC_A_1);
        test_state.assert_full_ne(&STATIC_A_0, &STATIC_B_0);

        test_state.assert_full_ne(&dynamic::sym0("foo"), &dynamic::sym0("bar"));
        test_state.assert_full_ne(&dynamic::sym0("foo"), &dynamic::sym1("foo"));
    }

    #[test]
    fn test_ord() {
        assert_ne!(STATIC_A_0.cmp(&STATIC_A_1), Ordering::Equal);
        assert_ne!(STATIC_A_0.cmp(&STATIC_B_0), Ordering::Equal);
        assert_ne!(STATIC_A_1.cmp(&STATIC_B_0), Ordering::Equal);
    }
}
