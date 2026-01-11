# SvS Preparation Week - Appointment Scheduler #

A Rust-based appointment scheduling system for SvS Preparation Week with a web interface.

## Features

- **CSV Parsing**: Reads appointment data from CSV files with resubmission handling
- **Smart Scheduling**: Automatic scheduling with priority-based slot assignment
- **Slot Stealing**: Advanced algorithm that can move players up to 5 levels deep to optimize assignments
- **Day-Specific Logic**: 
  - Construction Day: Prioritizes slot 49 for players who want research and have slot 1 available
  - Research Day: Automatically locks slot 1 for the player in Construction Day's slot 49
  - Troops Training Day: Standard priority-based scheduling
- **Web Interface**: 
  - Admin panel (password protected) for CSV upload
  - Statistics page showing alliance counts and time slot popularity (no player names)
  - Schedule display page for all three days
- **Modular Architecture**: Clean separation of concerns with dedicated modules for parsing, scheduling, and display

## Usage

### CLI Mode

```bash
cargo run
```

This will:
1. Process the CSV file (looks for `data/testData2.csv` or falls back to a hardcoded path)
2. Generate schedules for all three days
3. Print schedules to the terminal
4. Write schedule files: `schedule_construction.txt`, `schedule_research.txt`, `schedule_troops.txt`

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
│   ├── main.rs           # Main entry point (CLI and web server launcher)
│   ├── parser.rs         # CSV parsing and AppointmentEntry struct
│   ├── display.rs        # Terminal output and file writing functions
│   ├── web.rs            # Web server and API endpoints
│   └── schedule/         # Scheduling algorithm modules
│       ├── mod.rs        # Module declarations and public exports
│       ├── types.rs      # Data structures (ScheduledAppointment, DaySchedule, Move)
│       ├── slot_utils.rs # Slot conversion and ranking utilities
│       ├── move_chain.rs # Slot reassignment chain logic
│       ├── generic.rs    # Generic scheduling functions
│       ├── construction.rs # Construction day scheduler
│       ├── research.rs   # Research day scheduler
│       └── troops.rs     # Troops training day scheduler
├── templates/            # HTML templates
│   ├── index.html
│   ├── admin.html
│   ├── stats.html
│   └── schedules.html
├── static/               # Static assets
│   └── style.css
└── Cargo.toml
```

### Module Overview

- **`parser.rs`**: Handles CSV file parsing, time slot conversion, and data validation. Contains the `AppointmentEntry` struct that represents each player's appointment preferences.
- **`schedule/`**: Contains the scheduling algorithms, organized by function:
  - **`types.rs`**: Core data structures used throughout the scheduling system
  - **`slot_utils.rs`**: Utility functions for converting between time strings and slot numbers, and calculating slot popularity
  - **`move_chain.rs`**: Implements the slot "stealing" mechanism that can rearrange players up to 5 levels deep
  - **`generic.rs`**: Generic scheduling functions used by all day types
  - **`construction.rs`**: Specialized logic for Construction Day (handles slot 49 priority)
  - **`research.rs`**: Specialized logic for Research Day (handles locked slot 1 from Construction Day)
  - **`troops.rs`**: Simple wrapper for Troops Training Day scheduling
- **`display.rs`**: Handles all output formatting, including terminal display and file writing
- **`web.rs`**: Web server implementation using Actix-web, handles API endpoints and serves HTML pages

## Security Note

**Important**: Change the default admin password before deploying to production! Set the `ADMIN_PASSWORD` environment variable to a strong password.

