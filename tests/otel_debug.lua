-- Simple HAProxy Lua debug helpers
-- Load with: lua-load-per-thread otel_debug.lua
-- Use in config:
--   http-request lua.debug_entry
--   http-response lua.debug_resp

local function safe_get(txn, var)
  local ok, val = pcall(function() return txn:get_var(var) end)
  if ok then return val else return nil end
end

core.register_action("debug_entry", { "http-req" }, function(txn)
  local method = txn:get_method() or "-"
  local path = txn:get_path() or "-"
  local span_id = safe_get(txn, "txn.otel_span_id") or "-"
  local trace_id = safe_get(txn, "txn.otel_trace_id") or "-"
  txn.Info(string.format("lua.debug_entry: method=%s path=%s span.id=%s trace.id=%s", method, path, span_id, trace_id))
end)

core.register_action("debug_resp", { "http-res" }, function(txn)
  local status = (txn.sf and txn.sf.status and txn.sf:status()) or "-"
  local span_id = safe_get(txn, "txn.otel_span_id") or "-"
  local trace_id = safe_get(txn, "txn.otel_trace_id") or "-"
  txn.Info(string.format("lua.debug_resp: status=%s span.id=%s trace.id=%s", status, span_id, trace_id))
end)
