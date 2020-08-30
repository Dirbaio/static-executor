#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

use darling::FromMeta;
use proc_macro::{Diagnostic, Level, Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned;

#[derive(Debug, FromMeta)]
struct MacroArgs {
    #[darling(default)]
    pool_size: Option<usize>,
}

#[proc_macro_attribute]
pub fn task(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as syn::AttributeArgs);
    let mut task_fn = syn::parse_macro_input!(item as syn::ItemFn);

    let args = match MacroArgs::from_list(&args) {
        Ok(v) => v,
        Err(e) => {
            return TokenStream::from(e.write_errors());
        }
    };

    let pool_size: usize = args.pool_size.unwrap_or(1);

    let mut fail = false;
    if task_fn.sig.asyncness.is_none() {
        task_fn.sig.span().unwrap().error("task functions must be async").emit();
        fail = true;
    }
    if task_fn.sig.generics.params.len() != 0 {
        task_fn
            .sig
            .span()
            .unwrap()
            .error("task functions must not be generic")
            .emit();
        fail = true;
    }
    if pool_size < 1 {
        Span::call_site().error("pool_size must be 1 or greater").emit();
        fail = true
    }

    let mut arg_names: syn::punctuated::Punctuated<syn::Ident, syn::Token![,]> = syn::punctuated::Punctuated::new();
    let args = &task_fn.sig.inputs;

    for arg in args.iter() {
        match arg {
            syn::FnArg::Receiver(_) => {
                arg.span()
                    .unwrap()
                    .error("task functions must not have receiver arguments")
                    .emit();
                fail = true;
            }
            syn::FnArg::Typed(t) => match t.pat.as_ref() {
                syn::Pat::Ident(i) => arg_names.push(i.ident.clone()),
                _ => {
                    arg.span()
                        .unwrap()
                        .error("pattern matching in task arguments is not yet supporteds")
                        .emit();
                    fail = true;
                }
            },
        }
    }

    if fail {
        return TokenStream::new();
    }

    let name = task_fn.sig.ident.clone();

    let future_type_name = format_ident!("__static_executor_futtype_{}", name);
    let pool_type_name = format_ident!("__static_executor_pooltype_{}", name);
    let task_fn_name = format_ident!("__static_executor_task_{}", name);
    let create_fn_name = format_ident!("__static_executor_create_{}", name);

    let visibility = &task_fn.vis;

    task_fn.sig.ident = task_fn_name.clone();

    let result = quote! {
        #task_fn
        #[allow(non_camel_case_types)]
        type #future_type_name = impl ::core::future::Future + 'static;

        fn #create_fn_name(#args) -> #future_type_name {
            #task_fn_name(#arg_names)
        }

        #[allow(non_camel_case_types)]
        #visibility struct #pool_type_name {
            inner: [::static_executor::Task<#future_type_name>; #pool_size],
        }

        impl #pool_type_name {
            pub unsafe fn spawn(&'static self, #args) -> ::core::result::Result<(), ::static_executor::SpawnError> {
                unsafe { ::static_executor::Task::spawn(&self.inner, || #create_fn_name(#arg_names)) }
            }
        }

        #[allow(non_upper_case_globals)]
        #visibility static #name: #pool_type_name = #pool_type_name{ inner: [::static_executor::Task::new(); #pool_size] };
    };
    result.into()
}
