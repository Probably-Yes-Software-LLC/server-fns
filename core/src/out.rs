use itertools::Itertools;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token::Comma,
    Block, Expr, FnArg, Generics, Ident, ItemFn, PatType, ReturnType, TypeGenerics, WhereClause
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

            let router_fn = RouterFn::try_new(args_span, router_fn, input_args)?;
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

    use syn::WherePredicate;

    use super::*;

    impl RouterFn {
        pub fn try_new<'a>(
            span: Span,
            ident: Ident,
            inputs: impl IntoIterator<Item = (usize, &'a PatType)>
        ) -> Result<Self, syn::Error> {
            // Assemble generics and arguments.
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

                        build_args
                            .gens
                            .lt_token
                            .get_or_insert_with(|| parse_quote_spanned! { next_span => < });
                        build_args
                            .gens
                            .gt_token
                            .get_or_insert_with(|| parse_quote_spanned! { next_span => >});
                        build_args
                            .gens
                            .params
                            .push(parse_quote_spanned! { next_span => #state_type});
                        build_args
                            .gens
                            .make_where_clause()
                            .predicates
                            .extend::<Punctuated<WherePredicate, Comma>>(
                                parse_quote_spanned! { next_span =>
                                    #next_type: ::server_fns::axum::extract::FromRef<#state_type>,
                                    #state_type: ::std::marker::Send + ::std::marker::Sync
                                }
                            );

                        build_args.args.push(parse_quote_spanned! { next_span =>
                            #state_arg: #state_type
                        });

                        build_args
                            .state_args
                            .push(parse_quote_spanned! { next_span => #state_arg });
                    }

                    build_args
                });

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
