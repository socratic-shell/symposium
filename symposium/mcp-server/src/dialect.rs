use std::collections::BTreeMap;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;

use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

mod parser;
pub use parser::{Ast, parse};

#[derive(Clone)]
pub struct DialectInterpreter<U: Send> {
    functions: BTreeMap<
        String,
        (
            fn(
                &mut DialectInterpreter<U>,
                Value,
            ) -> Pin<Box<dyn Future<Output = anyhow::Result<Value>> + '_>>,
            &'static [&'static str], // parameter_order
        ),
    >,
    userdata: U,
}

impl<U: Send> DialectInterpreter<U> {
    pub fn new(userdata: U) -> Self {
        Self {
            functions: BTreeMap::new(),
            userdata,
        }
    }

    pub fn user_data(&self) -> &U {
        &self.userdata
    }

    pub fn add_function<F>(&mut self)
    where
        F: DialectFunction<U>,
    {
        let type_name = std::any::type_name::<F>();
        // Extract just the struct name from the full path (e.g., "module::Uppercase" -> "uppercase")
        let struct_name = type_name.split("::").last().unwrap_or(type_name);
        let type_name_lower = struct_name.to_ascii_lowercase();
        self.add_function_with_name::<F>(type_name_lower);
    }

    pub fn add_function_with_name<F>(&mut self, name: impl ToString)
    where
        F: DialectFunction<U>,
    {
        self.functions.insert(
            name.to_string(),
            (
                |interpreter, value| Box::pin(async move { interpreter.execute::<F>(value).await }),
                F::PARAMETER_ORDER,
            ),
        );
    }

    /// Add all standard IDE functions to the interpreter.
    /// Requires that U implements IpcClient for IDE communication.
    pub fn add_standard_ide_functions(&mut self)
    where
        U: crate::ide::IpcClient,
    {
        self.add_function::<crate::ide::FindDefinitions>();
        self.add_function_with_name::<crate::ide::FindDefinitions>("finddefinition");
        self.add_function::<crate::ide::FindReferences>();
        self.add_function::<crate::ide::Search>();
        self.add_function::<crate::ide::Lines>();
        self.add_function::<crate::ide::GitDiff>();
        self.add_function::<crate::ide::Comment>();
        self.add_function::<crate::ide::Action>();
    }

    pub fn evaluate(
        &mut self,
        program: &str,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Value>> + '_>> {
        let ast = parse(&program);
        Box::pin(async move { self.evaluate_ast(ast?).await })
    }

    pub fn evaluate_ast(
        &mut self,
        ast: Ast,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<Value>> + '_>> {
        Box::pin(async move {
            match ast {
                Ast::Call(name, args) => {
                    let mut evaluated_args = Vec::new();
                    for arg in args {
                        evaluated_args.push(self.evaluate_ast(arg).await?);
                    }
                    self.call_function_positional(name, evaluated_args).await
                }
                Ast::Int(n) => Ok(Value::Number(n.into())),
                Ast::String(s) => Ok(Value::String(s)),
                Ast::Boolean(b) => Ok(Value::Bool(b)),
                Ast::Array(elements) => {
                    let mut results = Vec::new();
                    for element in elements {
                        results.push(self.evaluate_ast(element).await?);
                    }
                    Ok(Value::Array(results))
                }
                Ast::Object(map) => {
                    let mut result_map = serde_json::Map::new();
                    for (key, value) in map {
                        let evaluated_value = self.evaluate_ast(value).await?;
                        result_map.insert(key, evaluated_value);
                    }
                    Ok(Value::Object(result_map))
                }
            }
        })
    }

    async fn call_function_positional(
        &mut self,
        name: String,
        args: Vec<Value>,
    ) -> anyhow::Result<Value> {
        let name_lower = name.to_ascii_lowercase();
        let (func, parameter_order) = self
            .functions
            .get(&name_lower)
            .ok_or_else(|| anyhow::anyhow!("unknown function: {}", name))?;

        // Map positional args to named object
        let mut arg_object = serde_json::Map::new();
        for (i, value) in args.into_iter().enumerate() {
            if let Some(&param_name) = parameter_order.get(i) {
                arg_object.insert(param_name.to_string(), value);
            } else {
                anyhow::bail!(
                    "too many arguments for function {}: expected {}, got {}",
                    name,
                    parameter_order.len(),
                    i + 1
                );
            }
        }

        func(self, Value::Object(arg_object)).await
    }

    async fn execute<F>(&mut self, value: Value) -> anyhow::Result<Value>
    where
        F: DialectFunction<U>,
    {
        let input: F = serde_json::from_value(value)?;
        let output: F::Output = input.execute(self).await?;
        Ok(serde_json::to_value(output)?)
    }
}

impl<U: Send> Deref for DialectInterpreter<U> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        &self.userdata
    }
}

impl<U: Send> DerefMut for DialectInterpreter<U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.userdata
    }
}

/// Implemented by Dialect functions. This is meant to be implemented
/// on a struct that also implements `Deserialize` and which
/// defines the arguments to the function:
///
/// ```rust,ignore
/// #[derive(Deserialize)]
/// pub struct TheFunction {
///    symbol: String,
///    path: Option<String>,
/// }
/// ```
///
/// The struct name becomes the function name
/// (note: Dialect is case-insensitive when it comes to function names).
/// The argument names are defined by the struct fields.
///
/// To invoke your function, the Dialect interpreter will
///
/// 1. evaluate the positional arguments to JSON values
/// 2. map them to named arguments using `PARAMETER_ORDER`
/// 3. deserialize that into the `Self` type to create an instance of `Self`
/// 4. invoke [`DialectFunction::execute`][].
///
/// # Parameter Order
///
/// Functions are called with positional arguments that are mapped to struct fields
/// using the `PARAMETER_ORDER` constant. For example, with `PARAMETER_ORDER = &["symbol", "path"]`,
/// the call `findDefinitions("MyClass", "src/main.rs")` becomes `{"symbol": "MyClass", "path": "src/main.rs"}`.
// ANCHOR: dialect_function_trait
pub trait DialectFunction<U: Send>: DeserializeOwned + Send {
    type Output: Serialize + Send;

    const PARAMETER_ORDER: &'static [&'static str];

    async fn execute(self, interpreter: &mut DialectInterpreter<U>)
    -> anyhow::Result<Self::Output>;
}
// ANCHOR_END: dialect_function_trait

/// Macro to implement DialectFunction for value types that evaluate to themselves
#[macro_export]
macro_rules! dialect_value {
    ($ty:ty { $($param:ident),* }) => {
        impl<U: Send> $crate::dialect::DialectFunction<U> for $ty {
            type Output = $ty;

            const PARAMETER_ORDER: &'static [&'static str] = &[$(stringify!($param)),*];

            async fn execute(
                self,
                _interpreter: &mut $crate::dialect::DialectInterpreter<U>,
            ) -> anyhow::Result<Self::Output> {
                Ok(self)
            }
        }
    };
}
