# K3s + Postgres Cutover Runbook

This runbook covers a one-time (big-bang) migration from file-based JSON/CSV storage to PostgreSQL with app deployment on a single-node k3s cluster.

## Preconditions

- Server A runs k3s and can reach Server B (Postgres host) on TCP 5432.
- Server B has PostgreSQL provisioned with database and user credentials.
- `DATABASE_URL` secret created in k8s (connection string like `postgres://USER:PASSWORD@HOST:PORT/DATABASE`).
- Legacy data directory (`data/`) is available on Server A.
- Backend image includes both binaries:
  - `/app/prep-appointments`
  - `/app/migrate_json_to_pg`

## Rehearsal Checklist (Required)

Substitute your real URL in place of the placeholder (copy from k8s secrets or `.env`):

`postgres://USER:PASSWORD@HOST:PORT/DATABASE`

1. Copy production `data/` to a staging environment.
2. Apply SQL schema migration:
   - `psql 'postgres://USER:PASSWORD@HOST:PORT/DATABASE' -f prep-appointments/migrations/0001_init.sql`
3. Run dry-run migration:
   - `DRY_RUN=true MIGRATE_DATA_DIR=/path/to/data DATABASE_URL='postgres://USER:PASSWORD@HOST:PORT/DATABASE' /app/migrate_json_to_pg`
4. Run real migration on staging and verify:
   - Account count parity
   - Form count parity (current + old)
   - Schedule/statistics parity for each `{account}:{server}`
   - Login, form submit, schedule generation API smoke tests

## Production Cutover Steps

1. **Announce maintenance window** and disable new writes.
2. **Back up legacy storage**:
   - `tar -czf legacy-data-backup-$(date +%F-%H%M).tgz data/`
3. **Create DB restore point**:
   - `pg_dump 'postgres://USER:PASSWORD@HOST:PORT/DATABASE' > pre-cutover-$(date +%F-%H%M).sql`
4. **Apply DB schema**:
   - `psql 'postgres://USER:PASSWORD@HOST:PORT/DATABASE' -f prep-appointments/migrations/0001_init.sql`
5. **Run migration job**:
   - `kubectl apply -f k8s/migration-job.yaml`
   - `kubectl logs -n kingshot job/json-to-pg-migration -f`
6. **Deploy backend with postgres storage**:
   - Ensure env includes:
     - `STORAGE_BACKEND=postgres`
     - `DATABASE_URL` (same connection string as above)
7. **Deploy frontend and ingress**:
   - `kubectl apply -f k8s/namespace.yaml`
   - `kubectl apply -f k8s/configmap.yaml`
   - `kubectl apply -f k8s/backend.yaml`
   - `kubectl apply -f k8s/frontend.yaml`
   - `kubectl apply -f k8s/ingress.yaml`
8. **Post-cutover validation**:
   - Login/logout
   - Form create/update
   - Form submit
   - Schedule generate/read/update
   - Feedback submit/list

## Schema update: Server Organisation / Tyrant (`0002`)

For databases that already have `0001` applied (e.g. production after cutover), apply further migrations with the same URL form:

```bash
psql 'postgres://USER:PASSWORD@HOST:PORT/DATABASE' -f prep-appointments/migrations/0002_server_org.sql
```

## Rollback

1. Route traffic back to old deployment.
2. Restore old runtime config (`STORAGE_BACKEND=json`).
3. Restore legacy data archive if needed.
4. Keep Postgres snapshot for audit and post-mortem.

## Fast Verification Queries

```sql
SELECT COUNT(*) FROM accounts;
SELECT COUNT(*) FROM forms WHERE archived = FALSE;
SELECT COUNT(*) FROM forms WHERE archived = TRUE;
SELECT COUNT(*) FROM schedules;
SELECT COUNT(*) FROM statistics;
SELECT COUNT(*) FROM feedback;
SELECT COUNT(*) FROM form_submissions;
```
