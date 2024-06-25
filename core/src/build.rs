pub(crate) mod args {
    use std::ops::Add;

    use itertools::Itertools;
    use proc_macro2::Span;
    use quote::format_ident;
    use syn::{
        parse_quote, parse_quote_spanned, punctuated::Punctuated, spanned::Spanned, token::Comma,
        Expr, ExprCall, FnArg, GenericParam, Generics, PatType, WhereClause, WherePredicate,
    };

    pub type ArgList = Punctuated<FnArg, Comma>;
    pub type ExprList = Punctuated<Expr, Comma>;
    pub type GenericsList = Punctuated<GenericParam, Comma>;
    pub type GenericConstraintsList = Punctuated<WherePredicate, Comma>;

    #[derive(Default)]
    pub struct Args {
        pub inner: ArgList,
        pub outer: Vec<OuterArg>,
        pub call: ExprList,
        pub router: Vec<RouterArg>,
    }

    pub struct OuterArg {
        pub arg: PatType,
        pub gen: Option<IntoGenerics>,
    }

    pub struct RouterArg {
        pub state: StateArg,
        pub call: Expr,
    }

    pub struct StateArg {
        pub arg: PatType,
        pub gen: IntoGenerics,
    }

    #[derive(Clone)]
    pub struct IntoGenerics {
        pub params: GenericsList,
        pub predicates: GenericConstraintsList,
    }

    struct ArgGroup {
        inner: PatType,
        outer: OuterArg,
        call: Expr,
        router: Option<RouterArg>,
    }

    impl TryFrom<Punctuated<FnArg, Comma>> for Args {
        type Error = syn::Error;

        /// Build [HandlerArgs] from the main annotated function's argument list.
        fn try_from(args: Punctuated<FnArg, Comma>) -> Result<Self, Self::Error> {
            args.into_iter()
                .enumerate()
                .map(ArgGroup::try_from)
                .fold_ok(Args::default(), Args::add)
        }
    }

    impl Add<ArgGroup> for Args {
        type Output = Self;

        /// Append [ArgGroup]s onto the end of the respective argument lists.
        fn add(mut self, rhs: ArgGroup) -> Self::Output {
            self.inner.push(rhs.inner.into());
            self.outer.push(rhs.outer);
            self.call.push(rhs.call);

            if let Some(router) = rhs.router {
                self.router.push(router);
            }

            self
        }
    }

    impl Add for IntoGenerics {
        type Output = Self;

        fn add(mut self, rhs: Self) -> Self::Output {
            self.params.extend(rhs.params);
            self.predicates.extend(rhs.predicates);
            self
        }
    }

    impl From<IntoGenerics> for Generics {
        fn from(IntoGenerics { params, predicates }: IntoGenerics) -> Self {
            Self {
                lt_token: Some(parse_quote!(<)),
                params,
                gt_token: Some(parse_quote!(>)),
                where_clause: Some(WhereClause {
                    where_token: parse_quote!(where),
                    predicates,
                }),
            }
        }
    }

    impl TryFrom<(usize, FnArg)> for ArgGroup {
        type Error = syn::Error;

        fn try_from((i, fn_arg): (usize, FnArg)) -> Result<Self, Self::Error> {
            let state_attr = parse_quote!(#[state]);
            let arg_num = format_ident!("arg{i}");

            match fn_arg {
                FnArg::Receiver(rec) => Err(reciever_error(rec.span())),
                FnArg::Typed(mut param) if param.attrs.contains(&state_attr) => {
                    let generic_state = format_ident!("State{i}");
                    let state_num = format_ident!("state{i}");

                    param.attrs.retain(|attr| attr != &state_attr);
                    let ty = &param.ty;

                    let state_generics = IntoGenerics {
                        params: parse_quote_spanned! { param.span() => #generic_state },
                        predicates: parse_quote_spanned! { param.span() =>
                            #ty: ::axum::extract::FromRef<#generic_state>,
                            #generic_state: ::std::marker::Send + ::std::marker::Sync
                        },
                    };

                    Ok(Self {
                        outer: OuterArg {
                            arg: parse_quote_spanned! { param.span() =>
                                ::axum::extract::State(#arg_num): ::axum::extract::State<#ty>
                            },
                            gen: Some(state_generics.clone()),
                        },
                        inner: parse_quote_spanned! { param.span() => #param },
                        call: parse_quote_spanned! { param.span() => #arg_num },
                        router: Some(RouterArg {
                            state: StateArg {
                                arg: parse_quote_spanned! { param.span() => #state_num: #generic_state },
                                gen: state_generics,
                            },
                            call: parse_quote_spanned! { param.span() => #state_num },
                        }),
                    })
                }
                FnArg::Typed(param) => Ok(Self {
                    inner: param.clone(),
                    call: parse_quote_spanned! { param.span() => #arg_num },
                    outer: OuterArg {
                        arg: PatType {
                            pat: parse_quote_spanned! { param.span() => #arg_num },
                            attrs: param.attrs,
                            colon_token: param.colon_token,
                            ty: param.ty,
                        },
                        gen: None,
                    },
                    router: None,
                }),
            }
        }
    }

    pub fn reciever_error(span: Span) -> syn::Error {
        syn::Error::new(
            span,
            "Reciever type 'self' is not supported in server functions.",
        )
    }
}
