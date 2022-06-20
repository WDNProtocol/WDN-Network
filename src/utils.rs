#[macro_export]
macro_rules! module_quick_from {
    ($t: ty, $te: ty) => {
        impl From<$t> for $te {
            fn from(cause: $t) -> $te {
                let mut e = <$te>::default();
                e.message = format!("{:?}", &cause);
                e
            }
        }
    };
}
