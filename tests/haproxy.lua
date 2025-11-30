local opentelemetry = require("haproxy_otel_module")

opentelemetry.register({
	name = "haproxy",
	otlp = {
		-- Use OTLP over HTTP (collector default port 4318) and correct path
		endpoint = "http://localhost:4318/v1/traces",
		protocol = "json",
	},
	sampler = "AlwaysOn",
	propagator = "zipkin",
})
