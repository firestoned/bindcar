# Performance

Performance optimization for bindcar and BIND9.

## bindcar Performance

Typical performance characteristics:

- **Latency**: 10-100ms per operation
- **Throughput**: 100-500 req/s per instance
- **Resource Usage**: Minimal (< 100Mi memory, < 0.1 CPU)

## BIND9 Performance

BIND9 is the primary performance factor:

- Query performance: 10,000+ queries/s
- Zone loading: Sub-second for small zones
- Zone transfers: Depends on zone size

## Optimization Strategies

- Use appropriate TTL values
- Minimize zone file size
- Optimize BIND9 configuration
- Use caching nameservers upstream

## Next Steps

- [Tuning](./tuning.md) - Performance tuning guide
