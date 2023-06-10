struct DisplayWrapper<'a> {
    inner: &'a dyn JSONifyable
}

impl<'a> std::fmt::Display for DisplayWrapper<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.to_json(f)
    }
}

pub trait JSONifyable {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = w;
        panic!("JSONifyable: to_json not implemented");
    }

    fn as_json_string(&self) -> String where Self: Sized {
        let wrapper = DisplayWrapper { inner: self };
        format!("{}", wrapper)
    }
}

impl JSONifyable for usize {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result { write!(w, "{:?}", self) }
}

impl JSONifyable for isize {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result { write!(w, "{:?}", self) }
}

impl JSONifyable for u64 {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result { write!(w, "{:?}", self) }
}

impl JSONifyable for i64 {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result { write!(w, "{:?}", self) }
}

impl JSONifyable for String {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result { write!(w, "{:?}", self) }
}

impl<T> JSONifyable for Option<T> where T: JSONifyable {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Some(x) => x.to_json(w),
            None => write!(w, "null")
        }
    }
}

impl<T> JSONifyable for Vec<T> where T: JSONifyable {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(w, "[ ")?;
        for (i, val) in self.iter().enumerate() {
            if i != 0 {
                write!(w, ", ")?;
            }
            val.to_json(w)?;
        }
        write!(w, " ]")
    }
}

impl<T> JSONifyable for std::collections::HashMap<String, T> where T: JSONifyable {
    fn to_json(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(w, "{{ ")?;
        for (i, (key, val)) in self.iter().enumerate() {
            if i != 0 {
                write!(w, ", ")?;
            }
            write!(w, "{:?}: ", key)?;
            val.to_json(w)?;
        }
        write!(w, " }}")
    }
}

#[cfg(test)]
mod tests {
    use super::JSONifyable;
    use jsonify_derive::JSONifyable;

    #[test]
    fn default() {
        #[derive(JSONifyable)]
        struct Foo;
    }

    #[derive(JSONifyable)]
    struct Stuff {
        foo: i64,
        name: String,
        numbers: Vec<usize>
    }

    #[test]
    fn stuff_1() {
        let stuff = Stuff {
            foo: 42,
            name: "Hello, World!".to_string(),
            numbers: vec![1, 2, 3, 4, 5]
        };

        let json = stuff.as_json_string();
        println!("stuff as json: {}", json);
        assert_eq!(json,
            "{\"foo\": 42, \"name\": \"Hello, World!\", \"numbers\": [ 1, 2, 3, 4, 5 ]}")
    }

}

