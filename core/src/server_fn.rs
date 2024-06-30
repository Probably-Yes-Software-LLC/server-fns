use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    self, parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token::Comma,
    Attribute, Block, Expr, FnArg, Generics, Ident, ItemFn, LitStr, PatType, Receiver, ReturnType,
    Token, Type, WherePredicate
};

use crate::parse::ServerFnArgs;

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
    pub output: ReturnType,
    pub block: Block,
    pub register_route: Expr
}

pub struct StatefulHandler {
    pub span: Span,
    pub ident: Ident,
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

fn make_where_predicate(span: Span, arg_type: &Type) -> WherePredicate {
    parse_quote_spanned! { span =>
        #arg_type: ::server_fns::axum::extract::FromRef<State>
    }
}

mod server_fn_impl {
    use super::*;

    impl ServerFn {
        pub fn try_new(fn_args: ServerFnArgs, mut server_fn: ItemFn) -> Result<Self, syn::Error> {
            let span = server_fn.span();
            let fn_ident = &server_fn.sig.ident;

            let ServerFnArgs {
                path: http_path,
                method,
                middlewares
            } = fn_args;

            let http_method = method.or

            let router_fn_ident = format_ident!("{fn_ident}_router");
            let router_mod_ident = format_ident!("__{router_fn_ident}");
            let stateful_fn_ident = format_ident!("{}_{fn_ident}", http_method);

            let args_span = server_fn.sig.inputs.span();
            let input_args = server_fn
                .sig
                .inputs
                .iter()
                .map(|arg| match arg {
                    FnArg::Receiver(rec) => Err(reciever_error(rec)),
                    FnArg::Typed(typ) => Ok(typ)
                })
                .collect::<Result<Vec<_>, _>>()?;

            let router_fn = RouterFn::try_new(
                args_span,
                router_fn_ident,
                input_args.clone(),
                http_path,
                http_method,
                &stateful_fn_ident,
                middlewares
            )?;
            let stateful_handler = StatefulHandler::try_new(
                args_span,
                stateful_fn_ident,
                input_args,
                &server_fn.sig.output,
                fn_ident
            )?;
            let inner_handler = InnerHandler::try_new(server_fn)?;

            Ok(Self {
                span,
                router_mod: router_mod_ident,
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
                pub mod #router_mod {
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
    use crate::{current_package, make_router, parse::Middleware};

    impl RouterFn {
        pub fn try_new<'a>(
            span: Span,
            ident: Ident,
            inputs: impl IntoIterator<Item = &'a PatType>,
            http_path: LitStr,
            http_method: Ident,
            handler_ident: &Ident,
            middlewares: Vec<Middleware>
        ) -> Result<Self, syn::Error> {
            let state_attr = state_attr();

            let mut gens = Generics {
                lt_token: Some(<Token![<]>::default()),
                gt_token: Some(<Token![>]>::default()),
                params: parse_quote_spanned! { span => State },
                where_clause: parse_quote_spanned! { span =>
                    where
                        State: ::std::clone::Clone +
                                ::std::marker::Send +
                                ::std::marker::Sync +
                                'static

                }
            };

            for next in inputs {
                if next.attrs.contains(&state_attr) {
                    let next_span = next.span();
                    let next_pred = make_where_predicate(next_span, &next.ty);

                    gens.make_where_clause().predicates.push(next_pred);
                }
            }

            let output = parse_quote_spanned! { span =>
                -> ::server_fns::axum::Router<State>
            };

            let layers = middlewares
                .into_iter()
                .map(|Middleware { strat, expr }| -> Expr {
                    match strat {
                        RoutingStrategy::AfterRouting => parse_quote_spanned! { span =>
                            .route_layer(#expr)
                        },
                        RoutingStrategy::BeforeRouting => parse_quote_spanned! { span =>
                            .layer(#expr)
                        }
                    }
                });

            let block = parse_quote_spanned! { span => {
                ::server_fns::axum::Router::new().route(
                    #http_path,
                    ::server_fns::axum::routing::#http_method(#handler_ident)
                )
                #(#layers)*
            }};

            let pkg_router_ident = make_router(current_package(span)?);
            let register_route = parse_quote_spanned! { span =>
                ::server_fns::inventory::submit! {
                    #pkg_router_ident::register(#http_path, #ident)
                }
            };

            Ok(Self {
                span,
                ident,
                gens,
                output,
                block,
                register_route
            })
        }
    }

    impl ToTokens for RouterFn {
        fn to_tokens(&self, tokens: &mut TokenStream2) {
            let Self {
                span,
                ident,
                gens,
                output,
                block,
                register_route
            } = self;

            let (_, gen_types, where_clause) = gens.split_for_impl();

            tokens.append_all(quote_spanned! { *span =>
                pub fn #ident #gen_types () #output
                #where_clause
                #block

                #register_route
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
            inputs: impl IntoIterator<Item = &'a PatType>,
            output: &ReturnType,
            handler_fn_ident: &Ident
        ) -> Result<Self, syn::Error> {
            #[derive(Default)]
            struct BuildArgs {
                args: Punctuated<FnArg, Comma>,
                handler_args: Punctuated<Expr, Comma>
            }

            let state_attr = state_attr();
            let mut build_args = BuildArgs::default();

            let inputs = inputs.into_iter().enumerate();

            for (i, next) in inputs {
                let next_span = next.span();
                let next_type = &next.ty;
                let arg_ident = format_ident!("arg{i}");

                let BuildArgs {
                    ref mut args,
                    ref mut handler_args
                } = build_args;

                args.push(if next.attrs.contains(&state_attr) {
                    parse_quote_spanned! { next_span =>
                        ::server_fns::axum::extract::State(#arg_ident):
                            ::server_fns::axum::extract::State<#next_type>
                    }
                } else {
                    parse_quote_spanned! { next_span => #arg_ident: #next_type }
                });

                handler_args.push(parse_quote_spanned! { next_span => #arg_ident });
            }

            let BuildArgs { args, handler_args } = build_args;

            let block = parse_quote_spanned! { span => {
                #handler_fn_ident(#handler_args).await
            }};

            Ok(Self {
                span,
                ident,
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
                args,
                output,
                block
            } = self;

            tokens
                .append_all(quote_spanned! { *span => pub async fn #ident (#args) #output #block });
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
