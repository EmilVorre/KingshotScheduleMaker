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

## HTTPS (Let's Encrypt + cert-manager, k3s Traefik)

1. **DNS:** Point your hostname (e.g. `ks.example.com`) at the cluster node’s public IP.

2. **Install cert-manager** (once per cluster; pick a [current release](https://github.com/cert-manager/cert-manager/releases)):

   ```bash
   kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.16.2/cert-manager.yaml
   kubectl -n cert-manager rollout status deploy/cert-manager --timeout=120s
   ```

3. **ClusterIssuer:** Edit `email:` in `cert-manager-letsencrypt-clusterissuer.yaml`, then:

   ```bash
   kubectl apply -f k8s/cert-manager-letsencrypt-clusterissuer.yaml
   ```

4. **Ingress host + TLS:** In `ingress.yaml`, replace every `ks.example.com` with your real domain (must match DNS). Apply:

   ```bash
   kubectl apply -f k8s/ingress.yaml
   ```

5. **Wait for the certificate** (secret `kingshot-tls` is filled by cert-manager):

   ```bash
   kubectl -n kingshot get certificate
   kubectl -n kingshot describe certificate kingshot-tls
   ```

   Port **80** must reach Traefik from the internet while the HTTP-01 challenge runs.

6. **App URLs:** Set `BASE_URL`, `FRONTEND_URL`, and OAuth callback URLs in `kingshot-secrets` to `https://your-domain` (and redeploy backend if needed).

## Apply order

```bash
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/secrets.example.yaml
kubectl apply -f k8s/backend.yaml
kubectl apply -f k8s/frontend.yaml
# After cert-manager + ClusterIssuer (see HTTPS section):
kubectl apply -f k8s/ingress.yaml
```

## Run migration job

```bash
kubectl apply -f k8s/migration-job.yaml
kubectl logs -n kingshot job/json-to-pg-migration -f
```
