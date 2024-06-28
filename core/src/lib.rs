mod macro_traits;
mod parse;
mod server_fn;
pub mod server_state;

pub mod router;

use std::env;

use convert_case::{Case, Casing};
pub use macro_traits::*;
use parse::{CollectMiddleware, RouteMeta};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Ident, ItemFn, ItemStruct};

use crate::{server_fn::ServerFn, server_state::ServerStateImpl};

pub struct ServerFnsAttr;

impl AttrMacro for ServerFnsAttr {
    type TokenStream = TokenStream2;
    type Error = syn::Error;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream
    ) -> Result<Self::TokenStream, Self::Error> {
        let annotated_fn: ItemFn = syn::parse2(body)?;
        let meta = RouteMeta::parse(args, &annotated_fn.sig.ident)?;
        let server_fn = ServerFn::try_new(meta, annotated_fn)?;

        Ok(quote!(#server_fn))
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
