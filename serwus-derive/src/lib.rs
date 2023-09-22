#![deny(rust_2018_idioms)]
#![warn(clippy::all)]

use proc_macro::TokenStream;
use quote::quote;

// Empty implementation for statsPresenter
#[proc_macro_derive(EmptyStats)]
pub fn empty_stats_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_empty_stats_macro(&ast)
}

fn impl_empty_stats_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl ::serwus::server::stats::StatsPresenter<()> for #name {
            fn is_ready(&self) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output=Result<bool, ::actix_web::Error>>>> {
                Box::pin(::std::future::ready(Ok(true)))
            }
            fn get_stats(&self) -> ::std::pin::Pin<Box<dyn ::std::future::Future<Output=Result<(), ::actix_web::Error>>>> {
                Box::pin(::std::future::ready(Ok(())))
            }
        }
    };
    gen.into()
}

#[proc_macro_derive(Canceled)]
pub fn canceled_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_canceled_macro(&ast)
}

fn impl_canceled_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl ::std::convert::From<::actix_web::error::BlockingError> for #name {
            fn from(b_err: ::actix_web::error::BlockingError) -> Self {
                Self::Canceled
            }
        }
    };
    gen.into()
}

/// Implements `ResponseError` for struct if it implements Into<ErrorBuilder>
#[proc_macro_derive(ResponseFromBuilder)]
pub fn response_from_builder_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_response_from_builder(&ast)
}

fn impl_response_from_builder(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl ::actix_web::ResponseError for #name {
            fn error_response(&self) -> ::actix_web::HttpResponse {
                ::serwus::server::json_error::ErrorBuilder::from(self)
                    .finish()
                    .error_response()
            }
        }
    };
    gen.into()
}
