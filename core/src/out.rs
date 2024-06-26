use itertools::Itertools;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Gt, Lt},
    Attribute, Block, Expr, FnArg, Generics, Ident, ItemFn, PatType, Receiver, ReturnType, Type,
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

pub fn reciever_error(rec: &Receiver) -> syn::Error {
    syn::Error::new(
        rec.span(),
        "Reciever type 'self' is not supported in server functions."
    )
}

fn state_attr() -> Attribute {
    parse_quote!(#[state])
}

fn make_where_predicates(
    span: Span,
    arg_type: &Type,
    state_type: &Ident
) -> Punctuated<WherePredicate, Comma> {
    parse_quote_spanned! { span =>
        #arg_type: ::server_fns::axum::extract::FromRef<#state_type>,
        #state_type: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + 'static
    }
}

mod server_fn {
    use super::*;

    impl ServerFn {
        pub fn try_new(meta: RouteMeta, server_fn: ItemFn) -> Result<Self, syn::Error> {
            let span = server_fn.span();
            let fn_ident = &server_fn.sig.ident;

            let RouteMeta {
                http_path,
                http_method
            } = meta;

            let router_fn = format_ident!("{fn_ident}_router");
            let router_mod = format_ident!("__{router_fn}");
            let stateful_handler_fn = format_ident!("{}_{fn_ident}", http_method);
            let route_expr = parse_quote_spanned! { span =>
                ::server_fns::axum::routing::#http_method(#stateful_handler_fn)
            };

            let args_span = server_fn.sig.inputs.span();
            let input_args = server_fn
                .sig
                .inputs
                .iter()
                .map(|arg| match arg {
                    FnArg::Receiver(rec) => Err(reciever_error(rec)),
                    FnArg::Typed(typ) => Ok(typ)
                })
                .fold_ok(vec![], |mut args, next| {
                    args.push(next);
                    args
                })?
                .into_iter()
                .enumerate();

            let router_fn = RouterFn::try_new(
                args_span,
                router_fn,
                input_args.clone(),
                http_path,
                route_expr
            )?;
            let stateful_handler = StatefulHandler::try_new(
                args_span,
                stateful_handler_fn,
                input_args,
                &server_fn.sig.output,
                fn_ident
            )?;
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
            http_path: LitStr,
            route_expr: Expr
        ) -> Result<Self, syn::Error> {
            #[derive(Default)]
            struct BuildArgs {
                gens: Generics,
                args: Punctuated<FnArg, Comma>,
                state_args: Vec<Expr>
            }

            let state_attr = state_attr();
            let mut build_args = BuildArgs::default();

            for (i, next) in inputs {
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
                    gens.make_where_clause()
                        .predicates
                        .extend(make_where_predicates(next_span, next_type, &state_type));

                    args.push(parse_quote_spanned! { next_span =>
                        #state_arg: #state_type
                    });

                    state_args.push(parse_quote_spanned! { next_span => #state_arg });
                }
            }

            let BuildArgs {
                gens,
                args,
                state_args
            } = build_args;

            let block = parse_quote_spanned! { span => {
                ::server_fns::axum::Router::new()
                    .route(#http_path, #route_expr)
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
        pub fn try_new<'a>(
            span: Span,
            ident: Ident,
            inputs: impl IntoIterator<Item = (usize, &'a PatType)>,
            output: &ReturnType,
            handler_fn_ident: &Ident
        ) -> Result<Self, syn::Error> {
            #[derive(Default)]
            struct BuildArgs {
                gens: Generics,
                args: Punctuated<FnArg, Comma>,
                handler_args: Punctuated<Expr, Comma>
            }

            let state_attr = state_attr();
            let mut build_args = BuildArgs::default();

            for (i, next) in inputs {
                let next_span = next.span();
                let next_type = &next.ty;
                let arg_ident = format_ident!("arg{i}");

                let BuildArgs {
                    ref mut gens,
                    ref mut args,
                    ref mut handler_args
                } = build_args;

                if next.attrs.contains(&state_attr) {
                    let state_type = format_ident!("State{i}");

                    if gens.lt_token.is_none() {
                        gens.lt_token = Some(Lt::default());
                    }
                    if gens.gt_token.is_none() {
                        gens.gt_token = Some(Gt::default());
                    }

                    gens.params
                        .push(parse_quote_spanned! { next_span => #state_type});
                    gens.make_where_clause()
                        .predicates
                        .extend(make_where_predicates(next_span, next_type, &state_type));

                    args.push(parse_quote_spanned! { next_span =>
                        ::server_fns::axum::extract::State(#arg_ident):
                            ::server_fns::axum::extract::State<#next_type>
                    });
                } else {
                    args.push(parse_quote_spanned! { next_span => #arg_ident: #next_type });
                }

                handler_args.push(parse_quote_spanned! { next_span => #arg_ident });
            }

            let BuildArgs {
                gens,
                args,
                handler_args
            } = build_args;

            let block = parse_quote_spanned! { span => {
                #handler_fn_ident(#handler_args).await
            }};

            Ok(Self {
                span,
                ident,
                gens,
                args,
                output: output.clone(),
                block
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

    impl InnerHandler {
        pub fn try_new(mut handler_fn: ItemFn) -> Result<Self, syn::Error> {
            let state_attr = state_attr();
            let span = handler_fn.span();

            for input in &mut handler_fn.sig.inputs {
                match input {
                    FnArg::Receiver(rec) => return Err(reciever_error(rec)),
                    FnArg::Typed(arg) => {
                        arg.attrs.retain(|attr| attr != &state_attr);
                    }
                }
            }

            Ok(Self { span, handler_fn })
        }
    }

    impl ToTokens for InnerHandler {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            let Self { span, handler_fn } = self;

            tokens.append_all(quote_spanned! { *span => #handler_fn });
        }
    }
}
