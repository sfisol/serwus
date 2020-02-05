// Empty implementation for statsPresenter

#![warn(clippy::all)]

extern crate proc_macro;

use crate::proc_macro::TokenStream;
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
