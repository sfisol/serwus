/// Depreacted, use [SanitizedString](crate::utils::SanitizedString)
#[deprecated]
pub trait Sanitize {
    fn sanitize(&mut self);
}

/// Depreacted, use [SanitizedString](crate::utils::SanitizedString)
#[deprecated]
#[macro_export]
macro_rules! sanitizable {
    ($model:ident, $( $field:ident ),* ) => {
        impl $crate::Sanitize for $model {
            fn sanitize(&mut self) {
                $(self.$field = self.$field.trim().to_string();)*
            }
        }
    };
}
