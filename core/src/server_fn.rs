use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    self, parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token::Comma,
    Attribute, Block, Expr, ExprMacro, FnArg, Generics, Ident, ItemConst, ItemFn, LitStr, PatType,
    Receiver, ReturnType, Token, Type, WherePredicate
};

use crate::parse::ServerFnArgs;

pub struct ServerFn {
    pub span: Span,
    pub route_const: ItemConst,
    pub format_url_fn: ItemFn,
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

pub(crate) fn reciever_error(rec: &Receiver) -> syn::Error {
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
    use convert_case::{Case, Casing};
    use itertools::Itertools;

    use super::*;

    impl ServerFn {
        pub fn try_new(fn_args: ServerFnArgs, server_fn: ItemFn) -> Result<Self, syn::Error> {
            let span = server_fn.span();
            let fn_ident = &server_fn.sig.ident;

            let ServerFnArgs {
                path,
                method,
                embed,
                middlewares
            } = fn_args;

            let http_method = method
                .map_or(Some("post".into()), |method| {
                    Some(method.to_string().to_lowercase())
                })
                .map(|method| Ident::new(&method, Span::mixed_site()))
                .unwrap();

            let http_path = path.unwrap_or_else(|| {
                LitStr::new(
                    &format!("/api/{}", fn_ident.to_string().to_case(Case::Kebab)),
                    fn_ident.span()
                )
            });

            let http_path_str = http_path.value();
            let mut route_parts = http_path_str.split('/').collect_vec();

            let format_url_params = route_parts
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    p.strip_prefix(':').map(|param| {
                        let param = format_ident!("{param}");
                        let pat_type = parse_quote_spanned! { http_path.span() =>
                            #param: impl ::std::fmt::Display
                        };
                        (i, param, pat_type)
                    })
                })
                .collect::<Vec<(_, _, PatType)>>();

            for (index, _, _) in &format_url_params {
                route_parts[*index] = "{}";
            }

            let format_url_param_count = format_url_params.len();

            let (format_url_param_names, format_url_params): (Vec<_>, Vec<_>) = format_url_params
                .into_iter()
                .map(|(_, n, p)| (n, p))
                .unzip();
            let format_url_fmt_str = route_parts.join("/");

            let router_fn_ident = format_ident!("{fn_ident}_router");
            let router_mod_ident = format_ident!("__{router_fn_ident}");
            let stateful_fn_ident = format_ident!("{http_method}_{fn_ident}");
            let route_const_ident = format_ident!(
                "{}",
                stateful_fn_ident.to_string().to_case(Case::UpperSnake)
            );
            let format_url_fn_ident = format_ident!("{stateful_fn_ident}_url");

            let route_const = parse_quote_spanned! { fn_ident.span() =>
                pub const #route_const_ident: &'static str = #http_path;
            };

            let format_url_fn: ItemFn = parse_quote_spanned! { http_path.span() =>
                pub fn #format_url_fn_ident(#(#format_url_params),*) -> String {
                    if #format_url_param_count > 0 {
                        ::std::format!(#format_url_fmt_str, #(#format_url_param_names),*)
                    } else {
                        #format_url_fmt_str.into()
                    }
                }
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

            let inner_handler = InnerHandler::try_new(embed, server_fn)?;

            Ok(Self {
                span,
                route_const,
                format_url_fn,
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
                route_const,
                format_url_fn,
                router_mod,
                router_fn,
                stateful_handler,
                inner_handler
            } = self;

            tokens.append_all(quote_spanned! { *span =>
                #[allow(unused, clippy::redundant_static_lifetimes)]
                #route_const

                #format_url_fn

                #[cfg(feature = "server")]
                mod #router_mod {
                    use super::*;

                    #router_fn
                    #stateful_handler
                }

                #[cfg(feature = "server")]
                #inner_handler
            });
        }
    }
}

mod router_fn {
    use super::*;
    use crate::{current_package, make_server_state, parse::Middleware};

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

