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
    parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token::Comma,
    Attribute, Expr, FnArg, Generics, ItemFn, LitStr, PatType, Signature,
};

pub struct ServerFnsAttr;

impl AttrMacro for ServerFnsAttr {
    type TokenStream = TokenStream2;
    type Error = syn::Error;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream,
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
                    output,
                },
            block,
        } = syn::parse2(body)?;

        let RouteMeta {
            http_path,
            http_method,
        } = RouteMeta::parse(args, &ident)?;

        // let middlewares = attrs.collect_middleware();

        let (inner_inputs, outer_inputs) =
            inputs.try_into().map(|HandlerArgs { inner, outer }| {
                (Punctuated::<_, Comma>::from_iter(inner), outer)
            })?;

        // Prepare output tokens

        let router_fn = format_ident!("{}_router", ident);
        let module = format_ident!("__{router_fn}");

        let handler_expr: Expr = parse_quote! {
            ::server_fns::axum::routing::#http_method (super::#ident)
        };

        let outer_handler: ItemFn = {
            let (args, generics) = outer_inputs.into_iter().fold(
                (Punctuated::<_, Comma>::new(), Generics::default()),
                |(
                    mut args,
                    Generics {
                        params,
                        where_clause,
                        ..
                    },
                ),
                 OuterArg {
                     arg: next_arg,
                     gen: next_gen,
                 }| {
                    args.push(next_arg);

                    if let Some(Generics {
                        params: next_params,
                        where_clause: next_where,
                        ..
                    }) = next_gen
                    {
                        let gen_params = params
                            .into_iter()
                            .chain(next_params)
                            .collect::<Punctuated<_, Comma>>();
                    }

                    (args, gens)
                },
            );

            parse_quote! {}
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
                output,
            },
            block,
        };

        Ok(quote! {
            mod #module {

                #[::server_fns::linkme::distributed_slice(::server_fns::axum_router::COLLATED_ROUTES)]
                static ROUTER: fn() -> ::server_fns::axum::Router = #router_fn;

                fn #router_fn <State> (state: State) -> ::server_fns::axum::Router
                where State: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + 'static
                {
                    ::server_fns::axum::Router::new()
                        .route(#http_path, #handler_expr)
                        .with_state(state)
                }
            }

            #inner_handler
        })
    }
}
