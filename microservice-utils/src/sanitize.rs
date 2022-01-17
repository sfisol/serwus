pub trait Sanitize {
    fn sanitize(&mut self);
}

#[macro_export]
macro_rules! sanitizable {
    ($model:ident, $( $field:ident ),* ) => {
        use $crate::Sanitize;
        impl Sanitize for $model {
            fn sanitize(&mut self) {
                $(self.$field = self.$field.trim().to_string();)*
            }
        }
    };
}
