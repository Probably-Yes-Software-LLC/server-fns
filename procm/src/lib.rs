use proc_macro::TokenStream;
use server_fns_core::{
    http_methods, AttrMacro, DeriveMacro, FnMacro, HttpMethod, LoadAssetInternalMacro,
    MiddlewareAttrMacro, ServerFnAttrMacro, ServerFnMethodAttr, ServerStateDeriveMacro,
    UseServerStateFnMacro
};

#[proc_macro_attribute]
pub fn server(args: TokenStream, body: TokenStream) -> TokenStream {
    ServerFnAttrMacro.transform(args, body)
}

#[proc_macro_attribute]
pub fn middleware(args: TokenStream, body: TokenStream) -> TokenStream {
    MiddlewareAttrMacro.transform(args, body)
}

#[proc_macro_derive(ServerState)]
pub fn server_state(item: TokenStream) -> TokenStream {
    ServerStateDeriveMacro.transform(item)
}

#[proc_macro]
pub fn use_server_state(item: TokenStream) -> TokenStream {
    UseServerStateFnMacro.transform(item)
}

#[proc_macro]
pub fn __load_asset(item: TokenStream) -> TokenStream {
    LoadAssetInternalMacro.transform(item)
}

// Build a proc-macro attribute function for the given http method.
macro_rules! make_method_macro {
    ($method:ident, $method_enum:expr) => {
        #[proc_macro_attribute]
        pub fn $method(args: TokenStream, body: TokenStream) -> TokenStream {
            ServerFnMethodAttr($method_enum).transform(args, body)
        }
    };
}

// Build a proc-macro for each http method supported.
http_methods!(foreach!(make_method_macro));