            let block = parse_quote_spanned! { span =>
                {
                    use ::server_fns::axum::Router;
                    use ::server_fns::axum::routing;

                    #[allow(clippy::let_and_return)]
                    let router = Router::new()
                        .route(#http_path, routing::#http_method(#handler_ident));

                    #(
                        ::server_fns::layer_middleware!(#middlewares for router);
                    )*

                    router
                }
            };

            let pkg_server_state = make_server_state(current_package(span)?);
            let register_route = parse_quote_spanned! { span =>
                ::server_fns::inventory::submit! {
                    <
                        #pkg_server_state
                        as
                        ::server_fns::server_state::ServerState
                    >
                    ::Router::register(#http_path, #ident)
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
                fn #ident #gen_types () #output
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

            tokens.append_all(quote_spanned! { *span =>
                async fn #ident (#args) #output #block
            });
        }
    }
}

mod inner_handler {
    use std::{
        env,
        ops::DerefMut,
        path::{PathBuf, MAIN_SEPARATOR}
    };

    use itertools::Itertools;
    use syn::{Local, LocalInit, Macro, Stmt};

    use super::*;

    impl InnerHandler {
        pub fn try_new(embed: Option<LitStr>, mut handler_fn: ItemFn) -> Result<Self, syn::Error> {
            let state_attr = state_attr();
            let span = handler_fn.span();

            // Strip #[state] attr from params
            for input in &mut handler_fn.sig.inputs {
                match input {
                    FnArg::Receiver(rec) => return Err(reciever_error(rec)),
                    FnArg::Typed(arg) => {
                        arg.attrs.retain(|attr| attr != &state_attr);
                    }
                }
            }

            for statement in &mut handler_fn.block.stmts {
                let (expr, runtime_lookup_path) = match statement {
                    Stmt::Local(Local {
                        init: Some(LocalInit { expr, .. }),
                        ..
                    }) => {
                        let Expr::Macro(ExprMacro {
                            mac: Macro { path, tokens, .. },
                            ..
                        }) = expr.as_ref()
                        else {
                            continue;
                        };

                        if !path.is_ident(stringify!(load_asset)) {
                            continue;
                        }

                        let tokens = tokens.clone();

                        (expr.deref_mut(), tokens)
                    }
                    Stmt::Expr(expr, _) => {
                        let Expr::Macro(ExprMacro {
                            mac: Macro { path, tokens, .. },
                            ..
                        }) = &expr
                        else {
                            continue;
                        };

                        if !path.is_ident(stringify!(load_asset)) {
                            continue;
                        }

                        let tokens = tokens.clone();

                        (expr, tokens)
                    }
                    _ => continue
                };

                let path_base = embed
                    .as_ref()
                    .map(|path| path.value())
                    .ok_or_else(|| {
                        syn::Error::new(span, "Missing `embed` parameter to server attribute")
                    })?
                    .split(MAIN_SEPARATOR)
                    .map(|comp| {
                        let Some(path) = comp.strip_prefix('$') else {
                            return Ok(comp.to_owned());
                        };

                        env::var(path)
                    })
                    .try_collect::<_, PathBuf, _>()
                    .map_err(|err| {
                        syn::Error::new(span, format!("Failed to resolve env var in path; {err}"))
                    })?;

                let canonical_base = match path_base.canonicalize() {
                    Ok(base) => base,
                    Err(_) => continue
                };

                let (path, dbg_base, rel_base) = if canonical_base.is_file() {
                    let file_name = canonical_base.file_name().and_then(|os| os.to_str());

                    let path = quote_spanned! { span => #file_name };
                    let dbg_base = canonical_base.parent().and_then(|p| p.to_str());
                    let rel_base = canonical_base.to_str();

                    (path, dbg_base, rel_base)
                } else {
                    let base = canonical_base.to_str();
                    (runtime_lookup_path, base, base)
                };

                *expr = parse_quote_spanned! { span =>
                    {
                        #[cfg(debug_assertions)]
                        #[allow(clippy::let_and_return)]
                        let embedded_asset = ::server_fns::__load_asset! {
                            FileAsset {
                                base: #dbg_base,
                                path: #path,
                            }
                        };

                        #[cfg(not(debug_assertions))]
                        #[allow(clippy::let_and_return)]
                        let embedded_asset = ::server_fns::__load_asset! {
                            StaticAsset {
                                base: #rel_base,
                                path: #path,
                            }
                        };

                        embedded_asset
                    }
                };
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
