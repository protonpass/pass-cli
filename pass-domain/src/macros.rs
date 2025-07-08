#[macro_export]
macro_rules! display_for_basic {
    ($t:ty) => {
        impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

#[macro_export]
macro_rules! assert_enum_variant {
    ($v:expr, $p:path) => {
        assert_enum_variant!($v, $p, "");
    };
    ($v:expr, $p:path, $($msg:tt)+) => {
        match &$v {
            $p { .. } => (),
            _ => panic!(r#"assertion failed: `(left == right)`
  left: `{:?}`,
 right: `{}`: {}"#, &$v, stringify!($p), format_args!($($msg)+)),
        }
    };
}

#[macro_export]
macro_rules! assert_false {
    ($val:expr) => {
        assert_false!($val, "");
    };
    ($val:expr, $($msg:tt)+) => {
        assert_eq!($val, false, "{}", format_args!($($msg)+));
    }
}

#[macro_export]
macro_rules! assert_true {
    ($val:expr) => {
        assert_true!($val, "Value should be true");
    };
    ($val:expr, $($msg:tt)+) => {
        assert!($val, "{}", format_args!($($msg)+));
    }
}
