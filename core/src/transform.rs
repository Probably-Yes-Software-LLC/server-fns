use std::env;

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::{spanned::Spanned, Ident, ItemFn, ItemStruct};

use crate::{
    middleware::MiddlewareImpl, parse::ServerFnArgs, server_fn::ServerFn,
    server_state::ServerStateImpl, AttrMacro, DeriveMacro, HttpMethod
};

pub(crate) fn current_package(span: Span) -> Result<String, syn::Error> {
    env::var("CARGO_PKG_NAME").map_err(|err| syn::Error::new(span, err.to_string()))
}

pub(crate) fn make_router(make: impl AsRef<str>) -> Ident {
    format_ident!("{}Router", make.as_ref().to_case(Case::Pascal))
}

pub struct ServerFnAttr;

impl ServerFnAttr {
    // pub fn transform_method<IntoTokens, FromTokens>(
    //     method: HttpMethod,
    //     args: IntoTokens,
    //     body: IntoTokens
    // ) -> FromTokens
    // where
    //     IntoTokens: Into<<Self as AttrMacro>::TokenStream>,
    //     FromTokens: From<<Self as AttrMacro>::TokenStream>
    // {
    //     let body = body.into();

    //     let mut args: ServerFnArgs = match syn::parse2(args.into()) {
    //         Ok(args) => args,
    //         Err(err) => {
    //             let error = format!("Invalid server_fn args; {err:?}");

    //             return Err(quote! {
    //                 const SERVER_ATTR_ERROR: [&'static str; 0] = [#error];
    //                 #body
    //             });
    //         }
    //     };
    //     args.method = Some(Ident::new(method.as_ref(), args.span()));

    //     Self::transform(args.into_token_stream(), body)
    // }
}

impl AttrMacro for ServerFnAttr {
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
    type TokenStream = <ServerFnAttr as AttrMacro>::TokenStream;
    type Error = <ServerFnAttr as AttrMacro>::Error;
    type Result = <ServerFnAttr as AttrMacro>::Result;

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
        args.method = Some(Ident::new(self.0.as_ref(), body.span()));

        ServerFnAttr.transform2(args.into_token_stream(), body)
    }
}

pub struct MiddlewareAttr;

impl AttrMacro for MiddlewareAttr {
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

pub struct ServerStateDerive;

impl DeriveMacro for ServerStateDerive {
    type TokenStream = TokenStream2;
    type Error = syn::Error;
    type Result = Result<Self::TokenStream, Self::Error>;

    fn transform2(&self, item: Self::TokenStream) -> Self::Result {
        let annotated_struct: ItemStruct = syn::parse2(item)?;
        let derive_impl = ServerStateImpl::try_new(annotated_struct)?;

        Ok(quote!(#derive_impl))
    }
}
