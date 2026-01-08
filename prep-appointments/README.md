# SvS Preparation Week - Appointment Scheduler

A Rust-based appointment scheduling system for SvS Preparation Week with a web interface.

## Features

- **CSV Parsing**: Reads appointment data from CSV files
- **Smart Scheduling**: Automatic scheduling with priority-based slot assignment
- **Slot Stealing**: Advanced algorithm that can move players up to 5 levels deep to optimize assignments
- **Web Interface**: 
  - Admin panel (password protected) for CSV upload
  - Statistics page showing alliance counts and time slot popularity (no player names)
  - Schedule display page for all three days

## Usage

### CLI Mode (Original)

```bash
cargo run
```

This will process the hardcoded CSV file and generate schedule text files.

### Web Server Mode

```bash
# Set admin password (optional, defaults to "admin123")
$env:ADMIN_PASSWORD="your-secure-password"

# Start web server on port 8080 (default)
cargo run web

# Or specify a custom port
cargo run web 3000
```

Then access:
- Home: http://localhost:8080
- Admin Panel: http://localhost:8080/admin
- Statistics: http://localhost:8080/stats
- Schedules: http://localhost:8080/schedules

## Admin Panel

1. Navigate to `/admin`
2. Enter the admin password (set via `ADMIN_PASSWORD` environment variable, or defaults to "admin123")
3. Upload a CSV file
4. The system will automatically process and generate schedules

## Statistics Page

Shows:
- **Alliance Request Counts**: How many requests from each alliance for each day type
- **Time Slot Popularity**: How many players requested each time slot for each day

**Note**: No player names are displayed on the statistics page for privacy.

## Schedule Display

View the complete schedules for:
- Construction Day
- Research Day
- Troops Training Day

Each schedule shows all 49 time slots with assigned players or [EMPTY] markers.

## Deployment to name.com

To deploy to name.com hosting:

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Upload the binary and required files:
   - `target/release/prep-appointments.exe` (or binary for your server OS)
   - `templates/` directory
   - `static/` directory

3. Set environment variables:
   - `ADMIN_PASSWORD`: Your secure admin password

4. Run the server:
   ```bash
   ./prep-appointments web 80
   ```

5. Configure your domain to point to the server and set up port forwarding if needed.

## Project Structure

```
prep-appointments/
├── src/
│   ├── main.rs      # Core scheduling logic
│   └── web.rs       # Web server and API endpoints
├── templates/       # HTML templates
│   ├── index.html
│   ├── admin.html
│   ├── stats.html
│   └── schedules.html
├── static/          # Static assets
│   └── style.css
└── Cargo.toml
```

## Security Note

**Important**: Change the default admin password before deploying to production! Set the `ADMIN_PASSWORD` environment variable to a strong password.

