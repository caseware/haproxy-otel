use std::collections::HashMap;

use haproxy_api::{Action, Core};
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub(crate) use cache::{get_context, remove_context, store_context};

pub fn register(lua: &Lua, options: LuaTable) -> LuaResult<()> {
    let core = Core::new(lua)?;

    let service_name = (options.get::<String>("name")).unwrap_or_else(|_| "haproxy".to_string());
    let sampler = (options.get::<Option<String>>("sampler")).unwrap_or_default();
    let propagator = (options.get::<Option<String>>("propagator")).unwrap_or_default();
    let otlp = (options.get::<LuaTable>("otlp")).unwrap_or_else(|_| lua.create_table().unwrap());
    let endpoint = (otlp.get::<Option<String>>("endpoint")).unwrap_or_default();
    let protocol = (otlp.get::<Option<String>>("protocol")).unwrap_or_default();
    
    // Extract headers from the otlp table if provided
    let headers = match otlp.get::<LuaTable>("headers") {
        Ok(headers_table) => {
            let mut headers_map = HashMap::new();
            for pair in headers_table.pairs::<String, String>() {
                if let Ok((key, value)) = pair {
                    headers_map.insert(key, value);
                }
            }
            if headers_map.is_empty() {
                None
            } else {
                Some(headers_map)
            }
        }
        Err(_) => None,
    };

    let options = exporter::Options {
        service_name: service_name.clone(),
        sampler: sampler.clone(),
        propagator: propagator.clone(),
        endpoint: endpoint.clone(),
        protocol: protocol.clone(),
        headers: headers.clone(),
    };
    lua.set_app_data(options.clone());

    // Log registration details via HAProxy core.log
    let _ = core.log(
        haproxy_api::LogLevel::Info,
        format!(
            "[haproxy-otel] register called: service_name={} endpoint={:?} protocol={:?} headers={} thread={}",
            service_name,
            endpoint.as_ref().unwrap_or(&"<default>".to_string()),
            protocol.as_ref().unwrap_or(&"<default>".to_string()),
            headers.as_ref().map(|h| h.len()).unwrap_or(0),
            core.thread().unwrap_or(0)
        ),
    );

    // Initialize exporter directly - OpenTelemetry global state handles idempotency
    let _ = core.log(
        haproxy_api::LogLevel::Info,
        format!("[haproxy-otel] calling exporter::init on thread {}", core.thread().unwrap_or(0)),
    );
    match exporter::init(options.clone()) {
        Ok(_) => {
            let _ = core.log(
                haproxy_api::LogLevel::Info,
                "[haproxy-otel] exporter init: SUCCESS".to_string(),
            );
        }
        Err(e) => {
            let _ = core.log(
                haproxy_api::LogLevel::Err,
                format!("[haproxy-otel] exporter init: FAILED - {}", e),
            );
        }
    }

    #[rustfmt::skip]
    core.register_action("start_server_span", &[Action::HttpReq], 0, span::start_server_span)?;
    core.register_action(
        "set_span_attribute_var",
        &[Action::HttpReq, Action::HttpRes, Action::HttpAfterRes],
        2,
        span::set_span_attribute,
    )?;
    core.register_filter::<filter::TraceFilter>("opentelemetry-trace")?;

    Ok(())
}

mod cache;
mod exporter;
mod filter;
mod runtime;
mod span;
