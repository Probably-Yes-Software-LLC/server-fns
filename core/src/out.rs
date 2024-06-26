use itertools::Itertools;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Gt, Lt},
    Block, Expr, FnArg, Generics, Ident, ItemFn, PatType, ReturnType, Type, TypeGenerics,
    WherePredicate
};

use crate::parse::RouteMeta;

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
        pub fn try_new(meta: RouteMeta, server_fn: ItemFn) -> Result<Self, syn::Error> {
            let span = server_fn.span();
            let fn_ident = &server_fn.sig.ident;

            let router_fn = format_ident!("{fn_ident}_router");
            let router_mod = format_ident!("__{router_fn}");
            let stateful_handler_fn = format_ident!("{}_{fn_ident}", meta.http_method);

            let args_span = server_fn.sig.inputs.span();
            let input_args = server_fn
                .sig
                .inputs
                .iter()
                .map(|arg| match arg {
                    FnArg::Receiver(rec) => Err(syn::Error::new(
                        rec.span(),
                        "Reciever type 'self' is not supported in server functions."
                    )),
                    FnArg::Typed(typ) => Ok(typ)
                })
                .fold_ok(vec![], |args, next| {
                    args.push(next);
                    args
                })?
                .into_iter()
                .enumerate();

            let router_fn = RouterFn::try_new(
                args_span,
                router_fn,
                input_args,
                &meta.http_path,
                &stateful_handler_fn
            )?;
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
    use syn::LitStr;

    use super::*;

    impl RouterFn {
        pub fn try_new<'a>(
            span: Span,
            ident: Ident,
            inputs: impl IntoIterator<Item = (usize, &'a PatType)>,
            http_path: &LitStr,
            stateful_handler_ident: &Ident
        ) -> Result<Self, syn::Error> {
            /// Assemble generics, arguments, and [axum::Router::with_state()] parameters.
            #[derive(Default)]
            struct BuildArgs {
                gens: Generics,
                args: Punctuated<FnArg, Comma>,
                state_args: Vec<Expr>
            }

            let state_attr = parse_quote!(#[state]);

            let BuildArgs {
                gens,
                args,
                state_args
            } = inputs
                .into_iter()
                .fold(BuildArgs::default(), |mut build_args, (i, next)| {
                    if next.attrs.contains(&state_attr) {
                        let next_span = next.span();
                        let next_type = &next.ty;

                        let state_type = format_ident!("State{i}");
                        let state_arg = format_ident!("state{i}");

                        let BuildArgs {
                            ref mut gens,
                            ref mut args,
                            ref mut state_args
                        } = build_args;

                        if gens.lt_token.is_none() {
                            gens.lt_token = Some(Lt::default());
                        }
                        if gens.gt_token.is_none() {
                            gens.gt_token = Some(Gt::default());
                        }

                        gens.params
                            .push(parse_quote_spanned! { next_span => #state_type});
                        gens.make_where_clause().predicates.extend(
                            StatefulHandler::make_where_predicates(
                                next_span,
                                &next_type,
                                &state_type
                            )
                        );

                        args.push(parse_quote_spanned! { next_span =>
                            #state_arg: #state_type
                        });

                        state_args.push(parse_quote_spanned! { next_span => #state_arg });
                    }

                    build_args
                });

            let block = parse_quote! {{
                ::server_fns::axum::Router::new()
                    .route(#http_path, #stateful_handler_ident)
                    #(.with_state(#state_args))*
            }};

            Ok(Self {
                span,
                ident,
                gens,
                args,
                block
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

    impl StatefulHandler {
        pub fn make_where_predicates(
            span: Span,
            arg_type: &Type,
            state_type: &Ident
        ) -> Punctuated<WherePredicate, Comma> {
            parse_quote_spanned! { span =>
                #arg_type: ::server_fns::axum::extract::FromRef<#state_type>,
                #state_type: ::std::marker::Send + ::std::marker::Sync
            }
        }

        pub fn try_new<'a>(
            span: Span,
            ident: Ident,
            inputs: impl IntoIterator<Item = (usize, &'a PatType)>,
            handler_fn_ident: &Ident
        ) -> Result<Self, syn::Error> {
            Ok(Self {
                span,
                ident,
                gens: todo!(),
                args: todo!(),
                output: todo!(),
                block: todo!()
            })
        }
    }

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
