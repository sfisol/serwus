#[deprecated]
#[macro_export]
macro_rules! wrap_display {
    ($id_type: ident) => {
        impl std::fmt::Display for $id_type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    }
}
