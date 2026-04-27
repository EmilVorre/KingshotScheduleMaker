# Kubernetes Manifests (k3s)

This directory contains baseline manifests for deploying Kingshot on a single-node k3s cluster.

## Files

- `namespace.yaml`: namespace `kingshot`
- `configmap.yaml`: non-secret app configuration
- `secrets.example.yaml`: template for required secrets (copy and fill values)
- `backend.yaml`: Rust API deployment/service
- `frontend.yaml`: React frontend deployment/service
- `ingress.yaml`: routes `/api` and `/form` to backend, everything else to frontend
- `migration-job.yaml`: one-time JSON/CSV -> Postgres migration job

## Apply order

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.example.yaml
kubectl apply -f k8s/backend.yaml
kubectl apply -f k8s/frontend.yaml
kubectl apply -f k8s/ingress.yaml
```

## Run migration job

```bash
kubectl apply -f k8s/migration-job.yaml
kubectl logs -n kingshot job/json-to-pg-migration -f
```
