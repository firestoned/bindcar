# Tuning

Performance tuning guide for bindcar and BIND9.

## bindcar Tuning

Resource limits:

```yaml
resources:
  requests:
    memory: "128Mi"
    cpu: "100m"
  limits:
    memory: "256Mi"
    cpu: "500m"
```

## BIND9 Tuning

Common BIND9 optimizations:

- Adjust `max-cache-size`
- Configure `rate-limit`
- Optimize `transfers-in` and `transfers-out`

## Monitoring

Monitor performance metrics:

- Request latency
- Error rates
- Resource usage

## Next Steps

- [Performance](./performance.md) - Performance overview
- [Monitoring](../operations/monitoring.md) - Monitoring guide
