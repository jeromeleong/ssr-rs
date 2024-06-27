// TODO: replace hashmap with more performant https://nnethercote.github.io/perf-book/hashing.html
use std::collections::HashMap;
use v8::Local;

#[derive(Debug)]
pub struct Ssr<'s, 'i> {
    isolate: *mut v8::OwnedIsolate,
    handle_scope: *mut v8::HandleScope<'s, ()>,
    fn_map: HashMap<String, v8::Local<'s, v8::Function>>,
    scope: *mut v8::ContextScope<'i, v8::HandleScope<'s>>,
}

impl Drop for Ssr<'_, '_> {
    fn drop(&mut self) {
        self.fn_map.clear();
        unsafe {
            let _ = Box::from_raw(self.scope);
            let _ = Box::from_raw(self.handle_scope);
            let _ = Box::from_raw(self.isolate);
        };
    }
}

impl<'s, 'i> Ssr<'s, 'i>
where
    's: 'i,
{
    /// Initialize a V8 js engine instance. It's mandatory to call it before
    /// any call to V8. The Ssr module needs this function call before any other
    /// operation.
    pub fn create_platform() {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    }

    /// It creates a new SSR instance.
    ///
    /// This function is expensive and it should be called as less as possible.
    ///
    /// Even though V8 allows multiple threads the Ssr struct created with this call can be accessed by just
    /// the thread that created it.
    ///
    /// Multiple instances are allowed.
    ///
    /// Entry point is the JS element that the bundler exposes. It has to be an empty string in
    /// case the bundle is exported as IIFE.
    ///
    /// Check the examples <a href="https://github.com/Valerioageno/ssr-rs/tree/main/examples/vite-react">vite-react</a> (for the IIFE example) and
    /// <a href="https://github.com/Valerioageno/ssr-rs/tree/main/examples/webpack-react">webpack-react</a> (for the bundle exported as variable).
    ///
    /// See the examples folder for more about using multiple parallel instances for multi-threaded
    /// execution.
    pub fn from(
        source: String,
        entry_point: &str,
        module_type: &str,
    ) -> Result<Self, &'static str> {
        let isolate = Box::into_raw(Box::new(v8::Isolate::new(v8::CreateParams::default())));

        let handle_scope = unsafe { Box::into_raw(Box::new(v8::HandleScope::new(&mut *isolate))) };

        let context = unsafe { v8::Context::new(&mut *handle_scope) };

        let scope_ptr =
            unsafe { Box::into_raw(Box::new(v8::ContextScope::new(&mut *handle_scope, context))) };

        let scope = unsafe { &mut *scope_ptr };

        match module_type {
            "esm" => {
                let module = load_module(scope, &source, "module.js")?;
                let exports = module.get_module_namespace().to_object(scope).unwrap();
                let mut fn_map: HashMap<String, v8::Local<v8::Function>> = HashMap::new();

                let props = exports
                    .get_own_property_names(scope, Default::default())
                    .unwrap();
                for i in 0..props.length() {
                    let key = props.get_index(scope, i).unwrap();
                    let key_str = key.to_string(scope).unwrap().to_rust_string_lossy(scope);
                    let val = exports.get(scope, key).unwrap();
                    let func = Local::<v8::Function>::try_from(val).unwrap();
                    fn_map.insert(key_str, func);
                }

                return Ok(Ssr {
                    isolate,
                    handle_scope,
                    fn_map,
                    scope: scope_ptr,
                });
            }
            "cjs" => {
                load_commonjs(scope, &source, "module.js")?;
            }
            _ => {
                return Err("Unsupported module type");
            }
        }

        let code = match v8::String::new(scope, &format!("{source};{entry_point}")) {
            Some(val) => val,
            None => return Err("Invalid JS: Strings are needed"),
        };

        let script = match v8::Script::compile(scope, code, None) {
            Some(val) => val,
            None => return Err("Invalid JS: There aren't runnable scripts"),
        };

        let exports = match script.run(scope) {
            Some(val) => val,
            None => return Err("Invalid JS: Execute your script with d8 to debug"),
        };

        let object = match exports.to_object(scope) {
            Some(val) => val,
            None => {
                return Err(
                    "Invalid JS: The script does not return any object after being executed",
                )
            }
        };

        let mut fn_map: HashMap<String, v8::Local<v8::Function>> = HashMap::new();

        if let Some(props) = object.get_own_property_names(scope, Default::default()) {
            fn_map = match Some(props)
                .iter()
                .enumerate()
                .map(
                    |(i, &p)| -> Result<(String, v8::Local<v8::Function>), &'static str> {
                        let name = match p.get_index(scope, i as u32) {
                            Some(val) => val,
                            None => return Err("Failed to get function name"),
                        };

                        let mut scope = v8::EscapableHandleScope::new(scope);

                        let func = match object.get(&mut scope, name) {
                            Some(val) => val,
                            None => return Err("Failed to get function from obj"),
                        };

                        let func = unsafe { v8::Local::<v8::Function>::cast(func) };

                        let fn_name = match name.to_string(&mut scope) {
                            Some(val) => val.to_rust_string_lossy(&mut scope),
                            None => return Err("Failed to find function name"),
                        };

                        Ok((fn_name, scope.escape(func)))
                    },
                )
                // TODO: collect directly the values into a map
                .collect()
            {
                Ok(val) => val,
                Err(err) => return Err(err),
            }
        }

        Ok(Ssr {
            isolate,
            handle_scope,
            fn_map,
            scope: scope_ptr,
        })
    }

    /// Execute the Javascript functions and return the result as string.
    pub fn render_to_string(&mut self, params: Option<&str>) -> Result<String, &'static str> {
        let scope = unsafe { &mut *self.scope };

        let params: v8::Local<v8::Value> = match v8::String::new(scope, params.unwrap_or("")) {
            Some(s) => s.into(),
            None => v8::undefined(scope).into(),
        };

        let undef = v8::undefined(scope).into();

        let mut rendered = String::new();

        // TODO: transform this into an iterator
        for key in self.fn_map.keys() {
            let result = match self.fn_map[key].call(scope, undef, &[params]) {
                Some(val) => val,
                None => return Err("Failed to call function"),
            };

            if result.is_promise() {
                let promise = match v8::Local::<v8::Promise>::try_from(result) {
                    Ok(val) => val,
                    Err(_) => return Err("Failed to cast main function to promise"),
                };

                while promise.state() == v8::PromiseState::Pending {
                    scope.perform_microtask_checkpoint();
                }

                let result = promise.result(scope);
                if result.is_null_or_undefined() {
                    return Err("Promise rejected");
                }

                let result_str = match result.to_string(scope) {
                    Some(val) => val.to_rust_string_lossy(scope),
                    None => return Err("Failed to parse the result to string"),
                };

                rendered = format!("{}{}", rendered, result_str);
            } else {
                let result_str = match result.to_string(scope) {
                    Some(val) => val.to_rust_string_lossy(scope),
                    None => return Err("Failed to parse the result to string"),
                };

                rendered = format!("{}{}", rendered, result_str);
            }
        }

        Ok(rendered)
    }
}

