local opentelemetry = require("haproxy_otel_module")

opentelemetry.register({
	name = "haproxy",
	otlp = {
		endpoint = "http://localhost:4318/v1/trace",
		protocol = "json",
		headers = {
			["X-Custom-Header"] = "custom-value",
			["api-key"] = "test-api-key-12345",
		},
	},
	sampler = "AlwaysOn",
	propagator = "zipkin",
})
