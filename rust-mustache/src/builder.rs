use std::cell::RefCell;
use std::collections::HashMap;
use rustc_serialize::Encodable;

use encoder;
use encoder::Error;
use super::{Data, StrVal, Bool, VecVal, Map, Fun};

/// `MapBuilder` is a helper type that construct `Data` types.
pub struct MapBuilder {
    data: HashMap<String, Data>,
}

impl MapBuilder {
    /// Create a `MapBuilder`
    #[inline]
    pub fn new() -> MapBuilder {
        MapBuilder {
            data: HashMap::new(),
        }
    }

    /// Add an `Encodable` to the `MapBuilder`.
    ///
    /// ```rust
    /// use mustache::MapBuilder;
    /// let data = MapBuilder::new()
    ///     .insert("name", &("Jane Austen")).ok().unwrap()
    ///     .insert("age", &41u).ok().unwrap()
    ///     .build();
    /// ```
    #[inline]
    pub fn insert<
        K: Str, T: Encodable
    >(self, key: K, value: &T) -> Result<MapBuilder, Error> {
        let MapBuilder { mut data } = self;
        let value = try!(encoder::encode(value));
        data.insert(key.as_slice().to_string(), value);
        Ok(MapBuilder { data: data })
    }

    /// Add a `String` to the `MapBuilder`.
    ///
    /// ```rust
    /// use mustache::MapBuilder;
    /// let data = MapBuilder::new()
    ///     .insert_str("name", "Jane Austen")
    ///     .build();
    /// ```
    #[inline]
    pub fn insert_str<
        K: Str, V: Str
    >(self, key: K, value: V) -> MapBuilder {
        let MapBuilder { mut data } = self;
        data.insert(key.as_slice().to_string(), StrVal(value.as_slice().to_string()));
        MapBuilder { data: data }
    }

    /// Add a `bool` to the `MapBuilder`.
    ///
    /// ```rust
    /// use mustache::MapBuilder;
    /// let data = MapBuilder::new()
    ///     .insert_bool("show", true)
    ///     .build();
    /// ```
    #[inline]
    pub fn insert_bool<K: Str>(self, key: K, value: bool) -> MapBuilder {
        let MapBuilder { mut data } = self;
        data.insert(key.as_slice().to_string(), Bool(value));
        MapBuilder { data: data }
    }

    /// Add a `Vec` to the `MapBuilder`.
    ///
    /// ```rust
    /// use mustache::MapBuilder;
    /// let data = MapBuilder::new()
    ///     .insert_vec("authors", |builder| {
    ///         builder
    ///             .push_str("Jane Austen")
    ///             .push_str("Lewis Carroll")
    ///     })
    ///     .build();
    /// ```
    #[inline]
    pub fn insert_vec<K: Str, F>(self, key: K, mut f: F) -> MapBuilder
        where F: FnMut(VecBuilder) -> VecBuilder {
        let MapBuilder { mut data } = self;
        let builder = f(VecBuilder::new());
        data.insert(key.as_slice().to_string(), builder.build());
        MapBuilder { data: data }
    }

    /// Add a `Map` to the `MapBuilder`.
    ///
    /// ```rust
    /// use mustache::MapBuilder;
    /// let data = MapBuilder::new()
    ///     .insert_map("person1", |builder| {
    ///         builder
    ///             .insert_str("first_name", "Jane")
    ///             .insert_str("last_name", "Austen")
    ///     })
    ///     .insert_map("person2", |builder| {
    ///         builder
    ///             .insert_str("first_name", "Lewis")
    ///             .insert_str("last_name", "Carroll")
    ///     })
    ///     .build();
    /// ```
    #[inline]
    pub fn insert_map<K: Str, F>(self, key: K, mut f: F) -> MapBuilder
        where F: FnMut(MapBuilder) -> MapBuilder {
        let MapBuilder { mut data } = self;
        let builder = f(MapBuilder::new());
        data.insert(key.as_slice().to_string(), builder.build());
        MapBuilder { data: data }
    }

