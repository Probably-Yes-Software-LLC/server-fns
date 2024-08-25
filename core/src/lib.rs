pub mod embed_asset;
mod macro_traits;
pub mod middleware;
mod parse;
mod server_fn;
pub mod server_router;
pub mod server_state;
mod transform;

pub use macro_traits::*;
pub use transform::*;

#[macro_export]
macro_rules! http_methods {
    // Inject the list of supported methods back into the macro call.
    (http_methods!() $call:ident!() $($with:tt)?) => {
        $crate::http_methods! {
            $call!
            [any delete get head options patch post put trace]
            $($with)?
        }
    };
    (len! $(())?) => {
        $crate::http_methods! {
            http_methods!()
            len!()
        }
    };
    (len! $(())? []) => {
        0usize
    };
    (len! $(())? [$next:ident $($methods:ident)*]) => {
        1usize + $crate::http_methods! {
            len!()
            [$($methods)*]
        }
    };
    (as_enum! $(())?) => {
        $crate::http_methods! {
            http_methods!()
            as_enum!()
        }
    };
    (as_enum! [$($methods:ident)+]) => {
        ::paste::paste! {
            pub enum HttpMethod {
                $(
                    [<$methods:camel>]
                ),+
            }

            impl AsRef<str> for HttpMethod {
                fn as_ref(&self) -> &str {
                    match self {
                        $(
                            Self::[<$methods:camel>] => ::std::stringify!($methods)
                        ),+
                    }
                }
            }
        }
    };
    (as_slice! $(())?) => {
        $crate::http_methods! {
            http_methods!()
            as_slice!()
        }
    };
    (as_slice! [$($methods:ident)+]) => {
        &[
            $(
                ::std::stringify!($methods)
            ),+
        ]
    };
    (contains! ($method:expr)) => {
        $crate::http_methods! {
            http_methods!()
            contains!()
            [$method]
        }
    };
    (contains! [$($methods:ident)+] [$method:expr]) => {
        {
            const HTTP_METHODS_COUNT: usize = $crate::http_methods!(len!());
            const SUPPORTED_HTTP_METHODS: [&'static str; HTTP_METHODS_COUNT] = [
                $(
                    ::std::stringify!($methods)
                ),+
            ];

            SUPPORTED_HTTP_METHODS.contains($method)
        }
    };
    (foreach! ($macro:ident $(!)? $(())?)) => {
        $crate::http_methods! {
            http_methods!()
            foreach!()
            [$macro]
        }
    };
    (foreach! [$($methods:ident)+] [$macro:ident]) => {
        ::paste::paste! {
            $(
                $macro! { $methods, HttpMethod::[<$methods:camel>] }
            )+
        }
    };
}

#[macro_export]
macro_rules! layer_middleware {
    (after_routing($mid:expr) for $router:ident) => {
        let $router = $router.route_layer($mid);
    };
    (before_routing($mid:expr) for $router:ident) => {
        let $router = $router.layer($mid);
    };
}

http_methods!(as_enum!());
