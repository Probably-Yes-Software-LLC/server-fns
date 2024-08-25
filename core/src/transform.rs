use std::env;

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{spanned::Spanned, ExprStruct, Ident, ItemFn, ItemStruct, TypePath};

use crate::{
    embed_asset::LoadAssetImpl, middleware::MiddlewareImpl, parse::ServerFnArgs,
    server_fn::ServerFn, server_state::ServerStateImpl, AttrMacro, DeriveMacro, FnMacro,
    HttpMethod
};

pub(crate) fn current_package(span: Span) -> Result<String, syn::Error> {
    env::var("CARGO_PKG_NAME").map_err(|err| syn::Error::new(span, err.to_string()))
}

pub(crate) fn make_router(make: impl AsRef<str>) -> Ident {
    format_ident!("{}Router", make.as_ref().to_case(Case::Pascal))
}

pub(crate) fn make_server_state(make: impl AsRef<str>) -> Ident {
    format_ident!("{}ServerState", make.as_ref().to_case(Case::Pascal))
}

pub struct ServerFnAttrMacro;

impl AttrMacro for ServerFnAttrMacro {
    type TokenStream = TokenStream2;
    type Error = TokenStream2;
    type Result = Result<Self::TokenStream, Self::Error>;

    fn transform2(&self, args: Self::TokenStream, body: Self::TokenStream) -> Self::Result {
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

pub struct ServerFnMethodAttr(pub HttpMethod);

impl AttrMacro for ServerFnMethodAttr {
    type TokenStream = <ServerFnAttrMacro as AttrMacro>::TokenStream;
    type Error = <ServerFnAttrMacro as AttrMacro>::Error;
    type Result = <ServerFnAttrMacro as AttrMacro>::Result;

    fn transform2(&self, args: Self::TokenStream, body: Self::TokenStream) -> Self::Result {
        let mut args: ServerFnArgs = match syn::parse2(args) {
            Ok(args) => args,
            Err(err) => {
                let error = format!("Invalid server_fn args; {err:?}");

                return Err(quote! {
                    const SERVER_ATTR_ERROR: [&'static str; 0] = [#error];
                    #body
                });
            }
        };

        // Set (or override) the method to the method macro used instead.
        args.method = Some(Ident::new(self.0.as_ref(), body.span()));

        ServerFnAttrMacro.transform2(args.into_token_stream(), body)
    }
}

pub struct MiddlewareAttrMacro;

impl AttrMacro for MiddlewareAttrMacro {
    type TokenStream = TokenStream2;
    type Error = TokenStream2;
    type Result = Result<Self::TokenStream, Self::Error>;

    fn transform2(&self, args: Self::TokenStream, body: Self::TokenStream) -> Self::Result {
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

pub struct ServerStateDeriveMacro;

impl DeriveMacro for ServerStateDeriveMacro {
    type TokenStream = TokenStream2;
    type Error = syn::Error;
    type Result = Result<Self::TokenStream, Self::Error>;

    fn transform2(&self, item: Self::TokenStream) -> Self::Result {
        let annotated_struct: ItemStruct = syn::parse2(item)?;
        let derive_impl = ServerStateImpl::try_new(annotated_struct)?;

        Ok(quote!(#derive_impl))
    }
}

pub struct UseServerStateFnMacro;

impl FnMacro for UseServerStateFnMacro {
    type TokenStream = TokenStream2;
    type Error = syn::Error;
    type Result = Result<Self::TokenStream, Self::Error>;

    fn transform2(&self, typ: Self::TokenStream) -> Self::Result {
        let typ: TypePath = syn::parse2(typ)?;
        let pkg_state = make_server_state(current_package(typ.span())?);

        Ok(quote! {
            #[cfg(feature = "server")]
            pub(crate) type #pkg_state = #typ;
        })
    }
}

pub struct LoadAssetInternalMacro;

impl FnMacro for LoadAssetInternalMacro {
    type TokenStream = TokenStream2;
    type Error = syn::Error;
    type Result = Result<Self::TokenStream, Self::Error>;

    fn transform2(&self, item: Self::TokenStream) -> Self::Result {
        let embed_input: ExprStruct = syn::parse2(item)?;
        let embed_impl = LoadAssetImpl::try_new(embed_input)?;

        Ok(quote!(#embed_impl))
    }
}
