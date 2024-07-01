mod macro_traits;
pub mod middleware;
mod parse;
mod server_fn;
pub mod server_router;
pub mod server_state;

use std::env;

use convert_case::{Case, Casing};
pub use macro_traits::*;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Ident, ItemFn, ItemStruct};

use crate::{middleware::MiddlewareImpl, server_fn::ServerFn, server_state::ServerStateImpl};

pub struct ServerFnsAttr;

impl AttrMacro for ServerFnsAttr {
    type TokenStream = TokenStream2;
    type Error = TokenStream2;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream
    ) -> Result<Self::TokenStream, Self::Error> {
        let annotated_fn = match syn::parse2::<ItemFn>(body.clone()) {
            Ok(fun) => fun,
            Err(err) => {
                let error = format!("Invalid server_fn input; {err:?}");

                return Err(quote! {
                    const SERVER_ATTR_ERROR: [&'static str; 0] = [#error];
                    #body
                });
            }
        };

        let args = match syn::parse2(args) {
            Ok(args) => args,
            Err(err) => {
                let error = format!("Invalid server_fn args; {err:?}");

                return Err(quote! {
                    const SERVER_ATTR_ERROR: [&'static str; 0] = [#error];
                    #body
                });
            }
        };

        let server_fn = match ServerFn::try_new(args, annotated_fn) {
            Ok(fun) => fun,
            Err(err) => {
                let error = format!("Error constructing server function route; {err:?}");

                return Err(quote! {
                    const SERVER_ATTR_ERROR: [&'static str; 0] = [#error];
                    #body
                });
            }
        };

        Ok(quote!(#server_fn))
    }
}

pub struct MiddlewareAttr;

impl AttrMacro for MiddlewareAttr {
    type TokenStream = TokenStream2;
    type Error = TokenStream2;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream
    ) -> Result<Self::TokenStream, Self::Error> {
        let annotated_fn = match syn::parse2::<ItemFn>(body.clone()) {
            Ok(fun) => fun,
            Err(err) => {
                let error = format!("Invalid middleware input; {err:?}");

                return Err(quote! {
                    const MIDDLEWARE_ATTR_ERROR: [&'static str; 0] = [#error];
                    #body
                });
            }
        };

        let middleware = match MiddlewareImpl::try_new(args, annotated_fn) {
            Ok(middleware) => middleware,
            Err(err) => {
                let error = format!("Invalid middleware input; {err:?}");

                return Err(quote! {
                    const MIDDLEWARE_ATTR_ERROR: [&'static str; 0] = [#error];
                    #body
                });
            }
        };

        Ok(quote!(#middleware))
    }
}

pub struct ServerStateDerive;

impl DeriveMacro for ServerStateDerive {
    type TokenStream = TokenStream2;
    type Error = syn::Error;

    fn transform2(item: Self::TokenStream) -> Result<Self::TokenStream, Self::Error> {
        let annotated_struct: ItemStruct = syn::parse2(item)?;
        let derive_impl = ServerStateImpl::try_new(annotated_struct)?;

        Ok(quote!(#derive_impl))
    }
}

fn current_package(span: Span) -> Result<String, syn::Error> {
    env::var("CARGO_PKG_NAME").map_err(|err| syn::Error::new(span, err.to_string()))
}

fn make_router(make: impl AsRef<str>) -> Ident {
    format_ident!("{}Router", make.as_ref().to_case(Case::Pascal))
}