/// Compile and evaluate an ECMAScript module.
fn load_module<'a>(
    scope: &mut v8::HandleScope<'a>,
    source: &str,
    file_name: &str,
) -> Result<v8::Local<'a, v8::Module>, &'static str> {
    let source_str = v8::String::new(scope, source).unwrap();
    let file_name_str = v8::String::new(scope, file_name).unwrap();
    let undefined = v8::undefined(scope).into();
    let origin = v8::ScriptOrigin::new(
        scope,
        file_name_str.into(),
        0,
        0,
        false,
        0,
        undefined,
        false,
        false,
        true, // is_module should be true for modules
    );
    let source = v8::script_compiler::Source::new(source_str, Some(&origin));
    let module = match v8::script_compiler::compile_module(scope, source) {
        Some(m) => m,
        None => return Err("Failed to compile module"),
    };
    match module.instantiate_module(scope, |_, _, _, _| None) {
        Some(_) => (),
        None => return Err("Failed to instantiate module"),
    }
    match module.evaluate(scope) {
        Some(_) => (),
        None => return Err("Failed to evaluate module"),
    }
    Ok(module)
}

/// Compile and evaluate a CommonJS and IIFE module.
fn load_commonjs<'a>(
    scope: &mut v8::HandleScope<'a>,
    source: &str,
    file_name: &str,
) -> Result<(), &'static str> {
    let source_str = format!("(function(require, module, exports) {{{}}})", source);
    let source_script = v8::String::new(scope, &source_str).unwrap();
    let file_name_str = v8::String::new(scope, file_name).unwrap();
    let undefined = v8::undefined(scope).into();
    let origin = v8::ScriptOrigin::new(
        scope,
        file_name_str.into(),
        0,
        0,
        false,
        0,
        undefined,
        false,
        false,
        false, // is_module should be false for CommonJS
    );
    let script = match v8::Script::compile(scope, source_script, Some(&origin)) {
        Some(s) => s,
        None => return Err("Failed to compile script"),
    };
    match script.run(scope) {
        Some(_) => Ok(()),
        None => Err("Failed to run script"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn init_test() {
        INIT.call_once(|| {
            Ssr::create_platform();
        })
    }

    #[test]
    fn test_render_simple_html() {
        init_test();

        let source =
            r##"var SSR = {x: () => "<html><body>Hello, world!</body></html>"};"##.to_string();

        let mut ssr = Ssr::from(source, "SSR", "cjs").unwrap();
        let html = ssr.render_to_string(None).unwrap();

        assert_eq!(html, "<html><body>Hello, world!</body></html>");
    }

    #[test]
    fn test_render_with_params() {
        init_test();

        let source =
            r##"var SSR = {x: (params) => `<html><body>${params}</body></html>`};"##.to_string();

        let mut ssr = Ssr::from(source, "SSR", "cjs").unwrap();
        let params = r#"{"message": "Hello, parameters!"}"#;
        let html = ssr.render_to_string(Some(params)).unwrap();

        assert_eq!(
            html,
            "<html><body>{\"message\": \"Hello, parameters!\"}</body></html>"
        );
    }

    #[test]
    fn test_esm_module() {
        init_test();

        let source = r##"
        export function render() {
            return "<html><body>ESM Hello, world!</body></html>";
        }
        "##
        .to_string();

        let mut ssr = Ssr::from(source, "render", "esm").unwrap();
        let html = ssr.render_to_string(None).unwrap();

        assert_eq!(html, "<html><body>ESM Hello, world!</body></html>");
    }

    #[test]
    fn test_invalid_js() {
        init_test();

        let source = r##"var SSR = {x: () => { throw new Error("Test error"); }};"##.to_string();

        let mut ssr = Ssr::from(source, "SSR", "cjs").unwrap();
        let result = ssr.render_to_string(None);
        assert!(result.is_err());
    }
}
