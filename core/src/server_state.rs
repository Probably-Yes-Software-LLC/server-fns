use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote_spanned, ToTokens, TokenStreamExt};
use syn::{spanned::Spanned, Ident, ItemStruct};

#[cfg(feature = "server")]
use crate::server_router::ServerRouter;
use crate::{current_package, make_router};

/// Trait corresponding to types that can be used as the state of an [axum::Router].
///
/// Defines a struct as the 'State' parameter of an [axum::Router<State>],
/// with routes collected at startup from the [ServerState::Router]'s
/// [inventory] of routes, identified with a [ServerRouter::State] of [Self].
#[cfg(feature = "server")]
pub trait ServerState: Clone + Send + Sync + Sized + 'static {
    /// The type through which routes are collected.
    type Router: ServerRouter<State = Self>;

    fn load_routes(self) -> axum::Router {
        Self::Router::load_routes().with_state(self)
    }
}

pub(crate) struct ServerStateImpl {
    pub span: Span,
    pub ident: Ident,
    pub current_package: String
}

impl ServerStateImpl {
    pub fn try_new(item: ItemStruct) -> Result<Self, syn::Error> {
        let current_package = current_package(item.span())?;

        Ok(Self {
            span: item.span(),
            ident: item.ident,
            current_package
        })
    }
}

impl ToTokens for ServerStateImpl {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            span,
            ident,
            current_package
        } = self;

        let ident_str = ident.to_string();
        let module = format_ident!(
            "__{}_{}",
            current_package.to_case(Case::Snake),
            ident_str.to_case(Case::Snake)
        );

        let state_router = make_router(&ident_str);
        let package_router = make_router(current_package);
        let router_fn_type = format_ident!("{state_router}Fn");

        tokens.append_all(quote_spanned! { *span =>
            #[cfg(feature = "server")]
            #[allow(unused)]
            pub(crate) type #package_router = #module::#state_router;

            #[cfg(feature = "server")]
            mod #module {
                #[automatically_derived]
                impl ::server_fns::server_state::ServerState for super::#ident {
                    type Router = #state_router;
                }

                type #router_fn_type = ::server_fns::server_router::RouterFn<super::#ident>;

                pub struct #state_router {
                    pub path: &'static str,
                    pub router_fn: #router_fn_type
                }

                impl #state_router {
                    pub const fn register(
                        path: &'static str,
                        router_fn: #router_fn_type
                    ) -> Self {
                        Self { path, router_fn }
                    }
                }

                #[automatically_derived]
                impl ::server_fns::server_router::ServerRouter for #state_router {
                    type State = super::#ident;

                    fn router(&self) -> ::server_fns::axum::Router<Self::State> {
                        (self.router_fn)()
                    }
                }

                ::server_fns::inventory::collect!(#state_router);
            }
        });
    }
}