    /// Add a function to the `MapBuilder`.
    ///
    /// ```rust
    /// use mustache::MapBuilder;
    /// let mut count = 0;
    /// let data = MapBuilder::new()
    ///     .insert_fn("increment", move |_| {
    ///         count += 1usize;
    ///         count.to_string()
    ///     })
    ///     .build();
    /// ```
    #[inline]
    pub fn insert_fn<K: Str, F>(self, key: K, f: F) -> MapBuilder
                                where F: FnMut(String) -> String + Send + 'static {
        let MapBuilder { mut data } = self;
        data.insert(key.as_slice().to_string(), Fun(RefCell::new(Box::new(f))));
        MapBuilder { data: data }
    }

    /// Return the built `Data`.
    #[inline]
    pub fn build(self) -> Data {
        Map(self.data)
    }
}

pub struct VecBuilder {
    data: Vec<Data>,
}

impl<'a> VecBuilder {
    /// Create a `VecBuilder`
    #[inline]
    pub fn new() -> VecBuilder {
        VecBuilder {
            data: Vec::new(),
        }
    }

    /// Add an `Encodable` to the `VecBuilder`.
    ///
    /// ```rust
    /// use mustache::{VecBuilder, Data};
    /// let data: Data = VecBuilder::new()
    ///     .push(& &"Jane Austen").ok().unwrap()
    ///     .push(&41u).ok().unwrap()
    ///     .build();
    /// ```
    #[inline]
    pub fn push<T: Encodable>(self, value: &T) -> Result<VecBuilder, Error> {
        let VecBuilder { mut data } = self;
        let value = try!(encoder::encode(value));
        data.push(value);
        Ok(VecBuilder { data: data })
    }

    /// Add a `String` to the `VecBuilder`.
    ///
    /// ```rust
    /// use mustache::VecBuilder;
    /// let data = VecBuilder::new()
    ///     .push_str("Jane Austen")
    ///     .push_str("Lewis Carroll")
    ///     .build();
    /// ```
    #[inline]
    pub fn push_str<T: Str>(self, value: T) -> VecBuilder {
        let VecBuilder { mut data } = self;
        data.push(StrVal(value.as_slice().to_string()));
        VecBuilder { data: data }
    }

    /// Add a `bool` to the `VecBuilder`.
    ///
    /// ```rust
    /// use mustache::VecBuilder;
    /// let data = VecBuilder::new()
    ///     .push_bool(false)
    ///     .push_bool(true)
    ///     .build();
    /// ```
    #[inline]
    pub fn push_bool(self, value: bool) -> VecBuilder {
        let VecBuilder { mut data } = self;
        data.push(Bool(value));
        VecBuilder { data: data }
    }

    /// Add a `Vec` to the `MapBuilder`.
    ///
    /// ```rust
    /// use mustache::VecBuilder;
    /// let data = VecBuilder::new()
    ///     .push_vec(|builder| {
    ///         builder
    ///             .push_str("Jane Austen".to_string())
    ///             .push_str("Lewis Carroll".to_string())
    ///     })
    ///     .build();
    /// ```
    #[inline]
    pub fn push_vec<F>(self, mut f: F) -> VecBuilder
        where F: FnMut(VecBuilder) -> VecBuilder {
        let VecBuilder { mut data } = self;
        let builder = f(VecBuilder::new());
        data.push(builder.build());
        VecBuilder { data: data }
    }

    /// Add a `Map` to the `VecBuilder`.
    ///
    /// ```rust
    /// use mustache::VecBuilder;
    /// let data = VecBuilder::new()
    ///     .push_map(|builder| {
    ///         builder
    ///             .insert_str("first_name".to_string(), "Jane".to_string())
    ///             .insert_str("last_name".to_string(), "Austen".to_string())
    ///     })
    ///     .push_map(|builder| {
    ///         builder
    ///             .insert_str("first_name".to_string(), "Lewis".to_string())
    ///             .insert_str("last_name".to_string(), "Carroll".to_string())
    ///     })
    ///     .build();
    /// ```
    #[inline]
    pub fn push_map<F>(self, mut f: F) -> VecBuilder
        where F: FnMut(MapBuilder) -> MapBuilder {
        let VecBuilder { mut data } = self;
        let builder = f(MapBuilder::new());
        data.push(builder.build());
        VecBuilder { data: data }
    }

    /// Add a function to the `VecBuilder`.
    ///
    /// ```rust
    /// use mustache::VecBuilder;
    /// let mut count = 0;
    /// let data = VecBuilder::new()
    ///     .push_fn(move |s| {
    ///         count += 1usize;
    ///         s + count.to_string().as_slice()
    ///     })
    ///     .build();
    /// ```
    #[inline]
    pub fn push_fn<F>(self, f: F) -> VecBuilder
                   where F: FnMut(String) -> String + Send + 'static {
        let VecBuilder { mut data } = self;
        data.push(Fun(RefCell::new(Box::new(f))));
        VecBuilder { data: data }
    }

    #[inline]
    pub fn build(self) -> Data {
        VecVal(self.data)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::super::{StrVal, Bool, VecVal, Map, Fun};
    use super::{MapBuilder, VecBuilder};

    #[test]
    fn test_empty_builders() {
        assert_eq!(
            MapBuilder::new().build(),
            Map(HashMap::new()));

        assert_eq!(
            VecBuilder::new().build(),
            VecVal(Vec::new()));
    }

    #[test]
    fn test_builders() {
        let mut pride_and_prejudice = HashMap::new();
        pride_and_prejudice.insert("title".to_string(), StrVal("Pride and Prejudice".to_string()));
        pride_and_prejudice.insert("publish_date".to_string(), StrVal("1813".to_string()));

        let mut m = HashMap::new();
        m.insert("first_name".to_string(), StrVal("Jane".to_string()));
        m.insert("last_name".to_string(), StrVal("Austen".to_string()));
        m.insert("age".to_string(), StrVal("41".to_string()));
        m.insert("died".to_string(), Bool(true));
        m.insert("works".to_string(), VecVal(vec!(
            StrVal("Sense and Sensibility".to_string()),
            Map(pride_and_prejudice))));

        assert_eq!(
            MapBuilder::new()
                .insert_str("first_name", "Jane")
                .insert_str("last_name", "Austen")
                .insert("age", &41usize).ok().unwrap()
                .insert_bool("died", true)
                .insert_vec("works", |builder| {
                    builder
                        .push_str("Sense and Sensibility")
                        .push_map(|builder| {
                            builder
                                .insert_str("title", "Pride and Prejudice")
                                .insert("publish_date", &1813usize).ok().unwrap()
                        })
                })
                .build(),
            Map(m));
    }

    #[test]
    fn test_map_fn_builder() {
        // We can't directly compare closures, so just make sure we thread
        // through the builder.

        let mut count = 0usize;
        let data = MapBuilder::new()
            .insert_fn("count".to_string(), move |s| {
                count += 1usize;
                s.clone() + count.to_string().as_slice()
            })
            .build();

        match data {
            Map(m) => {
                match *m.get(&"count".to_string()).unwrap() {
                    Fun(ref f) => {
                        let f = &mut *f.borrow_mut();
                        assert_eq!((*f)("count: ".to_string()), "count: 1".to_string());
                        assert_eq!((*f)("count: ".to_string()), "count: 2".to_string());
                        assert_eq!((*f)("count: ".to_string()), "count: 3".to_string());
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_vec_fn_builder() {
        // We can't directly compare closures, so just make sure we thread
        // through the builder.

        let mut count = 0usize;
        let data = VecBuilder::new()
            .push_fn(move |s| {
                count += 1usize;
                s + count.to_string().as_slice()
            })
            .build();

        match data {
            VecVal(vs) => {
                match vs.as_slice() {
                    [Fun(ref f)] => {
                        let f = &mut *f.borrow_mut();
                        assert_eq!((*f)("count: ".to_string()), "count: 1".to_string());
                        assert_eq!((*f)("count: ".to_string()), "count: 2".to_string());
                        assert_eq!((*f)("count: ".to_string()), "count: 3".to_string());
                    }
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }
}
