use std::fmt;
use std::rc::Rc;

/// A cloneable, type-erased callback.
///
/// Wraps a function `Fn(A) -> R` in an `Rc` so it can be cheaply cloned
/// and passed around as a prop.
pub struct Callback<A = (), R = ()> {
    f: Rc<dyn Fn(A) -> R>,
}

impl<A, R> Callback<A, R> {
    pub fn new(f: impl Fn(A) -> R + 'static) -> Self {
        Self { f: Rc::new(f) }
    }

    pub fn call(&self, arg: A) -> R {
        (self.f)(arg)
    }
}

impl<A, R> Clone for Callback<A, R> {
    fn clone(&self) -> Self {
        Self {
            f: Rc::clone(&self.f),
        }
    }
}

impl<A, R> fmt::Debug for Callback<A, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Callback(<fn>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_new_and_call() {
        let cb = Callback::new(|x: i32| x * 2);
        assert_eq!(cb.call(5), 10);
        assert_eq!(cb.call(0), 0);
        assert_eq!(cb.call(-3), -6);
    }

    #[test]
    fn callback_clone() {
        let cb = Callback::new(|x: i32| x + 1);
        let cb2 = cb.clone();
        assert_eq!(cb.call(10), 11);
        assert_eq!(cb2.call(10), 11);
    }

    #[test]
    fn callback_unit_args() {
        let called = Rc::new(std::cell::Cell::new(false));
        let called_clone = Rc::clone(&called);
        let cb = Callback::new(move |()| {
            called_clone.set(true);
        });
        cb.call(());
        assert!(called.get());
    }

    #[test]
    fn callback_string_return() {
        let cb = Callback::new(|name: &str| format!("Hello, {name}!"));
        assert_eq!(cb.call("world"), "Hello, world!");
    }

    #[test]
    fn callback_debug() {
        let cb = Callback::new(|_: ()| {});
        assert_eq!(format!("{cb:?}"), "Callback(<fn>)");
    }
}
