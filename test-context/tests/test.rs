use std::marker::PhantomData;

use rstest::rstest;
use test_context::{test_context, AsyncTestContext, TestContext};

struct Context {
    n: u32,
}

impl TestContext for Context {
    fn setup() -> Self {
        Self { n: 1 }
    }

    fn teardown(self) {
        if self.n != 1 {
            panic!("Number changed");
        }
    }
}

#[test_context(Context, "test_ctx")]
#[test]
fn test_sync_setup() {
    assert_eq!(test_ctx.n, 1);
}

#[test_context(Context, "test_ctx")]
#[test]
#[should_panic(expected = "Number changed")]
fn test_sync_teardown() {
    test_ctx.n = 2;
}

#[test_context(Context, "ctx")]
#[test]
#[should_panic(expected = "Number changed")]
fn test_panicking_teardown() {
    ctx.n = 2;
    panic!("First panic");
}

#[test_context(Context, "ctx")]
fn return_value_func() -> u32 {
    ctx.n
}

#[test]
fn includes_return_value() {
    assert_eq!(return_value_func(), 1);
}

struct ContextGeneric<T> {
    n: u32,
    _marker: PhantomData<T>,
}

struct ContextGenericType1;
impl TestContext for ContextGeneric<ContextGenericType1> {
    fn setup() -> Self {
        Self {
            n: 1,
            _marker: PhantomData,
        }
    }
}

#[test_context(ContextGeneric<ContextGenericType1>, "ctx")]
#[test]
fn test_generic_type() {
    assert_eq!(ctx.n, 1);
}

struct ContextGenericType2;
impl TestContext for ContextGeneric<ContextGenericType2> {
    fn setup() -> Self {
        Self {
            n: 2,
            _marker: PhantomData,
        }
    }
}

#[test_context(ContextGeneric<ContextGenericType2>, "ctx")]
#[test]
fn test_generic_type_other() {
    assert_eq!(ctx.n, 2);
}

struct AsyncContext {
    n: u32,
}

impl AsyncTestContext for AsyncContext {
    async fn setup() -> Self {
        Self { n: 1 }
    }

    async fn teardown(self) {
        if self.n != 1 {
            panic!("Number changed");
        }
    }
}

#[test_context(AsyncContext, "ctx")]
#[tokio::test]
async fn test_async_setup() {
    assert_eq!(ctx.n, 1);
}

#[test_context(AsyncContext, "ctx")]
#[tokio::test]
#[should_panic(expected = "Number changed")]
async fn test_async_teardown() {
    ctx.n = 2;
}

#[test_context(AsyncContext, "ctx")]
#[tokio::test]
#[should_panic(expected = "Number changed")]
async fn test_async_panicking_teardown() {
    ctx.n = 2;
    panic!("First panic");
}

#[test_context(AsyncContext, "ctx")]
async fn async_return_value_func() -> u32 {
    ctx.n
}

#[tokio::test]
async fn async_includes_return_value() {
    assert_eq!(async_return_value_func().await, 1);
}

#[test_context(AsyncContext, "ctx")]
#[test]
fn async_auto_impls_sync() {
    assert_eq!(ctx.n, 1);
}

#[test_context(Context, "test_data")]
#[test]
fn use_different_name() {
    assert_eq!(test_data.n, 1);
}

#[test_context(AsyncContext, "test_data")]
#[tokio::test]
async fn use_different_name_async() {
    assert_eq!(test_data.n, 1);
}

struct TeardownPanicContext {}

impl AsyncTestContext for TeardownPanicContext {
    async fn setup() -> Self {
        Self {}
    }

    async fn teardown(self) {
        panic!("boom!");
    }
}

#[test_context(TeardownPanicContext, "_ctx", skip_teardown)]
#[tokio::test]
async fn test_async_skip_teardown() {}

#[test_context(TeardownPanicContext, "_ctx", skip_teardown)]
#[test]
fn test_sync_skip_teardown() {}

struct GenericContext<T> {
    contents: T,
}

impl TestContext for GenericContext<u32> {
    fn setup() -> Self {
        Self { contents: 1 }
    }
}

impl TestContext for GenericContext<String> {
    fn setup() -> Self {
        Self {
            contents: "hello world".to_string(),
        }
    }
}

impl AsyncTestContext for GenericContext<u64> {
    async fn setup() -> Self {
        Self { contents: 1 }
    }
}

#[test_context(GenericContext<u32>, "ctx")]
#[test]
fn test_generic_with_u32() {
    assert_eq!(ctx.contents, 1);
}

#[test_context(GenericContext<String>, "ctx")]
#[test]
fn test_generic_with_string() {
    assert_eq!(ctx.contents, "hello world");
}

#[test_context(GenericContext<u64>, "ctx")]
#[tokio::test]
async fn test_async_generic() {
    assert_eq!(ctx.contents, 1);
}

struct MyContext {
    n: u32,
}

impl TestContext for MyContext {
    fn setup() -> Self {
        println!("Create this shit!");
        MyContext { n: 42 }
    }
    fn teardown(self) -> () {
        println!("Drop this shit! {}", self.n);
        drop(self);
    }
}

#[test_context(MyContext, "test_ctx")]
#[rstest]
#[case(2, 2, 4)]
#[test]
fn test_addition(#[case] a: i32, #[case] b: i32, #[case] expected: i32) {
    println!(" {} + {} = {}", a, b, expected);
    assert_eq!(a + b, expected);
    assert_eq!(test_ctx.n, 42);
}

struct MyAsyncContext {
    n: u32,
}
impl AsyncTestContext for MyAsyncContext {
    async fn setup() -> Self {
        println!("Create this shit!");
        MyAsyncContext { n: 42 }
    }
    async fn teardown(self) -> () {
        println!("Drop this shit! {}", self.n);
        drop(self);
    }
}

#[test_context(MyAsyncContext, "test_ctx")]
#[rstest]
#[case("Hello world!")]
#[tokio::test]
async fn test_rstest_with(#[case] _value: String) {
    println!("{}", test_ctx.n);
    println!("{}", _value);
}
