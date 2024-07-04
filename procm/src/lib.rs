use proc_macro::TokenStream;
use server_fns_core::{
    http_methods, AttrMacro, DeriveMacro, HttpMethod, MiddlewareAttr, ServerFnAttr,
    ServerFnMethodAttr, ServerStateDerive
};

#[proc_macro_attribute]
pub fn server(args: TokenStream, body: TokenStream) -> TokenStream {
    ServerFnAttr.transform(args, body)
}

#[proc_macro_attribute]
pub fn middleware(args: TokenStream, body: TokenStream) -> TokenStream {
    MiddlewareAttr.transform(args, body)
}

#[proc_macro_derive(ServerState)]
pub fn server_state(item: TokenStream) -> TokenStream {
    ServerStateDerive.transform(item)
}

macro_rules! make_method_macro {
    ($method:ident, $method_enum:expr) => {
        #[proc_macro_attribute]
        pub fn $method(args: TokenStream, body: TokenStream) -> TokenStream {
            ServerFnMethodAttr($method_enum).transform(args, body)
        }
    };
}

http_methods!(foreach!(make_method_macro));
