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
        {
            use microservice::stats::StatsPresenter;
            use actix_web::Error;
            use futures::future::{Future, ok as fut_ok};

            impl StatsPresenter<()> for #name {
                fn get_stats(&self) -> Box<dyn Future<Item = (), Error = Error>> {
                    Box::new(fut_ok(()))
                }
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
                ::microservice::server::json_error::ErrorBuilder::from(self)
                    .finish()
                    .error_response()
            }
        }
    };
    gen.into()
}
