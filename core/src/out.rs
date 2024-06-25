use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    punctuated::Punctuated, spanned::Spanned, token::Comma, Block, FnArg, Generics, Ident, ItemFn,
    ReturnType, TypeGenerics, WhereClause
};

pub struct ServerFn {
    pub span: Span,
    pub router_mod: Ident,
    pub router_fn: RouterFn,
    pub stateful_handler: StatefulHandler,
    pub inner_handler: InnerHandler
}

pub struct RouterFn {
    pub span: Span,
    pub ident: Ident,
    pub gens: Generics,
    pub args: Punctuated<FnArg, Comma>,
    pub block: Block
}

pub struct StatefulHandler {
    pub span: Span,
    pub ident: Ident,
    pub gens: Generics,
    pub args: Punctuated<FnArg, Comma>,
    pub output: ReturnType,
    pub block: Block
}

pub struct InnerHandler {
    pub span: Span,
    pub handler_fn: ItemFn
}

mod server_fn {

    use super::*;

    impl ServerFn {
        pub fn try_new(server_fn: ItemFn) -> Result<Self, syn::Error> {
            let span = server_fn.span();
            let fn_ident = &server_fn.sig.ident;

            let router_fn = format_ident!("{fn_ident}_router");
            let router_mod = format_ident!("__{router_fn}");

            let router_fn = RouterFn::try_new(router_fn, &server_fn.sig.inputs)?;
            let stateful_handler = StatefulHandler::try_new()?;
            let inner_handler = InnerHandler::try_new(server_fn)?;

            Ok(Self {
                span,
                router_mod,
                router_fn,
                stateful_handler,
                inner_handler
            })
        }
    }

    impl ToTokens for ServerFn {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            let Self {
                span,
                router_mod,
                router_fn,
                stateful_handler,
                inner_handler
            } = self;

            tokens.append_all(quote_spanned! { *span =>
                mod #router_mod {
                    use super::*;

                    #router_fn
                    #stateful_handler
                }
                #inner_handler
            });
        }
    }
}

mod router_fn {
    use super::*;

    impl RouterFn {
        pub fn try_new(
            ident: Ident,
            inputs: &Punctuated<FnArg, Comma>
        ) -> Result<Self, syn::Error> {
            let span = inputs.span();

            Ok(Self {
                span,
                ident,
                gens: todo!(),
                args: todo!(),
                block: todo!()
            })
        }
    }

    impl ToTokens for RouterFn {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            let Self {
                span,
                ident,
                gens,
                args,
                block
            } = self;

            let (_, gen_types, where_clause) = gens.split_for_impl();

            tokens.append_all(quote_spanned! { *span =>
                fn #ident #gen_types (#args) -> ::server_fns::axum::Router
                #where_clause
                #block
            });
        }
    }
}

mod stateful_handler {
    use super::*;

    impl ToTokens for StatefulHandler {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            let Self {
                span,
                ident,
                gens,
                args,
                output,
                block
            } = self;

            let (_, gen_types, where_clause) = gens.split_for_impl();

            tokens.append_all(quote_spanned! { *span =>
                async fn #ident #gen_types (#args) #output
                #where_clause
                #block
            });
        }
    }
}

mod inner_handler {
    use super::*;

    impl ToTokens for InnerHandler {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            let Self { span, handler_fn } = self;

            tokens.append_all(quote_spanned! { *span => #handler_fn });
        }
    }
}
