use proc_macro::TokenStream;
use server_fns_core::{
    http_methods, AttrMacro, DeriveMacro, MiddlewareAttr, ServerFnsAttr, ServerStateDerive
};

#[proc_macro_attribute]
pub fn server(args: TokenStream, body: TokenStream) -> TokenStream {
    ServerFnsAttr::transform(args, body)
}

#[proc_macro_attribute]
pub fn middleware(args: TokenStream, body: TokenStream) -> TokenStream {
    MiddlewareAttr::transform(args, body)
}

#[proc_macro_derive(ServerState)]
pub fn server_state(item: TokenStream) -> TokenStream {
    ServerStateDerive::transform(item)
}

macro_rules! make_method_macro {
    ($method:ident) => {
        #[proc_macro_attribute]
        pub fn $method(_args: TokenStream, body: TokenStream) -> TokenStream {
            body
        }
    };
}

http_methods!(foreach!(make_method_macro));
