mod build;
mod macro_traits;
mod out;
mod parse;

#[cfg(feature = "axum")]
pub mod axum_router;

pub use macro_traits::AttrMacro;
use parse::{CollectMiddleware, RouteMeta};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::ItemFn;

use crate::out::ServerFn;

pub struct ServerFnsAttr;

impl AttrMacro for ServerFnsAttr {
    type TokenStream = TokenStream2;
    type Error = syn::Error;

    fn transform2(
        args: Self::TokenStream,
        body: Self::TokenStream
    ) -> Result<Self::TokenStream, Self::Error> {
        let annotated_fn: ItemFn = syn::parse2(body)?;
        let meta = RouteMeta::parse(args, &annotated_fn.sig.ident)?;
        let server_fn = ServerFn::try_new(meta, annotated_fn)?;

        Ok(quote!(#server_fn))

        // let middlewares = attrs.collect_middleware();

        // let Args {
        //     inner: inner_inputs,
        //     outer: outer_inputs,
        //     call: call_inner,
        //     router: router_inputs
        // } = inputs.try_into()?;

        // Prepare output tokens

        // let router_fn = format_ident!("{ident}_router");
        // let module = format_ident!("__{router_fn}");
        // let outer_handler_fn = format_ident!("{http_method}_{ident}");

        // let handler_expr: Expr = parse_quote! {
        //     ::server_fns::axum::routing::#http_method(#outer_handler_fn)
        // };

        // let outer_handler: ItemFn = {
        //     let (args, generics) = outer_inputs.into_iter().fold(
        //         (Punctuated::<_, Comma>::new(), Option::<IntoGenerics>::None),
        //         |(mut args, mut gens),
        //          OuterArg {
        //              arg: next_arg,
        //              gen: next_gen
        //          }| {
        //             args.push(next_arg);

        //             if let Some(next_gen) = next_gen {
        //                 gens = gens
        //                     .map(|cur_gens| cur_gens + next_gen.clone())
        //                     .or(Some(next_gen));
        //             }

        //             (args, gens)
        //         }
        //     );

        //     let generics = generics.map(Generics::from).unwrap_or_default();
        //     let (_, type_gens, where_gens, ..) = generics.split_for_impl();

        //     let inputs = args
        //         .into_iter()
        //         .map_into::<FnArg>()
        //         .collect::<Punctuated<_, Comma>>();

        //     parse_quote! {
        //         async fn #outer_handler_fn #type_gens (#inputs) #output #where_gens {
        //             #ident(#call_inner).await
        //         }
        //     }
        // };

        // let inner_handler = ItemFn {
        //     attrs,
        //     vis,
        //     sig: Signature {
        //         constness,
        //         asyncness,
        //         unsafety,
        //         abi,
        //         fn_token,
        //         ident,
        //         generics,
        //         paren_token,
        //         inputs: inner_inputs,
        //         variadic,
        //         output
        //     },
        //     block
        // };

        // Ok(quote! {
        //     mod #module {
        //         use super::*;

        //         #[::server_fns::linkme::distributed_slice(::server_fns::axum_router::COLLATED_ROUTES)]
        //         static ROUTER: fn() -> ::server_fns::axum::Router = #router_fn;

        //         fn #router_fn <State> (state: State) -> ::server_fns::axum::Router
        //         where
        //             State: ::std::clone::Clone + ::std::marker::Send + ::std::marker::Sync + 'static
        //         {
        //             ::server_fns::axum::Router::new()
        //                 .route(#http_path, #handler_expr)
        //                 .with_state(state)
        //         }

        //         #outer_handler
        //     }

        //     #inner_handler
        // })
    }
}
