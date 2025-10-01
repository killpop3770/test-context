mod args;

use args::TestContextArgs;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Block, Ident};

/// Macro to use on tests to add the setup/teardown functionality of your context.
///
/// Ordering of this attribute is important, and typically `test_context` should come
/// before other test attributes. For example, the following is valid:
///
/// ```ignore
/// #[test_context(MyContext)]
/// #[test]
/// fn my_test() {
/// }
/// ```
///
/// The following is NOT valid...
///
/// ```ignore
/// #[test]
/// #[test_context(MyContext)]
/// fn my_test() {
/// }
/// ```
#[proc_macro_attribute]
pub fn test_context(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(attr as TestContextArgs);

    let mut input = syn::parse_macro_input!(item as syn::ItemFn);
    let body = &input.block;
    let is_async = input.sig.asyncness.is_some();

    let wrapper_body = if is_async {
        async_wrapper_body(args, body)
    } else {
        sync_wrapper_body(args, body)
    };

    input.block = Box::new(syn::parse2(wrapper_body).unwrap());

    quote! { #input }.into()
}

fn async_wrapper_body(args: TestContextArgs, body: &Box<Block>) -> proc_macro2::TokenStream {
    let context_type = args.context_type;
    let context_name = args.context_name;
    let result_name = format_ident!("wrapped_result");

    let body = if args.skip_teardown {
        quote! {
            let #context_name = <#context_type as test_context::AsyncTestContext>::setup().await;
            let #result_name = std::panic::AssertUnwindSafe( async {
                #body
            }).catch_unwind().await;
        }
    } else {
        quote! {
            let mut #context_name = <#context_type as test_context::AsyncTestContext>::setup().await;
            let #result_name = std::panic::AssertUnwindSafe( async {
                #body
            }).catch_unwind().await;
            <#context_type as test_context::AsyncTestContext>::teardown(#context_name).await;
        }
    };

    let handle_wrapped_result = handle_result(result_name);

    quote! {
        {
            use test_context::futures::FutureExt;
            #body
            #handle_wrapped_result
        }
    }
}

fn sync_wrapper_body(args: TestContextArgs, body: &Box<Block>) -> proc_macro2::TokenStream {
    let context_type = args.context_type;
    let context_name = args.context_name;
    let result_name = format_ident!("wrapped_result");

    let body = if args.skip_teardown {
        quote! {
            let #context_name = <#context_type as test_context::TestContext>::setup();
            let #result_name = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                #body
            }));
        }
    } else {
        quote! {
            let mut #context_name = <#context_type as test_context::TestContext>::setup();
            let #result_name = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                #body
            }));
            <#context_type as test_context::TestContext>::teardown(#context_name);
        }
    };

    let handle_wrapped_result = handle_result(result_name);

    quote! {
        {
            #body
            #handle_wrapped_result
        }
    }
}

fn handle_result(result_name: Ident) -> proc_macro2::TokenStream {
    quote! {
        match #result_name {
            Ok(value) => value,
            Err(err) => {
                std::panic::resume_unwind(err);
            }
        }
    }
}
