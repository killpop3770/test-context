mod args;

use std::hash::{DefaultHasher, Hash, Hasher};

use args::TestContextArgs;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, Block, FnArg, Ident, ItemFn, Token,
    Type,
};

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
    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let arguments = &input.sig.inputs;
    let body = &input.block;
    let attrs = &input.attrs;
    let is_async = input.sig.asyncness.is_some();

    let wrapper_body = if is_async {
        async_wrapper_body(args, body)
    } else {
        sync_wrapper_body(args, body)
    };

    let async_tag = if is_async {
        quote! { async }
    } else {
        quote! {}
    };

    input.block = Box::new(syn::parse2(wrapper_body).unwrap());

    quote! { #input }.into()

    // TODO: добавить аргумент контекста + проверка что все работает иначе паника
    // TODO: добавить возможность кастомного имени контекста
    // quote! {
    //     #(#attrs)*
    //     #async_tag fn #name(#arguments) #ret {
    //         #wrapper_body
    //     }
    // }
    // .into()
}

fn async_wrapper_body(args: TestContextArgs, body: &Box<Block>) -> proc_macro2::TokenStream {
    let context_type = args.context_type;
    let result_name = format_ident!("wrapped_result");

    let body = if args.skip_teardown {
        quote! {
            let ctx = <#context_type as test_context::AsyncTestContext>::setup().await;
            let #result_name = std::panic::AssertUnwindSafe( async { #body }).catch_unwind().await;
        }
    } else {
        quote! {
            let mut ctx = <#context_type as test_context::AsyncTestContext>::setup().await;
            let ctx_reference = &mut ctx;
            let #result_name = std::panic::AssertUnwindSafe( async { #body }).catch_unwind().await;
            <#context_type as test_context::AsyncTestContext>::teardown(ctx).await;
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
    let result_name = format_ident!("wrapped_result");

    let body = if args.skip_teardown {
        quote! {
            let ctx = <#context_type as test_context::TestContext>::setup();
            let #result_name = std::panic::catch_unwind(move || {
                #body
            });
        }
    } else {
        quote! {
            let mut ctx = <#context_type as test_context::TestContext>::setup();
            let mut pointer = std::panic::AssertUnwindSafe(&mut ctx);
            let #result_name = std::panic::catch_unwind(move || {
                #body
            });
            <#context_type as test_context::TestContext>::teardown(ctx);
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

#[proc_macro_attribute]
pub fn test_context_rstest(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as TestContextArgs);
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let context_type = &args.context_type;
    // let skip_teardown = args.skip_teardown;
    let name = &input.sig.ident;
    let body = &input.block;
    let attrs = &input.attrs;
    // let is_async = input.sig.asyncness.is_some();
    let output = &input.sig.output;
    let arguments = input.sig.inputs;

    let wrapper_body = quote! {
        use ::test_context::futures::FutureExt;

        let mut ctx = <#context_type as test_context::AsyncTestContext>::setup().await;
        let result = std::panic::AssertUnwindSafe(async { #body }).catch_unwind().await;
        <#context_type as test_context::AsyncTestContext>::teardown(ctx).await;
        match result {
            Ok(r) => r,
            Err(e) => std::panic::resume_unwind(e),
        }
    };

    quote! {
        #(#attrs)*
        async fn #name(#arguments) #output {
            #wrapper_body
        }
    }
    .into()
}

// #[proc_macro_attribute]
// pub fn test_context_rstest_redo(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let args = parse_macro_input!(attr as TestContextArgs);
//     let input = syn::parse_macro_input!(item as syn::ItemFn);

//     // let context_type = &args.context_type;
//     // let skip_teardown = args.skip_teardown;
//     let name = &input.sig.ident;
//     let body = &input.block;
//     let attrs = &input.attrs;
//     let is_async = input.sig.asyncness.is_some();
//     let output = &input.sig.output;
//     let arguments: Punctuated<FnArg, syn::token::Comma> = input.sig.inputs;

//     let wrapped_name = format_ident!("__test_context_wrapped_{}", name);

//     let wrapper_body = if is_async {
//         async_wrapper_body_redo(args, arguments.clone(), &wrapped_name)
//     } else {
//         // sync_wrapper_body(args, &wrapped_name)
//         quote! {}
//     };

//     let async_tag = if is_async {
//         quote! { async }
//     } else {
//         quote! {}
//     };

//     quote! {
//         #(#attrs)*
//         #async_tag fn #name() #output { // Генерируется только тип аргумента для дальнейшей работы, но не передается в функцию теста
//             #wrapper_body
//         }

//         #async_tag fn #wrapped_name(#arguments) #output {
//             #body
//         }
//     }
//     .into()
// }

// fn async_wrapper_body_redo(
//     args: TestContextArgs,
//     arguments: Punctuated<FnArg, syn::token::Comma>,
//     wrapped_name: &Ident,
// ) -> proc_macro2::TokenStream {
//     let context_type = args.context_type;
//     let result_name = format_ident!("wrapped_result");

//     let arg_names: Vec<_> = arguments
//         .iter()
//         .map(|arg| match arg {
//             FnArg::Receiver(_) => quote! { self },
//             FnArg::Typed(pat_type) => {
//                 let pat = &pat_type.pat;
//                 quote! { #pat }
//             }
//         })
//         .collect();

//     let body = quote! {
//         let mut ctx = <#context_type as test_context::AsyncTestContext>::setup().await;
//         let ctx_reference = &mut ctx;
//         let #result_name = std::panic::AssertUnwindSafe(
//             #wrapped_name(#(#arg_names)*, ctx_reference)
//         ).catch_unwind().await;
//         <#context_type as test_context::AsyncTestContext>::teardown(ctx).await;
//     };

//     let handle_wrapped_result = handle_result(result_name);

//     quote! {
//         {
//             use test_context::futures::FutureExt;
//             #body
//             #handle_wrapped_result
//         }
//     }
// }

// #[proc_macro_attribute]
// pub fn my_wrapper(attr: TokenStream, item: TokenStream) -> TokenStream {
//     let args = syn::parse_macro_input!(attr as TestContextArgs);
//     let input = parse_macro_input!(item as ItemFn);

//     let name = &input.sig.ident;
//     let body = &input.block;
//     let attrs = &input.attrs;
//     let output = &input.sig.output;
//     let arguments = &input.sig.inputs;

//     let is_async = input.sig.asyncness.is_some();
//     let inner_result = if is_async {
//         async_wrapper_body_redo(args, body)
//     } else {
//         quote! {}
//         // sync_wrapper_body_redo(args, body)
//     };

//     let async_tag = if is_async {
//         quote! { async }
//     } else {
//         quote! {}
//     };

//     quote! {
//         #(#attrs)*
//         #async_tag fn #name(#arguments) #output #inner_result
//     }
//     .into()
// }

// fn async_wrapper_body_redo(args: TestContextArgs, body: &Box<Block>) -> proc_macro2::TokenStream {
//     let context_type = args.context_type;
//     let result_name = format_ident!("wrapped_result");

//     let result_body = if args.skip_teardown {
//         quote! {
//             let ctx = <#context_type as test_context::AsyncTestContext>::setup().await;
//             let #result_name = std::panic::AssertUnwindSafe(
//                 #body(ctx)
//             ).catch_unwind().await;
//         }
//     } else {
//         quote! {
//             let mut ctx = <#context_type as test_context::AsyncTestContext>::setup().await;
//             let ctx_reference = &mut ctx;
//             let #result_name = std::panic::AssertUnwindSafe(
//                 #body(ctx_reference)
//             ).catch_unwind().await;
//             <#context_type as test_context::AsyncTestContext>::teardown(ctx).await;
//         }
//     };

//     let handle_wrapped_result = handle_result(result_name);

//     quote! {
//         {
//             use test_context::futures::FutureExt;
//             #result_body
//             #handle_wrapped_result
//         }
//     }
// }
