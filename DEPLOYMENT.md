# Deployment Guide

## VPS Requirements

**Minimum Recommended:**
- **CPU**: 2 vCPU (sufficient for moderate traffic)
- **RAM**: 4GB (plenty for typical usage)
- **Disk**: 5GB (more than enough for data storage)

## Building for Production

1. **Build optimized release binary:**
   ```bash
   cd prep-appointments
   cargo build --release
   ```

2. **The binary will be at:**
   ```
   target/release/prep-appointments
   ```

## Deployment Steps

1. **Upload to VPS:**
   - Binary: `target/release/prep-appointments`
   - Templates: `prep-appointments/templates/` directory
   - Static files: `prep-appointments/static/` directory (if any)
   - Create `data/` directory on the server

2. **Set up data directory:**
   ```bash
   mkdir -p data/current_forms data/old_forms data/schedules data/statistics
   ```

3. **Set environment variables (optional):**
   ```bash
   export DATA_DIR="/path/to/data"  # Defaults to ./data
   ```

4. **Run the server:**
   ```bash
   # Run on port 8080 (default)
   ./prep-appointments web
   
   # Or specify a port
   ./prep-appointments web 80
   ```

5. **Run as a service (systemd example):**
   Create `/etc/systemd/system/prep-appointments.service`:
   ```ini
   [Unit]
   Description=Prep Appointments Scheduler
   After=network.target

   [Service]
   Type=simple
   User=your-user
   WorkingDirectory=/path/to/KingshotScheduleMaker/prep-appointments
   ExecStart=/path/to/KingshotScheduleMaker/prep-appointments/target/release/prep-appointments web 8080
   Restart=always
   RestartSec=10

   [Install]
   WantedBy=multi-user.target
   ```

   Then:
   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable prep-appointments
   sudo systemctl start prep-appointments
   ```

6. **Set up reverse proxy (nginx example):**
   ```nginx
   server {
       listen 80;
       server_name your-domain.com;

       location / {
           proxy_pass http://localhost:8080;
           proxy_set_header Host $host;
           proxy_set_header X-Real-IP $remote_addr;
           proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
           proxy_set_header X-Forwarded-Proto $scheme;
       }
   }
   ```

## Resource Usage Estimates

- **Binary size**: ~10-20MB (release build)
- **Memory usage**: ~50-200MB at idle, ~500MB-1GB under moderate load
- **CPU usage**: <5% at idle, spikes during schedule generation
- **Disk usage**: 
  - Application: ~50MB
  - Data: Depends on usage (typically <1GB for hundreds of forms/submissions)

## Performance Tips

1. **Use release build** (already optimized in Cargo.toml)
2. **Monitor disk space** - old forms are archived but not deleted
3. **Consider periodic cleanup** of old forms if disk space becomes an issue
4. **Use a reverse proxy** (nginx/caddy) for SSL/TLS termination

## Monitoring

Check resource usage:
```bash
# Check memory and CPU
htop

# Check disk usage
du -sh data/

# Check if service is running
systemctl status prep-appointments
```
