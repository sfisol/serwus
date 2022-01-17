// Empty implementation for statsPresenter

#![deny(rust_2018_idioms)]
#![warn(clippy::all)]

use proc_macro::TokenStream;
use quote::quote;

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
                // match b_err {
                //     ::actix_web::error::BlockingError::Canceled => Self::Canceled,
                //     ::actix_web::error::BlockingError::Error(err) => err,
                // }
            }
        }
    };
    gen.into()
}
