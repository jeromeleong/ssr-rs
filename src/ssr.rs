use lru::LruCache;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Once;
use v8::{Context, Function, Local, PromiseState, Value};

static V8_INIT: Once = Once::new();

pub struct Ssr {
    isolate: Rc<RefCell<v8::OwnedIsolate>>,
    context: v8::Global<Context>,
    fn_map: Rc<RefCell<HashMap<String, v8::Global<Function>>>>,
    script_cache: Rc<RefCell<LruCache<String, v8::Global<v8::UnboundScript>>>>,
    loaded_scripts: Rc<RefCell<HashMap<String, ()>>>,
    render_cache: Rc<RefCell<HashMap<String, String>>>,
}

impl Default for Ssr {
    fn default() -> Self {
        Self::new()
    }
}

impl Ssr {
    pub fn init() {
        V8_INIT.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();
        });
    }

    pub fn new() -> Self {
        Self::init();

        let mut isolate = v8::Isolate::new(v8::CreateParams::default());

        let global_context = {
            let handle_scope = &mut v8::HandleScope::new(&mut isolate);
            let context = v8::Context::new(handle_scope, v8::ContextOptions::default());
            v8::Global::new(handle_scope, context)
        };

        Ssr {
            isolate: Rc::new(RefCell::new(isolate)),
            context: global_context,
            fn_map: Rc::new(RefCell::new(HashMap::new())),
            script_cache: Rc::new(RefCell::new(LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap(),
            ))),
            loaded_scripts: Rc::new(RefCell::new(HashMap::new())),
            render_cache: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn load(&self, source: &str, entry_point: &str, module_type: &str) -> Result<(), String> {
        if self.loaded_scripts.borrow().contains_key(source) {
            return Ok(());
        }

        let mut isolate = self.isolate.borrow_mut();
        let context = &self.context;
        let mut scope = v8::HandleScope::with_context(&mut *isolate, context);
        let context = Local::new(&mut scope, context);
        let mut scope = v8::ContextScope::new(&mut scope, context);

        match module_type {
            "esm" => Self::load_esm(
                &mut scope,
                source,
                entry_point,
                &mut self.fn_map.borrow_mut(),
            ),
            "cjs" => Self::load_cjs(
                &mut scope,
                source,
                entry_point,
                &mut self.fn_map.borrow_mut(),
                &mut self.script_cache.borrow_mut(),
            ),
            _ => Err("Unsupported module type".to_string()),
        }?;

        self.loaded_scripts
            .borrow_mut()
            .insert(source.to_string(), ());
        Ok(())
    }

    fn load_esm(
        scope: &mut v8::ContextScope<'_, v8::HandleScope>,
        source: &str,
        entry_point: &str,
        fn_map: &mut std::collections::HashMap<String, v8::Global<Function>>,
    ) -> Result<(), String> {
        Self::load_module(scope, source, "module.js")?;
        let global = scope.get_current_context().global(scope);
        let exports_str = v8::String::new(scope, "exports").unwrap();
        let exports = global
            .get(scope, exports_str.into())
            .ok_or("No exports found")?;
        let exports_obj = exports.to_object(scope).ok_or("Exports is not an object")?;
        let entry_point = v8::String::new(scope, entry_point).unwrap();
        let entry_func = exports_obj
            .get(scope, entry_point.into())
            .ok_or("Entry point not found in exports")?;
        if let Ok(func) = v8::Local::<v8::Function>::try_from(entry_func) {
            fn_map.insert("default".to_string(), v8::Global::new(scope, func));
            Ok(())
        } else {
            Err("Entry point is not a function".to_string())
        }
    }

    fn load_cjs(
        scope: &mut v8::ContextScope<'_, v8::HandleScope>,
        source: &str,
        entry_point: &str,
        fn_map: &mut std::collections::HashMap<String, v8::Global<Function>>,
        script_cache: &mut LruCache<String, v8::Global<v8::UnboundScript>>,
    ) -> Result<(), String> {
        Self::load_commonjs(scope, source, "module.js")?;
        let code = format!("{source};{entry_point}");
        let script = Self::compile_script(scope, &code, script_cache)?;

        let result = script.run(scope).ok_or("Failed to run script")?;
        let object = result
            .to_object(scope)
            .ok_or("Invalid JS: The script does not return any object after being executed")?;

        let props = object
            .get_own_property_names(scope, Default::default())
            .unwrap();
        for i in 0..props.length() {
            let key = props.get_index(scope, i).unwrap();
            let key_str = key.to_string(scope).unwrap().to_rust_string_lossy(scope);
            let val = object.get(scope, key).unwrap();
            if let Ok(func) = v8::Local::<v8::Function>::try_from(val) {
                fn_map.insert(key_str, v8::Global::new(scope, func));
            }
        }

        Ok(())
    }

    pub fn render_to_string(&self, params: Option<&str>) -> Result<String, String> {
        let cache_key = match params {
            Some(p) => p.to_string(),
            None => "".to_string(),
        };

        if let Some(cached_result) = self.render_cache.borrow().get(&cache_key) {
            return Ok(cached_result.clone());
        }

        let fn_map = self.fn_map.borrow();
        let results: Vec<Result<String, String>> = fn_map
            .iter()
            .map(|(_, func)| {
                let mut isolate = self.isolate.borrow_mut();
                let context = &self.context;
                let mut scope = v8::HandleScope::with_context(&mut *isolate, context);
                let context = Local::new(&mut scope, context);
                let mut scope = v8::ContextScope::new(&mut scope, context);

                let params: Local<Value> = match params {
                    Some(p) => v8::String::new(&mut scope, p).unwrap().into(),
                    None => v8::undefined(&mut scope).into(),
                };

                let undef = v8::undefined(&mut scope).into();

                let func = Local::new(&mut scope, func);
                let result = func
                    .call(&mut scope, undef, &[params])
                    .ok_or("Failed to call function")?;

                if result.is_promise() {
                    let promise = v8::Local::<v8::Promise>::try_from(result)
                        .map_err(|_| "Failed to cast main function to promise")?;

                    while promise.state() == PromiseState::Pending {
                        scope.perform_microtask_checkpoint();
                    }

                    let result = promise.result(&mut scope);
                    if result.is_null_or_undefined() {
                        return Err("Promise rejected".to_string());
                    }

                    Ok(result
                        .to_string(&mut scope)
                        .ok_or("Failed to parse the result to string")?
                        .to_rust_string_lossy(&mut scope))
                } else {
                    Ok(result
                        .to_string(&mut scope)
                        .ok_or("Failed to parse the result to string")?
                        .to_rust_string_lossy(&mut scope))
                }
            })
            .collect();

        let rendered = results
            .into_iter()
            .collect::<Result<Vec<String>, String>>()?
            .join("");

        self.render_cache
            .borrow_mut()
            .insert(cache_key, rendered.clone());
        Ok(rendered)
    }

    fn compile_script<'s>(
        scope: &mut v8::ContextScope<'s, v8::HandleScope>,
        source: &str,
        script_cache: &mut LruCache<String, v8::Global<v8::UnboundScript>>,
    ) -> Result<v8::Local<'s, v8::Script>, String> {
        if script_cache.get(source).is_some() {
            // If the script is in the cache, compile it again (we can't return the cached version directly)
            let source = v8::String::new(scope, source).ok_or("Failed to create V8 string")?;
            return v8::Script::compile(scope, source, None)
                .ok_or_else(|| "Failed to compile cached script".to_string());
        }

        let source = v8::String::new(scope, source).ok_or("Failed to create V8 string")?;
        let script = v8::Script::compile(scope, source, None).ok_or("Failed to compile script")?;

        let unbound_script = script.get_unbound_script(scope);
        script_cache.put(
            source.to_rust_string_lossy(scope),
            v8::Global::new(scope, unbound_script),
        );

        Ok(script)
    }

    fn load_module(
        scope: &mut v8::ContextScope<'_, v8::HandleScope>,
        source: &str,
        file_name: &str,
    ) -> Result<(), String> {
        let source_str = v8::String::new(scope, source).unwrap();
        let file_name_str = v8::String::new(scope, file_name).unwrap();

        let origin = v8::ScriptOrigin::new(
            scope,
            file_name_str.into(),
            0,
            0,
            false,
            0,
            None,
            false,
            false,
            true,
            None,
        );
        let mut source = v8::script_compiler::Source::new(source_str, Some(&origin));
        let module = v8::script_compiler::compile_module(scope, &mut source)
            .ok_or("Failed to compile module")?;

        module
            .instantiate_module(scope, |_, _, _, _| None)
            .ok_or("Failed to instantiate module")?;

        let result = module.evaluate(scope).ok_or("Failed to evaluate module")?;

        if result.is_promise() {
            let promise = v8::Local::<v8::Promise>::try_from(result).unwrap();
            while promise.state() == v8::PromiseState::Pending {
                scope.perform_microtask_checkpoint();
            }
            if promise.state() != v8::PromiseState::Fulfilled {
                return Err("Module evaluation promise rejected".to_string());
            }
        }

        // Set the exports object in the global scope
        let global = scope.get_current_context().global(scope);
        let exports = module.get_module_namespace();
        let exports_str = v8::String::new(scope, "exports").unwrap();

        // Create all necessary values before the call to set
        let exports_val = exports;
        let exports_str_val: v8::Local<v8::Value> = exports_str.into();

        global
            .set(scope, exports_str_val, exports_val)
            .ok_or("Failed to set exports")?;

        Ok(())
    }

    fn load_commonjs(
        scope: &mut v8::ContextScope<'_, v8::HandleScope>,
        source: &str,
        file_name: &str,
    ) -> Result<(), String> {
        let source_str = format!("(function(require, module, exports) {{{}}})", source);
        let source_script = v8::String::new(scope, &source_str).unwrap();
        let file_name_str = v8::String::new(scope, file_name).unwrap();

        let origin = v8::ScriptOrigin::new(
            scope,
            file_name_str.into(),
            0,
            0,
            false,
            0,
            None,
            false,
            false,
            false,
            None,
        );
        let script = v8::Script::compile(scope, source_script, Some(&origin))
            .ok_or("Failed to compile script")?;

        script.run(scope).ok_or("Failed to run script")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    pub fn init_test() {
        INIT.call_once(|| {
            Ssr::init();
        })
    }

    fn create_ssr(source: &str, entry_point: &str, module_type: &str) -> Ssr {
        let ssr = Ssr::new();
        ssr.load(source, entry_point, module_type).unwrap();
        ssr
    }

    #[test]
    fn test_render_simple_html() {
        init_test();

        let source = r##"var SSR = {x: () => "<html><body>Hello, world!</body></html>"};"##;

        let ssr = create_ssr(source, "SSR", "cjs");
        let html = ssr.render_to_string(None).unwrap();

        assert_eq!(html, "<html><body>Hello, world!</body></html>");
    }

    #[test]
    fn test_render_with_params() {
        init_test();

        let source = r##"var SSR = {x: (params) => `<html><body>${params}</body></html>`};"##;

        let ssr = create_ssr(source, "SSR", "cjs");
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
        "##;

        let ssr = create_ssr(source, "render", "esm");
        let html = ssr.render_to_string(None).unwrap();

        assert_eq!(html, "<html><body>ESM Hello, world!</body></html>");
    }

    #[test]
    fn test_invalid_js() {
        init_test();

        let source = r##"var SSR = {x: () => { throw new Error("Test error"); }};"##;

        let ssr = create_ssr(source, "SSR", "cjs");
        let result = ssr.render_to_string(None);
        assert!(result.is_err());
    }
}
