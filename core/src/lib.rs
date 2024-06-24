mod macro_traits;
mod parse;

#[cfg(feature = "axum")]
pub mod axum_router;

use itertools::Itertools;
pub use macro_traits::AttrMacro;
use parse::{CollectMiddleware, HandlerArgs, OuterArg, RouteMeta};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_quote, parse_quote_spanned,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Paren},
    Attribute, Expr, FnArg, Generics, ItemFn, LitStr, PatType, Signature, Visibility
};

use crate::parse::{reciever_error, IntoGenerics};

pub struct ServerFnsAttr;

impl AttrMacro for ServerFnsAttr {
    type TokenStream = TokenStream2;
    type Error = syn::Error;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream
    ) -> Result<Self::TokenStream, Self::Error> {
        let ItemFn {
            mut attrs,
            vis,
            sig:
                Signature {
                    constness,
                    asyncness,
                    unsafety,
                    abi,
                    fn_token,
                    ident,
                    generics,
                    paren_token,
                    inputs,
                    variadic,
                    output
                },
            block
        } = syn::parse2(body)?;

        let RouteMeta {
            http_path,
            http_method
        } = RouteMeta::parse(args, &ident)?;

        // let middlewares = attrs.collect_middleware();

        let HandlerArgs {
            inner: inner_inputs,
            outer: outer_inputs
        } = inputs.try_into()?;

        // Prepare output tokens

        let router_fn = format_ident!("{}_router", ident);
        let module = format_ident!("__{router_fn}");

        let handler_expr: Expr = parse_quote! {
            ::server_fns::axum::routing::#http_method (#ident)
        };

        let outer_handler: ItemFn = {
            let (args, generics) = outer_inputs.into_iter().fold(
                (Punctuated::<_, Comma>::new(), Option::<IntoGenerics>::None),
                |(mut args, mut gens),
                 OuterArg {
                     arg: next_arg,
                     gen: next_gen
                 }| {
                    args.push(next_arg);

                    if let Some(next_gen) = next_gen {
                        gens = gens
                            .map(|cur_gens| cur_gens + next_gen.clone())
                            .or(Some(next_gen));
                    }

                    (args, gens)
                }
            );

            let generics = generics.map(Generics::from).unwrap_or_default();
            let inner_call_params = args
                .clone()
                .into_iter()
                .map(|arg| match arg {
                    FnArg::Receiver(rec) => Err(reciever_error(rec.span())),
                    FnArg::Typed(param) => Ok(param.pat)
                })
                .fold_ok(Punctuated::<Expr, Comma>::new(), |mut params, next| {
                    params.push(parse_quote!(#next));
                    params
                })?;

            ItemFn {
                attrs: vec![],
                vis: Visibility::Inherited,
                sig: Signature {
                    constness: None,
                    asyncness: Some(parse_quote!(async)),
                    unsafety: None,
                    abi: None,
                    fn_token: parse_quote!(fn),
                    ident: ident.clone(),
                    generics,
                    paren_token: Paren::default(),
                    inputs: args,
                    variadic: None,
                    output: output.clone()
                },
                block: Box::new(parse_quote! {{
                    super::#ident (#inner_call_params).await
                }})
            }
        };

        let inner_handler = ItemFn {
            attrs,
            vis,
            sig: Signature {
                constness,
                asyncness,
                unsafety,
                abi,
                fn_token,
                ident,
                generics,
                paren_token,
                inputs: inner_inputs,
                variadic,
                output
            },
            block
        };

        Ok(quote! {
            mod #module {

                #[::server_fns::linkme::distributed_slice(::server_fns::axum_router::COLLATED_ROUTES)]
                static ROUTER: fn() -> ::server_fns::axum::Router = #router_fn;

                fn #router_fn <State> (state: State) -> ::server_fns::axum::Router
                where
                    State: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + 'static
                {
                    ::server_fns::axum::Router::new()
                        .route(#http_path, #handler_expr)
                        .with_state(state)
                }

                #outer_handler
            }

            #inner_handler
        })
    }
}
